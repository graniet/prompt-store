use crate::core::storage::{decrypt_prompt_title, AppCtx};
use console::style;

/// List every saved prompt.
pub fn run(ctx: &AppCtx) -> Result<(), String> {
    let mut set = Vec::new();
    if ctx.prompts_dir.exists() {
        for entry in
            std::fs::read_dir(&ctx.prompts_dir).map_err(|e| format!("Read dir error: {}", e))?
        {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;
            if let Ok((id, title)) = decrypt_prompt_title(&ent.path(), &ctx.cipher) {
                if let Ok(n) = id.parse::<u64>() {
                    set.push((n, title));
                }
            }
        }
    }
    if set.is_empty() {
        println!("{}", style("No saved prompts").green().bold());
    } else {
        set.sort_by_key(|(n, _)| *n);
        println!("{}", style("Saved Prompts:").green().bold());
        for (id, title) in set {
            println!("  {} {} - {}", style("•").green(), id, title);
        }
    }
    Ok(())
}
