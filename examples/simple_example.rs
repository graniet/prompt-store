//! Simple example demonstrating basic usage of the prompt store with OpenAI.
//!
//! This example shows how to:
//! - Initialize a prompt store
//! - Configure an OpenAI LLM backend
//! - Execute a prompt with variables
//! - Handle the result

use llm::builder::{LLMBackend, LLMBuilder};
use prompt_store::PromptStore;

#[tokio::main]
async fn main() {
    // Get the OpenAI API key from environment variables
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    // Configure the OpenAI LLM backend
    let openai_llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .api_key(api_key)
        .model("gpt-4o")
        .build()
        .unwrap();

    // Initialize the prompt store
    let store = PromptStore::init().unwrap();

    // Execute a prompt with variables and get the result
    let result = store
        .prompt("welcome")
        .vars([("name", "Alice")])
        .backend(openai_llm.as_ref())
        .run()
        .await
        .expect("Prompt execution failed");

    println!("Result: {:?}", result);
}
