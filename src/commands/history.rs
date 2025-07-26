use crate::core::storage::AppCtx;
use console::style;
use std::fs;

/// List backups for a prompt ID.
pub fn run(ctx: &AppCtx, id: &str) -> Result<(), String> {
    let mut backups = Vec::new();

    if ctx.workspaces_dir.exists() {
        for entry in
            fs::read_dir(&ctx.workspaces_dir).map_err(|e| format!("Read dir error: {}", e))?
        {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;
            let fname = ent.file_name();
            if let Some(name) = fname.to_str() {
                if name.starts_with(&format!("{}.", id)) && name.ends_with(".bak") {
                    backups.push(name.to_string());
                }
            }
        }
    }

    if backups.is_empty() {
        println!("{}", style("No backups").yellow());
    } else {
        backups.sort();
        println!("{}", style("Backups:").green().bold());
        for b in backups {
            println!("  {} {}", style("•").green(), b);
        }
    }
    Ok(())
}
