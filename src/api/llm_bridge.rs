//! Bridge types for interoperability with the `llm` crate.

use llm::chain::LLMRegistry;
use llm::LLMProvider;

/// Represents a backend for running prompts or chains.
///
/// This enum allows the fluent API to accept either a single LLM provider
/// or a registry of multiple providers, which is essential for multi-provider chains.
pub enum LLMBackendRef<'a> {
    /// A single LLM provider backend.
    Provider(&'a dyn LLMProvider),
    /// A registry of multiple LLM providers, identified by string keys.
    Registry(&'a LLMRegistry),
}

impl<'a> From<&'a dyn LLMProvider> for LLMBackendRef<'a> {
    fn from(llm: &'a dyn LLMProvider) -> Self {
        LLMBackendRef::Provider(llm)
    }
}

impl<'a> From<&'a LLMRegistry> for LLMBackendRef<'a> {
    fn from(reg: &'a LLMRegistry) -> Self {
        LLMBackendRef::Registry(reg)
    }
}
