use crate::core::storage::{decrypt_full_prompt, AppCtx, ChainData};
use aes_gcm::aead::Aead;
use aes_gcm::Nonce;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

struct WorkspaceContent {
    standalone_prompts: Vec<(String, String)>,
    chains: Vec<(String, String)>,
}

/// List every saved prompt and chain, with optional tag filtering.
pub fn run(ctx: &AppCtx, tags: &[String]) -> Result<(), String> {
    let tag_filter: HashSet<_> = tags.iter().map(|t| t.to_lowercase()).collect();
    let is_filtering = !tag_filter.is_empty();

    let mut workspaces: BTreeMap<String, WorkspaceContent> = BTreeMap::new();

    for entry in fs::read_dir(&ctx.workspaces_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let workspace_name = entry.file_name().to_string_lossy().to_string();
        let mut content = WorkspaceContent {
            standalone_prompts: Vec::new(),
            chains: Vec::new(),
        };

        for item in fs::read_dir(&path).map_err(|e| e.to_string())? {
            let item_path = item.map_err(|e| e.to_string())?.path();
            if item_path.is_dir() {
                // It's a chain
                if is_filtering {
                    continue;
                }
                if let Ok(chain_data) =
                    decrypt_chain_meta(&item_path.join("chain.meta"), &ctx.cipher)
                {
                    content.chains.push((chain_data.id, chain_data.title));
                }
            } else if item_path.extension().and_then(|s| s.to_str()) == Some("prompt") {
                // Standalone prompt
                if let Ok(prompt) = decrypt_full_prompt(&item_path, &ctx.cipher) {
                    if is_filtering {
                        let prompt_tags: HashSet<_> =
                            prompt.tags.iter().map(|t| t.to_lowercase()).collect();
                        if !tag_filter.is_subset(&prompt_tags) {
                            continue;
                        }
                    }
                    content.standalone_prompts.push((prompt.id, prompt.title));
                }
            }
        }

        content.standalone_prompts.sort_by(|a, b| a.0.cmp(&b.0));
        content.chains.sort_by(|a, b| a.0.cmp(&b.0));

        if !content.standalone_prompts.is_empty() || !content.chains.is_empty() {
            workspaces.insert(workspace_name, content);
        }
    }

    if workspaces.is_empty() {
        println!(
            "{}",
            style("No matching prompts or chains found.")
                .yellow()
                .bold()
        );
    } else {
        for (name, content) in workspaces {
            println!("\nWorkspace: {}", style(name.clone()).bold().cyan());
            if content.standalone_prompts.is_empty() && content.chains.is_empty() {
                println!("  (empty)");
                continue;
            }

            for (id, title) in content.standalone_prompts {
                let display_id = if name == "default" {
                    id.clone()
                } else {
                    format!("{}::{}", name, id)
                };
                println!(
                    "  {} {} - {}",
                    style("•").green(),
                    style(display_id).yellow(),
                    title
                );
            }
            for (id, title) in content.chains {
                let display_id = if name == "default" {
                    id.clone()
                } else {
                    format!("{}::{}", name, id)
                };
                println!(
                    "  {} {} (Chain) - {}",
                    style("•").blue(),
                    style(display_id).yellow(),
                    title
                );
            }
        }
    }
    Ok(())
}

fn decrypt_chain_meta(path: &Path, cipher: &aes_gcm::Aes256Gcm) -> Result<ChainData, String> {
    let plaintext = decrypt_file(path, cipher)?;
    serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON for ChainData".to_string())
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
