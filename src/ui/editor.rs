use crate::constants::{
    DEFAULT_EDITOR_FONT, DEFAULT_EDITOR_FONT_SIZE, DEFAULT_ZOOM_LEVEL, MAX_ZOOM_LEVEL,
    MIN_ZOOM_LEVEL, ZOOM_STEP,
};
use crate::AppState;
use adw::StyleManager;
use glib;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Box, Orientation, Revealer, RevealerTransitionType, ScrolledWindow, SearchEntry};
use sourceview5::prelude::*;
use sourceview5::{Buffer, LanguageManager, StyleSchemeManager, View};
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::prelude::*;

/// Creates the text editor component with LaTeX syntax highlighting, undo support,
/// and theme synchronization.
pub fn create_editor(style_manager: &StyleManager) -> (Buffer, View, ScrolledWindow) {
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
    editor_view.set_auto_indent(true);
    editor_view.set_insert_spaces_instead_of_tabs(true);
    editor_view.set_indent_width(4);
    buffer.set_highlight_matching_brackets(true);
    editor_view.set_smart_backspace(true);

    // Helper function to update theme
    fn update_editor_theme(buffer: &Buffer, is_dark: bool) {
        let scheme_manager = StyleSchemeManager::default();
        let scheme_id = if is_dark { "Adwaita-dark" } else { "Adwaita" };
        if let Some(scheme) = scheme_manager.scheme(scheme_id) {
            buffer.set_style_scheme(Some(&scheme));
        } else {
            let fallback = if is_dark { "classic-dark" } else { "classic" };
            if let Some(scheme) = scheme_manager.scheme(fallback) {
                buffer.set_style_scheme(Some(&scheme));
            }
        }
    }

    // Initial theme
    update_editor_theme(&buffer, style_manager.is_dark());

    // Listen for system theme changes
    style_manager.connect_dark_notify(glib::clone!(
        #[weak]
        buffer,
        move |sm| {
            update_editor_theme(&buffer, sm.is_dark());
        }
    ));

    let editor_scroll = ScrolledWindow::builder()
        .child(&editor_view)
        .hexpand(true)
        .vexpand(true)
        .build();

    (buffer, editor_view, editor_scroll)
}

/// Creates a search bar with a `Revealer` and a `SearchEntry`.
pub fn create_search_bar() -> (Revealer, SearchEntry) {
    let search_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .build();
    let search_entry = SearchEntry::builder()
        .hexpand(true)
        .placeholder_text("Search document...")
        .build();
    let search_box = Box::new(Orientation::Horizontal, 6);
    search_box.set_margin_start(12);
    search_box.set_margin_end(12);
    search_box.set_margin_top(6);
    search_box.set_margin_bottom(6);
    search_box.append(&search_entry);
    search_revealer.set_child(Some(&search_box));
    (search_revealer, search_entry)
}
/// Connects zoom handlers for keyboard shortcuts (Ctrl+Plus/Minus/0) and mouse scroll.
/// Also handles document search shortcuts (Ctrl+F, Escape).
pub fn connect_zoom_handlers(
    window: &adw::ApplicationWindow,
    state: Rc<RefCell<AppState>>,
    editor_view: &sourceview5::View,
    editor_scroll: &gtk4::ScrolledWindow,
    search_revealer: &gtk4::Revealer,
    search_entry: &gtk4::SearchEntry,
    web_view: &webkit6::WebView,
) {
    let zoom_provider = gtk4::CssProvider::new();
    #[allow(deprecated)]
    editor_view
        .style_context()
        .add_provider(&zoom_provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

    let apply_editor_zoom = {
        let zoom_provider = zoom_provider.clone();
        move |zoom: f64| {
            let size = (DEFAULT_EDITOR_FONT_SIZE as f64 * zoom) as i32;
            let css = format!(
                "textview {{ font-family: '{}'; font-size: {}pt; }}",
                DEFAULT_EDITOR_FONT, size
            );
            zoom_provider.load_from_string(&css);
        }
    };

    let apply_preview_zoom = {
        let web_view = web_view.clone();
        move |zoom: f64| {
            web_view.set_zoom_level(zoom);
        }
    };

    // Initial application
    apply_editor_zoom(DEFAULT_ZOOM_LEVEL);
    apply_preview_zoom(DEFAULT_ZOOM_LEVEL);

    // Keyboard zoom & search shortcuts
    let zoom_key_ctrl = gtk4::EventControllerKey::new();
    zoom_key_ctrl.set_propagation_phase(gtk4::PropagationPhase::Capture);
    window.add_controller(zoom_key_ctrl.clone());

    zoom_key_ctrl.connect_key_pressed({
        let state = state.clone();
        let apply_editor_zoom = apply_editor_zoom.clone();
        let apply_preview_zoom = apply_preview_zoom.clone();
        let window_weak = window.downgrade();
        let search_revealer_weak = search_revealer.downgrade();
        let search_entry_weak = search_entry.downgrade();
        let editor_view_weak = editor_view.downgrade();
        let editor_scroll_weak = editor_scroll.downgrade();

        move |_, key, _, modifier| {
            let window = match window_weak.upgrade() {
                Some(w) => w,
                None => return glib::Propagation::Proceed,
            };
            let search_revealer = match search_revealer_weak.upgrade() {
                Some(r) => r,
                None => return glib::Propagation::Proceed,
            };
            let search_entry = match search_entry_weak.upgrade() {
                Some(e) => e,
                None => return glib::Propagation::Proceed,
            };
            let editor_view = match editor_view_weak.upgrade() {
                Some(v) => v,
                None => return glib::Propagation::Proceed,
            };
            let editor_scroll = match editor_scroll_weak.upgrade() {
                Some(s) => s,
                None => return glib::Propagation::Proceed,
            };

            if modifier.contains(gdk::ModifierType::CONTROL_MASK) {
                let mut s = state.borrow_mut();
                let focus = gtk4::prelude::RootExt::focus(&window);

                // Determine which zoom to update based on focus
                let is_editor = focus
                    .as_ref()
                    .map(|f| f.is_ancestor(&editor_scroll))
                    .unwrap_or(true);

                match key {
                    gdk::Key::f => {
                        let is_revealed = search_revealer.reveals_child();
                        search_revealer.set_reveal_child(!is_revealed);
                        if !is_revealed {
                            search_entry.grab_focus();
                        } else {
                            editor_view.grab_focus();
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::Escape => {
                        if search_revealer.reveals_child() {
                            search_revealer.set_reveal_child(false);
                            editor_view.grab_focus();
                            return glib::Propagation::Stop;
                        }
                    }
                    gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => {
                        if is_editor {
                            s.editor_zoom = (s.editor_zoom + ZOOM_STEP).min(MAX_ZOOM_LEVEL);
                            apply_editor_zoom(s.editor_zoom);
                        } else {
                            s.preview_zoom = (s.preview_zoom + ZOOM_STEP).min(MAX_ZOOM_LEVEL);
                            apply_preview_zoom(s.preview_zoom);
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::minus | gdk::Key::underscore | gdk::Key::KP_Subtract => {
                        if is_editor {
                            s.editor_zoom = (s.editor_zoom - ZOOM_STEP).max(MIN_ZOOM_LEVEL);
                            apply_editor_zoom(s.editor_zoom);
                        } else {
                            s.preview_zoom = (s.preview_zoom - ZOOM_STEP).max(MIN_ZOOM_LEVEL);
                            apply_preview_zoom(s.preview_zoom);
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::_0 | gdk::Key::KP_0 => {
                        if is_editor {
                            s.editor_zoom = DEFAULT_ZOOM_LEVEL;
                            apply_editor_zoom(s.editor_zoom);
                        } else {
                            s.preview_zoom = DEFAULT_ZOOM_LEVEL;
                            apply_preview_zoom(s.preview_zoom);
                        }
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            }
            glib::Propagation::Proceed
        }
    });

    // Scroll zoom: tied to mouse position
    let editor_scroll_ctrl =
        gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    editor_scroll_ctrl.set_propagation_phase(gtk4::PropagationPhase::Capture);
    editor_view.add_controller(editor_scroll_ctrl.clone());
    editor_scroll_ctrl.connect_scroll({
        let state = state.clone();
        let apply_editor_zoom = apply_editor_zoom.clone();
        let ctrl_weak = editor_scroll_ctrl.downgrade();
        move |_, _, dy| {
            if let Some(ctrl) = ctrl_weak.upgrade() {
                if ctrl
                    .current_event_state()
                    .contains(gdk::ModifierType::CONTROL_MASK)
                {
                    let mut s = state.borrow_mut();
                    if dy < 0.0 {
                        s.editor_zoom = (s.editor_zoom + ZOOM_STEP).min(MAX_ZOOM_LEVEL);
                    } else {
                        s.editor_zoom = (s.editor_zoom - ZOOM_STEP).max(MIN_ZOOM_LEVEL);
                    }
                    apply_editor_zoom(s.editor_zoom);
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        }
    });

    let preview_scroll_ctrl =
        gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    preview_scroll_ctrl.set_propagation_phase(gtk4::PropagationPhase::Capture);
    web_view.add_controller(preview_scroll_ctrl.clone());
    preview_scroll_ctrl.connect_scroll({
        let state = state.clone();
        let apply_preview_zoom = apply_preview_zoom.clone();
        let ctrl_weak = preview_scroll_ctrl.downgrade();
        move |_, _, dy| {
            if let Some(ctrl) = ctrl_weak.upgrade() {
                if ctrl
                    .current_event_state()
                    .contains(gdk::ModifierType::CONTROL_MASK)
                {
                    let mut s = state.borrow_mut();
                    if dy < 0.0 {
                        s.preview_zoom = (s.preview_zoom + ZOOM_STEP).min(MAX_ZOOM_LEVEL);
                    } else {
                        s.preview_zoom = (s.preview_zoom - ZOOM_STEP).max(MIN_ZOOM_LEVEL);
                    }
                    apply_preview_zoom(s.preview_zoom);
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        }
    });
}

/// Connects the sidebar row activation to scroll the editor to the selected section.
pub fn connect_sidebar_activation(
    sidebar_list: &gtk4::ListBox,
    buffer: &sourceview5::Buffer,
    editor_view: &sourceview5::View,
) {
    sidebar_list.connect_row_activated(glib::clone!(
        #[weak]
        buffer,
        #[weak]
        editor_view,
        move |_, row| {
            let index = row.index();
            let text = crate::utils::buffer_to_string(buffer.upcast_ref());
            let sections = crate::utils::extract_sections(&text);

            if let Some((_, line)) = sections.get(index as usize) {
                let buf = buffer.upcast_ref::<gtk4::TextBuffer>();
                if let Some(mut iter) = buf.iter_at_line(*line) {
                    buf.place_cursor(&iter);
                    editor_view.scroll_to_iter(&mut iter, 0.0, false, 0.5, 0.5);
                    editor_view.grab_focus();
                }
            }
        }
    ));
}
