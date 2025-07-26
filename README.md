# Prompt Store

**Prompt Store** is a secure, encrypted CLI and Rust library for managing and orchestrating AI prompts. Built with **AES-256-GCM encryption**, it provides a robust vault for your prompt templates, allowing you to organize, version, and execute them across various LLM backends.

With a fluent, builder-style API and a powerful interactive CLI, you can create simple prompts or complex multi-step chains with advanced logic like conditional execution and parallel processing.

## Key Features

  - **Secure Vault**: All prompts are encrypted at rest with AES-256-GCM, optionally protected by a master password.
  - **Powerful CLI**: Manage your prompts with intuitive commands for creation, listing, editing, and versioning.
  - **Interactive Mode**: A REPL-style `interactive` command for rapid prompt exploration and execution.
  - **Tagging and Search**: Organize prompts with tags and find them easily with powerful search capabilities.
  - **Fluent Library API**: A developer-friendly, chainable API for integrating prompt execution into your Rust applications.
  - **Advanced Chaining**:
      - Build multi-step, multi-provider prompt chains programmatically.
      - Execute steps in **parallel** for improved performance.
      - Use **conditional steps** (`step_if`) for dynamic workflow logic.
      - Define **fallbacks** (`on_error`) for robust error handling.
  - **Variable Substitution**: Prompts are templates that accept variables from initial input and the outputs of previous steps.
  - **Version History**: Automatically creates backups on edits, allowing you to view history and revert to previous versions.

## Installation

### CLI Tool

You can install the command-line tool directly from the repository:

```shell
cargo install --git https://github.com/graniet/prompt-store.git
```

*Ensure `~/.cargo/bin` is in your shell's `PATH`.*

### Library

To use `prompt-store` as a library in your Rust project, add this to your `Cargo.toml`:

```toml
[dependencies]
prompt-store = { git = "https://github.com/graniet/prompt-store.git" }
```

## CLI Usage

The `prompt-store` CLI provides a comprehensive set of commands to manage your prompt vault.

  - **`prompt-store new`**: Interactively create a new standalone prompt.
  - **`prompt-store chain new`**: Interactively create a new multi-step prompt chain.
  - **`prompt-store list`**: Display all prompts and chains.
      - `prompt-store list --tag rust --tag api`: Filter standalone prompts by tags.
  - **`prompt-store get <id>`**: Display the content of a specific prompt.
  - **`prompt-store run <id> --var key=value`**: Render a prompt with variables.
  - **`prompt-store stats`**: Show statistics about your vault.
  - **`prompt-store interactive`**: Start an interactive REPL session.

For a full list of commands, run `prompt-store --help`.

## Library Usage

The library offers a powerful, fluent API for prompt execution and chaining, designed to be clean and intuitive.

### Basic Prompt Execution

```rust
use prompt_store::{PromptStore, RunOutput};
use llm::builder::{LLMBuilder, LLMBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the store once. For non-interactive use, see `PromptStore::with_password()`.
    let store = PromptStore::init()?;

    let openai_llm = LLMBuilder::new()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .model("gpt-4o-mini")
        .build()?;

    // Run a prompt stored with the ID 'welcome-prompt'
    let output = store.prompt("welcome-prompt")
        .vars([("name", "Alice")])
        .backend(openai_llm.as_ref())
        .run()
        .await?;

    if let RunOutput::Prompt(text) = output {
        println!("LLM Output: {}", text);
    }

    Ok(())
}
```

### Advanced Chain Execution

Dynamically build and run a chain with parallel steps, conditional logic, and error fallbacks.

```rust
use prompt_store::{PromptStore, RunOutput};
use llm::chain::LLMRegistry;
use llm::builder::LLMBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = PromptStore::init()?;

    // Setup multiple providers
    let mut registry = LLMRegistry::new();
    registry.insert("openai", LLMBuilder::new().api_key(std::env::var("OPENAI_API_KEY")?).build()?);
    registry.insert("failing_provider", llm::NoopLLM::new()); // For fallback demo

    let user_query = "Explain Rust's ownership model.";

    let outputs = store
        .chain(&registry)
        // 1. Sequential step
        .step("topic", "extract-topic-prompt")
            .with_provider("openai")
        
        // 2. Parallel steps
        .parallel(|group| {
            group
                .step("summary", "summarizer-prompt")
                .step("keywords", "keyword-extractor-prompt")
                    .with_provider("failing_provider") // This step will fail
        })
        .with_provider("openai") // Default provider for the parallel group
        
        // 3. Fallback for the failed "keywords" step
        .on_error_stored("basic-keyword-extractor")
            .with_provider("openai")
        
        // 4. Conditional step
        .step_if("tweet", "generate-tweet-prompt", |ctx| {
            ctx.get("summary").map_or(false, |s| s.len() > 50)
        })
            .with_provider("openai")
        
        .vars([("query", user_query)])
        .run()
        .await?;

    if let RunOutput::Chain(map) = outputs {
        println!("Summary: {}", map.get("summary").unwrap_or(&"N/A".into()));
        println!("Keywords (from fallback): {}", map.get("keywords").unwrap_or(&"N/A".into()));
        if let Some(tweet) = map.get("tweet") {
            println!("Generated Tweet: {}", tweet);
        }
    }

    Ok(())
}
```

## Examples

The `examples/` directory contains functional code demonstrating various features:

| Name                                       | Description                                                                 |
| ------------------------------------------ | --------------------------------------------------------------------------- |
| [`simple_example.rs`](examples/simple_example.rs)             | Basic execution of a single stored prompt.                                  |
| [`chain_example.rs`](examples/chain_example.rs)               | A multi-step chain using different providers and raw prompts.               |
| [`advanced_chain_example.rs`](examples/advanced_chain_example.rs) | Demonstrates conditional logic (`step_if`) for dynamic workflows.           |
| [`parallel_example.rs`](examples/parallel_example.rs)           | Showcases parallel execution, conditional logic, and error fallbacks together. |