use crate::core::storage::AppCtx;
use chrono::Local;
use console::style;
use std::fs;

/// Revert a prompt to a backup (latest if none provided).
pub fn run(ctx: &AppCtx, id: &str, ts: Option<&str>) -> Result<(), String> {
    let mut backups = Vec::new();

    if ctx.prompts_dir.exists() {
        for entry in fs::read_dir(&ctx.prompts_dir).map_err(|e| format!("Read dir error: {}", e))? {
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
        return Err("No backups found".to_string());
    }
    backups.sort();
    let target_name = match ts {
        Some(t) => {
            let n = format!("{}.{}.bak", id, t);
            if !backups.contains(&n) {
                return Err("Timestamp not found".to_string());
            }
            n
        }
        None => backups.last().unwrap().to_string(),
    };

    let backup_path = ctx.prompts_dir.join(&target_name);
    let main_path = ctx.prompt_path(id);

    if !main_path.exists() {
        return Err("Main prompt missing".to_string());
    }

    let current_ts = Local::now().format("%Y%m%d%H%M%S").to_string();
    let current_backup = ctx.prompts_dir.join(format!("{}.{}.bak", id, current_ts));
    fs::copy(&main_path, &current_backup).map_err(|e| format!("Backup current error: {}", e))?;

    fs::copy(&backup_path, &main_path).map_err(|e| format!("Revert error: {}", e))?;
    println!("{} reverted to {}", style("•").green().bold(), target_name);
    Ok(())
}
