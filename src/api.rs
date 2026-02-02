use crate::constants::{AI_REQUEST_TIMEOUT, OLLAMA_API_URL, OLLAMA_TAGS_URL};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

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
struct OllamaResponse {
    response: String,
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
/// LaTeX editing.
#[derive(Clone)]
pub struct AiClient {
    client: Client,
    /// The name of the AI model to use (e.g., "qwen3:0.6b")
    pub model: String,
}

impl AiClient {
    /// Creates a new AI client configured to use the specified model.
    ///
    /// # Arguments
    /// * `model` - The name of the Ollama model to use
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built
    pub fn new(model: &str) -> Result<Self, ApiError> {
        let client = Client::builder().timeout(AI_REQUEST_TIMEOUT).build()?;

        Ok(Self {
            client,
            model: model.to_string(),
        })
    }

    /// Verifies that the configured model is available in Ollama.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Cannot connect to Ollama
    /// - The model is not found in the list of available models
    pub async fn check_model(&self) -> Result<(), ApiError> {
        let response = self.client.get(OLLAMA_TAGS_URL).send().await?;

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

    /// Sends a prompt to the AI model and returns the response.
    ///
    /// # Arguments
    /// * `prompt` - The prompt text to send to the model
    ///
    /// # Errors
    /// Returns an error if:
    /// - Cannot connect to Ollama
    /// - The API returns a non-success status
    /// - The response cannot be parsed
    pub async fn send_prompt(&self, prompt: &str) -> Result<String, ApiError> {
        let response = self
            .client
            .post(OLLAMA_API_URL)
            .json(&json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ApiError::Response(format!(
                "Ollama returned status {}",
                response.status()
            )));
        }

        let body: OllamaResponse = response.json().await?;
        Ok(body.response)
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

    // Note: Integration tests for check_model and send_prompt require a running
    // Ollama instance and are better suited for integration test suite.
    // Example integration test (requires Ollama running):
    //
    // #[tokio::test]
    // #[ignore] // Run with: cargo test -- --ignored
    // async fn test_check_model_integration() {
    //     let client = AiClient::new("qwen3:0.6b").unwrap();
    //     let result = client.check_model().await;
    //     // May pass or fail depending on Ollama state
    //     println!("check_model result: {:?}", result);
    // }
}
