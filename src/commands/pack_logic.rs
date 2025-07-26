//! Shared logic for deploying and managing prompt packs.

use crate::core::storage::{AppCtx, PromptData};
use aes_gcm::aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use base64::{engine::general_purpose, Engine as _};
use dialoguer::Password;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Represents the metadata for a deployed pack in `deployed.json`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeployedInfo {
    pub alias: String,
    pub url: String,
    pub commit_hash: String,
}

/// Reads prompts from a local repository path, decrypts if necessary,
/// and installs them into the local secure cache.
pub fn install_pack_from_local_repo(
    ctx: &AppCtx,
    repo_path: &Path,
    alias: &str,
    password: Option<&str>,
) -> Result<usize, String> {
    let bundle_path = repo_path.join("prompts.bundle");
    let json_path = repo_path.join("prompts.json");

    let prompts: Vec<PromptData> = if bundle_path.exists() {
        let pass = match password {
            Some(p) => Ok(p.to_string()),
            None => Password::new()
                .with_prompt(format!("Enter password for pack '{}'", alias))
                .interact()
                .map_err(|e| e.to_string()),
        }?;
        decrypt_bundle(&bundle_path, &pass)?
    } else if json_path.exists() {
        let content = fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read prompts.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse prompts.json: {}", e))?
    } else {
        return Err("No 'prompts.bundle' or 'prompts.json' found in repository.".to_string());
    };

    let num_prompts = prompts.len();
    install_prompts_to_workspace(ctx, alias, prompts)?;
    Ok(num_prompts)
}

fn decrypt_bundle(path: &Path, password: &str) -> Result<Vec<PromptData>, String> {
    let encoded_string =
        fs::read_to_string(path).map_err(|e| format!("Failed to read bundle: {}", e))?;
    let decoded = general_purpose::STANDARD
        .decode(encoded_string.trim())
        .map_err(|_| "Invalid Base64 in bundle".to_string())?;

    if decoded.len() < 28 {
        // 16 for salt + 12 for nonce
        return Err("Bundle file is corrupted or too short.".to_string());
    }

    let salt = &decoded[0..16];
    let nonce = Nonce::from_slice(&decoded[16..28]);
    let ciphertext = &decoded[28..];

    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| "Key derivation (Argon2) failed".to_string())?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Failed to decrypt bundle. Invalid password?".to_string())?;

    serde_json::from_slice(&plaintext).map_err(|e| format!("Invalid JSON in bundle: {}", e))
}

fn install_prompts_to_workspace(
    ctx: &AppCtx,
    alias: &str,
    prompts: Vec<PromptData>,
) -> Result<(), String> {
    let workspace_dir = ctx.workspaces_dir.join(alias);
    if workspace_dir.exists() {
        fs::remove_dir_all(&workspace_dir)
            .map_err(|e| format!("Failed to clear old workspace cache: {}", e))?;
    }
    fs::create_dir_all(&workspace_dir)
        .map_err(|e| format!("Failed to create workspace directory: {}", e))?;

    for prompt in prompts {
        let original_id = prompt.id.clone();
        // The ID inside the file remains the simple one. The namespace is contextual.
        let json = serde_json::to_vec(&prompt).map_err(|e| e.to_string())?;

        let nonce = Aes256Gcm::generate_nonce(&mut rand::thread_rng());
        let encrypted = ctx
            .cipher
            .encrypt(&nonce, json.as_ref())
            .map_err(|_| "Local encryption failed")?;

        let mut out = Vec::new();
        out.extend_from_slice(nonce.as_slice());
        out.extend_from_slice(&encrypted);
        let encoded = general_purpose::STANDARD.encode(&out);

        let path = workspace_dir.join(format!("{}.prompt", original_id));
        fs::write(path, encoded).map_err(|e| e.to_string())?;
    }
    Ok(())
}
