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
    APP_ID, APP_NAME, DEFAULT_WINDOW_HEIGHT,
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
    let (
        ai_revealer,
        ai_entry,
        ai_spinner,
        ai_run_btn,
        reasoning_revealer,
        reasoning_view,
        suggestion_revealer,
        accept_btn,
        reject_btn,
        clear_btn,
    ) = ai::create_ai_panel();
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
        ai_cancellation: None,
        is_ai_generating: false,
        pending_suggestion: None,
        original_text_selection: None,
        config,
        preview_generator: Preview::new(),
        editor_zoom: DEFAULT_ZOOM_LEVEL,
        preview_zoom: DEFAULT_ZOOM_LEVEL,
        ai_history: Vec::new(),
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
    let ai_history_index: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

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
            crate::ui::settings::show_settings(window.upcast_ref(), state.clone(), Some(validate_ai.clone()));
        }
    ));

    let ai_entry_weak = ai_entry.downgrade();

    let trigger_ai = {
        let state = state.clone();
        let buffer = buffer.downgrade();
        let ai_run_btn = ai_run_btn.downgrade();
        let ai_spinner = ai_spinner.downgrade();
        let ai_entry = ai_entry.downgrade();
        let ai_status_label = ai_status_label.downgrade();
        let reasoning_revealer = reasoning_revealer.downgrade();
        let reasoning_view = reasoning_view.downgrade();
        let suggestion_revealer = suggestion_revealer.downgrade();
        let editor_view = editor_view.downgrade();

        move || {
            let ai_entry = if let Some(e) = ai_entry.upgrade() { e } else { return };
            let ai_buffer = ai_entry.buffer();
            let user_instruction = ai_buffer.text(&ai_buffer.start_iter(), &ai_buffer.end_iter(), false).to_string();
            if user_instruction.trim().is_empty() {
                return;
            }
            
            // Add to history if not duplicate of last
            {
                let mut s = state.borrow_mut();
                if s.ai_history.last() != Some(&user_instruction) {
                    s.ai_history.push(user_instruction.clone());
                }
            }

            let buffer = if let Some(b) = buffer.upgrade() { b } else { return };
            let ai_run_btn = if let Some(b) = ai_run_btn.upgrade() { b } else { return };
            let ai_spinner = if let Some(s) = ai_spinner.upgrade() { s } else { return };
            let ai_status_label = if let Some(l) = ai_status_label.upgrade() { l } else { return };
            let reasoning_revealer = if let Some(r) = reasoning_revealer.upgrade() { r } else { return };
            let reasoning_view = if let Some(v) = reasoning_view.upgrade() { v } else { return };
            let suggestion_revealer = if let Some(r) = suggestion_revealer.upgrade() { r } else { return };
            let editor_view = if let Some(v) = editor_view.upgrade() { v } else { return };

            let (start, end) = buffer.selection_bounds().unwrap_or_else(|| {
                let cursor = buffer.iter_at_mark(&buffer.mark("insert").unwrap());
                let mut s = cursor.clone();
                s.backward_visible_lines(5);
                let mut e = cursor.clone();
                e.forward_visible_lines(5);
                (s, e)
            });
            
            let selected_text = buffer.text(&start, &end, false).to_string();
            let provider_opt = state.borrow().ai_provider.clone();

            if let Some(provider) = provider_opt {
                // Cancel any existing generation
                if let Some(cancel) = state.borrow_mut().ai_cancellation.take() {
                    let _ = cancel.try_send(());
                }

                let (tx, mut rx) = tokio::sync::mpsc::channel(1);
                {
                    let mut s = state.borrow_mut();
                    s.ai_cancellation = Some(tx);
                    s.is_ai_generating = true;
                    s.pending_suggestion = None;
                    s.original_text_selection = Some(selected_text.clone());
                }

                ai_run_btn.set_sensitive(true);
                ai_run_btn.set_label("Stop");
                ai_run_btn.set_icon_name("process-stop-symbolic");
                ai_run_btn.add_css_class("destructive-action");
                ai_run_btn.remove_css_class("suggested-action");

                ai_spinner.start();
                ai_status_label.set_text("AI: Thinking...");
                reasoning_view.buffer().set_text("");
                reasoning_revealer.set_reveal_child(false);
                suggestion_revealer.set_reveal_child(false);
                
                // Disable editing while generating
                editor_view.set_editable(false);

                let ctx = glib::MainContext::default();
                ctx.spawn_local(glib::clone!(
                    #[strong]
                    state,
                    #[weak]
                    ai_run_btn,
                    #[weak]
                    ai_spinner,
                    #[weak]
                    ai_status_label,
                    #[weak]
                    buffer,
                    #[weak]
                    suggestion_revealer,
                    #[weak]
                    reasoning_revealer,
                    #[weak]
                    reasoning_view,
                    #[weak]
                    editor_view,
                    async move {
                        let system_prompt = state.borrow().config.get_active_provider()
                            .and_then(|p| p.system_prompt.clone())
                            .unwrap_or_else(|| "You are an expert LaTeX assistant. Your goal is to help users edit specific sections of their LaTeX documents. \
                                              \n\nCORE RULES:\n\
                                              - Output ONLY the modified LaTeX code for the provided snippet.\n\
                                              - Do NOT include markdown blocks like ```latex.\n\
                                              - Do NOT include conversational text, greetings, or explanations.\n\
                                              - If the user provides a small snippet, assume they want to edit it or add something relative to it.\n\
                                              - If adding a new environment (table, figure, etc.), ensure it is properly closed.\n\
                                              - Use ONLY standard LaTeX commands (article class). Avoid hallucinated commands like \\keywords (use \\paragraph{Keywords:} instead).\n\
                                              - Maintain the context of the surrounding code if applicable.".to_string());

                        let messages = vec![
                            Message {
                                role: MessageRole::System,
                                content: system_prompt,
                            },
                            Message {
                                role: MessageRole::User,
                                content: format!("Edit the following LaTeX snippet based on these instructions: {}\n\nSnippet:\n{}", user_instruction, selected_text),
                            }
                        ];

                        let mut full_content = String::new();
                        let mut full_reasoning = String::new();
                        let mut success = false;
                        let mut ai_started_typing = false;
                        let mut filter = crate::api::ThinkingFilter::new();

                        // buffer is already sourceview5::Buffer here because it was upgraded in trigger_ai?
                        // If it's not Option, then don't match it as Option.
                        
                        let reasoning_view = reasoning_view.downgrade();
                        let reasoning_revealer = reasoning_revealer.downgrade();
                        let ai_status_label = ai_status_label.downgrade();
                        let suggestion_revealer = suggestion_revealer.downgrade();
                        let ai_run_btn = ai_run_btn.downgrade();
                        let ai_spinner = ai_spinner.downgrade();
                        let editor_view = editor_view.downgrade();

                        // Create marks to track the start and end of the AI insertion
                        let start_mark = buffer.create_mark(None, &start, true);
                        let curr_mark = buffer.create_mark(None, &start, false);
                        let end_mark = buffer.create_mark(None, &end, false);

                        match provider.chat_stream(messages).await {
                            Ok(mut stream) => {
                                let mut cancelled = false;
                                loop {
                                    tokio::select! {
                                        _ = rx.recv() => {
                                            cancelled = true;
                                            break;
                                        }
                                        chunk_opt = stream.next() => {
                                            match chunk_opt {
                                                Some(Ok(chunk)) => {
                                                    let processed_chunks = match chunk {
                                                        AiChunk::Content(c) => filter.process(c),
                                                        AiChunk::Reasoning(r) => vec![AiChunk::Reasoning(r)],
                                                    };

                                                    for p_chunk in processed_chunks {
                                                        match p_chunk {
                                                            AiChunk::Content(c) => {
                                                                if !ai_started_typing {
                                                                    buffer.begin_user_action();
                                                                    // First time, delete the original selection
                                                                    let mut s = buffer.iter_at_mark(&start_mark);
                                                                    let mut e = buffer.iter_at_mark(&end_mark);
                                                                    buffer.delete(&mut s, &mut e);
                                                                    ai_started_typing = true;
                                                                }
                                                                
                                                                let mut current_iter = buffer.iter_at_mark(&curr_mark);
                                                                buffer.insert(&mut current_iter, &c);
                                                                full_content.push_str(&c);
                                                                
                                                                // Apply highlighting to the new chunk
                                                                let tag_start = buffer.iter_at_mark(&start_mark);
                                                                let tag_end = buffer.iter_at_mark(&curr_mark);
                                                                buffer.apply_tag_by_name("ai-suggestion", &tag_start, &tag_end);
                                                            }
                                                            AiChunk::Reasoning(r) => {
                                                                full_reasoning.push_str(&r);
                                                                if let Some(view) = reasoning_view.upgrade() {
                                                                    let rb = view.buffer();
                                                                    rb.insert(&mut rb.end_iter(), &r);
                                                                }
                                                                if let Some(rev) = reasoning_revealer.upgrade() {
                                                                    rev.set_reveal_child(true);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Some(Err(e)) => {
                                                    tracing::error!("Stream error: {}", e);
                                                    break;
                                                }
                                                None => {
                                                    success = true;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }

                                if cancelled {
                                    if ai_started_typing {
                                        buffer.end_user_action();
                                        buffer.undo();
                                    }
                                    if let Some(l) = ai_status_label.upgrade() {
                                        l.set_text("AI: Cancelled");
                                    }
                                } else if success {
                                    // Final check/cleanup
                                    let final_text = if full_content.contains("---") || full_content.contains("@@") {
                                         // If it looks like a patch, we might need to re-apply it properly
                                         full_content.clone()
                                    } else {
                                        crate::utils::extract_latex(&full_content)
                                    };

                                    if final_text != full_content {
                                        // If extraction changed things, update the buffer one last time
                                        let mut s = buffer.iter_at_mark(&start_mark);
                                        let mut e = buffer.iter_at_mark(&curr_mark);
                                        buffer.delete(&mut s, &mut e);
                                        
                                        let mut insert_iter = buffer.iter_at_mark(&start_mark);
                                        buffer.insert(&mut insert_iter, &final_text);
                                        
                                        let tag_start = buffer.iter_at_mark(&start_mark);
                                        let tag_end = buffer.iter_at_mark(&curr_mark);
                                        buffer.apply_tag_by_name("ai-suggestion", &tag_start, &tag_end);
                                    }

                                    state.borrow_mut().pending_suggestion = Some(final_text.clone());
                                    
                                    if ai_started_typing {
                                        buffer.end_user_action();
                                    }
                                    
                                    if let Some(rev) = suggestion_revealer.upgrade() {
                                        rev.set_reveal_child(true);
                                    }
                                }
                                
                                buffer.delete_mark(&start_mark);
                                buffer.delete_mark(&curr_mark);
                                buffer.delete_mark(&end_mark);
                            }
                            Err(e) => {
                                tracing::error!("AI Error: {}", e);
                            }
                        }

                        if let Some(btn) = ai_run_btn.upgrade() {
                            btn.set_sensitive(true);
                            btn.set_label("Generate");
                            btn.set_icon_name("system-run-symbolic");
                            btn.remove_css_class("destructive-action");
                            btn.add_css_class("suggested-action");
                        }

                        if let Some(s) = ai_spinner.upgrade() {
                            s.stop();
                        }
                        if let Some(l) = ai_status_label.upgrade() {
                            l.set_text("AI: Ready");
                        }
                        if let Some(v) = editor_view.upgrade() {
                            v.set_editable(true);
                        }
                        {
                            let mut s = state.borrow_mut();
                            s.ai_cancellation = None;
                            s.is_ai_generating = false;
                        }
                    }
                ));
            }
        }
    };
    let trigger_ai = Rc::new(trigger_ai);

    ai_run_btn.connect_clicked(glib::clone!(
        #[strong]
        trigger_ai,
        move |_| {
            trigger_ai();
        }
    ));

    let key_controller = gtk4::EventControllerKey::new();
    ai_entry_weak.upgrade().map(|e| {
        let trigger_ai = trigger_ai.clone();
        e.add_controller(key_controller.clone());
        key_controller.connect_key_pressed(glib::clone!(
            #[strong]
            state,
            #[strong]
            ai_history_index,
            move |controller, key, _, _| {
                let Some(ai_entry) = ai_entry_weak.upgrade() else { return glib::Propagation::Proceed };
                let ai_buffer = ai_entry.buffer();
                
                let history = state.borrow().ai_history.clone();
                let mut history_idx = ai_history_index.borrow_mut();
                match key {
                    gdk::Key::Up if !history.is_empty() => {
                        let new_idx = match *history_idx {
                            Some(idx) if idx > 0 => Some(idx - 1),
                            Some(idx) => Some(idx),
                            None => Some(history.len() - 1),
                        };
                        if let Some(idx) = new_idx {
                            ai_buffer.set_text(&history[idx]);
                            *history_idx = Some(idx);
                            glib::Propagation::Stop
                        } else {
                            glib::Propagation::Proceed
                        }
                    }
                    gdk::Key::Down if !history.is_empty() => {
                        let new_idx = match *history_idx {
                            Some(idx) if idx < history.len() - 1 => Some(idx + 1),
                            _ => None,
                        };
                        if let Some(idx) = new_idx {
                            ai_buffer.set_text(&history[idx]);
                            *history_idx = Some(idx);
                            glib::Propagation::Stop
                        } else {
                            ai_buffer.set_text("");
                            *history_idx = None;
                            glib::Propagation::Stop
                        }
                    }
                    gdk::Key::Return => {
                        let mask = controller.current_event_state();
                        if mask.contains(gdk::ModifierType::CONTROL_MASK) {
                            trigger_ai();
                            glib::Propagation::Stop
                        } else {
                            glib::Propagation::Proceed
                        }
                    }
                    _ => glib::Propagation::Proceed,
                }
            }
        ));
    });


    // We don't connect activate for TextView, we use the button or keyboard shortcuts in the controller

    accept_btn.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[weak]
        buffer,
        #[weak]
        suggestion_revealer,
        #[weak]
        ai_revealer,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.pending_suggestion = None;
                s.original_text_selection = None;
            }
            // Remove the highlighting tag
            let (start, end) = buffer.bounds();
            buffer.remove_tag_by_name("ai-suggestion", &start, &end);
            
            suggestion_revealer.set_reveal_child(false);
            ai_revealer.set_reveal_child(false);
            buffer.emit_by_name::<()>("changed", &[]);
        }
    ));

    reject_btn.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[weak]
        buffer,
        #[weak]
        suggestion_revealer,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.pending_suggestion = None;
                s.original_text_selection = None;
            }
            suggestion_revealer.set_reveal_child(false);
            // Undo the last user action (the AI insertion)
            buffer.undo();
        }
    ));

    clear_btn.connect_clicked(glib::clone!(
        #[weak]
        ai_entry,
        #[weak]
        reasoning_view,
        #[weak]
        reasoning_revealer,
        move |_| {
            ai_entry.buffer().set_text("");
            reasoning_view.buffer().set_text("");
            reasoning_revealer.set_reveal_child(false);
        }
    ));

    // Present window before starting background checks to avoid "GtkGizmo without allocation" warnings
    window.present();

    // AI Initialization Check
    validate_ai();
}
