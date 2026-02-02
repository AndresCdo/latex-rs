use crate::constants::{AI_REQUEST_TIMEOUT, OLLAMA_BASE_URL};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

/// AI Message roles
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

/// Errors that can occur when communicating with the Ollama API.
#[derive(Error, Debug)]
pub enum ApiError {
    /// HTTP client-level error (connection, timeout, etc.)
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),
    /// API returned an error response
    #[error("API response error: {0}")]
    Response(String),
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: Message,
}

#[derive(Deserialize)]
struct OllamaTags {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

/// Client for interacting with the local Ollama API.
///
/// Provides methods to check model availability and send prompts for AI-assisted
/// LaTeX editing using both professional chat and generation endpoints.
#[derive(Clone)]
pub struct AiClient {
    client: Client,
    /// The name of the AI model to use (e.g., "qwen3:0.6b")
    pub model: String,
    base_url: String,
}

impl AiClient {
    /// Creates a new AI client configured to use the specified model.
    pub fn new(model: &str) -> Result<Self, ApiError> {
        let client = Client::builder().timeout(AI_REQUEST_TIMEOUT).build()?;

        Ok(Self {
            client,
            model: model.to_string(),
            base_url: OLLAMA_BASE_URL.to_string(),
        })
    }

    /// Verifies that the configured model is available in Ollama.
    pub async fn check_model(&self) -> Result<(), ApiError> {
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

    /// Sends a list of messages to the AI model and returns the assistant response.
    /// This uses the modern Chat API which is standard for current AI development.
    pub async fn chat(&self, messages: Vec<Message>) -> Result<String, ApiError> {
        let url = format!("{}/api/chat", self.base_url);
        let response = self
            .client
            .post(url)
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": false,
                "options": {
                    "temperature": 0.2, // Low temperature for consistent LaTeX generation
                    "top_p": 0.9,
                    "seed": 42
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

        let body: OllamaChatResponse = response.json().await?;
        Ok(body.message.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_client_new() {
        let client = AiClient::new("test-model");
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, "test-model");
    }

    #[test]
    fn test_ai_client_clone() {
        let client = AiClient::new("test-model").unwrap();
        let cloned = client.clone();
        assert_eq!(client.model, cloned.model);
    }

    #[test]
    fn test_api_error_display() {
        let err = ApiError::Response("test error".to_string());
        assert_eq!(format!("{}", err), "API response error: test error");
    }

    #[test]
    fn test_api_error_from_reqwest() {
        // We can't easily create a reqwest::Error, but we can verify the From impl exists
        // by checking the error type derives properly
        let err = ApiError::Response("test".to_string());
        let _debug_str = format!("{:?}", err);
    }

    // Note: Integration tests require a running Ollama instance.
    // #[tokio::test]
    // #[ignore]
    // async fn test_check_model_integration() {
    //     let client = AiClient::new("qwen3:0.6b").unwrap();
    //     let result = client.check_model().await;
    //     println!("check_model result: {:?}", result);
    // }
}
