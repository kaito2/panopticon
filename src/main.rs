use anyhow::Result;
use clap::Parser;
use panopticon::cli::{Cli, Commands, commands, state};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let state = state::AppState::new();

    match cli.command {
        Commands::Task { action } => commands::task::handle(action, &state).await?,
        Commands::Agent { action } => commands::agent::handle(action, &state).await?,
        Commands::Demo => commands::demo::run(&state).await?,
    }

    Ok(())
}
