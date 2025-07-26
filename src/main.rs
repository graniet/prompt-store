use clap::Parser;
use prompt_store::cli::Cli;
use prompt_store::commands::dispatch;
use prompt_store::core::storage::AppCtx;

pub mod cli;

/// Entry point of the application
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("• {}", e);
        std::process::exit(1);
    }
}

/// Run the CLI application
async fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let ctx = AppCtx::init()?;
    dispatch(cli.command, &ctx).await
}
