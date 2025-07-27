use crate::core::storage::{AppCtx, ChainData, PromptData};
use crate::core::utils::{ensure_dir, new_id};
use crate::ui::theme;
use aes_gcm::aead::{Aead, AeadCore, OsRng};
use aes_gcm::Aes256Gcm;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use dialoguer::{Confirm, Editor, Input};
use std::fs;
use std::path::Path;

/// Creates a new prompt chain interactively in the default workspace.
pub fn run(ctx: &AppCtx) -> Result<(), String> {
    let theme = theme();
    let default_workspace = ctx.workspaces_dir.join("default");

    let title: String = Input::with_theme(&theme)
        .with_prompt("Chain Title")
        .interact_text()
        .map_err(|e| format!("Title error: {}", e))?;
    if title.trim().is_empty() {
        return Err("Title cannot be empty".to_string());
    }

    let chain_id = new_id(&default_workspace);
    let chain_dir = default_workspace.join(&chain_id);
    ensure_dir(&chain_dir)?;

    let chain_data = ChainData {
        id: chain_id.clone(),
        title: title.clone(),
    };

    let chain_meta_path = chain_dir.join("chain.meta");
    let json = serde_json::to_vec(&chain_data).map_err(|e| format!("Serialize error: {}", e))?;
    encrypt_and_write(&ctx.cipher, &chain_meta_path, &json)?;

    println!(
        "\n{} Chain '{}' created with ID {}.",
        style("•").green().bold(),
        style(&title).cyan(),
        style(&chain_id).yellow()
    );
    println!("Now, let's add prompts to the chain.");

    let mut step_counter = 1;
    loop {
        if !Confirm::with_theme(&theme)
            .with_prompt(format!("Add prompt #{}?", step_counter))
            .default(true)
            .interact()
            .map_err(|e| format!("Confirmation error: {}", e))?
        {
            break;
        }

        let prompt_title: String = Input::with_theme(&theme)
            .with_prompt(format!("Title for prompt #{}", step_counter))
            .interact_text()
            .map_err(|e| format!("Title error: {}", e))?;

        let tags_line: String = Input::with_theme(&theme)
            .with_prompt("Tags (comma‑separated, optional)")
            .allow_empty(true)
            .interact_text()
            .map_err(|e| format!("Tags error: {}", e))?;
        let tags: Vec<String> = tags_line
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let content = Editor::new()
            .edit("Enter prompt content. Use {{var}} for variables.")
            .map_err(|e| format!("Editor error: {}", e))?
            .unwrap_or_default();

        let prompt_id = format!("{}/{}", &chain_id, step_counter);
        let pd = PromptData {
            id: prompt_id,
            title: prompt_title.clone(),
            content,
            tags,
            schema: None, // Schemas are not defined for chain sub-prompts in this flow
        };

        let prompt_path = chain_dir.join(format!("{}.prompt", step_counter));
        let json = serde_json::to_vec(&pd).map_err(|e| format!("Serialize error: {}", e))?;
        encrypt_and_write(&ctx.cipher, &prompt_path, &json)?;

        println!(
            "  {} Added prompt '{}'",
            style("└─").green(),
            style(prompt_title).cyan()
        );
        step_counter += 1;
    }

    println!("\n{} Chain '{}' saved.", style("✔").green().bold(), title);
    Ok(())
}

fn encrypt_and_write(
    cipher: &Aes256Gcm,
    path: &Path,
    data: &[u8],
) -> Result<(), String> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher_bytes = cipher
        .encrypt(&nonce, data)
        .map_err(|_| "Encrypt error".to_string())?;

    let mut out = Vec::with_capacity(12 + cipher_bytes.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&cipher_bytes);
    let encoded = general_purpose::STANDARD.encode(&out);

    fs::write(path, encoded).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}