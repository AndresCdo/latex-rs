# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.5.0] - 2026-02-03

### Added
- **Welcome Page**: New `adw::StatusPage` that appears when no file is open.
- **Search Match Counter**: Real-time "X matches" tracking in the editor search bar.
- **AI History Persistence**: AI instructions are now saved to `config.toml` and can be navigated with Up/Down keys.
- **True Dark Mode**: CSS inversion filters for LaTeX SVGs in the preview pane.
- **Persistent Notifications**: Replaced transient toasts with `adw::Banner` for system status and `adw::AlertDialog` for paper summaries.
- **arXiv Search Sidebar**: Integrated arXiv search for research papers.

### Changed
- Modernized entire stack to GTK 4, Libadwaita 1.5, and WebKit 6.
- Optimized streaming text insertion in GTK editor using `TextMark` ($O(1)$ per chunk).
- Improved Ollama client with robust JSON stream parsing.

### Fixed
- UTF-8 robustness in AI stream handling via multi-byte boundary checking.
- `GtkGizmo` allocation warnings by reordering window presentation logic.
- Hallucination prevention: Added filters for non-standard LaTeX commands.
- Security: All system paths in error messages are now sanitized to `[TEMP_DIR]`.

## [1.4.0] - 2026-02-03

### Added
- **Smart Compilation**: Multi-pass LaTeX compilation (up to 3 passes) for Tables of Contents and cross-references.
- **Biber Integration**: Automatic detection and execution of `biber` for bibliographies.
- **Reasoning UI**: Integrated Thinking Area to handle DeepSeek-style reasoning tags with auto-scrolling `TextView`.

### Changed
- Refactored UI modules into specialized components.

## [1.1.0]

### Added
- Sidebar Outline for document navigation.
- Find/Search UI with match tracking.
- Zoom Support (Keyboard + Scroll).
- PDF Export functionality.

## [1.0.0]

### Added
- Initial release as a GTK-based LaTeX editor.
- Real-time LaTeX preview via `pdflatex`.
- Local AI integration via Ollama.

[1.5.0]: https://github.com/AndresCdo/latex-rs/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/AndresCdo/latex-rs/compare/v1.1.0...v1.4.0
[1.1.0]: https://github.com/AndresCdo/latex-rs/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/AndresCdo/latex-rs/releases/tag/v1.0.0
