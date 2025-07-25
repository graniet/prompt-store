use clap::{Parser, Subcommand};
use prompt_store::commands;
use prompt_store::core::storage::AppCtx;

#[derive(Parser)]
#[command(name = "prompt-store", version, about = "Encrypted prompts manager")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List all stored prompts
    List,
    /// Create a new prompt
    New,
    /// Get a specific prompt by ID
    Get { id: String },
    /// Edit an existing prompt
    Edit { id: String },
    /// Delete a prompt by ID
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
}

fn main() {
    if let Err(e) = run() {
        eprintln!("• {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let ctx = AppCtx::init()?;

    match cli.command {
        Cmd::List => commands::list::run(&ctx),
        Cmd::New => commands::new::run(&ctx),
        Cmd::Get { id } => commands::get::run(&ctx, &id),
        Cmd::Edit { id } => commands::edit::run(&ctx, &id),
        Cmd::Delete { id } => commands::delete::run(&ctx, &id),
        Cmd::Rename { id, title } => commands::rename::run(&ctx, &id, &title),
        Cmd::Search {
            query,
            tag,
            content,
        } => commands::search::run(&ctx, &query, tag.as_deref(), content),
        Cmd::Tag { id, changes } => commands::tag::run(&ctx, &id, &changes),
        Cmd::Copy { id } => commands::copy::run(&ctx, &id),
        Cmd::Run { id, vars } => commands::run::run(&ctx, &id, &vars),
        Cmd::Export { ids, out } => commands::export::run(&ctx, ids.as_deref(), &out),
        Cmd::Import { file } => commands::import::run(&ctx, &file),
        Cmd::History { id } => commands::history::run(&ctx, &id),
        Cmd::Revert { id, timestamp } => commands::revert::run(&ctx, &id, timestamp.as_deref()),
        Cmd::RotateKey { password } => commands::rotate_key::run(&ctx, password),
    }
}
