use crate::api::AiProvider;
use crate::config::AppConfig;
use crate::preview::Preview;
use std::path::PathBuf;
use std::sync::Arc;

/// Central application state holding shared data and clients.
pub struct AppState {
    /// Currently open file path, if any.
    pub current_file: Option<PathBuf>,
    /// Active AI Provider.
    pub ai_provider: Option<Arc<dyn AiProvider>>,
    /// Application configuration.
    pub config: AppConfig,
    /// LaTeX preview generator.
    pub preview_generator: Preview,
    /// Current zoom level for the text editor.
    pub editor_zoom: f64,
    /// Current zoom level for the WebKit preview.
    pub preview_zoom: f64,
}
