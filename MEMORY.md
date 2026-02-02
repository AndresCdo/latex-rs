# Project Memory: latex-rs

This file tracks the project's state, major decisions, and progress to maintain context across agent sessions.

## Current Project Status
- **Core Functionality**: Modern GTK 4 + Libadwaita UI with SourceView 5 editor and WebKit 6 preview.
- **Preview System**: Professional LaTeX rendering via KaTeX with live update and GitHub-style styling.
- **AI Integration**: Enhanced Ollama client with better error handling and modern async patterns.
- **Infrastructure**: Fully modernized codebase following Rust 2021 conventions and professional architecture.

## Major Decisions
- **Modernization**: Upgraded the entire stack from GTK 3/WebKit2GTK to GTK 4/Libadwaita/WebKit6 for a professional GNOME look and feel.
- **Dependency Update**: All crates updated to latest stable versions (2026 standards).
- **Built-in Undo**: Removed custom undo manager in favor of GTK 4's native `TextBuffer` undo/redo system.

## Pending Tasks
- [ ] Add PDF export functionality (likely using `headless_chrome` or `print_to_pdf`).
- [ ] Implement system-wide dark mode sync for the preview pane.
- [ ] Add project-wide configuration file (settings.toml).
- [ ] Internationalization (i18n) support.

## Completed Milestones
- [x] Initial fork and modification from `markdown-rs`.
- [x] Integration of KaTeX auto-render extension.
- [x] Centralization of agentic instructions.
