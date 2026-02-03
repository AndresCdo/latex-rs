use crate::api::AiProvider;
use crate::config::AppConfig;
use crate::preview::Preview;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Central application state holding shared data and clients.
pub struct AppState {
    /// Currently open file path, if any.
    pub current_file: Option<PathBuf>,
    /// Active AI Provider.
    pub ai_provider: Option<Arc<dyn AiProvider>>,
    /// AI Cancellation channel.
    pub ai_cancellation: Option<mpsc::Sender<()>>,
    /// Flag to indicate if AI is currently generating text.
    pub is_ai_generating: bool,
    /// Pending suggestion from AI.
    pub pending_suggestion: Option<String>,
    /// Original text that the suggestion would replace.
    pub original_text_selection: Option<String>,
    /// Application configuration.
    pub config: AppConfig,
    /// LaTeX preview generator.
    pub preview_generator: Preview,
    /// Current zoom level for the text editor.
    pub editor_zoom: f64,
    /// Current zoom level for the preview pane.
    pub preview_zoom: f64,
}
