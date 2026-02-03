use crate::queue::CompilationQueue;
use crate::state::AppState;
use crate::utils::buffer_to_string;
use adw::ToastOverlay;
use glib;
use gtk4::prelude::*;
use gtk4::{ListBox, ScrolledWindow};
use sourceview5::Buffer;
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::prelude::*;
use webkit6::WebView;

/// Creates the WebKit WebView for LaTeX preview and its scrolled window container.
pub fn create_preview() -> (WebView, ScrolledWindow) {
    let web_view = WebView::new();
    if let Some(settings) = webkit6::prelude::WebViewExt::settings(&web_view) {
        settings.set_zoom_text_only(false);
        settings.set_enable_developer_extras(true);
    }
    let preview_scroll = ScrolledWindow::builder()
        .child(&web_view)
        .hexpand(true)
        .vexpand(true)
        .build();
    (web_view, preview_scroll)
}

/// Connects the editor buffer change signal to the live preview compilation queue.
/// Also updates the sidebar outline when the document structure changes.
pub fn connect_live_preview(
    buffer: &Buffer,
    web_view: &WebView,
    sidebar_list: &ListBox,
    state: Rc<RefCell<AppState>>,
    toast_overlay: &ToastOverlay,
) {
    let preview = state.borrow().preview_generator.clone();
    let queue = CompilationQueue::new(preview);

    let web_view = web_view.clone();
    let sidebar_list = sidebar_list.clone();
    let state = state.clone();
    let toast_overlay = toast_overlay.clone();

    buffer.connect_changed(move |buf| {
        let text = buffer_to_string(buf.upcast_ref());
        if text.trim().is_empty() {
            web_view.load_html("", None::<&str>);
            return;
        }

        let queue = queue.clone();
        let web_view = web_view.clone();
        let sidebar_list = sidebar_list.clone();
        let _state = state.clone();
        let _toast_overlay = toast_overlay.clone();
        let text_for_enqueue = text.clone();
        let text_for_sections = text.clone();

        glib::MainContext::default().spawn_local(async move {
            match queue.enqueue(text_for_enqueue).await {
                Some(html) => {
                    web_view.load_html(&html, None::<&str>);

                    let sections = crate::utils::extract_sections(&text_for_sections);
                    sidebar_list.remove_all();
                    for (title_with_prefix, _line) in sections {
                        let row = gtk4::ListBoxRow::new();
                        let label = gtk4::Label::new(Some(&title_with_prefix));
                        label.set_xalign(0.0);
                        let prefix_spaces =
                            title_with_prefix.len() - title_with_prefix.trim_start().len();
                        let level = prefix_spaces / 2;
                        label.set_margin_start((level * 12) as i32);
                        row.set_child(Some(&label));
                        sidebar_list.append(&row);
                    }
                }
                None => {
                    tracing::debug!("Compilation queue full, request dropped");
                }
            }
        });
    });
}
