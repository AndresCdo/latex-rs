use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use thiserror::Error;

const OLLAMA_API_URL: &str = "http://localhost:11434/api/generate";
const OLLAMA_TAGS_URL: &str = "http://localhost:11434/api/tags";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),
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

#[derive(Clone)]
pub struct AiClient {
    client: Client,
    pub model: String,
}

impl AiClient {
    pub fn new(model: &str) -> Result<Self, ApiError> {
        let client = Client::builder().timeout(REQUEST_TIMEOUT).build()?;

        Ok(Self {
            client,
            model: model.to_string(),
        })
    }

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
