use crate::core::storage::{AppCtx, PromptData};
use aes_gcm::aead::Aead;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use std::fs;

/// Search prompts by title, optional tag, optional full‑text content.
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
        for entry in
            fs::read_dir(&ctx.workspaces_dir).map_err(|e| format!("Read dir error: {}", e))?
        {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;
            let encoded =
                fs::read_to_string(ent.path()).map_err(|e| format!("Read error: {}", e))?;
            let decoded = general_purpose::STANDARD
                .decode(encoded.trim_end())
                .map_err(|_| "Corrupted data".to_string())?;
            if decoded.len() < 12 {
                continue;
            }
            let (nonce_bytes, cipher_bytes) = decoded.split_at(12);
            let plaintext = ctx
                .cipher
                .decrypt(aes_gcm::Nonce::from_slice(nonce_bytes), cipher_bytes)
                .map_err(|_| "Decrypt error".to_string())?;
            let pd: PromptData =
                serde_json::from_slice(&plaintext).map_err(|_| "Invalid JSON".to_string())?;

            let mut match_ok = pd.title.to_lowercase().contains(&q);
            if search_content {
                match_ok |= pd.content.to_lowercase().contains(&q);
            }
            if let Some(t) = &tag {
                match_ok &= pd.tags.iter().any(|x| x.to_lowercase() == *t);
            }

            if match_ok {
                hits.push((pd.id, pd.title));
            }
        }
    }

    if hits.is_empty() {
        println!("{}", style("No match").yellow());
    } else {
        println!("{}", style("Matches:").green().bold());
        for (id, title) in hits {
            println!("  {} {} - {}", style("•").green(), id, title);
        }
    }
    Ok(())
}
