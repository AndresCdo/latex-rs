use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: String,
    pub active_model: String,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub active_provider: String,
    pub providers: Vec<ProviderConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_provider: "Ollama".to_string(),
            providers: vec![
                ProviderConfig {
                    name: "Ollama".to_string(),
                    api_key: None,
                    base_url: "http://localhost:11434".to_string(),
                    active_model: "qwen2.5:0.5b".to_string(),
                    system_prompt: None,
                },
                ProviderConfig {
                    name: "DeepSeek".to_string(),
                    api_key: None,
                    base_url: "https://api.deepseek.com/v1".to_string(),
                    active_model: "deepseek-reasoner".to_string(),
                    system_prompt: None,
                },
                ProviderConfig {
                    name: "OpenAI".to_string(),
                    api_key: None,
                    base_url: "https://api.openai.com/v1".to_string(),
                    active_model: "gpt-4o".to_string(),
                    system_prompt: None,
                },
            ],
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("latex-rs");
        path
    }

    pub fn config_file() -> PathBuf {
        let mut path = Self::config_dir();
        path.push("config.toml");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_file();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        let default = Self::default();
        let _ = default.save();
        default
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(Self::config_file(), content)?;
        Ok(())
    }

    pub fn get_active_provider(&self) -> Option<&ProviderConfig> {
        self.providers
            .iter()
            .find(|p| p.name == self.active_provider)
    }
}
