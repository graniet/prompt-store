//! # prompt-store: A secure, encrypted CLI and library for managing AI prompts.
//!
//! This crate provides both a command-line interface and a Rust library for securely
//! storing, organizing, and executing AI prompts and multi-step prompt chains.
//!
//! ## Library Usage Example
//!
//! ```no_run
//! use prompt_store::{PromptStore, RunOutput};
//! use llm::builder::{LLMBuilder, LLMBackend};
//! use llm::chain::LLMRegistry;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize the store once. This handles loading the encryption key.
//!     let store = PromptStore::init()?;
//!
//!     // Setup LLM providers
//!     let openai_key = std::env::var("OPENAI_API_KEY")?;
//!     let openai_llm = LLMBuilder::new()
//!         .backend(LLMBackend::OpenAI)
//!         .api_key(openai_key)
//!         .model("gpt-4o-mini")
//!         .build()?;
//!
//!     let mut registry = LLMRegistry::new();
//!     registry.insert("openai", openai_llm);
//!
//!     // --- Run a single stored prompt ---
//!     let output = store.prompt("my-prompt-id")
//!         .vars([("name", "Alice")])
//!         .backend(registry.get("openai").unwrap())
//!         .run()
//!         .await?;
//!
//!     if let RunOutput::Prompt(text) = output {
//!         println!("Single prompt output: {}", text);
//!     }
//!
//!     // --- Define and run a dynamic chain ---
//!     let chain_output = store.chain(&registry)
//!         .step("analysis", "prompt-for-analysis") // Loads a prompt with ID/title "prompt-for-analysis"
//!             .with_provider("openai")
//!         .step_raw("synthesis", "Synthesize the following: {{analysis}}") // Uses a raw string as a prompt
//!             .with_provider("openai")
//!         .vars([("input_data", "Some data to analyze.")])
//!         .run()
//!         .await?;
//!
//!     if let RunOutput::Chain(map) = chain_output {
//!         println!("Final chain output: {}", map.get("synthesis").unwrap());
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod commands;
pub mod core;
pub mod ui;

// Main library entry points
pub use api::{PromptStore, RunError, RunOutput, StoreError};
