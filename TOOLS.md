# Developer Tools & Workflow: latex-rs

## Development Environment

- **OS**: Linux (Primary), macOS, Windows (via GTK).
- **Toolchain**: `rustc` 1.75+ (Rust 2021 Edition)
- **System Dependencies**:
  - `libgtk-4-dev` - GTK 4 development libraries
  - `libadwaita-1-dev` - Libadwaita for GNOME styling
  - `libgtksourceview-5-dev` - Source code editing widget
  - `libwebkitgtk-6.0-dev` - WebKit for HTML preview

### Ubuntu/Debian

```shell
sudo apt install libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev libwebkitgtk-6.0-dev
```

### Fedora

```shell
sudo dnf install gtk4-devel libadwaita-devel gtksourceview5-devel webkitgtk6.0-devel
```

### Arch Linux

```shell
sudo pacman -S gtk4 libadwaita gtksourceview5 webkitgtk-6.0
```

### macOS (Homebrew)

```shell
brew install gtk4 libadwaita gtksourceview5 webkitgtk6
```

## Common Commands

| Command | Description |
|---------|-------------|
| `cargo run` | Run in debug mode |
| `cargo run --release` | Run optimized build |
| `cargo build --release` | Build release binary |
| `cargo check` | Fast syntax/type check |
| `cargo clippy -- -D warnings` | Lint with strict warnings |
| `cargo test` | Run unit tests |
| `cargo fmt` | Format code |

## Environment Setup for Testing

1. **LaTeX**: Required for preview functionality.

   ```shell
   # Ubuntu/Debian
   sudo apt install texlive-latex-base texlive-latex-extra poppler-utils
   ```

2. **Ollama**: Required for AI features.

   ```shell
   # Install Ollama
   curl -fsSL https://ollama.ai/install.sh | sh
   
   # Start server (background)
   ollama serve &
   
    # Pull recommended model
    ollama pull qwen2.5:0.5b

   ```

3. **Display**: Requires an active X11 or Wayland session for GTK to initialize.

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Missing GTK libraries | Ensure `pkg-config` can find `.pc` files for gtk4, gtksourceview-5, webkit2gtk-6.0 |
| WebView blank | Check if HTML in `preview.rs` is valid; WebKit may silently fail on errors |
| "bwrap: Permission denied" | Running in WSL/container; app auto-detects and disables sandbox |
| AI not responding | Ensure `ollama serve` is running and model is pulled |
| Preview not updating | Check terminal for LaTeX compilation errors |

## Project Structure

```
src/
├── main.rs       # Application entry point and UI
├── constants.rs  # Centralized configuration constants
├── preview.rs    # LaTeX → PDF → SVG pipeline
├── api/          # AI provider implementations
├── queue.rs      # Compilation queue (thread-safe)
└── utils.rs      # File I/O and patch utilities
```
