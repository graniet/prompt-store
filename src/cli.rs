//! Defines the command-line interface structure using clap.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "prompt-store", version, about = "Encrypted prompts manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// List all stored prompts and chains
    List {
        #[arg(long, help = "Filter prompts by tag(s)")]
        tag: Vec<String>,
    },
    /// Create a new prompt
    New,
    /// Get a specific prompt by ID (including namespaced IDs like `pack::id`)
    Get { id: String },
    /// Edit an existing prompt
    Edit { id: String },
    /// Delete a prompt or chain by ID
    Delete { id: String },
    /// Rename a prompt's title
    Rename {
        id: String,
        #[arg(long, help = "New title for the prompt")]
        title: String,
    },
    /// Search prompts by query, optionally filtering by tag or content
    Search {
        query: String,
        #[arg(long, help = "Filter by specific tag")]
        tag: Option<String>,
        #[arg(long, help = "Search in prompt content")]
        content: bool,
    },
    /// Tag a prompt with one or more tags
    #[command(about = "Tag a prompt with one or more tags")]
    Tag { id: String, changes: Vec<String> },
    /// Copy a prompt to clipboard
    Copy { id: String },
    /// Run a prompt with variable substitution
    Run {
        id: String,
        #[arg(long = "var", help = "Variable assignments in key=value format")]
        vars: Vec<String>,
    },
    /// Export prompts to a file
    Export {
        #[arg(long, help = "Comma-separated list of prompt IDs to export")]
        ids: Option<String>,
        #[arg(long, help = "Output file path")]
        out: String,
    },
    /// Import prompts from a file
    Import { file: String },
    /// Show prompt revision history
    History { id: String },
    /// Revert a prompt to a previous version
    Revert {
        id: String,
        #[arg(long, help = "Specific timestamp to revert to")]
        timestamp: Option<String>,
    },
    /// Rotate the encryption key
    RotateKey {
        #[arg(long, help = "Protect the new key with a password")]
        password: bool,
    },
    /// Manage prompt chains
    #[command(subcommand)]
    Chain(ChainCmd),
    /// Manage prompt packs for sharing and deployment
    #[command(subcommand)]
    Pack(PackCmd),
    /// Deploy a prompt pack from a git repository
    Deploy {
        /// URL of the git repository to deploy
        repo_url: String,
        /// Optional local alias for the pack
        #[arg(long)]
        alias: Option<String>,
        /// Password for private/encrypted packs (can also be set via PROMPT_PACK_PASSWORD env var)
        #[arg(long, env = "PROMPT_PACK_PASSWORD")]
        password: Option<String>,
    },
    /// Update deployed prompt pack(s)
    Update {
        /// The alias of a specific pack to update. If omitted, all packs are updated.
        alias: Option<String>,
    },
    /// Show store statistics
    Stats,
    /// Start an interactive session (REPL)
    Interactive,
}

#[derive(Subcommand)]
pub enum ChainCmd {
    /// Create a new multi-step prompt chain interactively
    New,
    /// Edit a chain's metadata (e.g., title)
    Edit { id: String },
    /// Add a new step to an existing chain
    AddStep { id: String },
    /// Remove a step from a chain
    RmStep {
        #[arg(help = "The ID of the step to remove (e.g., mychain/1)")]
        step_id: String,
    },
}

#[derive(Subcommand)]
pub enum PackCmd {
    /// Export a workspace to a 'prompts.bundle' file for sharing
    Export {
        /// Workspace to export (defaults to 'default')
        #[arg(long)]
        workspace: Option<String>,
    },
}
