pub mod commands;
pub mod state;

/// Task management actions (used by slash command dispatch).
pub enum TaskAction {
    /// Create a new task
    Create {
        name: String,
        description: String,
        complexity: f64,
        criticality: f64,
        verifiability: f64,
        reversibility: f64,
        capabilities: Option<String>,
    },
    /// List all tasks
    List,
    /// Get a task by ID
    Get { id: uuid::Uuid },
    /// Apply a state transition event
    Transition { id: uuid::Uuid, event: String },
    /// Decompose a task into subtasks
    Decompose { id: uuid::Uuid, strategy: String },
    /// Assign a task to an agent
    Assign { id: uuid::Uuid, agent: uuid::Uuid },
    /// Verify a completed task
    Verify { id: uuid::Uuid },
}

/// Agent management actions.
pub enum AgentAction {
    /// Register a new agent
    Register {
        name: String,
        capabilities: Option<String>,
    },
    /// List all agents
    List,
    /// Get an agent by ID
    Get { id: uuid::Uuid },
    /// Show agent reputation
    Reputation { id: uuid::Uuid },
}

/// Configuration actions.
pub enum ConfigAction {
    /// Initialize default config file
    Init,
    /// Show current configuration
    Show,
}
