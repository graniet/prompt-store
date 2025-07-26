use super::utils::ensure_dir;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::crypto::load_or_generate_key;

/// Data for a single, storable prompt.
#[derive(Serialize, Deserialize, Clone)]
pub struct PromptData {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
}

/// Metadata for a prompt chain.
#[derive(Serialize, Deserialize)]
pub struct ChainData {
    pub id: String,
    pub title: String,
}

/// Runtime context holding paths and encryption keys.
pub struct AppCtx {
    pub base_dir: PathBuf,
    pub prompts_dir: PathBuf,
    pub key_path: PathBuf,
    pub cipher: Aes256Gcm,
    pub key_bytes: Vec<u8>,
    pub key_from_password: bool,
}

impl AppCtx {
    pub fn init() -> Result<Self, String> {
        let home =
            env::var("HOME").map_err(|_| "Unable to determine HOME directory".to_string())?;
        let base_dir = PathBuf::from(home).join(".prompt-store");
        let key_dir = base_dir.join("keys");
        let key_path = key_dir.join("key.bin");
        let prompts_dir = base_dir.join("prompts");

        ensure_dir(&base_dir)?;
        ensure_dir(&key_dir)?;
        ensure_dir(&prompts_dir)?;

        let (key_bytes, key_from_password) = load_or_generate_key(&key_path)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        Ok(Self {
            base_dir,
            prompts_dir,
            key_path,
            cipher,
            key_bytes,
            key_from_password,
        })
    }

    /// Constructs the full path for a prompt file from its ID.
    /// Handles both standalone prompts (`id`) and chain steps (`chain_id/step_num`).
    pub fn prompt_path(&self, id: &str) -> PathBuf {
        if let Some((chain_id, step_id)) = id.split_once('/') {
            self.prompts_dir
                .join(chain_id)
                .join(format!("{}.prompt", step_id))
        } else {
            self.prompts_dir.join(format!("{}.prompt", id))
        }
    }
}

/// Decrypts a prompt file to read its ID and title.
pub fn decrypt_prompt_header(path: &Path, cipher: &Aes256Gcm) -> Result<(String, String), String> {
    let encoded = fs::read_to_string(path).map_err(|_| "Read error".to_string())?;
    let decoded = general_purpose::STANDARD
        .decode(encoded.trim_end())
        .map_err(|_| "Corrupted data".to_string())?;
    if decoded.len() < 12 {
        return Err("Corrupted data".to_string());
    }
    let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), cipher_bytes)
        .map_err(|_| "Decrypt error".to_string())?;
    let pd: PromptData =
        serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())?;
    Ok((pd.id, pd.title))
}
