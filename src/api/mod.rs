//! High-level fluent API for running prompts and chains.

mod error;
mod llm_bridge;
mod runner;
mod store;

pub use error::{RunError, StoreError};
pub use llm_bridge::LLMBackendRef;
pub use runner::{ChainRunner, PromptRunner};
pub use store::PromptStore;

/// Result of running a prompt or chain.
#[derive(Debug, Clone)]
pub enum RunOutput {
    /// Output of a single prompt run (text content generated or rendered).
    Prompt(String),
    /// Outputs of a multi-step chain run (map of step IDs to generated text).
    Chain(std::collections::HashMap<String, String>),
}
