use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::{Box, Label, ListBox, Orientation, Paned, ScrolledWindow};

/// Creates the main layout structure including the sidebar (outline),
/// the editor/preview split view, and the status bar.
pub fn create_main_layout(
    main_vbox: &gtk4::Box,
) -> (
    Paned,
    Paned,
    ListBox,
    ScrolledWindow,
    gtk4::Box,
    Label,
    Label,
    Label,
) {
    let paned = Paned::new(Orientation::Horizontal);
    paned.set_hexpand(true);
    paned.set_vexpand(true);
    paned.set_position(475); // Balanced split for Editor and Preview
    paned.set_wide_handle(true);

    let outer_paned = Paned::new(Orientation::Horizontal);
    outer_paned.set_hexpand(true);
    outer_paned.set_vexpand(true);
    outer_paned.set_position(250); // Initial sidebar width
    outer_paned.set_wide_handle(true);
    main_vbox.append(&outer_paned);

    // Sidebar Outline
    let sidebar_list = ListBox::new();
    let sidebar_scroll = ScrolledWindow::builder()
        .child(&sidebar_list)
        .vexpand(true)
        .width_request(200)
        .build();
    sidebar_scroll.add_css_class("sidebar");
    outer_paned.set_start_child(Some(&sidebar_scroll));
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
    main_vbox.append(&status_bar);

    (
        outer_paned,
        paned,
        sidebar_list,
        sidebar_scroll,
        status_bar,
        pos_label,
        word_count_label,
        ai_status_label,
    )
}
