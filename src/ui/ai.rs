use gtk4::prelude::*;
use gtk4::{
    Box, Button, Entry, Label, Orientation, PolicyType, Revealer, RevealerTransitionType,
    ScrolledWindow, Spinner,
};

/// Creates the AI assistant panel consisting of a `Revealer` containing
/// a text entry, a loading spinner, a run button, and a reasoning box.
pub fn create_ai_panel() -> (Revealer, Entry, Spinner, Button, Revealer, Label) {
    let container = Box::new(Orientation::Vertical, 0);

    let ai_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .child(&container)
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
    container.append(&ai_entry_box);

    // Reasoning Box
    let reasoning_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .build();

    let reasoning_box = Box::new(Orientation::Vertical, 4);
    reasoning_box.set_margin_start(12);
    reasoning_box.set_margin_end(12);
    reasoning_box.set_margin_bottom(6);
    reasoning_box.add_css_class("sidebar"); // Re-use sidebar style for border

    let reasoning_label = Label::builder().wrap(true).xalign(0.0).build();
    reasoning_label.add_css_class("dim-label");
    reasoning_label.set_selectable(true);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(100)
        .max_content_height(250)
        .child(&reasoning_label)
        .build();

    // Auto-scroll logic
    let adj = scroll.vadjustment();
    adj.connect_changed(move |a| {
        a.set_value(a.upper() - a.page_size());
    });

    let header_box = Box::new(Orientation::Horizontal, 6);
    let reasoning_label_title = Label::builder()
        .label("<b>Reasoning (DeepSeek R1)</b>")
        .use_markup(true)
        .xalign(0.0)
        .hexpand(true)
        .build();
    reasoning_label_title.add_css_class("dim-label");

    let close_btn = Button::builder()
        .icon_name("window-close-symbolic")
        .has_frame(false)
        .build();
    close_btn.connect_clicked(glib::clone!(
        #[weak]
        reasoning_revealer,
        move |_| {
            reasoning_revealer.set_reveal_child(false);
        }
    ));

    header_box.append(&reasoning_label_title);
    header_box.append(&close_btn);

    reasoning_box.append(&header_box);
    reasoning_box.append(&scroll);
    reasoning_revealer.set_child(Some(&reasoning_box));
    container.append(&reasoning_revealer);

    (
        ai_revealer,
        ai_entry,
        ai_spinner,
        ai_run_btn,
        reasoning_revealer,
        reasoning_label,
    )
}
