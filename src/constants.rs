//! Application-wide constants for latex-rs.
//!
//! Centralizes all magic numbers and configuration values to improve maintainability
//! and make the codebase self-documenting.

use std::time::Duration;

// ============================================================================
// Application Identity
// ============================================================================

/// GTK Application ID following reverse-DNS convention.
pub const APP_ID: &str = "com.github.latex-rs";

/// Application name displayed in window title.
pub const APP_NAME: &str = "LaTeX.rs Editor";

// ============================================================================
// Window Configuration
// ============================================================================

/// Default window width in pixels.
pub const DEFAULT_WINDOW_WIDTH: i32 = 1200;

/// Default window height in pixels.
pub const DEFAULT_WINDOW_HEIGHT: i32 = 800;

// ============================================================================
// Editor Configuration
// ============================================================================

// ============================================================================
// LaTeX Compilation
// ============================================================================

/// Maximum allowed LaTeX document size in bytes (10 MB).
/// Prevents DoS attacks via excessively large documents.
pub const MAX_LATEX_SIZE_BYTES: usize = 10 * 1024 * 1024;

/// Timeout for pdflatex and pdftocairo commands in seconds.
/// Prevents hung processes from blocking the application.
pub const COMPILE_TIMEOUT_SECS: u64 = 30;

/// Polling interval for process timeout checking (milliseconds).
/// Balances responsiveness vs CPU usage.
pub const PROCESS_POLL_INTERVAL_MS: u64 = 100;

/// Small delay to allow filesystem to flush SVG files (milliseconds).
/// Addresses rare timing issues on some filesystems.
pub const FS_FLUSH_DELAY_MS: u64 = 10;

// ============================================================================
// Compilation Queue
// ============================================================================

/// Buffer size for the compilation queue channel.
/// Size of 1 ensures only one compilation runs at a time,
/// with new requests replacing pending ones.
pub const COMPILATION_QUEUE_BUFFER: usize = 1;

// ============================================================================
// AI Client (Ollama)
// ============================================================================

/// Base URL for Ollama API.
pub const OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// HTTP request timeout for AI operations.
pub const AI_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum number of retry attempts for AI patch operations.
pub const AI_MAX_PATCH_ATTEMPTS: u32 = 3;

/// AI model temperature for LaTeX generation (lower = more deterministic).
pub const AI_TEMPERATURE: f64 = 0.2;

/// AI model top_p for nucleus sampling.
pub const AI_TOP_P: f64 = 0.9;

/// AI model random seed for reproducible outputs.
pub const AI_SEED: u64 = 42;

/// List of AI models to try in order of preference.
pub const AI_MODEL_PRIORITY: &[&str] = &["qwen3:0.6b", "qwen2.5-coder:3b", "llama3:8b", "mistral"];

// ============================================================================
// Security
// ============================================================================

/// Environment variable to check for WSL detection.
pub const WSL_INTEROP_ENV: &str = "WSL_INTEROP";

/// WebKit sandbox disable environment variable (legacy/broad).
pub const WEBKIT_SANDBOX_DISABLE_VAR: &str = "WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS";

/// WebKit sandbox disable environment variable (modern).
pub const WEBKIT_SANDBOX_DISABLE_VAR_MODERN: &str = "WEBKIT_DISABLE_SANDBOX";

// ============================================================================
// UI Configuration
// ============================================================================

/// Default editor font family.
pub const DEFAULT_EDITOR_FONT: &str = "Monospace";

/// Default editor font size in points.
pub const DEFAULT_EDITOR_FONT_SIZE: i32 = 12;

/// Default zoom level (1.0 = 100%).
pub const DEFAULT_ZOOM_LEVEL: f64 = 1.0;

/// Minimum zoom level.
pub const MIN_ZOOM_LEVEL: f64 = 0.5;

/// Maximum zoom level.
pub const MAX_ZOOM_LEVEL: f64 = 3.0;

/// Zoom step for keyboard shortcuts.
pub const ZOOM_STEP: f64 = 0.1;
