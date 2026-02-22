pub mod commands;
pub mod state;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "panopticon", about = "Intelligent AI Delegation Framework")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Task management
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    /// Agent management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Run a demo delegation lifecycle
    Demo,
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// Create a new task
    Create {
        /// Task name
        #[arg(long)]
        name: String,
        /// Task description
        #[arg(long)]
        description: String,
        /// Complexity (0.0-1.0)
        #[arg(long, default_value_t = 0.5)]
        complexity: f64,
        /// Criticality (0.0-1.0)
        #[arg(long, default_value_t = 0.5)]
        criticality: f64,
        /// Verifiability (0.0-1.0)
        #[arg(long, default_value_t = 0.5)]
        verifiability: f64,
        /// Reversibility (0.0-1.0)
        #[arg(long, default_value_t = 0.5)]
        reversibility: f64,
    },
    /// List all tasks
    List,
    /// Get a task by ID
    Get {
        /// Task UUID
        id: uuid::Uuid,
    },
    /// Apply a state transition event
    Transition {
        /// Task UUID
        id: uuid::Uuid,
        /// Event name (e.g. StartDecomposition, SkipDecomposition, StartNegotiation, ...)
        #[arg(long)]
        event: String,
    },
    /// Decompose a task into subtasks
    Decompose {
        /// Task UUID
        id: uuid::Uuid,
        /// Strategy: sequential, parallel, hybrid
        #[arg(long, default_value = "hybrid")]
        strategy: String,
    },
}

#[derive(Subcommand)]
pub enum AgentAction {
    /// Register a new agent
    Register {
        /// Agent name
        #[arg(long)]
        name: String,
        /// Capabilities (comma-separated)
        #[arg(long)]
        capabilities: Option<String>,
    },
    /// List all agents
    List,
    /// Get an agent by ID
    Get {
        /// Agent UUID
        id: uuid::Uuid,
    },
    /// Show agent reputation
    Reputation {
        /// Agent UUID
        id: uuid::Uuid,
    },
}
