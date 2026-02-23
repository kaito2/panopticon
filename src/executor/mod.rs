pub mod claude;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::error::PanopticonError;
use crate::verification::TaskResult;

/// Context passed to the executor for a task run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Working directory for the execution.
    pub working_dir: Option<String>,
    /// Extra context/instructions to prepend to the prompt.
    pub system_prompt: Option<String>,
}

/// Trait for agent executors.
#[async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Execute a task and return the result.
    async fn execute(
        &self,
        task: &crate::types::Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult, PanopticonError>;

    /// Check if the executor backend is available.
    async fn health_check(&self) -> Result<bool, PanopticonError>;

    /// Name of this executor.
    fn name(&self) -> &str;
}

pub use claude::ClaudeExecutor;
