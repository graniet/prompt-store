use crate::core::storage::{AppCtx, PromptData};
use aes_gcm::{
    aead::{Aead, AeadCore, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use console::style;
use serde_json;
use std::fs;

/// Rename a prompt.
pub fn run(ctx: &AppCtx, id: &str, title: &str) -> Result<(), String> {
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
        .decrypt(Nonce::from_slice(nonce_bytes), cipher_bytes)
        .map_err(|_| "Decrypt error".to_string())?;
    let mut pd: PromptData =
        serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())?;

    pd.title = title.to_string();

    let json = serde_json::to_vec(&pd).map_err(|e| format!("Serialize error: {}", e))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher_bytes = ctx
        .cipher
        .encrypt(&nonce, json.as_ref())
        .map_err(|_| "Encrypt error".to_string())?;

    let mut out = Vec::with_capacity(12 + cipher_bytes.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&cipher_bytes);
    let encoded_out = general_purpose::STANDARD.encode(&out);

    fs::write(&path, encoded_out).map_err(|e| format!("Write error: {}", e))?;
    println!("{} prompt {} renamed", style("•").green().bold(), id);
    Ok(())
}
