use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// External triggers originating from outside the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExternalTrigger {
    TaskSpecChanged { task_id: Uuid },
    ResourceFluctuation { resource_name: String, delta: f64 },
    PriorityChanged { task_id: Uuid, new_priority: f64 },
    SecurityThreat { agent_id: Uuid, description: String },
}

/// Internal triggers originating from monitoring or other subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InternalTrigger {
    PerformanceDegraded {
        task_id: Uuid,
        agent_id: Uuid,
        metric: String,
        value: f64,
    },
    BudgetExceeded {
        task_id: Uuid,
        consumed: f64,
        limit: f64,
    },
    VerificationFailed {
        task_id: Uuid,
        reason: String,
    },
    AgentUnresponsive {
        agent_id: Uuid,
    },
}

/// A coordination trigger that can be either external or internal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationTrigger {
    External(ExternalTrigger),
    Internal(InternalTrigger),
}
