use crate::commands::pack_logic::{install_pack_from_local_repo, DeployedInfo};
use crate::core::storage::AppCtx;
use console::style;
use git2::Repository;
use std::collections::HashMap;
use std::fs;

/// Deploy a prompt pack from a git repository.
pub async fn run(
    ctx: &AppCtx,
    repo_url: &str,
    alias: Option<&str>,
    password: Option<&str>,
) -> Result<(), String> {
    let pack_alias = alias.map(String::from).unwrap_or_else(|| {
        repo_url
            .split('/')
            .last()
            .unwrap_or("default-pack")
            .trim_end_matches(".git")
            .to_string()
    });

    let registry_path = ctx.registries_dir.join(&pack_alias);
    if registry_path.exists() {
        return Err(format!(
            "A pack with alias '{}' already exists. Use 'update' to get the latest version.",
            pack_alias
        ));
    }

    println!("Cloning {}...", repo_url);
    let repo = Repository::clone(repo_url, &registry_path)
        .map_err(|e| format!("Failed to clone repository: {}", e))?;

    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD for repo: {}", e))?;
    let commit_hash = head
        .target()
        .ok_or_else(|| "Invalid HEAD commit".to_string())?
        .to_string();

    let num_prompts = install_pack_from_local_repo(ctx, &registry_path, &pack_alias, password)?;
    update_deployment_manifest(ctx, &pack_alias, repo_url, &commit_hash)?;

    println!(
        "{} Successfully deployed {} prompts from pack '{}'.",
        style("✔").green(),
        num_prompts,
        style(pack_alias).yellow()
    );
    Ok(())
}

fn update_deployment_manifest(
    ctx: &AppCtx,
    alias: &str,
    url: &str,
    commit_hash: &str,
) -> Result<(), String> {
    let manifest_path = ctx.base_dir.join("deployed.json");
    let mut manifest: HashMap<String, DeployedInfo> = if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    let info = DeployedInfo {
        alias: alias.to_string(),
        url: url.to_string(),
        commit_hash: commit_hash.to_string(),
    };
    manifest.insert(alias.to_string(), info);

    let content = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(manifest_path, content).map_err(|e| e.to_string())?;
    Ok(())
}
