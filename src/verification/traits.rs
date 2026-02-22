use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::error::PanopticonError;
use crate::types::task::Task;

/// The result of a task execution, submitted for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub agent_id: Uuid,
    pub output: serde_json::Value,
    pub completed_at: DateTime<Utc>,
    pub resource_consumed: f64,
}

/// Outcome of a verification check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationOutcome {
    Passed { confidence: f64 },
    Failed { reason: String },
    Inconclusive,
}

/// Trait for all verification strategies.
#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(
        &self,
        task: &Task,
        result: &TaskResult,
    ) -> Result<VerificationOutcome, PanopticonError>;

    fn name(&self) -> &str;
}
