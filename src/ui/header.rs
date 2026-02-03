use adw::{HeaderBar, WindowTitle};
use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::{Box, Button, Orientation, ToggleButton};

/// Creates the application header bar containing file operations and AI/Sidebar toggles.
pub fn create_header_bar() -> (
    HeaderBar,
    WindowTitle,
    Button,
    Button,
    Button,
    Button,
    Button,
    Button,
    ToggleButton,
) {
    let header_bar = HeaderBar::new();
    let view_title = WindowTitle::new("LaTeX.rs Editor", "");
    header_bar.set_title_widget(Some(&view_title));

    // Left actions group
    let left_box = Box::new(Orientation::Horizontal, 0);
    left_box.add_css_class("linked");

    let new_btn = Button::builder()
        .icon_name("document-new-symbolic")
        .tooltip_text("New Document")
        .build();
    let open_btn = Button::builder()
        .icon_name("document-open-symbolic")
        .tooltip_text("Open File")
        .build();
    let save_btn = Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Save File")
        .build();
    let export_btn = Button::builder()
        .icon_name("document-send-symbolic")
        .tooltip_text("Export PDF")
        .build();

    left_box.append(&new_btn);
    left_box.append(&open_btn);
    left_box.append(&save_btn);
    left_box.append(&export_btn);
    header_bar.pack_start(&left_box);

    // Right actions
    let settings_btn = Button::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text("Settings")
        .build();

    let ai_btn = Button::builder()
        .icon_name("starred-symbolic")
        .tooltip_text("AI Assistant")
        .sensitive(false)
        .build();
    ai_btn.add_css_class("suggested-action");

    let sidebar_toggle = ToggleButton::builder()
        .icon_name("sidebar-show-symbolic")
        .tooltip_text("Toggle Outline")
        .active(true)
        .build();

    header_bar.pack_end(&sidebar_toggle);
    header_bar.pack_end(&settings_btn);
    header_bar.pack_end(&ai_btn);

    (
        header_bar,
        view_title,
        new_btn,
        open_btn,
        save_btn,
        export_btn,
        settings_btn,
        ai_btn,
        sidebar_toggle,
    )
}
