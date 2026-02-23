use anyhow::{Result, bail};
use std::path::Path;
use uuid::Uuid;

use crate::cli::commands;
use crate::cli::state::AppState;
use crate::repl::output;

/// The result of dispatching a slash command.
pub enum SlashResult {
    /// Command handled, continue REPL.
    Continue,
    /// User requested quit.
    Quit,
}

/// Parse and dispatch a slash command line.
/// The `input` must start with '/'.
pub async fn dispatch(input: &str, state: &AppState, state_dir: &Path) -> Result<SlashResult> {
    let trimmed = input.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("");
    let args_str = parts.next().unwrap_or("").trim();

    match cmd {
        "/quit" | "/exit" | "/q" => {
            return Ok(SlashResult::Quit);
        }

        "/help" | "/h" => {
            output::print_help();
        }

        "/status" | "/s" => {
            commands::status::handle(state).await?;
        }

        "/demo" => {
            commands::demo::run(state).await?;
        }

        "/plan" => {
            if args_str.is_empty() {
                bail!("Usage: /plan <goal>");
            }
            commands::plan::handle(args_str, "sonnet", state).await?;
        }

        "/execute" | "/exec" => {
            if args_str.is_empty() || args_str == "--all" {
                commands::execute::handle(None, true, "sonnet", state).await?;
            } else {
                let id: Uuid = args_str
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid UUID: {args_str}"))?;
                commands::execute::handle(Some(id), false, "sonnet", state).await?;
            }
        }

        "/task" => {
            dispatch_task(args_str, state).await?;
        }

        "/agent" => {
            dispatch_agent(args_str, state).await?;
        }

        "/config" => {
            dispatch_config(args_str, state_dir).await?;
        }

        other => {
            output::print_error(&format!("Unknown command: {other}. Type /help for a list."));
        }
    }

    Ok(SlashResult::Continue)
}

/// Dispatch `/task` subcommands.
async fn dispatch_task(args: &str, state: &AppState) -> Result<()> {
    let mut parts = args.splitn(2, char::is_whitespace);
    let sub = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();

    match sub {
        "list" | "ls" | "" => {
            use crate::cli::TaskAction;
            commands::task::handle(TaskAction::List, state).await?;
        }
        "get" => {
            let id: Uuid = rest
                .parse()
                .map_err(|_| anyhow::anyhow!("Usage: /task get <uuid>"))?;
            use crate::cli::TaskAction;
            commands::task::handle(TaskAction::Get { id }, state).await?;
        }
        "create" => {
            bail!(
                "Use /plan <goal> to create tasks via Claude, or:\n  \
                 /task create --name <n> --description <d>"
            );
        }
        other => {
            bail!("Unknown task subcommand: {other}\nAvailable: list, get, create");
        }
    }
    Ok(())
}

/// Dispatch `/agent` subcommands.
async fn dispatch_agent(args: &str, state: &AppState) -> Result<()> {
    let mut parts = args.splitn(2, char::is_whitespace);
    let sub = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();

    match sub {
        "list" | "ls" | "" => {
            use crate::cli::AgentAction;
            commands::agent::handle(AgentAction::List, state).await?;
        }
        "get" => {
            let id: Uuid = rest
                .parse()
                .map_err(|_| anyhow::anyhow!("Usage: /agent get <uuid>"))?;
            use crate::cli::AgentAction;
            commands::agent::handle(AgentAction::Get { id }, state).await?;
        }
        "reputation" | "rep" => {
            let id: Uuid = rest
                .parse()
                .map_err(|_| anyhow::anyhow!("Usage: /agent reputation <uuid>"))?;
            use crate::cli::AgentAction;
            commands::agent::handle(AgentAction::Reputation { id }, state).await?;
        }
        other => {
            bail!("Unknown agent subcommand: {other}\nAvailable: list, get, reputation");
        }
    }
    Ok(())
}

/// Dispatch `/config` subcommands.
async fn dispatch_config(args: &str, state_dir: &Path) -> Result<()> {
    let sub = args.split_whitespace().next().unwrap_or("");
    match sub {
        "show" | "" => {
            use crate::cli::ConfigAction;
            commands::config::handle(ConfigAction::Show, state_dir).await?;
        }
        "init" => {
            use crate::cli::ConfigAction;
            commands::config::handle(ConfigAction::Init, state_dir).await?;
        }
        other => {
            bail!("Unknown config subcommand: {other}\nAvailable: show, init");
        }
    }
    Ok(())
}
