use crate::core::storage::AppCtx;
use console::style;
use std::fs;

/// Delete a prompt.
pub fn run(ctx: &AppCtx, id: &str) -> Result<(), String> {
    let path = ctx.prompt_path(id);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Delete error: {}", e))?;
        println!("{} prompt {} deleted", style("•").green().bold(), id);
        Ok(())
    } else {
        Err(format!("No prompt with ID {}", id))
    }
}
