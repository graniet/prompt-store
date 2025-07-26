use crate::core::{
    storage::{AppCtx, PromptData},
    utils::new_id,
};
use aes_gcm::{
    aead::{Aead, AeadCore, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use console::style;
use serde_json;
use std::fs;

/// Import prompts from encrypted file.
pub fn run(ctx: &AppCtx, file: &str) -> Result<(), String> {
    let encoded = fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
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
    let bundle: Vec<PromptData> =
        serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())?;

    for mut pd in bundle {
        let mut target_id = pd.id.clone();
        while ctx.prompt_path(&target_id).exists() {
            target_id = new_id(&ctx.workspaces_dir);
        }
        pd.id = target_id.clone();

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
        fs::write(ctx.prompt_path(&pd.id), encoded_out)
            .map_err(|e| format!("Write error: {}", e))?;
    }

    println!("{} imported", style("•").green().bold());
    Ok(())
}
