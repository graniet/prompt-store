use crate::core::storage::{decrypt_prompt_header, AppCtx, ChainData};
use aes_gcm::aead::Aead;
use aes_gcm::Nonce;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use std::fs;
use std::path::Path;

enum ListItem {
    Standalone(String, String),
    Chain {
        id: String,
        title: String,
        steps: Vec<(String, String)>,
    },
}

/// List every saved prompt and chain.
pub fn run(ctx: &AppCtx) -> Result<(), String> {
    let mut items = Vec::new();

    if ctx.prompts_dir.exists() {
        for entry in fs::read_dir(&ctx.prompts_dir).map_err(|e| format!("Read dir error: {}", e))? {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;
            let path = ent.path();

            if path.is_dir() {
                // This is a chain
                let chain_meta_path = path.join("chain.meta");
                if let Ok(chain_data) = decrypt_chain_meta(&chain_meta_path, &ctx.cipher) {
                    let mut steps = vec![];
                    for step_entry in
                        fs::read_dir(&path).map_err(|e| format!("Read dir error: {}", e))?
                    {
                        let step_ent = step_entry.map_err(|e| format!("Dir read error: {}", e))?;
                        let step_path = step_ent.path();
                        if step_path.is_file()
                            && step_path.extension().and_then(|s| s.to_str()) == Some("prompt")
                        {
                            if let Ok((id, title)) = decrypt_prompt_header(&step_path, &ctx.cipher)
                            {
                                let step_num_str = id.split('/').last().unwrap_or("0");
                                if let Ok(step_num) = step_num_str.parse::<u32>() {
                                    steps.push((step_num, id, title));
                                }
                            }
                        }
                    }
                    steps.sort_by_key(|(k, _, _)| *k);
                    let formatted_steps = steps
                        .into_iter()
                        .map(|(_, id, title)| (id, title))
                        .collect();

                    items.push(ListItem::Chain {
                        id: chain_data.id,
                        title: chain_data.title,
                        steps: formatted_steps,
                    });
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("prompt") {
                // This is a standalone prompt
                if let Ok((id, title)) = decrypt_prompt_header(&path, &ctx.cipher) {
                    items.push(ListItem::Standalone(id, title));
                }
            }
        }
    }

    if items.is_empty() {
        println!("{}", style("No saved prompts or chains").green().bold());
    } else {
        items.sort_by(|a, b| {
            let id_a = match a {
                ListItem::Standalone(id, _) => id,
                ListItem::Chain { id, .. } => id,
            };
            let id_b = match b {
                ListItem::Standalone(id, _) => id,
                ListItem::Chain { id, .. } => id,
            };
            id_a.cmp(id_b)
        });

        println!("{}", style("Saved Prompts & Chains:").green().bold());
        for item in items {
            match item {
                ListItem::Standalone(id, title) => {
                    println!(
                        "  {} {} - {}",
                        style("•").green(),
                        style(id).yellow(),
                        title
                    );
                }
                ListItem::Chain { id, title, steps } => {
                    println!(
                        "  {} {} (Chain) - {}",
                        style("•").blue(),
                        style(id).yellow(),
                        title
                    );
                    for (i, (step_id, step_title)) in steps.iter().enumerate() {
                        let prefix = if i == steps.len() - 1 {
                            "  └─"
                        } else {
                            "  ├─"
                        };
                        println!("{} {} - {}", prefix, style(step_id).dim(), step_title);
                    }
                }
            }
        }
    }
    Ok(())
}

fn decrypt_chain_meta(path: &Path, cipher: &aes_gcm::Aes256Gcm) -> Result<ChainData, String> {
    let encoded = fs::read_to_string(path).map_err(|_| "Read error".to_string())?;
    let decoded = general_purpose::STANDARD
        .decode(encoded.trim_end())
        .map_err(|_| "Corrupted data".to_string())?;
    if decoded.len() < 12 {
        return Err("Corrupted data".to_string());
    }
    let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), cipher_bytes)
        .map_err(|_| "Decrypt error".to_string())?;
    serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())
}
