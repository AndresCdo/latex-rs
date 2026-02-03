use gtk4::prelude::*;
use gtk4::{
    Box, Button, Label, Orientation, PolicyType, Revealer, RevealerTransitionType, ScrolledWindow,
    Spinner, TextView,
};

/// Creates the AI assistant panel consisting of a `Revealer` containing
/// a text entry, a loading spinner, a run button, and a reasoning box.
pub fn create_ai_panel() -> (
    Revealer,
    TextView,
    Spinner,
    Button,
    Revealer,
    TextView,
    Revealer,
    Button,
    Button,
    Button,
) {
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

    let ai_entry = TextView::builder()
        .wrap_mode(gtk4::WrapMode::Word)
        .hexpand(true)
        .accepts_tab(false)
        .height_request(36)
        .build();
    // placeholder-text is only available in GTK 4.12+ for TextView
    // Since we are using an environment that might have older GTK4,
    // we should use a more compatible way if possible, or skip it.
    // However, the user wants the feature.
    // Let's check if it exists via introspection or just use GtkEntry if appropriate.
    // Actually, let's just remove the set_property call that panics if not found.

    ai_entry.add_css_class("view");
    ai_entry.add_css_class("sidebar"); // Use sidebar class for border styling

    let ai_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(36)
        .max_content_height(150)
        .child(&ai_entry)
        .hexpand(true)
        .build();

    let clear_btn = Button::builder()
        .icon_name("edit-clear-all-symbolic")
        .has_frame(false)
        .tooltip_text("Clear input and reasoning")
        .valign(gtk4::Align::Start)
        .build();

    let ai_spinner = Spinner::builder()
        .valign(gtk4::Align::Start)
        .margin_top(8)
        .build();
    let ai_run_btn = Button::builder()
        .label("Generate")
        .icon_name("system-run-symbolic")
        .valign(gtk4::Align::Start)
        .build();
    ai_run_btn.add_css_class("suggested-action");

    ai_entry_box.append(&ai_scroll);
    ai_entry_box.append(&clear_btn);
    ai_entry_box.append(&ai_spinner);
    ai_entry_box.append(&ai_run_btn);
    container.append(&ai_entry_box);

    // Suggestion Actions (Accept/Reject)
    let suggestion_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .build();
    let suggestion_box = Box::new(Orientation::Horizontal, 6);
    suggestion_box.set_margin_start(12);
    suggestion_box.set_margin_end(12);
    suggestion_box.set_margin_bottom(6);

    let accept_btn = Button::builder()
        .label("Accept Suggestion")
        .icon_name("emblem-ok-symbolic")
        .tooltip_text("Accept AI changes and merge into document")
        .hexpand(true)
        .build();
    accept_btn.add_css_class("suggested-action");

    let reject_btn = Button::builder()
        .label("Reject")
        .icon_name("edit-clear-symbolic")
        .tooltip_text("Discard AI changes and restore original text")
        .build();
    reject_btn.add_css_class("destructive-action");

    suggestion_box.append(&accept_btn);
    suggestion_box.append(&reject_btn);
    suggestion_revealer.set_child(Some(&suggestion_box));
    container.append(&suggestion_revealer);

    // Reasoning Box
    let reasoning_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .build();

    let reasoning_box = Box::new(Orientation::Vertical, 4);
    reasoning_box.set_margin_start(12);
    reasoning_box.set_margin_end(12);
    reasoning_box.set_margin_bottom(6);
    reasoning_box.add_css_class("sidebar"); // Re-use sidebar style for border

    let reasoning_view = TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .wrap_mode(gtk4::WrapMode::Word)
        .build();
    reasoning_view.add_css_class("dim-label");

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(100)
        .max_content_height(250)
        .child(&reasoning_view)
        .build();

    // Auto-scroll logic: scroll to bottom when content changes
    let buffer = reasoning_view.buffer();
    let end_mark = buffer.create_mark(None, &buffer.end_iter(), false);
    buffer.connect_changed(glib::clone!(
        #[weak]
        reasoning_view,
        move |_| {
            reasoning_view.scroll_to_mark(&end_mark, 0.0, true, 0.0, 1.0);
        }
    ));

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
        reasoning_view,
        suggestion_revealer,
        accept_btn,
        reject_btn,
        clear_btn,
    )
}
