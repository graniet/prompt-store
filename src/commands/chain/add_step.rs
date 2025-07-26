use crate::core::storage::{AppCtx, PromptData};
use crate::ui::theme;
use aes_gcm::aead::{Aead, AeadCore, OsRng};
use aes_gcm::Aes256Gcm;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use dialoguer::{Editor, Input};
use std::fs;

/// Add a new prompt step to an existing chain.
pub fn run(ctx: &AppCtx, chain_id: &str) -> Result<(), String> {
    let chain_dir = ctx.workspaces_dir.join(chain_id);
    if !chain_dir.is_dir() {
        return Err(format!("Chain with ID '{}' not found.", chain_id));
    }

    let mut max_step = 0;
    for entry in fs::read_dir(&chain_dir).map_err(|e| format!("Read error: {}", e))? {
        if let Ok(entry) = entry {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                if let Ok(num) = stem.parse::<u32>() {
                    if num > max_step {
                        max_step = num;
                    }
                }
            }
        }
    }
    let next_step = max_step + 1;
    println!(
        "Adding new prompt as step #{} to chain '{}'",
        next_step, chain_id
    );

    let theme = theme();
    let prompt_title: String = Input::with_theme(&theme)
        .with_prompt(format!("Title for prompt #{}", next_step))
        .interact_text()
        .map_err(|e| format!("Title error: {}", e))?;

    let tags_line: String = Input::with_theme(&theme)
        .with_prompt("Tags (comma-separated, optional)")
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

    let prompt_id = format!("{}/{}", chain_id, next_step);
    let pd = PromptData {
        id: prompt_id,
        title: prompt_title.clone(),
        content,
        tags,
        schema: None, // Schemas are not defined for chain sub-prompts in this flow
    };

    let prompt_path = chain_dir.join(format!("{}.prompt", next_step));
    let json = serde_json::to_vec(&pd).map_err(|e| format!("Serialize error: {}", e))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher_bytes = ctx
        .cipher
        .encrypt(&nonce, json.as_ref())
        .map_err(|_| "Encrypt error")?;
    let mut out = Vec::with_capacity(12 + cipher_bytes.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&cipher_bytes);
    let encoded = general_purpose::STANDARD.encode(&out);

    fs::write(prompt_path, encoded).map_err(|e| format!("Write error: {}", e))?;

    println!(
        "{} Added prompt '{}' to chain '{}'.",
        style("•").green().bold(),
        style(prompt_title).cyan(),
        style(chain_id).yellow()
    );
    Ok(())
}
