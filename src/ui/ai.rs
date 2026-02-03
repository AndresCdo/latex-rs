use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::{Box, Button, Entry, Orientation, Revealer, RevealerTransitionType, Spinner};

/// Creates the AI assistant panel consisting of a `Revealer` containing
/// a text entry, a loading spinner, and a run button.
pub fn create_ai_panel() -> (Revealer, Entry, Spinner, Button) {
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

    let ai_spinner = Spinner::new();
    let ai_run_btn = Button::builder()
        .label("Generate")
        .icon_name("system-run-symbolic")
        .build();
    ai_run_btn.add_css_class("suggested-action");

    ai_entry_box.append(&ai_entry);
    ai_entry_box.append(&ai_spinner);
    ai_entry_box.append(&ai_run_btn);
    ai_revealer.set_child(Some(&ai_entry_box));

    (ai_revealer, ai_entry, ai_spinner, ai_run_btn)
}
