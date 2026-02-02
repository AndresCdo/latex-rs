# Copilot Instructions for latex-rs

## Project Overview
`latex-rs` is a GTK-based LaTeX editor written in Rust. It features real-time LaTeX preview via KaTeX (rendered in high-performance WebKit2GTK) and AI assistance via local Ollama integration.

## Architecture & Data Flow
- **UI Framework**: GTK 3 via `gtk-rs`. UI is defined programmatically in [src/main.rs](src/main.rs).
- **Editor**: Uses `sourceview` for LaTeX syntax highlighting. Configuration is in `utils::configure_sourceview`.
- **Preview**: [src/preview.rs](src/preview.rs) generates an HTML string with KaTeX/Highlight.js (loaded via CDN) using the `horrorshow` macro.
- **AI Integration**: [src/api.rs](src/api.rs) performs async POST requests to a local Ollama instance (`http://localhost:11434`).
- **Sync/Async**: GTK operates on a synchronous main loop. Async AI calls must be handled carefully within GTK signal handlers using `glib::MainContext`.

## Critical Patterns & Conventions
- **Closures & Cloning**: Use the `@strong` macro defined in [src/utils.rs](src/utils.rs) to clone GTK widgets into closures.
  ```rust
  text_buffer.connect_changed(clone!(@strong web_view, preview => move |buffer| { ... }));
  ```
- **HTML Templating**: [src/preview.rs](src/preview.rs) uses `horrorshow`'s `html!` macro. Match the existing structure when adding scripts or styles to the preview.
- **Latex Detection**: Math is detected using Regex in `preview::render_latex_to_html` before being passed to the webview's KaTeX auto-render extension.

## Developer Workflows
- **Building**: Requires system dependencies: `libgtk-3-dev`, `libgtksourceview-3.0-dev`, `libwebkit2gtk-4.0-dev`.
- **Running**: `cargo run` starts the GUI. Needs Ollama running locally for AI features.
- **Testing**: UI tests are difficult; focus on unit testing logic in [src/preview.rs](src/preview.rs) and [src/utils.rs](src/utils.rs) if possible.

## Key Files
- [src/main.rs](src/main.rs): Application entry point and UI signal logic.
- [src/preview.rs](src/preview.rs): HTML/KaTeX rendering pipeline.
- [src/api.rs](src/api.rs): Client for Ollama API.
- [src/utils.rs](src/utils.rs): Shared helper functions and the `clone!` macro.
