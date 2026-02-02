# Agentic Skills & Patterns: latex-rs

Specific technical patterns to follow when modifying this modernized codebase.

## 1. GTK Signal Handling (The `glib::clone!` Pattern)

GTK 4 widgets in Rust require explicit cloning for use in closures.

**Rules**:

- Use the built-in `glib::clone!` macro
- Use `@strong` for objects you need to keep alive
- Use `@weak` for widgets that might be destroyed (prevents use-after-free)

**Example**:

```rust
button.connect_clicked(glib::clone!(
    #[weak]
    window,
    #[strong]
    state,
    move |_| {
        window.present();
        state.borrow_mut().do_something();
    }
));
```

## 2. LaTeX Rendering Pipeline

The preview system uses a native LaTeX compilation pipeline:

```
LaTeX Source → pdflatex → PDF → pdftocairo → SVG → WebKit 6
```

**Security flags**:

- `-no-shell-escape`: Prevents `\write18` command execution

**Files**: [preview.rs](src/preview.rs)

## 3. Centralized Configuration

All magic numbers and configuration values are in [constants.rs](src/constants.rs).

**Pattern**:

```rust
use crate::constants::{COMPILE_TIMEOUT_SECS, MAX_LATEX_SIZE_BYTES};

// Use constants instead of magic numbers
if latex.len() > MAX_LATEX_SIZE_BYTES { ... }
```

**Key constants**:

| Constant | Value | Purpose |
|----------|-------|---------|
| `PREVIEW_DEBOUNCE_MS` | 250 | Delay before preview update |
| `COMPILE_TIMEOUT_SECS` | 30 | Max time for pdflatex |
| `MAX_LATEX_SIZE_BYTES` | 10MB | Input size limit |
| `AI_REQUEST_TIMEOUT` | 60s | Ollama API timeout |

## 4. Libadwaita & GNOME HIG

Always prioritize the Libadwaita widget set for a professional appearance.

**Widgets**:

- Use `adw::HeaderBar`, `adw::Application`, `adw::WindowTitle`
- Use `adw::Toast` for notifications
- Use `adw::ToastOverlay` as window child

**Styling**:

- Use standard Adwaita CSS classes: `suggested-action`, `destructive-action`, `pill`
- Avoid custom CSS when Adwaita classes exist

## 5. Async/Await with GTK

**Pattern**: Use `glib::MainContext::default().spawn_local` for async tasks that need UI access.

```rust
let ctx = glib::MainContext::default();
ctx.spawn_local(glib::clone!(
    #[weak]
    buffer,
    async move {
        let result = some_async_operation().await;
        buffer.set_text(&result);  // Safe UI update
    }
));
```

**Blocking operations**: Use `tokio::task::spawn_blocking` for CPU-intensive work:

```rust
let html = tokio::task::spawn_blocking(move || preview.render(&latex))
    .await
    .unwrap_or_else(|e| format!("Error: {}", e));
```

## 6. Error Handling

**Pattern**: Use `thiserror` for library errors, `anyhow` for application errors.

```rust
// In api.rs - library-style error
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),
}

// In utils.rs - application-style error
pub fn open_file(path: &Path) -> Result<String> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open: {:?}", path))?;
    // ...
}
```

## 7. Compilation Queue

The queue ensures thread-safe LaTeX compilation:

- Single-item buffer prevents concurrent compilations
- `try_send` drops new requests if queue is full
- Worker handle stored for graceful shutdown

**File**: [queue.rs](src/queue.rs)

## 8. Testing Patterns

**Unit tests**: Place in same file under `#[cfg(test)]` module.

**Naming**: Use descriptive names like `test_apply_patch_with_markdown_wrapper`.

**GTK limitation**: Tests requiring GTK initialization need integration test setup.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name_scenario() {
        // Arrange
        let input = "...";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert!(result.is_ok());
    }
}
```

## 9. Ollama AI Client

**Model priority**: Configured in `constants::AI_MODEL_PRIORITY`.

**Error handling**: Always handle connection failures gracefully.

**File**: [api.rs](src/api.rs)
