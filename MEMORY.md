# Project Memory: latex-rs

This file tracks the project's state, major decisions, and progress to maintain context across agent sessions.

## Migration Milestones

- **v1.1.0 - The Feature Release**:
  - Integrated Sidebar Outline for navigation.
  - New Find/Search UI with match tracking.
  - Zoom Support (Keyboard + Scroll).
  - PDF Export functionality.
  - CI/CD & Binary Release automation.

## Current Project Status

- **Core Functionality**: Modern GTK 4 + Libadwaita UI with SourceView 5 editor and WebKit 6 preview.
- **Preview System**: Professional LaTeX rendering via pdflatex → pdftocairo → SVG pipeline displayed in WebKit 6.
- **AI Integration**: Enhanced Ollama client with better error handling and modern async patterns.
- **Infrastructure**: Fully modernized codebase following Rust 2021 conventions with centralized configuration.

## Major Decisions

- **Modernization**: Upgraded the entire stack from GTK 3/WebKit2GTK to GTK 4/Libadwaita/WebKit6 for a professional GNOME look and feel.
- **Dependency Update**: All crates updated to latest stable versions (2026 standards).
- **Built-in Undo**: Removed custom undo manager in favor of GTK 4's native `TextBuffer` undo/redo system.
- **Centralized Constants**: All magic numbers and configuration values moved to `src/constants.rs`.

- [x] **v1.4.0 - Smart Pipeline & Reasoning Update (2026-02-03)**:
  - **Smart Compilation**: Implemented multi-pass LaTeX compilation (up to 3 passes) for Tables of Contents and cross-references.
  - **Biber Integration**: Automatic detection and execution of `biber` for bibliographies.
  - **Reasoning UI**: Integrated `ThinkingFilter` to handle DeepSeek-style reasoning tags with auto-scrolling `TextView`.
  - **UTF-8 Robustness**: Fixed "char boundary" panic in AI stream handling via safe multi-byte boundary checking.
  - **Hallucination Prevention**: Added post-processing filters for non-standard LaTeX commands (e.g., `\keywords` fix).
  - **GTK Optimization**: Resolved `GtkGizmo` allocation warnings by reordering window presentation logic.
  - **Dependency Check**: Added `biber` to system requirement verification.

## Completed Milestones

- [x] Initial fork and modification from `markdown-rs`.
- [x] Integration of pdflatex-based preview (replaced KaTeX with native LaTeX compilation).
- [x] Inline AI Assistant prompting UI with dynamic GtkRevealer.
- [x] Standardization on `qwen2.5:0.5b` as default AI model.
- [x] **AI Assistant feature fixes (2026-02-03)**:
  - Implemented robust JSON stream parsing for Ollama and OpenAI providers.
  - Added support for `reasoning` content (DeepSeek/R1 models) in UI.
  - Optimized streaming text insertion in GTK editor using `TextMark` ($O(1)$ per chunk).
  - Wired missing `ai_btn` and `settings_btn` signals in `src/main.rs`.
  - Stored original selection in state to support future undo/diff features.
- [x] **Thinking Area Fixes (2026-02-03)**:
  - Implemented `ThinkingFilter` to extract `<think>` tags from content streams.
  - Replaced `reasoning_label` with `TextView` for better streaming performance and auto-scrolling.
  - Robust partial tag handling for fragmented network chunks.
  - Added dedicated auto-scroll logic using `TextMark`.
- [x] Centralization of agentic instructions.
- [x] **New Features Sprint (Ctrl+Export & Ctrl+Zoom)**:
  - **PDF Export**: Added background PDF generation and file picker.
  - **Zoom Support**: Implemented Ctrl+Scroll and Ctrl+Keyboard zoom logic.
- [x] **Security hardening sprint (2026-02-02)**:
  - **Path sanitization**: All system paths in error messages replaced with `[TEMP_DIR]`
  - **Process timeouts**: 30-second timeout for pdflatex/pdftocairo with automatic termination
  - **Shell escape prevention**: Added `-no-shell-escape` flag to pdflatex

  - **Input size limit**: Documents >10MB rejected to prevent DoS
  - **Conditional sandbox**: WebKit sandbox only disabled when needed (WSL, containers, and Ubuntu 24.04 AppArmor detection)
  - **HTML escaping**: All error messages properly escaped for XSS prevention
  - **Sequential compilation queue**: Single-threaded LaTeX compilation to prevent temp file corruption
  - **Worker handle management**: Proper JoinHandle storage for graceful shutdown
  - **Async callback safety**: Proper use of `#[weak]` references in GTK signal handlers
  - **Content Security Policy**: CSP headers in HTML preview to restrict script execution
  - **Dependency cleanup**: Removed unused dependencies (comrak, regex, once_cell)
- [x] Dark mode sync for preview pane (via CSS media queries).
- [x] **Code quality improvements (2026-02-02)**:
  - Created `src/constants.rs` module with all configuration values
  - Pre-allocated String capacity in SVG wrapper for better performance
  - Added comprehensive unit tests for utils.rs (patch application, LaTeX extraction)
  - Added unit tests for api.rs (client creation, error handling)
  - Updated TOOLS.md with correct GTK 4 dependencies
  - Fixed README.md duplicated sections and incorrect KaTeX reference
  - Improved documentation with doc comments on public APIs

## Security Architecture Decisions

1. **Error Message Sanitization**:
   - All temporary directory paths are replaced with `[TEMP_DIR]` before being shown to users
   - Prevents information leakage about system structure
   - Implemented via `sanitize_paths()` in `preview.rs`

2. **Process Execution Safety**:
   - `run_command_with_timeout()` function enforces 30-second maximum execution time
   - Uses `try_wait()` with polling to avoid platform-specific `wait_timeout` issues
   - Automatically kills stalled processes to prevent DoS
   - Timeout and polling interval configurable via `constants.rs`

3. **LaTeX Compilation Security**:
   - `-no-shell-escape`: Prevents `\write18` command execution
   - `-openin-any=p`: Restricts file access to parent directories only (paranoid mode)
   - Input size validation: Rejects documents >10MB (`MAX_LATEX_SIZE_BYTES`)
   - Single-item MPSC channel ensures sequential compilation

4. **Web Security**:
   - CSP headers: `default-src 'self'; script-src 'none'; style-src 'unsafe-inline';`
   - X-Frame-Options: DENY
   - X-Content-Type-Options: nosniff
   - All user-generated content HTML-escaped via `html_escape::encode_text`

5. **WebKit Sandbox**:
   - Conditional disable only when environment requires it (WSL, containers)
   - Detection via `WSL_INTEROP`, `container` env vars, and marker files
   - Warning logged when sandbox is disabled

6. **GTK Memory Safety**:
   - All async callbacks use `#[weak]` attribute for widget references
   - Prevents use-after-free if widgets are destroyed during async operations
   - GTK's internal reference counting handles cleanup automatically

## Performance Considerations

- **Timeout duration**: 30 seconds chosen as reasonable for complex LaTeX documents
- **Queue size**: Single-item buffer minimizes memory while preventing concurrent execution
- **Polling interval**: 100ms for process timeout checking balances responsiveness vs CPU usage
- **Debounce delay**: 250ms for preview updates prevents excessive recompilation
- **String pre-allocation**: SVG wrapper pre-calculates capacity to reduce allocations
- **Temp file cleanup**: Automatic via `tempfile::tempdir()` destructor

## Test Coverage

| Module | Tests | Coverage |
|--------|-------|----------|
| `utils.rs` | 14 tests | ~80% (file ops, patch, extract) |
| `api.rs` | 4 tests | ~40% (client creation, errors) |
| `preview.rs` | 1 test | ~5% (path sanitization) |
| `queue.rs` | 1 test | ~10% (basic sequential) |

**Total**: 19 tests passing
