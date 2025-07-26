use crate::commands::pack_logic::{install_pack_from_local_repo, DeployedInfo};
use crate::core::storage::AppCtx;
use console::style;
use git2::{build::CheckoutBuilder, FetchOptions, Repository};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Update deployed prompt pack(s).
pub async fn run(ctx: &AppCtx, alias_filter: Option<&str>) -> Result<(), String> {
    let manifest_path = ctx.base_dir.join("deployed.json");
    if !manifest_path.exists() {
        println!("No packs deployed yet. Use 'prompt-store deploy' to add one.");
        return Ok(());
    }

    let content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let mut manifest: HashMap<String, DeployedInfo> =
        serde_json::from_str(&content).unwrap_or_default();

    let packs_to_update: Vec<DeployedInfo> = manifest
        .values()
        .filter(|info| alias_filter.map_or(true, |alias| info.alias == alias))
        .cloned()
        .collect();

    if packs_to_update.is_empty() {
        return if let Some(alias) = alias_filter {
            Err(format!("Pack with alias '{}' not found.", alias))
        } else {
            println!("No packs to update.");
            Ok(())
        };
    }

    for pack in packs_to_update {
        println!(
            "Checking for updates in '{}'...",
            style(&pack.alias).yellow()
        );
        let repo_path = ctx.registries_dir.join(&pack.alias);

        let new_hash = pull_repo(&repo_path, &pack.alias)?;

        if new_hash == pack.commit_hash {
            println!("Pack '{}' is up to date.", style(&pack.alias).green());
            continue;
        }

        println!(
            "Updating '{}' from {} to {}...",
            style(&pack.alias).yellow(),
            &pack.commit_hash[..7],
            &new_hash[..7]
        );

        let password = env::var("PROMPT_PACK_PASSWORD").ok();
        install_pack_from_local_repo(ctx, &repo_path, &pack.alias, password.as_deref())?;

        // Update the manifest with the new hash
        if let Some(info) = manifest.get_mut(&pack.alias) {
            info.commit_hash = new_hash;
        }
    }

    let updated_content = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(manifest_path, updated_content).map_err(|e| e.to_string())?;

    Ok(())
}

fn pull_repo(repo_path: &Path, alias: &str) -> Result<String, String> {
    let repo = Repository::open(repo_path)
        .map_err(|e| format!("Failed to open local repository for '{}': {}", alias, e))?;

    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;

    // Simple fetch
    let mut fo = FetchOptions::new();
    remote
        .fetch(&["main"], Some(&mut fo), None)
        .map_err(|e| format!("Failed to fetch updates for '{}': {}", alias, e))?;

    let fetch_head = repo
        .find_reference("FETCH_HEAD")
        .map_err(|e| e.to_string())?;
    let fetch_commit = fetch_head.peel_to_commit().map_err(|e| e.to_string())?;

    // Simple fast-forward merge
    let main_ref_name = "refs/heads/main";
    let mut main_ref = repo
        .find_reference(main_ref_name)
        .map_err(|e| e.to_string())?;
    main_ref
        .set_target(fetch_commit.id(), "Fast-forward update")
        .map_err(|e| e.to_string())?;
    repo.set_head(main_ref_name).map_err(|e| e.to_string())?;
    repo.checkout_head(Some(CheckoutBuilder::new().force()))
        .map_err(|e| e.to_string())?;

    Ok(fetch_commit.id().to_string())
}
