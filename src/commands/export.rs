use crate::core::storage::{AppCtx, PromptData};
use aes_gcm::{
    aead::{Aead, AeadCore, OsRng},
    Aes256Gcm,
};
use base64::{engine::general_purpose, Engine as _};
use console::style;
use serde_json;
use std::fs;

/// Export prompts to encrypted file.
pub fn run(ctx: &AppCtx, ids: Option<&str>, out_path: &str) -> Result<(), String> {
    let wanted: Option<Vec<&str>> = ids.map(|s| s.split(',').map(|x| x.trim()).collect());
    let mut bundle = Vec::new();

    if ctx.prompts_dir.exists() {
        for entry in fs::read_dir(&ctx.prompts_dir).map_err(|e| format!("Read dir error: {}", e))? {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;

            let file_id_string = ent.file_name().to_string_lossy().to_string();
            let file_id = file_id_string.split('.').next().unwrap_or("").to_string();

            if let Some(ref list) = wanted {
                if !list.contains(&file_id.as_str()) {
                    continue;
                }
            }

            let encoded =
                fs::read_to_string(ent.path()).map_err(|e| format!("Read error: {}", e))?;
            let decoded = general_purpose::STANDARD
                .decode(encoded.trim_end())
                .map_err(|_| "Corrupted data".to_string())?;
            if decoded.len() < 12 {
                continue;
            }
            let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
            let plaintext = ctx
                .cipher
                .decrypt(aes_gcm::Nonce::from_slice(nonce_bytes), cipher_bytes)
                .map_err(|_| "Decrypt error".to_string())?;
            let pd: PromptData =
                serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())?;
            bundle.push(pd);
        }
    }

    let serialized = serde_json::to_vec(&bundle).map_err(|e| format!("Serialize error: {}", e))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher_bytes = ctx
        .cipher
        .encrypt(&nonce, serialized.as_ref())
        .map_err(|_| "Encrypt error".to_string())?;
    let mut out = Vec::with_capacity(12 + cipher_bytes.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&cipher_bytes);
    let encoded = general_purpose::STANDARD.encode(&out);

    fs::write(out_path, encoded).map_err(|e| format!("Write error: {}", e))?;
    println!("{} exported to {}", style("•").green().bold(), out_path);
    Ok(())
}
