//! This example demonstrates advanced chain logic: conditional steps, parallel execution, and fallbacks.
//!
//! Pre-requisites:
//! 1. Run `prompt-store new` to create the following prompts:
//!    - Title: "Extract Topic", Content: "What is the main topic of this text? Text: {{query}}"
//!    - Title: "Summarizer", Content: "Summarize this text about {{topic}}: {{query}}"
//!    - Title: "Keyword Extractor", Content: "List 5 keywords for this text about {{topic}}: {{query}}"
//!    - Title: "Basic Keyword Extractor", Content: "List basic keywords for: {{query}}"
//!    - Title: "Generate Tweet", Content: "Write a tweet about this summary: {{summary}}"
//! 2. Set `OPENAI_API_KEY` environment variable.

use llm::builder::{LLMBackend, LLMBuilder};
use llm::chain::LLMRegistry;
use prompt_store::{PromptStore, RunError, RunOutput};

#[tokio::main]
async fn main() -> Result<(), RunError> {
    let store = PromptStore::init()?;

    let openai_llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .api_key(std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set"))
        .model("gpt-4o-mini")
        .build()
        .unwrap();

    let mut registry = LLMRegistry::new();
    registry.insert("openai", openai_llm);

    let user_query = "Rust is a systems programming language focused on safety, speed, and concurrency. It achieves these goals without a garbage collector, using a unique ownership model with a borrow checker.";

    println!("--- Running Advanced Chain ---");

    let outputs = store
        .chain(&registry)
        // 1. First step runs sequentially
        .step("topic", "Extract Topic")
        .with_provider("openai")
        // 2. These two steps run in parallel, as they only depend on the previous context
        .parallel(|group| {
            group
                .step("summary", "Summarizer")
                // This step will fail because the provider doesn't exist
                .step("keywords", "Keyword Extractor")
                .with_provider("failing_provider")
        })
        .with_provider("openai") // Default provider for the group
        // 3. This is a fallback for the "keywords" step. It runs only if the main step fails.
        .on_error_stored("Basic Keyword Extractor")
        .with_provider("openai")
        // 4. This step runs only if the summary contains the word "safety"
        .step_if("tweet", "Generate Tweet", |ctx| {
            ctx.get("summary")
                .map_or(false, |s| s.to_lowercase().contains("safety"))
        })
        .with_provider("openai")
        .vars([("query", user_query)])
        .run()
        .await?;

    if let RunOutput::Chain(map) = outputs {
        println!("\n--- Chain Execution Complete ---");
        println!("\n[1] Topic: {}", map.get("topic").unwrap_or(&"N/A".into()));
        println!(
            "\n[2a] Summary: {}",
            map.get("summary").unwrap_or(&"N/A".into())
        );
        println!(
            "\n[2b] Keywords (used fallback): {}",
            map.get("keywords").unwrap_or(&"N/A".into())
        );

        if let Some(tweet) = map.get("tweet") {
            println!("\n[3] Conditional Tweet: {}", tweet);
        } else {
            println!("\n[3] Conditional Tweet: SKIPPED (condition not met)");
        }
    }

    Ok(())
}
