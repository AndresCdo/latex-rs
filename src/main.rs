mod api;
mod config;
mod constants;
mod preview;
mod queue;
mod state;
mod ui;
mod utils;

use crate::api::{AiChunk, Message, MessageRole};
use crate::config::AppConfig;
use crate::constants::{
    AI_MAX_PATCH_ATTEMPTS, APP_ID, APP_NAME, DEFAULT_WINDOW_HEIGHT,
    DEFAULT_WINDOW_WIDTH, DEFAULT_ZOOM_LEVEL, WEBKIT_SANDBOX_DISABLE_VAR,
    WEBKIT_SANDBOX_DISABLE_VAR_MODERN, WSL_INTEROP_ENV,
};
use crate::preview::Preview;
use crate::state::AppState;
use crate::ui::{ai, editor, file_ops, header, layout, webview};
use adw::prelude::*;
use adw::{Application, ApplicationWindow};
use futures::StreamExt;
use gtk4::{gdk, glib, Box, Orientation};
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Detects if running in an environment that requires WebKit sandbox to be disabled.
/// Returns true for WSL, containers, or environments without proper namespace support.
fn needs_webkit_sandbox_disabled() -> bool {
    // Check for WSL (Windows Subsystem for Linux)
    if std::env::var(WSL_INTEROP_ENV).is_ok() {
        tracing::info!("WSL detected - WebKit sandbox will be disabled");
        return true;
    }

    // Check for systemd-nspawn or other container indicators
    if std::path::Path::new("/run/.containerenv").exists()
        || std::path::Path::new("/.dockerenv").exists()
        || std::path::Path::new("/.flatpak-info").exists()
    {
        tracing::info!("Container/Flatpak marker file detected - WebKit sandbox will be disabled");
        return true;
    }

    // Check for Snap packages
    if std::env::var("SNAP").is_ok() {
        tracing::info!("Snap environment detected - WebKit sandbox will be disabled");
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
            tracing::info!(
                "Unprivileged user namespaces are disabled - WebKit sandbox will be disabled"
            );
            return true;
        }
    }

    // Check for Ubuntu 24.04+ AppArmor user namespace restrictions
    if let Ok(content) =
        std::fs::read_to_string("/proc/sys/kernel/apparmor_restrict_unprivileged_userns")
    {
        if content.trim() == "1" {
            tracing::info!(
                "AppArmor user namespace restrictions detected - WebKit sandbox will be disabled"
            );
            return true;
        }
    }

    // Check cgroups for container indicators
    if let Ok(cgroups) = std::fs::read_to_string("/proc/1/cgroup") {
        if cgroups.contains("docker") || cgroups.contains("kubepods") || cgroups.contains("lxc") {
            tracing::info!(
                "Cgroups indicate container environment - WebKit sandbox will be disabled"
            );
            return true;
        }
    }

    false
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    // Initialize tracing for professional logging
    tracing_subscriber::fmt::init();

    // Conditionally disable WebKit sandbox only in environments that require it
    // (WSL, containers, etc.) to prevent "bwrap: setting up uid map: Permission denied"
    if needs_webkit_sandbox_disabled() {
        // SAFETY: This is set early in main before any threads are spawned
        unsafe {
            std::env::set_var(WEBKIT_SANDBOX_DISABLE_VAR, "1");
            std::env::set_var(WEBKIT_SANDBOX_DISABLE_VAR_MODERN, "1");
        }
        tracing::warn!(
            "WebKit sandbox disabled. This reduces security but is required in some environments."
        );
    }

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
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

    let main_vbox = Box::new(Orientation::Vertical, 0);
    toast_overlay.set_child(Some(&main_vbox));

    // Custom CSS for better UI
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(
        "
        .dim-label { opacity: 0.7; font-size: 0.9em; }
        .sidebar { border-right: 1px solid alpha(@borders, 0.5); background: @view_bg_color; }
        .linked button { border-radius: 0; }
        .linked button:first-child { border-top-left-radius: 6px; border-bottom-left-radius: 6px; }
        .linked button:last-child { border-top-right-radius: 6px; border-bottom-right-radius: 6px; }
    ",
    );
    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Header Bar
    let (header_bar, view_title, new_btn, open_btn, save_btn, export_btn, settings_btn, ai_btn, sidebar_toggle) =
        header::create_header_bar();
    main_vbox.append(&header_bar);

    // AI Prompt Entry (Revealer)
    let (ai_revealer, ai_entry, ai_spinner, ai_run_btn, reasoning_revealer, reasoning_label) = ai::create_ai_panel();
    main_vbox.append(&ai_revealer);

    // Sidebar & Content Split
    let (
        _outer_paned,
        paned,
        sidebar_list,
        sidebar_scroll,
        _status_bar,
        pos_label,
        word_count_label,
        ai_status_label,
    ) = layout::create_main_layout(&main_vbox);

    let config = AppConfig::load();

    let state = Rc::new(RefCell::new(AppState {
        current_file: None,
        ai_provider: None,
        config,
        preview_generator: Preview::new(),
        editor_zoom: DEFAULT_ZOOM_LEVEL,
        preview_zoom: DEFAULT_ZOOM_LEVEL,
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

    let validate_ai = Rc::new(glib::clone!(
        #[strong]
        state,
        #[weak]
        ai_btn,
        #[weak]
        ai_status_label,
        move || {
            let ctx = glib::MainContext::default();
            ctx.spawn_local(glib::clone!(
                #[strong]
                state,
                #[weak]
                ai_btn,
                #[weak]
                ai_status_label,
                async move {
                    ai_btn.set_sensitive(false);
                    ai_status_label.set_text("AI: Initializing...");

                    let config = state.borrow().config.clone();
                    let active_config = config.get_active_provider();

                    if let Some(p_config) = active_config {
                        let provider = crate::api::create_provider(p_config);
                        match provider.check_availability().await {
                            Ok(_) => {
                                let name = provider.name().to_string();
                                let model = p_config.active_model.clone();
                                state.borrow_mut().ai_provider = Some(provider);
                                ai_btn.set_sensitive(true);
                                ai_btn.set_tooltip_text(Some(&format!(
                                    "AI ready (Provider: {}, Model: {})",
                                    name, model
                                )));
                                ai_status_label.set_text(&format!("AI: Ready ({})", model));
                                tracing::info!(
                                    "AI Assistant initialized: {} with model {}",
                                    name,
                                    model
                                );
                            }
                            Err(e) => {
                                ai_btn.set_tooltip_text(Some(&format!(
                                    "AI provider unavailable: {}. Check settings.",
                                    e
                                )));
                                ai_status_label.set_text("AI: Unavailable");
                                tracing::error!("AI check failed: {}", e);
                            }
                        }
                    } else {
                        ai_status_label.set_text("AI: Not Configured");
                    }
                }
            ));
        }
    ));

    // AI Initialization Check
    validate_ai();

    // Sidebar logic
    sidebar_toggle.connect_active_notify(glib::clone!(
        #[weak]
        sidebar_scroll,
        move |btn| {
            sidebar_scroll.set_visible(btn.is_active());
        }
    ));

    // Editor
    let style_manager = adw::StyleManager::default();
    let (buffer, editor_view, editor_scroll) = editor::create_editor(&style_manager);

    // Search Bar
    let (search_revealer, search_entry) = editor::create_search_bar();

    let editor_container = Box::new(Orientation::Vertical, 0);
    editor_container.append(&search_revealer);
    editor_container.append(&editor_scroll);
    paned.set_start_child(Some(&editor_container));

    // Preview
    let (web_view, preview_scroll) = webview::create_preview();
    paned.set_end_child(Some(&preview_scroll));

    // Search Logic
    let search_settings = sourceview5::SearchSettings::new();
    let search_context = sourceview5::SearchContext::new(&buffer, Some(&search_settings));
    search_context.set_highlight(true);

    search_entry.connect_search_changed(glib::clone!(
        #[weak]
        search_settings,
        move |entry| {
            let text = entry.text();
            if text.is_empty() {
                search_settings.set_search_text(None::<&str>);
            } else {
                search_settings.set_search_text(Some(text.as_str()));
            }
        }
    ));

    search_entry.connect_next_match(glib::clone!(
        #[weak]
        search_context,
        #[weak]
        editor_view,
        #[weak]
        buffer,
        move |_| {
            let buf = buffer.upcast_ref::<gtk4::TextBuffer>();
            let iter = if let Some(cursor_mark) = buf.mark("insert") {
                buf.iter_at_mark(&cursor_mark)
            } else {
                buf.start_iter()
            };
            if let Some((start, end, _)) = search_context.forward(&iter) {
                buf.select_range(&start, &end);
                editor_view.scroll_to_iter(&mut start.clone(), 0.0, false, 0.5, 0.5);
            }
        }
    ));

    search_entry.connect_previous_match(glib::clone!(
        #[weak]
        search_context,
        #[weak]
        editor_view,
        #[weak]
        buffer,
        move |_| {
            let buf = buffer.upcast_ref::<gtk4::TextBuffer>();
            let iter = if let Some(cursor_mark) = buf.mark("insert") {
                buf.iter_at_mark(&cursor_mark)
            } else {
                buf.start_iter()
            };
            if let Some((start, end, _)) = search_context.backward(&iter) {
                buf.select_range(&start, &end);
                editor_view.scroll_to_iter(&mut start.clone(), 0.0, false, 0.5, 0.5);
            }
        }
    ));

    // Zoom handlers
    editor::connect_zoom_handlers(
        &window,
        state.clone(),
        &editor_view,
        &editor_scroll,
        &search_revealer,
        &search_entry,
        &web_view,
    );
    editor::connect_sidebar_activation(&sidebar_list, &buffer, &editor_view);

    // Live preview handler
    webview::connect_live_preview(
        &buffer,
        &web_view,
        &sidebar_list,
        state.clone(),
        &toast_overlay,
    );

    // Export PDF handler
    file_ops::connect_export_pdf(&export_btn, &window, &buffer, state.clone(), &toast_overlay);

    // File operations and status bar
    file_ops::connect_file_operations(
        &new_btn,
        &open_btn,
        &save_btn,
        &window,
        &buffer,
        state.clone(),
        &view_title,
        &pos_label,
        &word_count_label,
    );

    // AI Assistant Toggle
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

    settings_btn.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[weak]
        window,
        #[strong]
        validate_ai,
        move |_| {
            ui::settings::show_settings(
                window.upcast_ref(),
                state.clone(),
                Some(validate_ai.clone()),
            );
        }
    ));

    // Common AI logic
    let trigger_ai = Rc::new(glib::clone!(
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
        #[weak]
        ai_status_label,
        #[weak]
        reasoning_revealer,
        #[weak]
        reasoning_label,
        move || {
            let user_instruction = ai_entry.text().to_string();
            let text = crate::utils::buffer_to_string(buffer.upcast_ref());
            let provider_opt = state.borrow().ai_provider.clone();

            if let Some(provider) = provider_opt {
                ai_run_btn.set_sensitive(false);
                ai_spinner.start();
                ai_status_label.set_text("AI: Thinking...");
                reasoning_revealer.set_reveal_child(false);

                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let is_empty = text.trim().is_empty();
                    let use_full_document = is_empty || text.len() < 20000;

                    let mut messages = vec![
                        Message {
                            role: MessageRole::System,
                            content: "You are an expert LaTeX assistant. Your goal is to help users write, fix, and enhance LaTeX documents. \
                                      \n\nCORE RULES:\n\
                                      - Output ONLY the LaTeX code or requested data.\n\
                                      - Use markdown blocks: ```latex ... ``` for full documents or ```diff ... ``` for updates.\n\
                                      - Do NOT include conversational text, greetings, or explanations.\n\
                                      - ALWAYS start full documents with '\\documentclass{article}'.\n\
                                      - NEVER use '\\documentclass{amsmath}' or other package names as classes.\n\
                                      - Use standard packages: amsmath, amssymb, amsthm, geometry.\n\
                                      - NO external dependencies, NO local images, NO custom .bib files.\n\
                                      - If you define theorems, include '\\newtheorem{theorem}{Theorem}' in the preamble.".to_string(),
                        }
                    ];

                    if use_full_document {
                        let user_prompt = if is_empty {
                            if user_instruction.is_empty() {
                                "Generate a professional LaTeX starter template using '\\documentclass{article}'.".to_string()
                            } else {
                                format!("Generate a professional LaTeX starter template using '\\documentclass{{article}}' based on: {}", user_instruction)
                            }
                        } else {
                            format!("Update the following LaTeX document (using '\\documentclass{{article}}') based on these instructions: {}\n\nDocument:\n{}", user_instruction, text)
                        };

                        messages.push(Message {
                            role: MessageRole::User,
                            content: user_prompt,
                        });
                    } else {
                        messages.push(Message {
                            role: MessageRole::User,
                            content: format!(
                                "Update this document based on: {}\nReturn ONLY a unified diff (diff -u) in a ```diff``` block.\n\nDocument:\n{}",
                                user_instruction, text
                            ),
                        });
                    }

                    let mut success = false;
                    let mut attempts = 0;
                    let max_attempts = AI_MAX_PATCH_ATTEMPTS;

                    while attempts < max_attempts {
                        attempts += 1;

                        match provider.chat_stream(messages.clone()).await {
                            Ok(mut stream) => {
                                let mut full_content = String::new();
                                let mut full_reasoning = String::new();

                                while let Some(chunk_result) = stream.next().await {
                                    match chunk_result {
                                        Ok(chunk) => {
                                            match chunk {
                                                AiChunk::Content(c) => {
                                                    full_content.push_str(&c);
                                                    // We don't update buffer in real-time for diff/full extraction logic yet,
                                                    // but we could show a preview or just wait for completion.
                                                    // For now, let's just collect it.
                                                }
                                                AiChunk::Reasoning(r) => {
                                                    full_reasoning.push_str(&r);
                                                    reasoning_label.set_text(&full_reasoning);
                                                    reasoning_revealer.set_reveal_child(true);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Stream error: {}", e);
                                            break;
                                        }
                                    }
                                }

                                if use_full_document {
                                    let cleaned = crate::utils::extract_latex(&full_content);
                                    if !cleaned.is_empty() {
                                        if cleaned == text {
                                            tracing::warn!("AI returned identical content");
                                            toast_overlay.add_toast(adw::Toast::new("AI suggests no changes needed."));
                                            success = true;
                                        } else {
                                            buffer.set_text(&cleaned);
                                            tracing::info!("AI Assistant: Document updated (Full Mode)");
                                            success = true;
                                        }
                                    }
                                } else {
                                    match crate::utils::apply_patch(&text, &full_content) {
                                        Ok(new_text) => {
                                            if new_text == text {
                                                tracing::warn!("AI diff resulted in no change");
                                                toast_overlay.add_toast(adw::Toast::new("AI diff suggested no changes."));
                                            } else {
                                                buffer.set_text(&new_text);
                                                tracing::info!("AI Assistant: Document updated (Diff Mode)");
                                            }
                                            success = true;
                                        }
                                        Err(e) => {
                                            tracing::error!("Patch failed (Attempt {}): {}", attempts, e);
                                            if attempts < max_attempts {
                                                messages.push(Message { role: MessageRole::Assistant, content: full_content });
                                                messages.push(Message {
                                                    role: MessageRole::User,
                                                    content: "That diff was invalid. Provide the FULL enhanced LaTeX document instead within a ```latex``` block.".to_string()
                                                });
                                                continue;
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                            Err(e) => {
                                tracing::error!("AI Connection Error: {}", e);
                                toast_overlay.add_toast(adw::Toast::new(&format!("AI Error: {}", e)));
                                break;
                            }
                        }
                    }

                    if !success {
                        toast_overlay.add_toast(adw::Toast::new("AI failed to provide a valid update."));
                    } else {
                        ai_entry.set_text("");
                        if reasoning_label.text().is_empty() {
                            ai_revealer.set_reveal_child(false);
                        }
                    }

                    ai_run_btn.set_sensitive(true);
                    ai_spinner.stop();
                    ai_status_label.set_text("AI: Ready");
                });
            }
        }
    ));

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
