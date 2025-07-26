use crate::core::{
    storage::{AppCtx, PromptData, PromptSchema},
    utils::new_id,
};
use aes_gcm::{
    aead::{Aead, AeadCore, OsRng},
    Aes256Gcm,
};
use base64::{engine::general_purpose, Engine as _};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Editor, Input};
use serde_json::Value;
use std::fs;

/// Create a new prompt in the default workspace.
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

    let content = Editor::new()
        .edit("Enter your prompt content here.")
        .map_err(|e| format!("Editor error: {}", e))?
        .unwrap_or_default();

    let mut schema = None;
    if Confirm::with_theme(&theme)
        .with_prompt("Define an I/O schema for this prompt?")
        .default(false)
        .interact()
        .unwrap_or(false)
    {
        println!(
            "{}",
            style("Opening editor for schema... (use JSON format)").yellow()
        );
        let schema_template = r#"{
  "inputs": {
    "type": "object",
    "properties": {
      "variable_name": { "type": "string", "description": "Description of the variable." }
    },
    "required": ["variable_name"]
  },
  "output": {
    "type": "object",
    "properties": {
      "output_field": { "type": "string", "description": "Description of the output field." }
    },
    "required": ["output_field"]
  }
}"#;
        let schema_str = Editor::new()
            .edit(schema_template)
            .map_err(|e| format!("Editor error: {}", e))?
            .unwrap_or_default();

        if !schema_str.trim().is_empty() {
            let schema_json: Value = serde_json::from_str(&schema_str)
                .map_err(|e| format!("Invalid JSON in schema: {}", e))?;
            let inputs = schema_json.get("inputs").cloned();
            let output = schema_json.get("output").cloned();
            schema = Some(PromptSchema { inputs, output });
        }
    }

    let default_workspace = ctx.workspaces_dir.join("default");
    let id = new_id(&default_workspace);
    let pd = PromptData {
        id: id.clone(),
        title: title.clone(),
        content,
        tags,
        schema,
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

    // Use prompt_path with the implicit default workspace
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
