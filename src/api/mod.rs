use serde::{Deserialize, Serialize};
use thiserror::Error;
use async_trait::async_trait;
use std::sync::Arc;
use crate::config::ProviderConfig;
use futures::Stream;
use std::pin::Pin;

pub mod ollama;
pub mod openai_compat;

use crate::api::ollama::OllamaProvider;
use crate::api::openai_compat::OpenAiCompatibleProvider;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("API response error: {0}")]
    Response(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Stream error: {0}")]
    Stream(String),
}

pub type AiStream = Pin<Box<dyn Stream<Item = Result<AiChunk, ApiError>> + Send>>;

#[derive(Debug, Clone)]
pub enum AiChunk {
    Content(String),
    Reasoning(String),
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn chat_stream(&self, messages: Vec<Message>) -> Result<AiStream, ApiError>;
    async fn check_availability(&self) -> Result<(), ApiError>;
}

pub fn create_provider(config: &ProviderConfig) -> Arc<dyn AiProvider> {
    match config.name.as_str() {
        "Ollama" => Arc::new(OllamaProvider::new(
            config.active_model.clone(),
            config.base_url.clone(),
        )),
        _ => Arc::new(OpenAiCompatibleProvider::new(
            config.name.clone(),
            config.active_model.clone(),
            config.base_url.clone(),
            config.api_key.clone(),
        )),
    }
}
