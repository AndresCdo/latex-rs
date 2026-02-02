mod api;
mod preview;
mod utils;

use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar, WindowTitle};
use gtk4::{
    glib, ScrolledWindow, Paned, Orientation, FileDialog
};
use sourceview5::prelude::*;
use sourceview5::{Buffer, View, LanguageManager};
use webkit6::prelude::*;
use webkit6::WebView;
use std::rc::Rc;
use std::path::PathBuf;
use std::cell::RefCell;
use crate::api::AiClient;
use crate::preview::Preview;
use crate::utils::{open_file, save_file};

const APP_ID: &str = "com.github.latex-rs";

#[tokio::main]
async fn main() -> glib::ExitCode {
    // Disable WebKit sandbox to prevent "bwrap: setting up uid map: Permission denied"
    // and "dbus-proxy" failures in some Linux environments (WSL, containers, etc.)
    std::env::set_var("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS", "1");

    // Initialize tracing for professional logging
    tracing_subscriber::fmt::init();
    
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    
    app.run()
}

struct AppState {
    current_file: Option<PathBuf>,
    ai_client: Option<AiClient>,
    preview_generator: Preview,
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(1200)
        .default_height(800)
        .title("LaTeX.rs Editor")
        .build();

    let toast_overlay = adw::ToastOverlay::new();
    window.set_content(Some(&toast_overlay));

    let content_box = gtk4::Box::new(Orientation::Vertical, 0);
    toast_overlay.set_child(Some(&content_box));

    // Header Bar
    let header_bar = HeaderBar::new();
    let view_title = WindowTitle::new("LaTeX.rs Editor", "");
    header_bar.set_title_widget(Some(&view_title));
    
    let open_btn = gtk4::Button::with_label("Open");
    let save_btn = gtk4::Button::with_label("Save");
    let ai_btn = gtk4::Button::with_label("AI Assistant");
    ai_btn.add_css_class("suggested-action");
    ai_btn.set_sensitive(false); // Enable only after check
    ai_btn.set_tooltip_text(Some("Checking Ollama status..."));

    let ai_spinner = gtk4::Spinner::new();
    ai_spinner.set_margin_end(8);

    header_bar.pack_start(&open_btn);
    header_bar.pack_start(&save_btn);
    header_bar.pack_end(&ai_btn);
    header_bar.pack_end(&ai_spinner);
    content_box.append(&header_bar);

    let state = Rc::new(RefCell::new(AppState {
        current_file: None,
        ai_client: None,
        preview_generator: Preview::new(),
    }));

    // Dependency check
    let missing_deps = crate::utils::check_dependencies();
    if !missing_deps.is_empty() {
        let msg = format!("Missing dependencies: {}. Preview may not work.", missing_deps.join(", "));
        toast_overlay.add_toast(adw::Toast::new(&msg));
        tracing::warn!(msg);
    }

    // AI Initialization Check
    let ctx = glib::MainContext::default();
    ctx.spawn_local(glib::clone!(#[strong] state, #[weak] ai_btn, async move {
        let models = ["qwen2.5-coder:3b", "llama3.2:3b", "llama3:8b", "mistral"];
        let mut final_client = None;

        for model_name in models {
            let client = AiClient::new(model_name).ok();
            if let Some(c) = client {
                if c.check_model().await.is_ok() {
                    final_client = Some(c);
                    break;
                }
            }
        }

        if let Some(client) = final_client {
            let model_name = client.model.clone();
            state.borrow_mut().ai_client = Some(client);
            ai_btn.set_sensitive(true);
            ai_btn.set_tooltip_text(Some(&format!("AI ready (Model: {})", model_name)));
            tracing::info!("AI Assistant initialized with model: {}", model_name);
        } else {
            ai_btn.set_tooltip_text(Some("Ollama not found or models missing. AI features disabled."));
            tracing::warn!("AI Assistant could not be initialized.");
        }
    }));

    // Editor & Preview
    let paned = Paned::new(Orientation::Horizontal);
    paned.set_hexpand(true);
    paned.set_vexpand(true);
    content_box.append(&paned);

    // Editor
    let lang_manager = LanguageManager::default();
    let lang = lang_manager.language("latex");
    let buffer = Buffer::new(None);
    buffer.set_language(lang.as_ref());
    buffer.set_highlight_syntax(true);
    buffer.set_enable_undo(true);

    let editor_view = View::with_buffer(&buffer);
    editor_view.set_monospace(true);
    editor_view.set_show_line_numbers(true);
    editor_view.set_highlight_current_line(true);
    
    let editor_scroll = ScrolledWindow::builder()
        .child(&editor_view)
        .hexpand(true)
        .vexpand(true)
        .build();
    paned.set_start_child(Some(&editor_scroll));

    // Preview
    let web_view = WebView::new();
    let preview_scroll = ScrolledWindow::builder()
        .child(&web_view)
        .hexpand(true)
        .vexpand(true)
        .build();
    paned.set_end_child(Some(&preview_scroll));

    // Logic: Live Preview
    let preview_gen = state.borrow().preview_generator.clone();
    let debounce_id = Rc::new(RefCell::new(None::<glib::SourceId>));

    buffer.connect_changed(glib::clone!(#[weak] web_view, #[strong] debounce_id, move |buf| {
        if let Some(source_id) = debounce_id.borrow_mut().take() {
            // Safely remove the source - it may have already fired, which is OK
            let _ = source_id.remove();
        }

        let text = crate::utils::buffer_to_string(buf.upcast_ref());
        let preview_gen = preview_gen.clone();
        
        let debounce_id_clone = debounce_id.clone();
        let source_id = glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
            // If this timer runs, it implies it wasn't cancelled.
            // We MUST clear the RefCell so subsequent keystrokes don't try to remove this already-dead source.
            *debounce_id_clone.borrow_mut() = None;

            let ctx = glib::MainContext::default();
            // Move blocking compilation out of the main thread
            ctx.spawn_local(async move {
                let html = tokio::task::spawn_blocking(move || {
                    preview_gen.render(&text)
                }).await.unwrap_or_else(|e| format!("Render Task Error: {}", e));
                
                web_view.load_html(&html, None);
            });
        });

        *debounce_id.borrow_mut() = Some(source_id);
    }));

    // Logic: Open
    open_btn.connect_clicked(glib::clone!(#[weak] window, #[weak] buffer, #[strong] state, #[weak] view_title, move |_| {
        let dialog = FileDialog::builder()
            .title("Open File")
            .build();

        dialog.open(Some(&window), None::<&gio::Cancellable>, glib::clone!(#[strong] state, #[weak] buffer, #[weak] view_title, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    if let Ok(content) = open_file(&path) {
                        buffer.set_text(&content);
                        state.borrow_mut().current_file = Some(path.to_path_buf());
                        view_title.set_subtitle(&path.to_string_lossy());
                    }
                }
            }
        }));
    }));

    // Logic: Save
    save_btn.connect_clicked(glib::clone!(#[weak] window, #[weak] buffer, #[strong] state, #[weak] view_title, move |_| {
        let path_opt = state.borrow().current_file.clone();
        if let Some(path) = path_opt {
             if let Err(e) = save_file(&path, buffer.upcast_ref()) {
                 tracing::error!("Failed to save: {}", e);
             }
        } else {
            let dialog = FileDialog::builder()
                .title("Save File")
                .build();
            
            dialog.save(Some(&window), None::<&gio::Cancellable>, glib::clone!(#[strong] state, #[weak] buffer, #[weak] view_title, move |res| {
                if let Ok(file) = res {
                    if let Some(path) = file.path() {
                        if save_file(&path, buffer.upcast_ref()).is_ok() {
                            state.borrow_mut().current_file = Some(path.to_path_buf());
                            view_title.set_subtitle(&path.to_string_lossy());
                        }
                    }
                }
            }));
        }
    }));

    // Logic: AI Assistant
    ai_btn.connect_clicked(glib::clone!(#[strong] state, #[weak] buffer, #[weak] ai_btn, #[weak] ai_spinner, #[weak] toast_overlay, move |_| {
        let text = crate::utils::buffer_to_string(buffer.upcast_ref());
        let client_opt = state.borrow().ai_client.clone();
        
        if let Some(client) = client_opt {
            ai_btn.set_sensitive(false);
            ai_spinner.start();

            let ctx = glib::MainContext::default();
            ctx.spawn_local(async move {
                let is_empty = text.trim().is_empty();
                
                let mut current_prompt = if is_empty {
                    "SYSTEM: You are a LaTeX expert. \
                    \nUSER: Generate a professional LaTeX starter template. \
                    \nCONSTRAINTS: \
                    \n1. Use 'article' class. \
                    \n2. NO external files, NO images, NO custom .bib files. \
                    \n3. Use only standard packages (amsmath, amssymb, amsthm, geometry). \
                    \n4. Output ONLY the LaTeX code inside a markdown block: ```latex ... ```. \
                    \n5. DO NOT provide any conversational text.".to_string()
                } else {
                    format!(
                        "SYSTEM: You are a LaTeX expert. \
                        \nUSER: Enhance this document. \
                        \nCONSTRAINTS: \
                        \n1. Return ONLY a unified diff (diff -u). \
                        \n2. Output the diff inside a markdown block: ```diff ... ```. \
                        \n3. DO NOT include any explanation. \
                        \n4. Start the diff with ---. \
                        \n\nDocument:\n{}", 
                        text
                    )
                };

                let mut attempts = 0;
                let max_attempts = if is_empty { 1 } else { 3 };
                let mut success = false;

                while attempts < max_attempts {
                    attempts += 1;
                    
                    if !is_empty && attempts == max_attempts {
                        current_prompt = format!(
                            "SYSTEM: The previous diff results were invalid. \
                            \nUSER: Provide the FULL ENHANCED LATEX DOCUMENT now. \
                            \nCONSTRAINTS: \
                            \n1. Output ONLY the LaTeX code inside a markdown block: ```latex ... ```. \
                            \n2. DO NOT provide any conversational text.\n\nDocument:\n{}", text
                        );
                    }

                    match client.send_prompt(&current_prompt).await {
                        Ok(response) => {
                            if is_empty || (attempts == max_attempts) {
                                // Final fallback or initial template: treat as full file
                                let cleaned = crate::utils::extract_latex(&response);
                                if !cleaned.is_empty() {
                                    buffer.set_text(&cleaned);
                                    tracing::info!("AI Assistant: Updated document (Full mode)");
                                    success = true;
                                }
                                break;
                            }

                            match crate::utils::apply_patch(&text, &response) {
                                Ok(new_text) => {
                                    if new_text == text {
                                        tracing::warn!("AI Assistant: Applied patch result is identical to original text");
                                    } else {
                                        tracing::info!("AI Assistant: Text changed! Original: {} chars, New: {} chars", text.len(), new_text.len());
                                        buffer.set_text(&new_text);
                                        tracing::info!("AI Assistant: Buffer updated successfully");
                                    }
                                    tracing::info!("AI Assistant: Successfully applied patch on attempt {}", attempts);
                                    success = true;
                                    break;
                                }
                                Err(e) => {
                                    tracing::error!("Patch Error (Attempt {}/{}): {}", attempts, max_attempts, e);
                                    if attempts < max_attempts {
                                        current_prompt = format!(
                                            "The unified diff was invalid: {}. \
                                            Please provide a valid unified diff (diff -u) now. \
                                            Make sure every line in the hunk starts with '+', '-', or ' '. \
                                            Do NOT use '...' or skip lines.\n\n\
                                            Document:\n{}",
                                            e, text
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("AI Connection Error: {}", e);
                            toast_overlay.add_toast(adw::Toast::new(&format!("AI Error: {}", e)));
                            break;
                        }
                    }
                }

                if !success && attempts >= max_attempts {
                    toast_overlay.add_toast(adw::Toast::new("AI failed to provide a valid update after multiple attempts."));
                }

                ai_btn.set_sensitive(true);
                ai_spinner.stop();
            });
        }
    }));

    window.present();
}
