use crate::api::PromptStore;
use crate::core::config::load_llm_registry;
use crate::core::storage::{parse_id, AppCtx};
use aes_gcm::aead::Aead;
use aes_gcm::Nonce;
use base64::{engine::general_purpose, Engine as _};
use console::style;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum StepDefinition {
    Sequential(Step),
    Parallel { parallel: Vec<Step> },
}

#[derive(Deserialize, Debug)]
struct Step {
    id: String,
    prompt: String,
    provider: String,
    #[serde(rename = "if", default)]
    condition: Option<Condition>,
    #[serde(default)]
    on_error: Option<FallbackStep>,
}

#[derive(Deserialize, Debug, Clone)]
struct Condition {
    variable: String,
    contains: Option<String>,
    equals: Option<String>,
}

#[derive(Deserialize, Debug)]
struct FallbackStep {
    prompt: String,
    // provider field can be added here if needed, for now assumes same provider
}

#[derive(Deserialize, Debug)]
struct ChainFile {
    #[serde(default)]
    vars: HashMap<String, String>,
    steps: Vec<StepDefinition>,
}

/// Run a stored prompt chain.
pub async fn run(ctx: &AppCtx, id: &str, vars_override: &[String]) -> Result<(), String> {
    let (workspace, local_id) = parse_id(id);
    let chain_path = ctx
        .workspaces_dir
        .join(workspace)
        .join("chains")
        .join(format!("{}.chain", local_id));

    if !chain_path.exists() {
        return Err(format!("Chain with ID '{}' not found.", id));
    }

    let encrypted_b64 = fs::read_to_string(&chain_path).map_err(|e| e.to_string())?;
    let encrypted_bytes = general_purpose::STANDARD
        .decode(encrypted_b64.trim())
        .map_err(|e| e.to_string())?;
    let (nonce, ciphertext) = encrypted_bytes.split_at(12);
    let yaml_bytes = ctx
        .cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| "Failed to decrypt chain file. Check master password.".to_string())?;

    let mut chain_def: ChainFile =
        serde_yaml::from_slice(&yaml_bytes).map_err(|e| format!("Failed to parse chain file: {}", e))?;

    // Override variables from CLI
    for var_pair in vars_override {
        if let Some((key, value)) = var_pair.split_once('=') {
            chain_def.vars.insert(key.to_string(), value.to_string());
        }
    }

    let registry = load_llm_registry()?;
    if registry.backends.is_empty() {
        println!("{}", style("Warning: No LLM providers configured in ~/.prompt-store/config.toml. Chain execution may fail.").yellow());
    }
    
    let store = PromptStore::init().map_err(|e| e.to_string())?;
    let mut runner = store.chain(&registry).vars(chain_def.vars);

    for step_def in chain_def.steps {
        runner = match step_def {
            StepDefinition::Sequential(step) => {
                let runner_with_step = if let Some(cond) = step.condition {
                    runner.step_if(&step.id, &step.prompt, move |ctx| check_condition(ctx, &cond))
                } else {
                    runner.step(&step.id, &step.prompt)
                };

                let runner_with_fallback = if let Some(fallback) = step.on_error {
                    runner_with_step.on_error_stored(&fallback.prompt)
                } else {
                    runner_with_step
                };
                
                runner_with_fallback.with_provider(&step.provider)
            }
            StepDefinition::Parallel { parallel } => {
                runner.parallel(|group| {
                    let mut current_group = group;
                    for step in parallel {
                        let step_id = step.id.clone();
                        let prompt = step.prompt.clone();
                        let provider = step.provider.clone();

                        let group_with_step = if let Some(cond) = step.condition {
                            current_group.step_if(&step_id, &prompt, move |ctx| check_condition(ctx, &cond))
                        } else {
                            current_group.step(&step_id, &prompt)
                        };

                        let group_with_fallback = if let Some(fallback) = step.on_error {
                             group_with_step.on_error_stored(&fallback.prompt)
                        } else {
                            group_with_step
                        };

                        current_group = group_with_fallback.with_provider(&provider);
                    }
                    current_group
                })
            }
        };
    }

    println!("Executing chain '{}'...", style(id).yellow());
    match runner.run().await {
        Ok(output) => {
            println!("{}", style("✔ Chain execution complete.").green());
            println!("{:#?}", output);
        }
        Err(e) => return Err(format!("Chain execution failed: {}", e)),
    }

    Ok(())
}

fn check_condition(ctx: &HashMap<String, String>, cond: &Condition) -> bool {
    if let Some(val) = ctx.get(&cond.variable) {
        if let Some(expected) = &cond.equals {
            return val == expected;
        }
        if let Some(substring) = &cond.contains {
            return val.contains(substring);
        }
    }
    false
}