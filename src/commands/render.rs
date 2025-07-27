use crate::core::storage::{decrypt_full_prompt, AppCtx};
use regex::Regex;
use std::collections::HashMap;

/// Render a template prompt with variables and print it to stdout.
pub fn run(ctx: &AppCtx, id: &str, vars: &[String]) -> Result<(), String> {
    let mut map = HashMap::new();
    for v in vars {
        if let Some((key, value)) = v.split_once('=') {
            map.insert(key.trim(), value.trim());
        }
    }

    let path = ctx.prompt_path(id);
    if !path.exists() {
        return Err(format!("No prompt with ID '{}'", id));
    }

    let pd = decrypt_full_prompt(&path, &ctx.cipher)?;

    let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    let rendered = re.replace_all(&pd.content, |caps: &regex::Captures| {
        map.get(&caps[1]).copied().unwrap_or("").to_string()
    });

    println!("{}", rendered);
    Ok(())
}