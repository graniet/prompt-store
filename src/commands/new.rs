use crate::core::{
    storage::{AppCtx, PromptData},
    utils::new_id,
};
use aes_gcm::{
    aead::{Aead, AeadCore, OsRng},
    Aes256Gcm,
};
use base64::{engine::general_purpose, Engine as _};
use console::style;
use dialoguer::{theme::ColorfulTheme, Editor, Input};
use std::fs;

/// Create a new prompt.
pub fn run(ctx: &AppCtx) -> Result<(), String> {
    let theme = ColorfulTheme::default();

    let title: String = Input::with_theme(&theme)
        .with_prompt("Title")
        .interact_text()
        .map_err(|e| format!("Title error: {}", e))?;
    if title.trim().is_empty() {
        return Err("Title cannot be empty".to_string());
    }

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

    println!("{}", style("Opening editor…").yellow().bold());
    let content = Editor::new()
        .edit("Enter your prompt content here.")
        .map_err(|e| format!("Editor error: {}", e))?
        .unwrap_or_default();

    let id = new_id(&ctx.prompts_dir);
    let pd = PromptData {
        id: id.clone(),
        title: title.clone(),
        content,
        tags,
    };

    let json = serde_json::to_vec(&pd).map_err(|e| format!("Serialize error: {}", e))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher_bytes = ctx
        .cipher
        .encrypt(&nonce, json.as_ref())
        .map_err(|_| "Encrypt error".to_string())?;

    let mut out = Vec::with_capacity(12 + cipher_bytes.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&cipher_bytes);
    let encoded = general_purpose::STANDARD.encode(&out);

    let path = ctx.prompt_path(&id);
    fs::write(&path, encoded).map_err(|e| format!("Write error: {}", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).ok();
    }
    println!(
        "{} Prompt saved with ID {} and title '{}'",
        style("•").green().bold(),
        style(&id).yellow(),
        title
    );
    Ok(())
}
