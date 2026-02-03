# Contributing to LaTeX-rs

Thank you for your interest in contributing to `latex-rs`! We welcome contributions of all kinds, from bug fixes to new features and documentation improvements.

## Development Environment Setup

Before you begin, ensure you have the required system dependencies installed. Refer to [TOOLS.md](TOOLS.md) for a detailed list for your operating system.

Basic requirements:
- Rust 1.75 or later
- GTK 4, Libadwaita 1.5+, SourceView 5, and WebKit 6
- `pdflatex` (TeX Live or MiKTeX)
- `pdftocairo` (Poppler Utils)
- `ollama` (for AI feature development)

## Workflow

1.  **Fork and Clone**: Create your own fork of the repository and clone it locally.
2.  **Branching**: Create a new branch for your work. Use descriptive names like `feature/sidebar-search` or `fix/crash-on-save`.
3.  **Code Style**: 
    - Follow standard Rust formatting using `cargo fmt`.
    - Adhere to the patterns defined in [SKILL.md](SKILL.md), especially regarding GTK signal handlers and async safety.
    - Avoid introducing new external dependencies unless strictly necessary.
4.  **Testing**:
    - Add unit tests for any new logic in `utils.rs`, `api/`, or `queue.rs`.
    - Run existing tests with `cargo test`.
    - Manually verify UI changes by running the application.
5.  **Commit Messages**: Write clear, concise commit messages. Use the imperative mood (e.g., "Add search highlighting" instead of "Added search highlighting").
6.  **Pull Request**: Submit a PR to the `main` branch. Provide a detailed description of your changes and why they are necessary.

## Coding Standards

- **Memory Safety**: Use `glib::clone!` with `#[weak]` references for all GTK signal handlers that access outer scope widgets.
- **UI Consistency**: Follow the GNOME Human Interface Guidelines (HIG). Use Libadwaita widgets where possible.
- **Constants**: Place all magic numbers, timeouts, and configuration strings in `src/constants.rs`.
- **Documentation**: Use doc comments (`///`) for public functions and modules.

## Code of Conduct

Please be respectful and professional in all interactions. We aim to foster a welcoming and inclusive community.
