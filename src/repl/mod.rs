pub mod output;
pub mod router;
pub mod session;
pub mod slash;

use anyhow::Result;
use rustyline::DefaultEditor;
use std::path::PathBuf;

use crate::cli::state::AppState;
use crate::persistence::FileStore;
use crate::repl::session::Session;

/// Run the interactive REPL loop.
pub async fn run() -> Result<()> {
    // Resolve state directory.
    let state_dir = match std::env::var("PANOPTICON_STATE_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => FileStore::default_state_dir(),
    };

    let store = FileStore::new(&state_dir);
    let state = AppState::load_from(&store).await?;

    // Load config to get max_context_messages.
    let config = crate::config::PanopticonConfig::load(&state_dir).unwrap_or_default();

    let mut session = Session::new(config.max_context_messages as usize);

    output::print_welcome();

    // Set up rustyline editor — use spawn_blocking to coexist with tokio.
    let mut rl = DefaultEditor::new()?;

    loop {
        let prompt = output::build_prompt(&state);

        // Read line in a blocking thread so we don't block the tokio runtime.
        let line = {
            let prompt_clone = prompt.clone();
            tokio::task::spawn_blocking(move || {
                // Create a temporary editor for the blocking read.
                // This is a workaround because rustyline's Editor is not Send.
                let mut editor = DefaultEditor::new().expect("failed to create editor");
                editor.readline(&prompt_clone)
            })
            .await?
        };

        match line {
            Ok(input) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Add to history on the main editor (best-effort).
                let _ = rl.add_history_entry(trimmed);

                let result = if trimmed.starts_with('/') {
                    // Slash command — dispatch directly.
                    match slash::dispatch(trimmed, &state, &state_dir).await {
                        Ok(slash::SlashResult::Quit) => break,
                        Ok(slash::SlashResult::Continue) => Ok(()),
                        Err(e) => Err(e),
                    }
                } else {
                    // Natural language — route through Claude.
                    router::route_natural_language(trimmed, &state, &mut session).await
                };

                if let Err(e) = result {
                    output::print_error(&format!("{e:#}"));
                }

                // Auto-save after each command.
                if let Err(e) = state.save_to(&store).await {
                    output::print_error(&format!("Failed to save state: {e}"));
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                // Ctrl-C — ignore, just print a new prompt.
                println!("(Use /quit or Ctrl-D to exit)");
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                // Ctrl-D — exit.
                break;
            }
            Err(e) => {
                output::print_error(&format!("Input error: {e}"));
                break;
            }
        }
    }

    // Final save.
    state.save_to(&store).await?;
    output::print_success("State saved. Goodbye.");
    Ok(())
}
