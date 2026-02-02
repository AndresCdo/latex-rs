# Developer Tools & Workflow: latex-rs

## Development Environment
- **OS**: Linux (Primary), macOS, Windows (via GTK).
- **Toolchain**: `rustc` 1.70+
- **System Deps**: 
  - `libgtk-3-dev`
  - `libgtksourceview-3.0-dev`
  - `libwebkit2gtk-4.0-dev`

## Common Commands
- **Run project**: `cargo run`
- **Build release**: `cargo build --release`
- **Check code**: `cargo check`
- **Lint**: `cargo clippy -- -D warnings`
- **Test (Logic)**: `cargo test`

## Environment Setup for Testing
1. **Ollama**: Must be running for AI features to work.
   - Run `ollama serve` in a background terminal.
   - Run `ollama pull qwen3:0.6b` to ensure the recommended small model is available.
2. **Display**: Requires an active X11 or Wayland session for GTK to initialize.

## Troubleshooting
- **Missing libraries**: Ensure `pkg-config` can find the `.pc` files for gtk-3, gtksourceview-3.0, and webkit2gtk-4.0.
- **WebView Blank**: Check if the HTML generated in `preview.rs` is valid. Use `WebKit2GTK` inspector if possible (right-click the preview area).
