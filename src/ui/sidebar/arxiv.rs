use gtk4::prelude::*;
use gtk4::{Box, Label, ListBox, Orientation, ScrolledWindow, SearchEntry};

pub fn create_arxiv_pane() -> (Box, SearchEntry, ListBox) {
    let container = Box::new(Orientation::Vertical, 6);
    container.set_margin_start(6);
    container.set_margin_end(6);
    container.set_margin_top(6);

    let search_entry = SearchEntry::builder()
        .placeholder_text("Search arXiv...")
        .build();

    let list_box = ListBox::new();
    list_box.add_css_class("boxed-list");

    let scrolled_window = ScrolledWindow::builder()
        .child(&list_box)
        .vexpand(true)
        .build();

    let info_label = Label::new(Some("Search for papers by title, author, or ID"));
    info_label.add_css_class("dim-label");
    info_label.set_margin_top(12);
    info_label.set_margin_bottom(12);
    info_label.set_wrap(true);

    container.append(&search_entry);
    container.append(&scrolled_window);
    // Placeholder message
    list_box.append(&info_label);

    (container, search_entry, list_box)
}
