use crate::core::storage::{decrypt_prompt_header, AppCtx, ChainData, PromptData};
use aes_gcm::aead::Aead;
use aes_gcm::Nonce;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

enum ListItem {
    Standalone(String, String),
    Chain { id: String, title: String },
}

/// List every saved prompt and chain, with optional tag filtering.
pub fn run(ctx: &AppCtx, tags: &[String]) -> Result<(), String> {
    let mut items = Vec::new();
    let tag_filter: HashSet<_> = tags.iter().map(|t| t.to_lowercase()).collect();
    let is_filtering = !tag_filter.is_empty();

    if ctx.prompts_dir.exists() {
        for entry in fs::read_dir(&ctx.prompts_dir).map_err(|e| format!("Read dir error: {}", e))? {
            let ent = entry.map_err(|e| format!("Dir entry error: {}", e))?;
            let path = ent.path();

            if path.is_dir() {
                // This is a chain
                if is_filtering {
                    continue;
                } // Note: Tag filtering doesn't apply to chains for now.
                let chain_meta_path = path.join("chain.meta");
                if let Ok(chain_data) = decrypt_chain_meta(&chain_meta_path, &ctx.cipher) {
                    items.push(ListItem::Chain {
                        id: chain_data.id,
                        title: chain_data.title,
                    });
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("prompt") {
                // This is a standalone prompt
                if let Ok(prompt) = decrypt_full_prompt(&path, &ctx.cipher) {
                    if is_filtering {
                        let prompt_tags: HashSet<_> =
                            prompt.tags.iter().map(|t| t.to_lowercase()).collect();
                        if !tag_filter.is_subset(&prompt_tags) {
                            continue;
                        }
                    }
                    items.push(ListItem::Standalone(prompt.id, prompt.title));
                }
            }
        }
    }

    if items.is_empty() {
        println!(
            "{}",
            style("No matching prompts or chains found.")
                .yellow()
                .bold()
        );
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
                ListItem::Standalone(ref id, ref title) => {
                    println!(
                        "  {} {} - {}",
                        style("•").green(),
                        style(id).yellow(),
                        title
                    );
                }
                ListItem::Chain { ref id, ref title } => {
                    println!(
                        "  {} {} (Chain) - {}",
                        style("•").blue(),
                        style(id).yellow(),
                        title
                    );
                    let chain_dir = ctx.prompts_dir.join(id);
                    if let Ok(entries) = fs::read_dir(chain_dir) {
                        let mut steps: Vec<(u32, String, String)> = entries
                            .filter_map(|entry| {
                                let path = entry.ok()?.path();
                                if path.is_file()
                                    && path.extension().and_then(|s| s.to_str()) == Some("prompt")
                                {
                                    if let Ok((step_id, step_title)) =
                                        decrypt_prompt_header(&path, &ctx.cipher)
                                    {
                                        let step_num =
                                            path.file_stem()?.to_str()?.parse::<u32>().ok()?;
                                        return Some((step_num, step_id, step_title));
                                    }
                                }
                                None
                            })
                            .collect();

                        steps.sort_by_key(|(k, _, _)| *k);

                        for (i, (_, step_id, step_title)) in steps.iter().enumerate() {
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
    }
    Ok(())
}

fn decrypt_chain_meta(path: &Path, cipher: &aes_gcm::Aes256Gcm) -> Result<ChainData, String> {
    let plaintext = decrypt_file(path, cipher)?;
    serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON for ChainData".to_string())
}

fn decrypt_full_prompt(path: &Path, cipher: &aes_gcm::Aes256Gcm) -> Result<PromptData, String> {
    let plaintext = decrypt_file(path, cipher)?;
    serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON for PromptData".to_string())
}

fn decrypt_file(path: &Path, cipher: &aes_gcm::Aes256Gcm) -> Result<Vec<u8>, String> {
    let encoded = fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    let decoded = general_purpose::STANDARD
        .decode(encoded.trim_end())
        .map_err(|_| "Corrupted data".to_string())?;
    if decoded.len() < 12 {
        return Err("Corrupted data".to_string());
    }
    let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), cipher_bytes)
        .map_err(|_| "Decrypt error".to_string())
}
