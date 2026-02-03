use crate::ui::sidebar;
use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::{Box, Label, ListBox, Orientation, Paned, SearchEntry};

/// Creates the main layout structure including the sidebar hub,
/// the editor/preview split view, and the status bar.
pub fn create_main_layout(
    _main_vbox: &gtk4::Box,
) -> (
    Paned,
    Paned,
    ListBox,        // Outline list
    adw::ViewStack, // Sidebar hub
    gtk4::Box,      // Sidebar container
    gtk4::Box,      // Status bar
    Label,
    Label,
    Label,
    SearchEntry, // Arxiv search
    ListBox,     // Arxiv results
) {
    let paned = Paned::new(Orientation::Horizontal);
    paned.set_hexpand(true);
    paned.set_vexpand(true);
    paned.set_position(475); // Balanced split for Editor and Preview
    paned.set_wide_handle(true);

    let outer_paned = Paned::new(Orientation::Horizontal);
    outer_paned.set_hexpand(true);
    outer_paned.set_vexpand(true);
    outer_paned.set_position(280); // Slightly wider for hub
    outer_paned.set_wide_handle(true);

    // We'll let main.rs decide where to append outer_paned

    // Sidebar Hub
    let (sidebar_hub, outline_list, arxiv_search, arxiv_list) = sidebar::create_sidebar_hub();
    let sidebar_container = Box::new(Orientation::Vertical, 0);
    sidebar_container.add_css_class("sidebar");
    sidebar_container.set_width_request(250);

    sidebar_container.append(&sidebar_hub);

    outer_paned.set_start_child(Some(&sidebar_container));
    outer_paned.set_end_child(Some(&paned));

    // Status Bar
    let status_bar = Box::new(Orientation::Horizontal, 12);
    status_bar.set_margin_start(12);
    status_bar.set_margin_end(12);
    status_bar.set_margin_top(4);
    status_bar.set_margin_bottom(4);
    status_bar.add_css_class("dim-label");

    let pos_label = Label::new(Some("Line: 1, Col: 1"));
    let word_count_label = Label::new(Some("Words: 0"));
    let ai_status_label = Label::new(Some("AI: Checking..."));
    ai_status_label.set_hexpand(true);
    ai_status_label.set_halign(gtk4::Align::End);

    status_bar.append(&pos_label);
    status_bar.append(&word_count_label);
    status_bar.append(&ai_status_label);
    // main_vbox.append(&status_bar); // Let main.rs handle this

    (
        outer_paned,
        paned,
        outline_list,
        sidebar_hub,
        sidebar_container,
        status_bar,
        pos_label,
        word_count_label,
        ai_status_label,
        arxiv_search,
        arxiv_list,
    )
}
