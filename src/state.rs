use crate::api::AiClient;
use crate::preview::Preview;
use std::path::PathBuf;

/// Central application state holding shared data and clients.
pub struct AppState {
    /// Currently open file path, if any.
    pub current_file: Option<PathBuf>,
    /// AI Client for Ollama integration.
    pub ai_client: Option<AiClient>,
    /// LaTeX preview generator.
    pub preview_generator: Preview,
    /// Current zoom level for the text editor.
    pub editor_zoom: f64,
    /// Current zoom level for the WebKit preview.
    pub preview_zoom: f64,
}
