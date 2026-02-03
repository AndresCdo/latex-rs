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
}

pub type AiStream = Pin<Box<dyn Stream<Item = Result<AiChunk, ApiError>> + Send>>;

#[derive(Debug, Clone)]
pub enum AiChunk {
    Content(String),
    Reasoning(String),
}

pub struct ThinkingFilter {
    inside_think: bool,
    buffer: String,
}

impl ThinkingFilter {
    pub fn new() -> Self {
        Self {
            inside_think: false,
            buffer: String::new(),
        }
    }

    pub fn process(&mut self, text: String) -> Vec<AiChunk> {
        let mut chunks = Vec::new();
        self.buffer.push_str(&text);

        loop {
            if !self.inside_think {
                if let Some(start_pos) = self.buffer.find("<think>") {
                    let content: String = self.buffer.drain(..start_pos).collect();
                    if !content.is_empty() {
                        chunks.push(AiChunk::Content(content));
                    }
                    self.buffer.drain(.."<think>".len());
                    self.inside_think = true;
                } else {
                    // Look for potential start of <think> at the end
                    let mut found_partial = false;
                    let tag = "<think>";
                    // We check from the end of the buffer, but must stay on char boundaries
                    // Actually, for ASCII tags, we can just check the bytes.
                    let bytes = self.buffer.as_bytes();
                    for i in (1..tag.len()).rev() {
                        if bytes.len() >= i {
                            let suffix = &bytes[bytes.len() - i..];
                            if tag.as_bytes().starts_with(suffix) {
                                // Potential partial tag. We must not drain this part.
                                // But we must ensure the split point is a char boundary.
                                let split_pos = bytes.len() - i;
                                if self.buffer.is_char_boundary(split_pos) {
                                    let content: String = self.buffer.drain(..split_pos).collect();
                                    if !content.is_empty() {
                                        chunks.push(AiChunk::Content(content));
                                    }
                                    found_partial = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !found_partial {
                        let content: String = self.buffer.drain(..).collect::<String>();
                        if !content.is_empty() {
                            chunks.push(AiChunk::Content(content));
                        }
                    }
                    break;
                }
            } else {
                if let Some(end_pos) = self.buffer.find("</think>") {
                    let reasoning: String = self.buffer.drain(..end_pos).collect();
                    if !reasoning.is_empty() {
                        chunks.push(AiChunk::Reasoning(reasoning));
                    }
                    self.buffer.drain(.."</think>".len());
                    self.inside_think = false;
                } else {
                    // Look for potential start of </think> at the end
                    let mut found_partial = false;
                    let tag = "</think>";
                    let bytes = self.buffer.as_bytes();
                    for i in (1..tag.len()).rev() {
                        if bytes.len() >= i {
                            let suffix = &bytes[bytes.len() - i..];
                            if tag.as_bytes().starts_with(suffix) {
                                let split_pos = bytes.len() - i;
                                if self.buffer.is_char_boundary(split_pos) {
                                    let reasoning: String = self.buffer.drain(..split_pos).collect();
                                    if !reasoning.is_empty() {
                                        chunks.push(AiChunk::Reasoning(reasoning));
                                    }
                                    found_partial = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !found_partial {
                        let reasoning: String = self.buffer.drain(..).collect::<String>();
                        if !reasoning.is_empty() {
                            chunks.push(AiChunk::Reasoning(reasoning));
                        }
                    }
                    break;
                }
            }
        }
        chunks
    }
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
