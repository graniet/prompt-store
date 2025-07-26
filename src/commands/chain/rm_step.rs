use crate::core::storage::AppCtx;
use console::style;
use dialoguer::Confirm;
use std::fs;

/// Remove a step from a chain.
pub fn run(ctx: &AppCtx, step_id: &str) -> Result<(), String> {
    let Some((chain_id, step_num_str)) = step_id.split_once('/') else {
        return Err("Invalid step ID format. Use 'chain_id/step_number'.".to_string());
    };

    let path = ctx.prompt_path(step_id);
    if !path.exists() {
        return Err(format!(
            "Step '{}' not found in chain '{}'.",
            step_num_str, chain_id
        ));
    }

    if Confirm::new()
        .with_prompt(format!("Are you sure you want to delete step {}?", step_id))
        .default(false)
        .interact()
        .unwrap_or(false)
    {
        fs::remove_file(path).map_err(|e| format!("Failed to delete step: {}", e))?;
        println!("{} Step '{}' removed.", style("•").green().bold(), step_id);
    } else {
        println!("Deletion cancelled.");
    }
    Ok(())
}
