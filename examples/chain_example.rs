//! This example demonstrates how to use the PromptStore with multi-step chains
//! that execute different prompts using different LLM providers.
//!
//! The example shows:
//! - Setting up multiple LLM providers (OpenAI and Anthropic)
//! - Creating a chain that uses stored prompts by ID
//! - Passing outputs between chain steps using template variables
//! - Using different providers for different steps

use llm::builder::{LLMBackend, LLMBuilder};
use llm::chain::LLMRegistry;
use llm::chain::MultiChainStepMode;
use prompt_store::{PromptStore, RunError, RunOutput};

#[tokio::main]
async fn main() -> Result<(), RunError> {
    // 1. Initialize the store once. This loads keys and configuration.
    // Or use with_password("password") to use a password to decrypt the vault of the prompts.
    let password = std::env::var("PROMPT_STORE_PASSWORD")
        .expect("PROMPT_STORE_PASSWORD must be set for this example.");
    let store = PromptStore::with_password(&password)?;

    // 2. Set up the LLM providers and a registry to hold them.
    let openai_llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .api_key(std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set"))
        .model("gpt-4o-mini")
        .max_tokens(1000)
        .build()
        .unwrap();

    let anthropic_llm = LLMBuilder::new()
        .backend(LLMBackend::Anthropic)
        .api_key(std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set"))
        .model("claude-3-5-sonnet-20240620")
        .max_tokens(1000)
        .build()
        .unwrap();

    let mut registry = LLMRegistry::new();
    registry.insert("openai_fast", openai_llm);
    registry.insert("anthropic_strong", anthropic_llm);

    // 3. Define and run the chain fluently, loading prompts from the store.
    let user_question = "How does photosynthesis work at the molecular level?";

    println!("Executing prompt chain for: \"{}\"", user_question);

    let outputs = store
        .chain(&registry) // Start a chain with the provider registry.
        // Step 1: uses the prompt with id "9k6zezem".
        // Its output will be available as the `{{analyse}}` variable.
        .step("analyse", "9k6zezem")
        .with_mode(MultiChainStepMode::Chat)
        .with_provider("openai_fast")
        // Step 2: uses the prompt with id "uetgwnq1".
        // It implicitly uses the `{{analyse}}` output from the previous step.
        .step("suggestions", "uetgwnq1")
        .with_mode(MultiChainStepMode::Chat)
        .with_provider("anthropic_strong")
        // Step 3: uses the prompt with id "dkeodfyp".
        // It can use both the initial `{{query}}` and `{{suggestions}}`.
        .step("final_response", "dkeodfyp")
        .with_mode(MultiChainStepMode::Chat)
        .with_provider("anthropic_strong")
        .step_raw(
            "raw",
            "Synthesize the following: {{final_response}} in 2 sentences.",
        )
        .with_mode(MultiChainStepMode::Chat)
        .with_provider("anthropic_strong")
        // Provide the initial variable for the first step.
        .vars([("query", user_question)])
        .run()
        .await?;

    // 4. Process the results.
    if let RunOutput::Chain(map) = outputs {
        println!("\n--- Chain Execution Complete ---");
        println!(
            "\n[✅] Final Answer (from 'final_response' step):\n{}",
            map.get("final_response").unwrap_or(&"N/A".to_string())
        );
        println!("\n--- Intermediate Steps ---");
        println!(
            "\n[1] Analysis ('analyse'):\n{}",
            map.get("analyse").unwrap_or(&"N/A".to_string())
        );
        println!(
            "\n[2] Suggestions ('suggestions'):\n{}",
            map.get("suggestions").unwrap_or(&"N/A".to_string())
        );
        println!(
            "\n[3] Raw ('raw'):\n{}",
            map.get("raw").unwrap_or(&"N/A".to_string())
        );
    }

    Ok(())
}
