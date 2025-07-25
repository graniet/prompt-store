use crate::core::storage::{AppCtx, PromptData};
use aes_gcm::aead::Aead;
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use std::collections::HashMap;
use std::fs;

/// Render a template prompt with variables.
pub fn run(ctx: &AppCtx, id: &str, vars: &[String]) -> Result<(), String> {
    let mut map = HashMap::new();
    for v in vars {
        let parts: Vec<&str> = v.splitn(2, '=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].trim(), parts[1].trim());
        }
    }

    let path = ctx.prompt_path(id);
    if !path.exists() {
        return Err(format!("No prompt with ID {}", id));
    }

    let encoded = fs::read_to_string(&path).map_err(|e| format!("Read error: {}", e))?;
    let decoded = general_purpose::STANDARD
        .decode(encoded.trim_end())
        .map_err(|_| "Corrupted data".to_string())?;
    if decoded.len() < 12 {
        return Err("Corrupted data".to_string());
    }

    let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
    let plaintext = ctx
        .cipher
        .decrypt(aes_gcm::Nonce::from_slice(nonce_bytes), cipher_bytes)
        .map_err(|_| "Decrypt error".to_string())?;
    let pd: PromptData =
        serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())?;

    let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    let rendered = re.replace_all(&pd.content, |caps: &regex::Captures| {
        map.get(&caps[1]).copied().unwrap_or("").to_string()
    });

    println!("{}", rendered);
    Ok(())
}
