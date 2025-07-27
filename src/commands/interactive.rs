use crate::cli::Cli;
use crate::commands::dispatch;
use crate::core::storage::AppCtx;
use clap::Parser;
use console::style;
use dialoguer::Input;

/// Run the CLI in interactive REPL mode.
pub fn run(ctx: &AppCtx) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        println!("Entering interactive mode. Type 'exit' or 'quit' to leave.");
        loop {
            let input: String = Input::new()
                .with_prompt(format!("{}", style("ps >").blue().bold()))
                .interact_text()
                .map_err(|e| format!("Input error: {}", e))?;

            let input = input.trim();
            if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                break;
            }
            if input.is_empty() {
                continue;
            }

            let args = match shell_words::split(input) {
                Ok(args) => args,
                Err(e) => {
                    eprintln!("• Error parsing command: {}", e);
                    continue;
                }
            };

            let full_args = std::iter::once("prompt-store".to_string()).chain(args);

            match Cli::try_parse_from(full_args) {
                Ok(cli) => {
                    if let Err(e) = dispatch(cli.command, ctx).await {
                        eprintln!("• {}", e);
                    }
                }
                Err(e) => {
                    e.print().unwrap_or(());
                }
            }
        }
        Ok(())
    })
}