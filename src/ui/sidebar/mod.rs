pub mod arxiv;
pub mod outline;

use gtk4::{ListBox, SearchEntry};

pub fn create_sidebar_hub() -> (
    adw::ViewStack,
    ListBox,     // Outline list
    SearchEntry, // Arxiv search
    ListBox,     // Arxiv results
) {
    let stack = adw::ViewStack::new();

    let (outline_pane, outline_list) = outline::create_outline_pane();
    let (arxiv_pane, arxiv_search, arxiv_list) = arxiv::create_arxiv_pane();

    let outline_page = stack.add_titled(&outline_pane, Some("outline"), "Outline");
    outline_page.set_icon_name(Some("view-list-bullet-symbolic"));

    let arxiv_page = stack.add_titled(&arxiv_pane, Some("arxiv"), "arXiv");
    arxiv_page.set_icon_name(Some("system-search-symbolic"));

    (stack, outline_list, arxiv_search, arxiv_list)
}
