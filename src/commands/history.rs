use crate::core::storage::{parse_id, AppCtx};
use console::style;
use std::fs;

/// List backups for a prompt ID.
pub fn run(ctx: &AppCtx, id: &str) -> Result<(), String> {
    let (workspace, local_id) = parse_id(id);
    let workspace_path = ctx.workspaces_dir.join(workspace);
    let mut backups = Vec::new();

    if workspace_path.exists() {
        for entry in fs::read_dir(&workspace_path).map_err(|e| format!("Read dir error: {}", e))? {
            let ent = entry.map_err(|e| format!("Dir read error: {}", e))?;
            let fname = ent.file_name();
            if let Some(name) = fname.to_str() {
                if name.starts_with(&format!("{}.", local_id)) && name.ends_with(".bak") {
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