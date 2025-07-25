use crate::core::storage::{AppCtx, PromptData};
use aes_gcm::aead::Aead;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use copypasta::{ClipboardContext, ClipboardProvider};
use std::fs;

/// Copy prompt content to clipboard.
pub fn run(ctx: &AppCtx, id: &str) -> Result<(), String> {
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

    let mut ctx_clip = ClipboardContext::new().map_err(|e| format!("Clipboard error: {}", e))?;
    ctx_clip
        .set_contents(pd.content.clone())
        .map_err(|e| format!("Clipboard set error: {}", e))?;

    println!("{} copied to clipboard", style("•").green().bold());
    Ok(())
}
