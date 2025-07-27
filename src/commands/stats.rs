use crate::core::storage::{decrypt_full_prompt, AppCtx};
use console::style;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Display statistics about the prompt store.
pub fn run(ctx: &AppCtx) -> Result<(), String> {
    let mut standalone_prompts = 0;
    let mut chain_count = 0;
    let mut prompts_in_chains = 0;
    let mut tag_counts: HashMap<String, usize> = HashMap::new();

    if ctx.workspaces_dir.exists() {
        for workspace_entry in fs::read_dir(&ctx.workspaces_dir).map_err(|e| e.to_string())? {
            let workspace_path = workspace_entry.map_err(|e| e.to_string())?.path();
            if !workspace_path.is_dir() {
                continue;
            }
            for entry in fs::read_dir(&workspace_path).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.is_dir() {
                    chain_count += 1;
                    prompts_in_chains += process_directory(&path, &ctx.cipher, &mut tag_counts)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("prompt") {
                    standalone_prompts += 1;
                    if let Ok(prompt) = decrypt_full_prompt(&path, &ctx.cipher) {
                        for tag in prompt.tags {
                            *tag_counts.entry(tag).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    println!("{}", style("Prompt Store Statistics").bold().underlined());
    println!(
        "{}: {}",
        style("Total Chains").cyan(),
        style(chain_count).yellow()
    );
    println!(
        "{}: {}",
        style("Total Standalone Prompts").cyan(),
        style(standalone_prompts).yellow()
    );
    println!(
        "{}: {}",
        style("Prompts within Chains").cyan(),
        style(prompts_in_chains).yellow()
    );
    println!(
        "{}: {}",
        style("Total Prompts").cyan(),
        style(standalone_prompts + prompts_in_chains).yellow()
    );

    if !tag_counts.is_empty() {
        let mut sorted_tags: Vec<_> = tag_counts.into_iter().collect();
        sorted_tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        println!("\n{}", style("Top Tags:").bold().underlined());
        for (tag, count) in sorted_tags.iter().take(10) {
            println!("  - {} ({})", style(tag).green(), count);
        }
    }

    Ok(())
}

fn process_directory(
    dir: &Path,
    cipher: &aes_gcm::Aes256Gcm,
    tag_counts: &mut HashMap<String, usize>,
) -> Result<u32, String> {
    let mut count = 0;
    for entry in fs::read_dir(dir).map_err(|e| format!("Read dir error: {}", e))? {
        let path = entry.map_err(|e| format!("Dir entry error: {}", e))?.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("prompt") {
            count += 1;
            if let Ok(prompt) = decrypt_full_prompt(&path, cipher) {
                for tag in prompt.tags {
                    *tag_counts.entry(tag).or_insert(0) += 1;
                }
            }
        }
    }
    Ok(count)
}