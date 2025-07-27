use crate::core::storage::{parse_id, AppCtx, ChainData};
use crate::ui::theme;
use aes_gcm::aead::{Aead, AeadCore, OsRng};
use aes_gcm::Aes256Gcm;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use dialoguer::Input;
use std::fs;
use std::path::Path;

/// Edit the title of an existing chain.
pub fn run(ctx: &AppCtx, chain_id: &str) -> Result<(), String> {
    let (workspace, local_id) = parse_id(chain_id);
    let chain_dir = ctx.workspaces_dir.join(workspace).join(local_id);

    if !chain_dir.is_dir() {
        return Err(format!("Chain with ID '{}' not found.", chain_id));
    }
    let meta_path = chain_dir.join("chain.meta");
    if !meta_path.exists() {
        return Err(format!("Chain metadata for '{}' is missing.", chain_id));
    }

    let encoded = fs::read_to_string(&meta_path).map_err(|e| format!("Read error: {}", e))?;
    let decoded = general_purpose::STANDARD
        .decode(encoded.trim_end())
        .map_err(|_| "Corrupted data".to_string())?;
    let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
    let plaintext = ctx
        .cipher
        .decrypt(aes_gcm::Nonce::from_slice(nonce_bytes), cipher_bytes)
        .map_err(|_| "Decrypt error".to_string())?;
    let mut chain_data: ChainData =
        serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())?;

    let new_title: String = Input::with_theme(&theme())
        .with_prompt("New chain title")
        .default(chain_data.title.clone())
        .interact_text()
        .map_err(|e| format!("Input error: {}", e))?;

    chain_data.title = new_title;

    let json = serde_json::to_vec(&chain_data).map_err(|e| format!("Serialize error: {}", e))?;
    encrypt_and_write(&ctx.cipher, &meta_path, &json)?;

    println!(
        "{} Chain '{}' title updated.",
        style("•").green().bold(),
        chain_id
    );
    Ok(())
}

fn encrypt_and_write(cipher: &Aes256Gcm, path: &Path, data: &[u8]) -> Result<(), String> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher_bytes = cipher
        .encrypt(&nonce, data)
        .map_err(|_| "Encrypt error".to_string())?;
    let mut out = Vec::with_capacity(12 + cipher_bytes.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&cipher_bytes);
    let encoded = general_purpose::STANDARD.encode(&out);
    fs::write(path, encoded).map_err(|e| format!("Write error: {}", e))
}