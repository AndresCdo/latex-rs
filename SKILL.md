# Agentic Skills & Patterns: latex-rs

Specific technical patterns to follow when modifying this modernized codebase.

## 1. GTK Signal Handling (The `glib::clone!` Pattern)
GTK 4 widgets in Rust require explicit cloning for use in closures.
- **Rules**:
  - Use the built-in `glib::clone!` macro.
  - Use `@strong` for objects you need to keep alive, and `@weak` for widgets that might be destroyed.
- **Example**:
  ```rust
  button.connect_clicked(glib::clone!(@weak window => move |_| {
      window.present();
  }));
  ```

## 2. Advanced LaTeX Rendering
The preview pipeline uses WebKit 6 and KaTeX.
- **Pipeline**: `Raw LaTeX/MD` -> `Comrak (math_dollars enabled)` -> `Regex (Custom math wrappers)` -> `WebKit 6 (Dark/Light aware CSS)`.
- **Files**: `src/preview.rs`

## 3. Libadwaita & GNOME HIG
Always prioritize the Libadwaita widget set for a professional appearance.
- **Widgets**: Use `adw::HeaderBar`, `adw::Application`, `adw::WindowTitle` instead of plain GTK equivalents where possible.
- **Styling**: Use standard Adwaita CSS classes like `suggested-action`, `pill`, etc.

## 4. UI/Main Thread async integration
- **Pattern**: Use `glib::MainContext::default().spawn_local` for spawning async tasks that need to interact with UI directly without blocking the main loop.
- **AI Integration**: AI responses are fetched async and use `MainContext` to update the `sourceview5::Buffer`.

## 5. Ollama AI Client
- **Model**: Defaulting to `qwen2.5-coder:3b` for superior LaTeX and coding capabilities.
- **Interface**: Uses `thiserror` and `anyhow` for robust error reporting.
