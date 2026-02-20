use thiserror::Error;

use crate::task::{TaskEvent, TaskState};

#[derive(Debug, Error)]
pub enum PanopticonError {
    #[error("State transition error: cannot transition from {from:?} via {event:?}")]
    InvalidStateTransition { from: TaskState, event: TaskEvent },

    #[error("Task not found: {0}")]
    TaskNotFound(uuid::Uuid),

    #[error("Agent not found: {0}")]
    AgentNotFound(uuid::Uuid),

    #[error("Contract not found: {0}")]
    ContractNotFound(uuid::Uuid),

    #[error("Capability mismatch: agent lacks required capability '{0}'")]
    CapabilityMismatch(String),

    #[error("Reputation below threshold: {score} < {threshold}")]
    ReputationBelowThreshold { score: f64, threshold: f64 },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Ledger error: {0}")]
    LedgerError(String),

    #[error("Decomposition error: {0}")]
    DecompositionError(String),

    #[error("Assignment error: {0}")]
    AssignmentError(String),

    #[error("Monitoring error: {0}")]
    MonitoringError(String),

    #[error("Security threat detected: {0}")]
    SecurityThreat(String),

    #[error("Circuit breaker open for agent {0}")]
    CircuitBreakerOpen(uuid::Uuid),

    #[error("Dispute error: {0}")]
    DisputeError(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, PanopticonError>;
