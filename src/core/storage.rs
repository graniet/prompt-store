use super::utils::ensure_dir;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::crypto::load_or_generate_key;

/// Data for a single, storable prompt, including an optional I/O schema.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PromptData {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<PromptSchema>,
}

/// Defines the expected inputs and output format (as a JSON Schema value) for a prompt.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PromptSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
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
    pub workspaces_dir: PathBuf,
    pub registries_dir: PathBuf,
    pub key_path: PathBuf,
    pub cipher: Aes256Gcm,
}

/// Parses a prompt identifier into its workspace and local ID components.
/// If no workspace is specified (i.e., no `::`), it defaults to the "default" workspace.
pub fn parse_id(id: &str) -> (String, String) {
    match id.split_once("::") {
        Some((workspace, prompt_id)) => (workspace.to_string(), prompt_id.to_string()),
        None => ("default".to_string(), id.to_string()),
    }
}

impl AppCtx {
    /// Initializes the application context, creating necessary directories and loading the encryption key.
    pub fn init() -> Result<Self, String> {
        let home =
            env::var("HOME").map_err(|_| "Unable to determine HOME directory".to_string())?;
        let base_dir = PathBuf::from(home).join(".prompt-store");
        let key_dir = base_dir.join("keys");
        let key_path = key_dir.join("key.bin");
        let workspaces_dir = base_dir.join("workspaces");
        let registries_dir = base_dir.join("registries");

        ensure_dir(&base_dir)?;
        ensure_dir(&key_dir)?;
        ensure_dir(&workspaces_dir)?;
        ensure_dir(&workspaces_dir.join("default"))?; // Ensure default workspace exists
        ensure_dir(&registries_dir)?;

        let (key_bytes, _) = load_or_generate_key(&key_path)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        Ok(Self {
            base_dir,
            workspaces_dir,
            registries_dir,
            key_path,
            cipher,
        })
    }

    /// Constructs the full path for a prompt file from its full ID.
    pub fn prompt_path(&self, full_id: &str) -> PathBuf {
        let (workspace, local_id) = parse_id(full_id);
        let workspace_path = self.workspaces_dir.join(workspace);

        if let Some((chain_id, step_id)) = local_id.split_once('/') {
            workspace_path
                .join(chain_id)
                .join(format!("{}.prompt", step_id))
        } else {
            workspace_path.join(format!("{}.prompt", local_id))
        }
    }
}

/// Decrypts a prompt file to read its full data.
pub fn decrypt_full_prompt(path: &Path, cipher: &Aes256Gcm) -> Result<PromptData, String> {
    let encoded = fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
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
    serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())
}
