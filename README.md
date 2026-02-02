# LaTeX-rs Professional

A high-performance, modern, and professional LaTeX editor with local AI integration and real-time Libadwaita preview.

![screenshot](./screenshot.png)

## Overview

LaTeX-rs has been modernized to provide a world-class scientific writing experience. Built with **Rust**, **GTK 4**, and **Libadwaita**, it follows the latest GNOME HIG (Human Interface Guidelines) to provide a seamless, beautiful, and distraction-free environment for professional researchers and students.

## Features

- **Professional UI**: Native Libadwaita interface with adaptive layouts and modern GNOME styling.
- **Real-time LaTeX Preview**: Ultra-fast rendering via **WebKit 6** and **KaTeX**.
- **Privacy-First AI**: Integrated 100% local AI assistance via **Ollama** (supports Qwen, Llama, and more).
- **Professional Editor**: Advanced syntax highlighting and editing features powered by **SourceView 5**.
- **Native Performance**: Built entirely in Rust for maximum reliability and speed.
- **Modern Standards**: Uses GTK 4, WebKit 6, and follows the latest Rust 2021/2024 conventions.

## Build Requirements

LaTeX-rs uses the latest GTK stack. Ensure you have the following system dependencies installed:

### Ubuntu/Debian
```shell
sudo apt install libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev libwebkitgtk-6.0-dev
```

### Fedora
```shell
sudo dnf install gtk4-devel libadwaita-devel gtksourceview5-devel webkitgtk6.0-devel
```

### macOS (Homebrew)
```shell
brew install gtk4 libadwaita gtksourceview5 webkitgtk6
```

## Installation

1. **Install Ollama**: Download and run [Ollama](https://ollama.ai/) for local AI features.
   ```shell
   ollama pull qwen2.5-coder:3b  # Recommended for LaTeX and Code
   ```

2. **Clone & Run**:
   ```shell
   git clone https://github.com/AndresCdo/latex-rs.git
   cd latex-rs
   cargo run --release
   ```

## AI Capabilities

Unlock the power of local LLMs directly in your editor:
- **Auto-Correction**: Fix LaTeX syntax errors instantly.
- **Scientific Assistance**: Generate complex mathematical equations from natural language.
- **Document Refactoring**: Improve the structure and flow of your scientific papers.

## Architecture

Modernized from the ground up:
- **UI Architecture**: Programmatic GTK 4 with Libadwaita components.
- **Async Runtime**: Powered by **Tokio** and **GLib MainContext** for non-blocking UI.
- **Error Handling**: Robust error reporting using `anyhow` and `thiserror`.
- **Modern Tooling**: Leveraging `tracing` for professional-grade logging.

## Usage

- **Open/Save**: Use toolbar buttons or File menu
- **Real-time preview**: Left pane edits LaTeX, right pane shows rendered output
- **AI Assistant**: Click AI button to ask questions about LaTeX, generate equations, or review content
- **Keyboard shortcuts**: Standard GTK shortcuts apply

## AI Capabilities

The editor integrates with Ollama to provide:

- **Scientific reasoning**: Ask questions about mathematics, physics, or other sciences
- **Equation generation**: Generate LaTeX equations from natural language
- **Paper review**: Get feedback on your LaTeX document structure and content
- **Code explanation**: Understand complex LaTeX packages and macros

## Architecture

Built on the foundation of `markdown-rs`, this editor extends the original markdown editor with:

1. **LaTeX preview system**: Replaces markdown rendering with KaTeX-based LaTeX rendering
2. **AI integration**: Adds Ollama API client for local AI processing
3. **Sourceview integration**: Provides LaTeX syntax highlighting
4. **Modular design**: Components can be reused or replaced independently

## Contributing

Contributions welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- [markdown-rs](https://github.com/nilgradisnik/markdown-rs) for the original editor
- [gtk-rs](https://gtk-rs.org/) for Rust bindings to GTK
- [Ollama](https://ollama.ai/) for local AI inference
- [KaTeX](https://katex.org/) for fast LaTeX rendering
- [OpenAI Prism](https://prism.openai.com/) for inspiration