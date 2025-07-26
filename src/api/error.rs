//! Error types for the library API.

use llm::error::LLMError;
use thiserror::Error;

/// Errors related to the prompt store (file access, crypto, etc.).
#[derive(Error, Debug)]
pub enum StoreError {
    /// An error occurred during store initialization.
    #[error("Failed to initialize store: {0}")]
    Init(String),

    /// The requested prompt or chain could not be found by its ID or title.
    #[error("Prompt or chain '{0}' not found")]
    NotFound(String),

    /// A given ID exists for both a prompt and a chain, causing ambiguity.
    #[error("ID '{0}' is ambiguous (found both a prompt and a chain)")]
    AmbiguousId(String),

    /// A given title matches multiple prompts or chains.
    #[error("Title '{0}' is ambiguous (multiple matches found)")]
    AmbiguousTitle(String),

    /// The API was used with an invalid configuration.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// An underlying file I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A cryptographic operation (encryption/decryption) failed.
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// Failed to serialize or deserialize data.
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A comprehensive error type for all operations in the library API.
#[derive(Error, Debug)]
pub enum RunError {
    /// An error originating from the prompt store itself.
    #[error(transparent)]
    Store(#[from] StoreError),

    /// An error originating from the underlying LLM backend.
    #[error("LLM backend error: {0}")]
    LLM(#[from] LLMError),
}