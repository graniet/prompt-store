use crate::core::storage::{decrypt_prompt_header, AppCtx};
use console::style;

/// List every saved prompt.
pub fn run(ctx: &AppCtx) -> Result<(), String> {
    let mut set = Vec::new();
    if ctx.prompts_dir.exists() {
        for entry in
            std::fs::read_dir(&ctx.prompts_dir).map_err(|e| format!("Read dir error: {}", e))?
        {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;
            if let Ok((id, title)) = decrypt_prompt_header(&ent.path(), &ctx.cipher) {
                set.push((id, title));
            }
        }
    }
    if set.is_empty() {
        println!("{}", style("No saved prompts").green().bold());
    } else {
        set.sort_by(|a, b| a.0.cmp(&b.0));
        println!("{}", style("Saved Prompts:").green().bold());
        for (id, title) in set {
            println!("  {} {} - {}", style("•").green(), style(id).yellow(), title);
        }
    }
    Ok(())
}