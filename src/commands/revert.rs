use crate::core::storage::{parse_id, AppCtx};
use chrono::Local;
use console::style;
use std::fs;

/// Revert a prompt to a backup (latest if none provided).
pub fn run(ctx: &AppCtx, id: &str, ts: Option<&str>) -> Result<(), String> {
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
        return Err("No backups found".to_string());
    }
    backups.sort();
    let target_name = match ts {
        Some(t) => {
            let n = format!("{}.{}.bak", local_id, t);
            if !backups.contains(&n) {
                return Err("Timestamp not found".to_string());
            }
            n
        }
        None => backups.last().unwrap().to_string(),
    };

    let backup_path = workspace_path.join(&target_name);
    let main_path = ctx.prompt_path(id);

    if !main_path.exists() {
        return Err("Main prompt missing".to_string());
    }

    let current_ts = Local::now().format("%Y%m%d%H%M%S").to_string();
    let current_backup = workspace_path.join(format!("{}.{}.bak", local_id, current_ts));
    fs::copy(&main_path, &current_backup).map_err(|e| format!("Backup current error: {}", e))?;

    fs::copy(&backup_path, &main_path).map_err(|e| format!("Revert error: {}", e))?;
    println!("{} reverted to {}", style("•").green().bold(), target_name);
    Ok(())
}
