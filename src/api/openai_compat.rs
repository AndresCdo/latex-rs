use crate::api::{AiChunk, AiProvider, AiStream, ApiError, Message};
use crate::constants::{AI_REQUEST_TIMEOUT, AI_SEED, AI_TEMPERATURE, AI_TOP_P};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

pub struct OpenAiCompatibleProvider {
    client: Client,
    pub name: String,
    pub model: String,
    pub base_url: String,
    pub api_key: Option<String>,
}

impl OpenAiCompatibleProvider {
    pub fn new(name: String, model: String, base_url: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(AI_REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            client,
            name,
            model,
            base_url,
            api_key,
        }
    }
}

#[derive(Deserialize)]
struct OpenAiStreamResponse {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
}

#[derive(Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[async_trait]
impl AiProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check_availability(&self) -> Result<(), ApiError> {
        if self.api_key.is_none() {
            return Err(ApiError::Config("API Key is missing".to_string()));
        }

        let url = format!("{}/models", self.base_url);
        let mut request = self.client.get(url);

        if let Some(ref key) = self.api_key {
            request = request.bearer_auth(key);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ApiError::Response(format!(
                "API returned status {}",
                response.status()
            )))
        }
    }

    async fn chat_stream(&self, messages: Vec<Message>) -> Result<AiStream, ApiError> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut request = self.client.post(url);

        if let Some(ref key) = self.api_key {
            request = request.bearer_auth(key);
        }

        let response = request
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "temperature": AI_TEMPERATURE,
                "top_p": AI_TOP_P,
                "seed": AI_SEED
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Response(format!(
                "API error ({}): {}",
                status, body
            )));
        }

        let stream = response
            .bytes_stream()
            .map(|item| item.map_err(ApiError::HttpClient))
            .filter_map(|item| async move {
                match item {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let mut chunks = Vec::new();
                        for line in text.lines() {
                            if line.is_empty() || line == "data: [DONE]" {
                                continue;
                            }
                            if let Some(json_str) = line.strip_prefix("data: ") {
                                if let Ok(chunk) = serde_json::from_str::<OpenAiStreamResponse>(json_str) {
                                    if let Some(choice) = chunk.choices.first() {
                                        if let Some(content) = &choice.delta.content {
                                            chunks.push(Ok(AiChunk::Content(content.clone())));
                                        }
                                        if let Some(reasoning) = &choice.delta.reasoning_content {
                                            chunks.push(Ok(AiChunk::Reasoning(reasoning.clone())));
                                        }
                                    }
                                }
                            }
                        }
                        if chunks.is_empty() {
                            None
                        } else {
                            Some(futures::stream::iter(chunks))
                        }
                    }
                    Err(e) => Some(futures::stream::iter(vec![Err(e)])),
                }
            })
            .flatten();

        Ok(Box::pin(stream))
    }
}
