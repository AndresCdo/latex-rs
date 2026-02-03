# Copilot Instructions for latex-rs

## Project Overview
`latex-rs` is a professional, modern LaTeX editor written in Rust using GTK 4 and Libadwaita. It features real-time LaTeX preview via a native `pdflatex` → `pdftocairo` pipeline and AI assistance via local Ollama integration.

## Architecture & Data Flow
- **UI Framework**: GTK 4 and Libadwaita. UI is defined programmatically in [src/main.rs](src/main.rs) and modularized in `src/ui/`.
- **Editor**: Uses `sourceview5` for advanced LaTeX syntax highlighting. Configuration is in `src/ui/editor.rs`.
- **Preview**: [src/preview.rs](src/preview.rs) handles compilation. It produces an HTML wrapper with embedded SVGs for each page, rendered in **WebKit 6**.
- **AI Integration**: [src/api/](src/api/) contains providers for Ollama and OpenAI-compatible APIs. It uses async streaming for real-time response insertion.
- **Sync/Async**: GTK operates on a synchronous main loop. Async operations (AI, compilation) use `glib::MainContext::default().spawn_local` or `tokio` tasks.

## Critical Patterns & Conventions
- **Closures & Cloning**: Use the `glib::clone!` macro for cloning GTK widgets into closures. Use `#[weak]` for widgets that might be destroyed.
  ```rust
  button.connect_clicked(glib::clone!(#[weak] window, #[strong] state => move |_| { ... }));
  ```
- **Centralized Constants**: All magic numbers, timeouts, and limits must be defined in [src/constants.rs](src/constants.rs).
- **Security**: LaTeX compilation must use `-no-shell-escape` and `-openin-any=p`. Path sanitization in [src/preview.rs](src/preview.rs) prevents leaking system info.
- **Async Safety**: Always check if a widget is still valid (e.g., via `upgrade()`) before acting on it after an `.await`.

## Developer Workflows
- **Building**: Requires system dependencies: `libgtk-4-dev`, `libadwaita-1-dev`, `libgtksourceview-5-dev`, `libwebkitgtk-6.0-dev`.
- **Running**: `cargo run` starts the GUI. Needs `pdflatex` and `pdftocairo` (poppler-utils) for the preview.
- **Testing**: Business logic in `utils.rs` and `api/` is unit tested. UI components should be verified manually via `cargo run`.

## Key Files
- [src/main.rs](src/main.rs): Application entry point and main window assembly.
- [src/preview.rs](src/preview.rs): Native LaTeX rendering pipeline.
- [src/queue.rs](src/queue.rs): Sequential compilation queue to prevent race conditions.
- [src/constants.rs](src/constants.rs): Centralized configuration.
- [src/ui/](src/ui/): Modular UI components (AI assistant, Editor, Layout).
