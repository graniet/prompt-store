//! The main entry point for interacting with the prompt store.

use crate::core::crypto::decrypt_key_with_password;
use crate::core::storage::{AppCtx, PromptData};
use crate::core::utils::ensure_dir;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose, Engine as _};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::error::StoreError;
use super::llm_bridge::LLMBackendRef;
use super::runner::{ChainRunner, PromptRunner};

/// The main entry point for interacting with the prompt store.
///
/// This structure is designed to be created once and shared throughout your application.
/// It holds the necessary context, including the encryption cipher.
pub struct PromptStore {
    pub(crate) ctx: AppCtx,
}

impl PromptStore {
    fn new_from_key(key_bytes: Vec<u8>) -> Result<Self, StoreError> {
        let home = env::var("HOME").map_err(|e| StoreError::Init(e.to_string()))?;
        let base_dir = PathBuf::from(home).join(".prompt-store");
        let key_path = base_dir.join("keys").join("key.bin");
        let workspaces_dir = base_dir.join("workspaces");
        let registries_dir = base_dir.join("registries");

        ensure_dir(&base_dir).map_err(StoreError::Init)?;
        ensure_dir(&workspaces_dir).map_err(StoreError::Init)?;
        ensure_dir(&registries_dir).map_err(StoreError::Init)?;
        ensure_dir(&workspaces_dir.join("default")).map_err(StoreError::Init)?;

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        let ctx = AppCtx {
            base_dir,
            workspaces_dir,
            registries_dir,
            key_path,
            cipher,
        };

        Ok(Self { ctx })
    }

    /// Initializes the PromptStore by prompting for a password if the key is encrypted.
    ///
    /// This function will locate `~/.prompt-store`, load the encryption key,
    /// and interactively prompt for a password if required.
    pub fn init() -> Result<Self, StoreError> {
        let ctx = AppCtx::init().map_err(StoreError::Init)?;
        Ok(Self { ctx })
    }

    /// Initializes the PromptStore non-interactively with a password.
    ///
    /// This is useful for server environments where interactive prompts are not possible.
    /// The password can be provided from an environment variable or a secret manager.
    ///
    /// # Arguments
    ///
    /// * `password` - The password to decrypt the master key.
    pub fn with_password(password: &str) -> Result<Self, StoreError> {
        let home = env::var("HOME").map_err(|e| StoreError::Init(e.to_string()))?;
        let key_path = PathBuf::from(home)
            .join(".prompt-store")
            .join("keys")
            .join("key.bin");

        if !key_path.exists() {
            return Err(StoreError::Init(
                "Key file does not exist. Run interactively once to create it.".to_string(),
            ));
        }

        let key_data = fs::read(&key_path)?;
        let decrypted_key =
            decrypt_key_with_password(&key_data, password).map_err(StoreError::Init)?;

        Self::new_from_key(decrypted_key)
    }

    /// Creates a runner for executing a single prompt.
    ///
    /// # Arguments
    ///
    /// * `id_or_title` - The ID or exact title of the prompt to run.
    pub fn prompt<'a>(&'a self, id_or_title: &'a str) -> PromptRunner<'a> {
        PromptRunner::new(self, id_or_title)
    }

    /// Creates a runner to define and execute a chain of prompts.
    ///
    /// # Arguments
    ///
    /// * `backend` - The LLM backend to use for the chain. This must be a type
    ///   that can be converted into `LLMBackendRef`, typically a `&LLMRegistry`.
    pub fn chain<'a, B: Into<LLMBackendRef<'a>>>(&'a self, backend: B) -> ChainRunner<'a> {
        ChainRunner::new(self, backend.into())
    }

    /// Internal logic for finding and decrypting a prompt by its ID or title.
    /// Searches local prompts, chain prompts, and cached prompts from deployed packs.
    pub(crate) fn find_prompt(&self, id_or_title: &str) -> Result<PromptData, StoreError> {
        // First, try to load by full ID directly (e.g., "abcdef12", "chain/1", or "pack::abc").
        let prompt_path = self.ctx.prompt_path(id_or_title);
        if prompt_path.exists() {
            return self.decrypt_prompt_file(&prompt_path);
        }

        // If not found, search all prompts by title. This is more expensive.
        let mut found_prompts = vec![];
        if self.ctx.workspaces_dir.exists() {
            self.find_prompts_by_title_recursive(
                &self.ctx.workspaces_dir,
                id_or_title,
                &mut found_prompts,
            )?;
        }

        if found_prompts.len() == 1 {
            Ok(found_prompts.remove(0))
        } else if found_prompts.is_empty() {
            Err(StoreError::NotFound(id_or_title.to_string()))
        } else {
            Err(StoreError::AmbiguousTitle(id_or_title.to_string()))
        }
    }

    /// Recursive helper to find prompts by title.
    fn find_prompts_by_title_recursive(
        &self,
        dir: &Path,
        title_query: &str,
        found: &mut Vec<PromptData>,
    ) -> Result<(), StoreError> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                self.find_prompts_by_title_recursive(&path, title_query, found)?;
            } else if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("prompt")
            {
                if let Ok(pd) = self.decrypt_prompt_file(&path) {
                    if pd.title.eq_ignore_ascii_case(title_query) {
                        found.push(pd);
                    }
                }
            }
        }
        Ok(())
    }

    /// Helper to decrypt a single prompt file.
    fn decrypt_prompt_file(&self, path: &Path) -> Result<PromptData, StoreError> {
        let encoded = fs::read_to_string(path)?;
        let decoded = general_purpose::STANDARD
            .decode(encoded.trim_end())
            .map_err(|_| StoreError::Crypto("Invalid Base64 data.".to_string()))?;

        if decoded.len() < 12 {
            return Err(StoreError::Crypto(
                "Data is too short to be valid.".to_string(),
            ));
        }

        let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
        let plaintext = self
            .ctx
            .cipher
            .decrypt(Nonce::from_slice(nonce_bytes), cipher_bytes)
            .map_err(|_| {
                StoreError::Crypto("Decryption failed. Check key or password.".to_string())
            })?;

        Ok(serde_json::from_slice(&plaintext)?)
    }
}