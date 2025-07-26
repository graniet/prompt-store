use crate::core::storage::{decrypt_full_prompt, parse_id, AppCtx, PromptSchema};
use aes_gcm::{
    aead::{Aead, AeadCore, OsRng},
    Aes256Gcm,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use console::style;
use dialoguer::{theme::ColorfulTheme, Editor, Select};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Edit a prompt's content or schema and create a timestamped backup.
pub fn run(ctx: &AppCtx, id: &str) -> Result<(), String> {
    let path = ctx.prompt_path(id);
    if !path.exists() {
        return Err(format!("No prompt with ID '{}'", id));
    }

    let mut pd = decrypt_full_prompt(&path, &ctx.cipher)?;
    let original_pd = pd.clone();
    let theme = ColorfulTheme::default();

    loop {
        let selections = &["Edit Content", "Edit Schema", "Finish Editing"];
        let selection = Select::with_theme(&theme)
            .with_prompt("What would you like to do?")
            .default(0)
            .items(&selections[..])
            .interact()
            .map_err(|e| e.to_string())?;

        match selection {
            0 => {
                // Edit Content
                let edited = Editor::new()
                    .edit(&pd.content)
                    .map_err(|e| format!("Editor error: {}", e))?
                    .unwrap_or_default();
                pd.content = edited;
                println!("{}", style("Content updated.").green());
            }
            1 => {
                // Edit Schema
                let current_schema_str = pd.schema.as_ref().map_or_else(
                    || "{}".to_string(),
                    |s| serde_json::to_string_pretty(s).unwrap_or_else(|_| "{}".to_string()),
                );

                let new_schema_str = Editor::new()
                    .edit(&current_schema_str)
                    .map_err(|e| format!("Editor error: {}", e))?
                    .unwrap_or_default();

                if new_schema_str.trim().is_empty() || new_schema_str.trim() == "{}" {
                    pd.schema = None;
                    println!("{}", style("Schema removed.").yellow());
                } else {
                    let schema_json: Value = serde_json::from_str(&new_schema_str)
                        .map_err(|e| format!("Invalid JSON in schema: {}", e))?;
                    pd.schema = Some(PromptSchema {
                        inputs: schema_json.get("inputs").cloned(),
                        output: schema_json.get("output").cloned(),
                    });
                    println!("{}", style("Schema updated.").green());
                }
            }
            _ => break, // Finish Editing
        }
    }

    let original_json = serde_json::to_vec(&original_pd).unwrap();
    let new_json = serde_json::to_vec(&pd).unwrap();

    if original_json == new_json {
        println!(
            "{}",
            style("No changes detected. Nothing to save.").yellow()
        );
        return Ok(());
    }

    // Create backup
    let ts = Local::now().format("%Y%m%d%H%M%S").to_string();
    let (_workspace, local_id) = parse_id(id);
    let mut bak_path = PathBuf::from(&path);
    bak_path.set_file_name(format!("{}.{}.bak", local_id, ts));
    fs::copy(&path, &bak_path).map_err(|e| format!("Backup error: {}", e))?;

    // Save new version
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher_bytes = ctx
        .cipher
        .encrypt(&nonce, new_json.as_ref())
        .map_err(|_| "Encrypt error".to_string())?;

    let mut out = Vec::with_capacity(12 + cipher_bytes.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&cipher_bytes);
    let encoded_out = general_purpose::STANDARD.encode(&out);

    fs::write(&path, encoded_out).map_err(|e| format!("Write error: {}", e))?;
    println!(
        "{} Prompt '{}' updated successfully.",
        style("✔").green().bold(),
        id
    );
    Ok(())
}
