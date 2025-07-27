//! Manages the loading of LLM provider configurations.

use llm::builder::{LLMBackend, LLMBuilder};
use llm::chain::LLMRegistry;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default)]
    providers: HashMap<String, ProviderConfig>,
}

#[derive(Deserialize, Debug)]
struct ProviderConfig {
    backend: String,
    model: String,
    api_key_env: Option<String>,
    base_url: Option<String>,
}

/// Loads the LLM provider configurations from `~/.prompt-store/config.toml`
/// and builds an LLMRegistry.
pub fn load_llm_registry() -> Result<LLMRegistry, String> {
    let home = env::var("HOME").map_err(|_| "Unable to determine HOME directory".to_string())?;
    let config_path = PathBuf::from(home)
        .join(".prompt-store")
        .join("config.toml");

    if !config_path.exists() {
        // Return an empty registry if no config file is found, commands will warn the user.
        return Ok(LLMRegistry::new());
    }

    let config_content =
        fs::read_to_string(config_path).map_err(|e| format!("Failed to read config.toml: {}", e))?;
    let config: Config =
        toml::from_str(&config_content).map_err(|e| format!("Failed to parse config.toml: {}", e))?;

    let mut registry = LLMRegistry::new();

    for (name, provider_conf) in config.providers {
        let backend = LLMBackend::from_str(&provider_conf.backend)
            .map_err(|_| format!("Invalid backend '{}' for provider '{}'", provider_conf.backend, name))?;

        let api_key_env_var = provider_conf.api_key_env.unwrap_or_else(|| match backend {
            LLMBackend::OpenAI => "OPENAI_API_KEY".to_string(),
            LLMBackend::Anthropic => "ANTHROPIC_API_KEY".to_string(),
            _ => "".to_string(),
        });

        let api_key = if !api_key_env_var.is_empty() {
            env::var(&api_key_env_var).map_err(|_| {
                format!(
                    "Environment variable '{}' not set for provider '{}'",
                    api_key_env_var, name
                )
            })?
        } else {
            "".to_string() // Some backends like Ollama don't require a key
        };

        let mut builder = LLMBuilder::new()
            .backend(backend)
            .model(&provider_conf.model);
        
        if !api_key.is_empty() {
            builder = builder.api_key(api_key);
        }
        if let Some(base_url) = provider_conf.base_url {
            builder = builder.base_url(base_url);
        }

        let provider = builder.build().map_err(|e| e.to_string())?;
        registry.insert(&name, provider);
    }

    Ok(registry)
}