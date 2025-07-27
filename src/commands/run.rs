use crate::core::storage::{decrypt_full_prompt, AppCtx};
use llm::{
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};
use regex::Regex;
use spinners::{Spinner, Spinners};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

/// Execute a prompt with an LLM and print the response.
pub async fn run(
    ctx: &AppCtx,
    id: &str,
    backend: &str,
    vars: &[String],
) -> Result<(), String> {
    let mut map = HashMap::new();
    for v in vars {
        if let Some((key, value)) = v.split_once('=') {
            map.insert(key.trim(), value.trim());
        }
    }

    let path = ctx.prompt_path(id);
    if !path.exists() {
        return Err(format!("No prompt with ID '{}'", id));
    }

    let pd = decrypt_full_prompt(&path, &ctx.cipher)?;

    let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    let rendered = re
        .replace_all(&pd.content, |caps: &regex::Captures| {
            map.get(&caps[1]).copied().unwrap_or("").to_string()
        })
        .to_string();

    let (provider_str, model) = backend
        .split_once(':')
        .ok_or("Invalid backend format. Use 'provider:model'")?;
    let provider =
        LLMBackend::from_str(provider_str).map_err(|_| format!("Unknown provider: {}", provider_str))?;

    let api_key_env_var = match provider {
        LLMBackend::OpenAI => "OPENAI_API_KEY",
        LLMBackend::Anthropic => "ANTHROPIC_API_KEY",
        LLMBackend::Google => "GOOGLE_API_KEY",
        LLMBackend::Groq => "GROQ_API_KEY",
        LLMBackend::Ollama => "OLLAMA_API_KEY",
        LLMBackend::XAI => "XAI_API_KEY",
        LLMBackend::Cohere => "COHERE_API_KEY",
        LLMBackend::DeepSeek => "DEEPSEEK_API_KEY",
        LLMBackend::Mistral => "MISTRAL_API_KEY",
        _ => return Err("Provider not yet supported for direct CLI execution.".to_string()),
    };

    let api_key = env::var(api_key_env_var)
        .map_err(|_| format!("API key env var '{}' not found.", api_key_env_var))?;

    let llm = LLMBuilder::new()
        .backend(provider)
        .api_key(api_key)
        .model(model)
        .build()
        .map_err(|e| e.to_string())?;

    let mut sp = Spinner::new(Spinners::Dots9, "Waiting for LLM response...".into());

    let messages = vec![ChatMessage::user().content(&rendered).build()];
    let response = llm.chat(&messages).await.map_err(|e| e.to_string())?;
    let result = response.text().unwrap_or_default();

    sp.stop_with_message("✔ Response received.".into());
    println!("\n{}", result);

    Ok(())
}