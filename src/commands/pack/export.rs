use crate::core::storage::{decrypt_full_prompt, AppCtx, PromptData};
use aes_gcm::aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes256Gcm, Key};
use argon2::Argon2;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use dialoguer::Password;
use rand::RngCore;
use std::fs;
use std::path::Path;

/// Export all prompts from a specified workspace to a 'prompts.bundle' file.
pub fn run(ctx: &AppCtx, workspace: Option<&str>) -> Result<(), String> {
    let workspace_name = workspace.unwrap_or("default");
    let workspace_path = ctx.workspaces_dir.join(workspace_name);
    let output_file = "prompts.bundle";

    if !workspace_path.is_dir() {
        return Err(format!("Workspace '{}' not found.", workspace_name));
    }

    let mut prompts = Vec::new();
    find_prompts_recursive(&workspace_path, &ctx.cipher, &mut prompts)?;

    if prompts.is_empty() {
        return Err(format!(
            "No prompts found in the '{}' workspace to export.",
            workspace_name
        ));
    }

    let password = Password::new()
        .with_prompt("Enter a password to encrypt the pack")
        .with_confirmation("Confirm password", "Passwords do not match.")
        .interact()
        .map_err(|e| format!("Password input error: {}", e))?;

    let serialized =
        serde_json::to_vec(&prompts).map_err(|e| format!("Serialization failed: {}", e))?;

    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), &salt, &mut key)
        .map_err(|_| "KDF error".to_string())?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Aes256Gcm::generate_nonce(&mut rand::thread_rng());
    let encrypted_data = cipher
        .encrypt(&nonce, serialized.as_ref())
        .map_err(|_| "Encryption failed")?;

    let mut final_data = Vec::new();
    final_data.extend_from_slice(&salt);
    final_data.extend_from_slice(nonce.as_slice());
    final_data.extend_from_slice(&encrypted_data);

    let encoded = general_purpose::STANDARD.encode(&final_data);
    fs::write(output_file, encoded).map_err(|e| format!("Failed to write bundle: {}", e))?;

    println!(
        "{} Successfully exported {} prompts from workspace '{}' to {}",
        style("✔").green(),
        prompts.len(),
        workspace_name,
        style(output_file).yellow()
    );
    Ok(())
}

fn find_prompts_recursive(
    dir: &Path,
    cipher: &Aes256Gcm,
    prompts: &mut Vec<PromptData>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| format!("Read dir error: {}", e))? {
        let path = entry.map_err(|e| format!("Dir entry error: {}", e))?.path();
        if path.is_dir() {
            find_prompts_recursive(&path, cipher, prompts)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("prompt") {
            if let Ok(mut prompt) = decrypt_full_prompt(&path, cipher) {
                // We strip the workspace from the ID for portability
                prompt.id = crate::core::storage::parse_id(&prompt.id).1;
                prompts.push(prompt);
            }
        }
    }
    Ok(())
}
