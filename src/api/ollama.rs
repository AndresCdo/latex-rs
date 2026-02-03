use crate::api::{AiChunk, AiProvider, AiStream, ApiError, Message};
use crate::constants::{AI_REQUEST_TIMEOUT, AI_SEED, AI_TEMPERATURE, AI_TOP_P};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

pub struct OllamaProvider {
    client: Client,
    pub model: String,
    pub base_url: String,
}

impl OllamaProvider {
    pub fn new(model: String, base_url: String) -> Self {
        let client = Client::builder()
            .timeout(AI_REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            client,
            model,
            base_url,
        }
    }
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
    #[serde(default)]
    reasoning: Option<String>,
}

#[derive(Deserialize)]
struct OllamaTags {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[async_trait]
impl AiProvider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }

    async fn check_availability(&self) -> Result<(), ApiError> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(ApiError::Response(format!(
                "Ollama returned status {}",
                response.status()
            )));
        }

        let tags: OllamaTags = response.json().await?;
        if tags
            .models
            .iter()
            .any(|m| m.name == self.model || m.name.starts_with(&format!("{}:", self.model)))
        {
            Ok(())
        } else {
            Err(ApiError::Response(format!(
                "Model {} not found in Ollama",
                self.model
            )))
        }
    }

    async fn chat_stream(&self, messages: Vec<Message>) -> Result<AiStream, ApiError> {
        let url = format!("{}/api/chat", self.base_url);
        let response = self
            .client
            .post(url)
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "options": {
                    "temperature": AI_TEMPERATURE,
                    "top_p": AI_TOP_P,
                    "seed": AI_SEED
                }
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Response(format!(
                "Ollama chat error ({}): {}",
                status, body
            )));
        }

        let stream = response
            .bytes_stream()
            .map(|item| item.map_err(ApiError::HttpClient))
            .scan(Vec::new(), |buffer, item| {
                let res = match item {
                    Ok(bytes) => {
                        buffer.extend_from_slice(&bytes);
                        let mut chunks = Vec::new();
                        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                            let line: Vec<u8> = buffer.drain(..=pos).collect();
                            if let Ok(chunk) = serde_json::from_slice::<OllamaChatResponse>(&line) {
                                if let Some(r) = chunk.message.reasoning {
                                    chunks.push(Ok(AiChunk::Reasoning(r)));
                                }
                                if !chunk.message.content.is_empty() {
                                    chunks.push(Ok(AiChunk::Content(chunk.message.content)));
                                }
                            }
                        }
                        Some(futures::stream::iter(chunks))
                    }
                    Err(e) => Some(futures::stream::iter(vec![Err(e)])),
                };
                futures::future::ready(res)
            })
            .flatten();

        Ok(Box::pin(stream))
    }
}
