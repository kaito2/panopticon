use colored::Colorize;

use crate::cli::state::AppState;

/// Print the welcome banner on REPL startup.
pub fn print_welcome() {
    println!(
        "\n{}",
        "Panopticon — Intelligent AI Delegation Framework"
            .bold()
            .cyan()
    );
    println!(
        "Type natural language or {} for commands. {} to exit.\n",
        "/help".bold(),
        "Ctrl-D".bold(),
    );
}

/// Build the prompt string showing current state counts.
pub fn build_prompt(state: &AppState) -> String {
    let task_count = state.tasks.len();
    let agent_count = state.agents.len();
    format!(
        "{} [{} tasks, {} agents]\n{} ",
        "panopticon".bold().cyan(),
        task_count.to_string().yellow(),
        agent_count.to_string().yellow(),
        ">".bold().green(),
    )
}

/// Print a system-level informational message.
pub fn print_info(msg: &str) {
    println!("{} {}", "ℹ".blue(), msg);
}

/// Print an error message.
pub fn print_error(msg: &str) {
    println!("{} {}", "✗".red(), msg.red());
}

/// Print a success message.
pub fn print_success(msg: &str) {
    println!("{} {}", "✓".green(), msg);
}

/// Print the help text for available slash commands.
pub fn print_help() {
    println!("{}", "Available commands:".bold());
    println!("  {}        Plan task decomposition from a goal", "/plan <goal>".cyan());
    println!(
        "  {}  Execute tasks (by ID or --all)",
        "/execute [id|--all]".cyan()
    );
    println!("  {}           Show task/agent dashboard", "/status".cyan());
    println!("  {}       Task management subcommands", "/task <sub>".cyan());
    println!("  {}      Agent management subcommands", "/agent <sub>".cyan());
    println!(
        "  {}     Configuration management",
        "/config <sub>".cyan()
    );
    println!("  {}             Run a demo delegation lifecycle", "/demo".cyan());
    println!("  {}             Show this help", "/help".cyan());
    println!("  {}             Exit the REPL", "/quit".cyan());
    println!();
    println!(
        "Or just type {} to interact via Claude.",
        "natural language".italic()
    );
}
