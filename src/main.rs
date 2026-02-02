mod api;
mod constants;
mod preview;
mod queue;
mod utils;

use crate::api::AiClient;
use crate::constants::{
    AI_MAX_PATCH_ATTEMPTS, AI_MODEL_PRIORITY, APP_ID, APP_NAME, CONTAINER_ENV,
    DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, PREVIEW_DEBOUNCE_MS, WEBKIT_SANDBOX_DISABLE_VAR,
    WSL_INTEROP_ENV,
};
use crate::preview::Preview;
use crate::queue::CompilationQueue;
use crate::utils::{open_file, save_file};
use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar, WindowTitle};
use gtk4::{
    glib, Box, Button, Entry, FileDialog, Orientation, Paned, Revealer, RevealerTransitionType,
    ScrolledWindow, Spinner,
};
use sourceview5::prelude::*;
use sourceview5::{Buffer, LanguageManager, View};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use webkit6::prelude::*;
use webkit6::WebView;

/// Detects if running in an environment that requires WebKit sandbox to be disabled.
/// Returns true for WSL, containers, or environments without proper namespace support.
fn needs_webkit_sandbox_disabled() -> bool {
    // Check for WSL (Windows Subsystem for Linux)
    if std::env::var(WSL_INTEROP_ENV).is_ok() {
        tracing::info!("WSL detected - WebKit sandbox will be disabled");
        return true;
    }

    // Check for container runtime (Docker, Podman, etc.)
    if std::env::var(CONTAINER_ENV).is_ok() {
        tracing::info!("Container environment detected - WebKit sandbox will be disabled");
        return true;
    }

    // Check for systemd-nspawn or other container indicators
    if std::path::Path::new("/run/.containerenv").exists()
        || std::path::Path::new("/.dockerenv").exists()
    {
        tracing::info!("Container marker file detected - WebKit sandbox will be disabled");
        return true;
    }

    // Check if user explicitly requested sandbox disable via environment
    if std::env::var(WEBKIT_SANDBOX_DISABLE_VAR).is_ok() {
        tracing::warn!("WebKit sandbox explicitly disabled by user environment variable");
        return true;
    }

    // Check /proc/version for WSL indicators (reliable fallback if env vars are missing)
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        let version_lower = version.to_lowercase();
        if version_lower.contains("microsoft") || version_lower.contains("wsl") {
             tracing::info!("/proc/version indicates WSL - WebKit sandbox will be disabled");
             return true;
        }
    }

    // Check for disabled user namespaces (common on Debian/Arch)
    if let Ok(content) = std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone") {
        if content.trim() == "0" {
            tracing::info!("Unprivileged user namespaces are disabled - WebKit sandbox will be disabled");
            return true;
        }
    }

    // Check cgroups for container indicators
    if let Ok(cgroups) = std::fs::read_to_string("/proc/1/cgroup") {
         if cgroups.contains("docker") || cgroups.contains("kubepods") || cgroups.contains("lxc") {
             tracing::info!("Cgroups indicate container environment - WebKit sandbox will be disabled");
             return true;
         }
    }

    false
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    // Conditionally disable WebKit sandbox only in environments that require it
    // (WSL, containers, etc.) to prevent "bwrap: setting up uid map: Permission denied"
    if needs_webkit_sandbox_disabled() {
        // SAFETY: This is set early in main before any threads are spawned
        unsafe {
            std::env::set_var(WEBKIT_SANDBOX_DISABLE_VAR, "1");
        }
        tracing::warn!(
            "WebKit sandbox disabled. This reduces security but is required in some environments."
        );
    }

    // Initialize tracing for professional logging
    tracing_subscriber::fmt::init();

    let app = Application::builder().application_id(APP_ID).build();

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
        .default_width(DEFAULT_WINDOW_WIDTH)
        .default_height(DEFAULT_WINDOW_HEIGHT)
        .title(APP_NAME)
        .build();

    let toast_overlay = adw::ToastOverlay::new();
    window.set_content(Some(&toast_overlay));

    let content_box = Box::new(Orientation::Vertical, 0);
    toast_overlay.set_child(Some(&content_box));

    // Header Bar
    let header_bar = HeaderBar::new();
    let view_title = WindowTitle::new("LaTeX.rs Editor", "");
    header_bar.set_title_widget(Some(&view_title));

    let open_btn = Button::with_label("Open");
    let save_btn = Button::with_label("Save");
    let ai_btn = Button::with_label("AI Assistant");
    ai_btn.add_css_class("suggested-action");
    ai_btn.set_sensitive(false); // Enable only after check
    ai_btn.set_tooltip_text(Some("Checking Ollama status..."));

    let ai_spinner = Spinner::new();
    ai_spinner.set_margin_end(8);

    header_bar.pack_start(&open_btn);
    header_bar.pack_start(&save_btn);
    header_bar.pack_end(&ai_btn);
    header_bar.pack_end(&ai_spinner);
    content_box.append(&header_bar);

    // AI Prompt Entry (Revealer)
    let ai_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .build();

    let ai_entry_box = Box::new(Orientation::Horizontal, 6);
    ai_entry_box.set_margin_start(12);
    ai_entry_box.set_margin_end(12);
    ai_entry_box.set_margin_top(6);
    ai_entry_box.set_margin_bottom(6);

    let ai_entry = Entry::builder()
        .placeholder_text("Tell AI what to do (e.g., 'Add a table of contents', 'Fix grammar')...")
        .hexpand(true)
        .build();

    let ai_run_btn = Button::with_label("Generate");
    ai_run_btn.add_css_class("suggested-action");

    ai_entry_box.append(&ai_entry);
    ai_entry_box.append(&ai_run_btn);
    ai_revealer.set_child(Some(&ai_entry_box));
    content_box.append(&ai_revealer);

    let state = Rc::new(RefCell::new(AppState {
        current_file: None,
        ai_client: None,
        preview_generator: Preview::new(),
    }));

    // Dependency check
    let missing_deps = crate::utils::check_dependencies();
    if !missing_deps.is_empty() {
        let msg = format!(
            "Missing dependencies: {}. Preview may not work.",
            missing_deps.join(", ")
        );
        toast_overlay.add_toast(adw::Toast::new(&msg));
        tracing::warn!(msg);
    }

    // AI Initialization Check
    let ctx = glib::MainContext::default();
    ctx.spawn_local(glib::clone!(
        #[strong]
        state,
        #[weak]
        ai_btn,
        async move {
            let mut final_client = None;

            for model_name in AI_MODEL_PRIORITY {
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
                ai_btn.set_tooltip_text(Some(
                    "Ollama not found or models missing. AI features disabled.",
                ));
                tracing::warn!("AI Assistant could not be initialized.");
            }
        }
    ));

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
    let queue = CompilationQueue::new(preview_gen.clone());
    let debounce_id = Rc::new(RefCell::new(None::<glib::SourceId>));

    buffer.connect_changed(glib::clone!(
        #[weak]
        web_view,
        #[strong]
        debounce_id,
        move |buf| {
            if let Some(source_id) = debounce_id.borrow_mut().take() {
                // Safely remove the source - it may have already fired, which is OK
                source_id.remove();
            }

            let text = crate::utils::buffer_to_string(buf.upcast_ref());
            let queue = queue.clone();

            let debounce_id_clone = debounce_id.clone();
            let queue_clone = queue.clone();
            let source_id =
                glib::timeout_add_local_once(std::time::Duration::from_millis(PREVIEW_DEBOUNCE_MS), move || {
                    // If this timer runs, it implies it wasn't cancelled.
                    // We MUST clear the RefCell so subsequent keystrokes don't try to remove this already-dead source.
                    *debounce_id_clone.borrow_mut() = None;

                    let ctx = glib::MainContext::default();
                    // Move blocking compilation out of the main thread
                    ctx.spawn_local(async move {
                        let html = queue_clone.enqueue(text).await
                            .unwrap_or_else(|| "Compilation cancelled".to_string());
                        web_view.load_html(&html, None);
                    });
                });

            *debounce_id.borrow_mut() = Some(source_id);
        }
    ));

    // Logic: Open
    open_btn.connect_clicked(glib::clone!(
        #[weak]
        window,
        #[weak]
        buffer,
        #[strong]
        state,
        #[weak]
        view_title,
        move |_| {
            let dialog = FileDialog::builder().title("Open File").build();

            dialog.open(
                Some(&window),
                None::<&gio::Cancellable>,
                glib::clone!(
                    #[strong]
                    state,
                    #[weak]
                    buffer,
                    #[weak]
                    view_title,
                    move |res| {
                        if let Ok(file) = res {
                            if let Some(path) = file.path() {
                                if let Ok(content) = open_file(&path) {
                                    buffer.set_text(&content);
                                    state.borrow_mut().current_file = Some(path.to_path_buf());
                                    view_title.set_subtitle(&path.to_string_lossy());
                                }
                            }
                        }
                    }
                ),
            );
        }
    ));

    // Logic: Save
    save_btn.connect_clicked(glib::clone!(
        #[weak]
        window,
        #[weak]
        buffer,
        #[strong]
        state,
        #[weak]
        view_title,
        move |_| {
            let path_opt = state.borrow().current_file.clone();
            if let Some(path) = path_opt {
                if let Err(e) = save_file(&path, buffer.upcast_ref()) {
                    tracing::error!("Failed to save: {}", e);
                }
            } else {
                let dialog = FileDialog::builder().title("Save File").build();

                dialog.save(
                    Some(&window),
                    None::<&gio::Cancellable>,
                    glib::clone!(
                        #[strong]
                        state,
                        #[weak]
                        buffer,
                        #[weak]
                        view_title,
                        move |res| {
                            if let Ok(file) = res {
                                if let Some(path) = file.path() {
                                    if save_file(&path, buffer.upcast_ref()).is_ok() {
                                        state.borrow_mut().current_file = Some(path.to_path_buf());
                                        view_title.set_subtitle(&path.to_string_lossy());
                                    }
                                }
                            }
                        }
                    ),
                );
            }
        }
    ));

    // Logic: AI Assistant Toggle
    ai_btn.connect_clicked(glib::clone!(
        #[weak]
        ai_revealer,
        #[weak]
        ai_entry,
        move |_| {
            let is_revealed = ai_revealer.reveals_child();
            ai_revealer.set_reveal_child(!is_revealed);
            if !is_revealed {
                ai_entry.grab_focus();
            }
        }
    ));

    // Common AI logic
    let trigger_ai = glib::clone!(
        #[strong]
        state,
        #[weak]
        buffer,
        #[weak]
        ai_run_btn,
        #[weak]
        ai_spinner,
        #[weak]
        toast_overlay,
        #[weak]
        ai_entry,
        #[weak]
        ai_revealer,
        move || {
            let user_instruction = ai_entry.text().to_string();
            let text = crate::utils::buffer_to_string(buffer.upcast_ref());
            let client_opt = state.borrow().ai_client.clone();

            if let Some(client) = client_opt {
                ai_run_btn.set_sensitive(false);
                ai_spinner.start();

                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let is_empty = text.trim().is_empty();

                    let mut current_prompt = if is_empty {
                        let user_suffix = if user_instruction.is_empty() {
                            "Generate a professional LaTeX starter template.".to_string()
                        } else {
                            format!(
                                "Generate a professional LaTeX starter template based on these instructions: {}",
                                user_instruction
                            )
                        };
                        format!(
                            "SYSTEM: You are a LaTeX expert. \
                            \nUSER: {} \
                            \nCONSTRAINTS: \
                            \n1. Use 'article' class. \
                            \n2. NO external files, NO images, NO custom .bib files. \
                            \n3. Use standard packages (amsmath, amssymb, amsthm, geometry). \
                            \n4. If you use theorems/lemmas, include '\\newtheorem{{theorem}}{{Theorem}}' in the preamble. \
                            \n5. Output ONLY the LaTeX code inside a markdown block: ```latex ... ```. \
                            \n6. DO NOT provide any conversational text.",
                            user_suffix
                        )
                    } else {
                        let user_suffix = if user_instruction.is_empty() {
                            "Enhance this document.".to_string()
                        } else {
                            format!(
                                "Enhance this document based on these instructions: {}",
                                user_instruction
                            )
                        };
                        format!(
                            "SYSTEM: You are a LaTeX expert. \
                            \nUSER: {} \
                            \nCONSTRAINTS: \
                            \n1. Return ONLY a unified diff (diff -u). \
                            \n2. Output the diff inside a markdown block: ```diff ... ```. \
                            \n3. DO NOT include any explanation. \
                            \n4. Start the diff with ---. \
                            \n\nDocument:\n{}",
                            user_suffix, text
                        )
                    };

                    let mut attempts = 0;
                    let max_attempts = if is_empty { 1 } else { AI_MAX_PATCH_ATTEMPTS };
                    let mut success = false;

                    while attempts < max_attempts {
                        attempts += 1;

                        if !is_empty && attempts == max_attempts {
                            current_prompt = format!(
                                "SYSTEM: The previous diff results were invalid. \
                                \nUSER: Provide the FULL ENHANCED LATEX DOCUMENT now. \
                                \nCONSTRAINTS: \
                                \n1. Output ONLY the LaTeX code inside a markdown block: ```latex ... ```. \
                                \n2. DO NOT provide any conversational text.\n\nDocument:\n{}",
                                text
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
                                            tracing::info!(
                                                "AI Assistant: Text changed! Original: {} chars, New: {} chars",
                                                text.len(),
                                                new_text.len()
                                            );
                                            buffer.set_text(&new_text);
                                            tracing::info!("AI Assistant: Buffer updated successfully");
                                        }
                                        tracing::info!(
                                            "AI Assistant: Successfully applied patch on attempt {}",
                                            attempts
                                        );
                                        success = true;
                                        break;
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Patch Error (Attempt {}/{}): {}",
                                            attempts,
                                            max_attempts,
                                            e
                                        );
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
                        toast_overlay.add_toast(adw::Toast::new(
                            "AI failed to provide a valid update after multiple attempts.",
                        ));
                    }

                    if success {
                        ai_entry.set_text("");
                        ai_revealer.set_reveal_child(false);
                    }

                    ai_run_btn.set_sensitive(true);
                    ai_spinner.stop();
                });
            }
        }
    );

    ai_run_btn.connect_clicked(glib::clone!(
        #[strong]
        trigger_ai,
        move |_| {
            trigger_ai();
        }
    ));

    ai_entry.connect_activate(glib::clone!(
        #[strong]
        trigger_ai,
        move |_| {
            trigger_ai();
        }
    ));

    window.present();
}
