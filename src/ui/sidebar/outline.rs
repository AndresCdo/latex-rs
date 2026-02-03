use gtk4::{ListBox, ScrolledWindow};

pub fn create_outline_pane() -> (ScrolledWindow, ListBox) {
    let list_box = ListBox::new();
    let scrolled_window = ScrolledWindow::builder()
        .child(&list_box)
        .vexpand(true)
        .build();

    (scrolled_window, list_box)
}
