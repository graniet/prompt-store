use crate::core::storage::{decrypt_full_prompt, AppCtx, PromptData};
use aes_gcm::{
    aead::{Aead, AeadCore, OsRng},
    Aes256Gcm,
};
use base64::{engine::general_purpose, Engine as _};
use console::style;
use std::fs;

/// Export specified prompts from the default workspace for personal backup.
/// The output file is encrypted with the user's local master key.
pub fn run(ctx: &AppCtx, ids: Option<&str>, out_path: &str) -> Result<(), String> {
    let mut bundle: Vec<PromptData> = Vec::new();
    let default_workspace = ctx.workspaces_dir.join("default");

    if let Some(id_list_str) = ids {
        // Export specific prompts by ID
        let id_list: Vec<&str> = id_list_str.split(',').map(|s| s.trim()).collect();
        for id in id_list {
            let prompt_path = ctx.prompt_path(id); // This correctly defaults to the 'default' workspace
            if !prompt_path.exists() {
                return Err(format!(
                    "Prompt with ID '{}' not found in default workspace.",
                    id
                ));
            }
            bundle.push(decrypt_full_prompt(&prompt_path, &ctx.cipher)?);
        }
    } else {
        // Export all prompts from the default workspace
        if !default_workspace.is_dir() {
            return Err("Default workspace does not exist.".to_string());
        }
        for entry in fs::read_dir(&default_workspace).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("prompt") {
                bundle.push(decrypt_full_prompt(&path, &ctx.cipher)?);
            }
            // Note: This simple export does not recurse into chains.
        }
    }

    if bundle.is_empty() {
        return Err("No prompts found to export.".to_string());
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
    println!(
        "{} Successfully exported {} prompts to {}",
        style("•").green().bold(),
        bundle.len(),
        out_path
    );
    Ok(())
}
