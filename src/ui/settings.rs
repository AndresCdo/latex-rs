use crate::state::AppState;
use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesPage, PreferencesWindow};
use gtk4::{glib, DropDown, Entry, PasswordEntry, StringList};
use std::cell::RefCell;
use std::rc::Rc;

pub fn show_settings(
    parent: &gtk4::Window,
    state: Rc<RefCell<AppState>>,
    on_settings_closed: Option<Rc<dyn Fn()>>,
) {
    let window = PreferencesWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title("Settings")
        .default_width(500)
        .build();

    let page = PreferencesPage::new();
    page.set_title("AI Configuration");
    page.set_icon_name(Some("starred-symbolic"));
    window.add(&page);

    let group = PreferencesGroup::new();
    group.set_title("Provider Settings");
    group.set_description(Some(
        "Configure your AI backends (Ollama, OpenAI, DeepSeek)",
    ));
    page.add(&group);

    let config = state.borrow().config.clone();
    let provider_names: Vec<String> = config.providers.iter().map(|p| p.name.clone()).collect();
    let model_names = StringList::new(
        provider_names
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .as_slice(),
    );

    let provider_row = ActionRow::builder()
        .title("Active Provider")
        .subtitle("Select which AI service to use")
        .build();

    let provider_dropdown = DropDown::builder()
        .model(&model_names)
        .valign(gtk4::Align::Center)
        .build();

    let current_index = config
        .providers
        .iter()
        .position(|p| p.name == config.active_provider)
        .unwrap_or(0);
    provider_dropdown.set_selected(current_index as u32);

    provider_row.add_suffix(&provider_dropdown);
    group.add(&provider_row);

    // Dynamic fields based on selection
    let api_key_row = ActionRow::builder()
        .title("API Key")
        .subtitle("Your provider API key (hidden)")
        .build();
    let api_key_entry = PasswordEntry::builder()
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .build();
    api_key_row.add_suffix(&api_key_entry);
    group.add(&api_key_row);

    let url_row = ActionRow::builder()
        .title("Base URL")
        .subtitle("API endpoint for the provider")
        .build();
    let url_entry = Entry::builder()
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .build();
    url_row.add_suffix(&url_entry);
    group.add(&url_row);

    let model_row = ActionRow::builder()
        .title("Model Name")
        .subtitle("Specific model ID (e.g. gpt-4o, deepseek-reasoner)")
        .build();
    let model_entry = Entry::builder()
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .build();
    model_row.add_suffix(&model_entry);
    group.add(&model_row);

    // Helper to update fields
    let update_fields = {
        let provider_dropdown = provider_dropdown.downgrade();
        let api_key_entry = api_key_entry.downgrade();
        let url_entry = url_entry.downgrade();
        let model_entry = model_entry.downgrade();
        let state = state.clone();
        move || {
            let provider_dropdown = match provider_dropdown.upgrade() {
                Some(p) => p,
                None => return,
            };
            let api_key_entry = match api_key_entry.upgrade() {
                Some(e) => e,
                None => return,
            };
            let url_entry = match url_entry.upgrade() {
                Some(e) => e,
                None => return,
            };
            let model_entry = match model_entry.upgrade() {
                Some(e) => e,
                None => return,
            };

            let config = state.borrow().config.clone();
            let selected = provider_dropdown.selected();
            if let Some(p) = config.providers.get(selected as usize) {
                api_key_entry.set_text(p.api_key.as_deref().unwrap_or(""));
                url_entry.set_text(&p.base_url);
                model_entry.set_text(&p.active_model);
            }
        }
    };

    update_fields();

    provider_dropdown.connect_selected_notify(glib::clone!(
        #[strong]
        update_fields,
        move |_| {
            update_fields();
        }
    ));

    window.connect_close_request(glib::clone!(
        #[strong]
        state,
        #[strong]
        provider_dropdown,
        #[strong]
        api_key_entry,
        #[strong]
        url_entry,
        #[strong]
        model_entry,
        move |_| {
            let mut s = state.borrow_mut();
            let selected = provider_dropdown.selected();

            let config_clone = s.config.clone();
            if let Some(p_name) = config_clone
                .providers
                .get(selected as usize)
                .map(|p| p.name.clone())
            {
                s.config.active_provider = p_name;
            }

            if let Some(p) = s.config.providers.get_mut(selected as usize) {
                let key = api_key_entry.text().to_string();
                p.api_key = if key.is_empty() { None } else { Some(key) };
                p.base_url = url_entry.text().to_string();
                p.active_model = model_entry.text().to_string();
            }

            let _ = s.config.save();

            if let Some(p_config) = s.config.get_active_provider() {
                s.ai_provider = Some(crate::api::create_provider(p_config));
            }

            if let Some(on_closed) = &on_settings_closed {
                on_closed();
            }

            glib::Propagation::Proceed
        }
    ));

    window.present();
}
