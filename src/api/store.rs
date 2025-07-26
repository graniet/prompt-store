//! The main entry point for interacting with the prompt store.

use crate::core::storage::{AppCtx, PromptData};
use aes_gcm::aead::Aead;
use aes_gcm::Nonce;
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::Path;

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
    /// Initializes the PromptStore.
    ///
    /// This function will locate the `~/.prompt-store` directory, load the encryption key,
    /// or generate a new one if it doesn't exist. It may interactively prompt for a password
    /// if the key is password-protected.
    pub fn init() -> Result<Self, StoreError> {
        let ctx = AppCtx::init().map_err(StoreError::Init)?;
        Ok(Self { ctx })
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
    pub(crate) fn find_prompt(&self, id_or_title: &str) -> Result<PromptData, StoreError> {
        // First, try to load by ID directly.
        let prompt_path = self.ctx.prompt_path(id_or_title);
        if prompt_path.exists() {
            return self.decrypt_prompt_file(&prompt_path);
        }

        // If not found by ID, search by title. This is more expensive.
        let mut found_prompts = vec![];
        if self.ctx.prompts_dir.exists() {
            for entry in fs::read_dir(&self.ctx.prompts_dir)? {
                let path = entry?.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("prompt") {
                    if let Ok(pd) = self.decrypt_prompt_file(&path) {
                        if pd.title.eq_ignore_ascii_case(id_or_title) {
                            found_prompts.push(pd);
                        }
                    }
                }
            }
        }

        if found_prompts.len() == 1 {
            Ok(found_prompts.remove(0))
        } else if found_prompts.is_empty() {
            Err(StoreError::NotFound(id_or_title.to_string()))
        } else {
            Err(StoreError::AmbiguousTitle(id_or_title.to_string()))
        }
    }

    /// Helper to decrypt a single prompt file.
    fn decrypt_prompt_file(&self, path: &Path) -> Result<PromptData, StoreError> {
        let encoded = fs::read_to_string(path)?;
        let decoded = general_purpose::STANDARD
            .decode(encoded.trim_end())
            .map_err(|_| StoreError::Crypto("Invalid Base64 data.".to_string()))?;

        if decoded.len() < 12 {
            return Err(StoreError::Crypto("Data is too short to be valid.".to_string()));
        }

        let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
        let plaintext = self
            .ctx
            .cipher
            .decrypt(Nonce::from_slice(nonce_bytes), cipher_bytes)
            .map_err(|_| StoreError::Crypto("Decryption failed. Check key or password.".to_string()))?;

        Ok(serde_json::from_slice(&plaintext)?)
    }
}