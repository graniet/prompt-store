use crate::cli::{ChainCmd, Cmd};
use crate::core::storage::AppCtx;

pub mod chain;
pub mod copy;
pub mod delete;
pub mod edit;
pub mod export;
pub mod get;
pub mod history;
pub mod import;
pub mod interactive;
pub mod list;
pub mod new;
pub mod rename;
pub mod revert;
pub mod rotate_key;
pub mod run;
pub mod search;
pub mod stats;
pub mod tag;

/// Dispatches the parsed command to the appropriate handler.
pub fn dispatch(command: Cmd, ctx: &AppCtx) -> Result<(), String> {
    match command {
        Cmd::List { tag } => list::run(ctx, &tag),
        Cmd::New => new::run(ctx),
        Cmd::Get { id } => get::run(ctx, &id),
        Cmd::Edit { id } => edit::run(ctx, &id),
        Cmd::Delete { id } => delete::run(ctx, &id),
        Cmd::Rename { id, title } => rename::run(ctx, &id, &title),
        Cmd::Search {
            query,
            tag,
            content,
        } => search::run(ctx, &query, tag.as_deref(), content),
        Cmd::Tag { id, changes } => tag::run(ctx, &id, &changes),
        Cmd::Copy { id } => copy::run(ctx, &id),
        Cmd::Run { id, vars } => run::run(ctx, &id, &vars),
        Cmd::Export { ids, out } => export::run(ctx, ids.as_deref(), &out),
        Cmd::Import { file } => import::run(ctx, &file),
        Cmd::History { id } => history::run(ctx, &id),
        Cmd::Revert { id, timestamp } => revert::run(ctx, &id, timestamp.as_deref()),
        Cmd::RotateKey { password } => rotate_key::run(ctx, password),
        Cmd::Stats => stats::run(ctx),
        Cmd::Interactive => interactive::run(ctx),
        Cmd::Chain(chain_cmd) => match chain_cmd {
            ChainCmd::New => chain::new::run(ctx),
            ChainCmd::Edit { id } => chain::edit::run(ctx, &id),
            ChainCmd::AddStep { id } => chain::add_step::run(ctx, &id),
            ChainCmd::RmStep { step_id } => chain::rm_step::run(ctx, &step_id),
        },
    }
}
