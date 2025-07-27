use crate::core::storage::{decrypt_full_prompt, AppCtx};
use console::style;
use std::fs;
use std::path::Path;

/// Search prompts by title, optional tag, optional full-text content across all workspaces.
pub fn run(
    ctx: &AppCtx,
    query: &str,
    tag_filter: Option<&str>,
    search_content: bool,
) -> Result<(), String> {
    let q = query.to_lowercase();
    let tag = tag_filter.map(|s| s.to_lowercase());
    let mut hits = Vec::new();

    if ctx.workspaces_dir.exists() {
        for workspace_entry in fs::read_dir(&ctx.workspaces_dir).map_err(|e| e.to_string())? {
            let workspace_path = workspace_entry.map_err(|e| e.to_string())?.path();
            if workspace_path.is_dir() {
                find_prompts_recursive(&workspace_path, &ctx, &q, &tag, search_content, &mut hits)?;
            }
        }
    }

    if hits.is_empty() {
        println!("{}", style("No match").yellow());
    } else {
        println!("{}", style("Matches:").green().bold());
        for (id, title) in hits {
            println!("  {} {} - {}", style("•").green(), style(id).yellow(), title);
        }
    }
    Ok(())
}

fn find_prompts_recursive(
    dir: &Path,
    ctx: &AppCtx,
    q: &str,
    tag: &Option<String>,
    search_content: bool,
    hits: &mut Vec<(String, String)>,
) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            find_prompts_recursive(&path, ctx, q, tag, search_content, hits)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("prompt") {
            if let Ok(pd) = decrypt_full_prompt(&path, &ctx.cipher) {
                let mut match_ok = pd.title.to_lowercase().contains(q);
                if search_content {
                    match_ok |= pd.content.to_lowercase().contains(q);
                }
                if let Some(t) = tag {
                    match_ok &= pd.tags.iter().any(|x| x.to_lowercase() == *t);
                }

                if match_ok {
                    hits.push((pd.id, pd.title));
                }
            }
        }
    }
    Ok(())
}