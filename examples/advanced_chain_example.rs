//! This example demonstrates advanced chain logic: conditional steps.
//!
//! Pre-requisites:
//! 1. Run `prompt-store new` to create the following prompts:
//!    - Title: "Sentiment Check", Content: "Is the following user feedback positive or negative? Answer with only the word 'positive' or 'negative'. Feedback: {{feedback}}"
//!    - Title: "Positive Reply", Content: "Write a short, cheerful thank you message for this positive feedback: {{feedback}}"
//!    - Title: "Negative Reply", Content: "Write a short, apologetic response and offer to help with this negative feedback: {{feedback}}"
//! 2. Ensure your key is password protected: `prompt-store rotate-key --password`
//! 3. Set `PROMPT_STORE_PASSWORD` and `OPENAI_API_KEY` environment variables.

use llm::builder::{LLMBackend, LLMBuilder};
use llm::chain::LLMRegistry;
use prompt_store::{PromptStore, RunError, RunOutput};

#[tokio::main]
async fn main() -> Result<(), RunError> {
    let password = std::env::var("PROMPT_STORE_PASSWORD")
        .expect("PROMPT_STORE_PASSWORD must be set for this example.");
    let store = PromptStore::with_password(&password)?;

    let openai_llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .api_key(std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set"))
        .model("gpt-4o-mini")
        .build()
        .unwrap();

    let mut registry = LLMRegistry::new();
    registry.insert("openai", openai_llm);

    // --- Test with negative feedback ---
    println!("--- Testing with NEGATIVE feedback ---");
    let user_feedback_negative = "The app keeps crashing, it's unusable!";
    let outputs_neg = run_chain(&store, &registry, user_feedback_negative).await?;
    if let RunOutput::Chain(map) = outputs_neg {
        // We expect `negative_reply` to exist, but `positive_reply` should not.
        assert!(map.contains_key("negative_reply"));
        assert!(!map.contains_key("positive_reply"));
        println!("\nFinal Response:\n{}", map.get("negative_reply").unwrap());
    }

    // --- Test with positive feedback ---
    println!("\n--- Testing with POSITIVE feedback ---");
    let user_feedback_positive = "I love the new update, it's so fast!";
    let outputs_pos = run_chain(&store, &registry, user_feedback_positive).await?;
    if let RunOutput::Chain(map) = outputs_pos {
        // We expect `positive_reply` to exist, but `negative_reply` should not.
        assert!(map.contains_key("positive_reply"));
        assert!(!map.contains_key("negative_reply"));
        println!("\nFinal Response:\n{}", map.get("positive_reply").unwrap());
    }

    Ok(())
}

async fn run_chain(
    store: &PromptStore,
    registry: &LLMRegistry,
    feedback: &str,
) -> Result<RunOutput, RunError> {
    store
        .chain(registry)
        // Step 1: Always run sentiment analysis.
        .step("sentiment", "Sentiment Check")
            .with_provider("openai")
        // Step 2 (Conditional): Only run if the sentiment is "positive".
        .step_if("positive_reply", "Positive Reply", |prev_outputs| {
            matches!(prev_outputs.get("sentiment"), Some(s) if s.trim().eq_ignore_ascii_case("positive"))
        })
            .with_provider("openai")

        // Step 3 (Conditional): Only run if the sentiment is "negative".
        .step_if("negative_reply", "Negative Reply", |prev_outputs| {
            matches!(prev_outputs.get("sentiment"), Some(s) if s.trim().eq_ignore_ascii_case("negative"))
        })
            .with_provider("openai")

        .vars([("feedback", feedback)])
        .run()
        .await
}
