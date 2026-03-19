use crate::{anthropic::AnthropicProvider, openai::OpenAIProvider, LanguageModel};
use anyhow::{bail, Result};
use c2_config::Config;
use std::sync::Arc;

pub struct ProviderRegistry {
    model: Arc<dyn LanguageModel>,
}

impl ProviderRegistry {
    pub async fn from_config(config: &Config) -> Result<Self> {
        let provider_cfg = config.provider.as_ref();
        let provider_id = provider_cfg.map(|p| p.id.as_str()).unwrap_or("anthropic");
        let api_key = provider_cfg
            .and_then(|p| p.api_key.as_deref())
            .or_else(|| {
                // fallback to known env vars
                None // will be populated below
            });

        let model_id = config.model.clone();
        let base_url = provider_cfg.and_then(|p| p.base_url.clone());

        let model: Arc<dyn LanguageModel> = match provider_id {
            "anthropic" => {
                let key = api_key
                    .map(|k| k.to_string())
                    .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                    .or_else(|| std::env::var("CC_API_KEY").ok());
                let key = match key {
                    Some(k) if !k.is_empty() => k,
                    _ => bail!("No API key for Anthropic. Set ANTHROPIC_API_KEY or CC_API_KEY."),
                };
                Arc::new(AnthropicProvider::new(key, model_id, base_url))
            }
            "openai" => {
                let key = api_key
                    .map(|k| k.to_string())
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .or_else(|| std::env::var("CC_API_KEY").ok());
                let key = match key {
                    Some(k) if !k.is_empty() => k,
                    _ => bail!("No API key for OpenAI. Set OPENAI_API_KEY or CC_API_KEY."),
                };
                Arc::new(OpenAIProvider::new(key, model_id, base_url))
            }
            // openai-compatible (ollama, vllm, lm-studio, openrouter, …)
            other => {
                let key = api_key
                    .map(|k| k.to_string())
                    .or_else(|| std::env::var("CC_API_KEY").ok())
                    .unwrap_or_else(|| "sk-dummy".to_string());
                let url = base_url.unwrap_or_else(|| "http://localhost:11434/v1/chat/completions".to_string());
                tracing::info!("Using OpenAI-compatible provider '{}' at {}", other, url);
                Arc::new(OpenAIProvider::new(key, model_id, Some(url)))
            }
        };

        Ok(Self { model })
    }

    pub fn model(&self) -> Arc<dyn LanguageModel> {
        self.model.clone()
    }
}
