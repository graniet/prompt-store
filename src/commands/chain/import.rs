use crate::core::storage::AppCtx;
use aes_gcm::aead::{Aead, AeadCore};
use base64::{engine::general_purpose, Engine};
use console::style;
use std::fs;

/// Import a YAML chain definition into the default workspace.
pub fn run(ctx: &AppCtx, file_path: &str, id: &str) -> Result<(), String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read chain definition file '{}': {}", file_path, e))?;

    // Basic validation: check if it's valid YAML
    let _: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| format!("Invalid YAML format: {}", e))?;

    let workspace_path = ctx.workspaces_dir.join("default");
    let chains_dir = workspace_path.join("chains");
    if !chains_dir.exists() {
        fs::create_dir_all(&chains_dir).map_err(|e| e.to_string())?;
    }

    let target_path = chains_dir.join(format!("{}.chain", id));
    if target_path.exists() {
        return Err(format!(
            "A chain with ID '{}' already exists in the default workspace.",
            id
        ));
    }

    let nonce = aes_gcm::Aes256Gcm::generate_nonce(&mut rand::rngs::OsRng);
    let encrypted_content = ctx
        .cipher
        .encrypt(&nonce, content.as_bytes())
        .map_err(|_| "Failed to encrypt chain definition".to_string())?;

    let mut out = Vec::with_capacity(12 + encrypted_content.len());
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(&encrypted_content);
    let encoded = general_purpose::STANDARD.encode(&out);

    fs::write(&target_path, encoded)
        .map_err(|e| format!("Failed to write encrypted chain file: {}", e))?;

    println!(
        "{} Successfully imported chain '{}' into the default workspace.",
        style("✔").green(),
        style(id).yellow()
    );

    Ok(())
}