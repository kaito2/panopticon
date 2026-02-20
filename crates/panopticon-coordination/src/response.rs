use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An action to take in response to a coordination trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseAction {
    /// Adjust parameters for a running task (e.g. timeout, resource limits).
    AdjustParameters {
        task_id: Uuid,
        adjustments: serde_json::Value,
    },
    /// Redelegate the task to a different agent.
    Redelegate { task_id: Uuid, from_agent_id: Uuid },
    /// Redecompose the task into new subtasks.
    Redecompose { task_id: Uuid },
    /// Escalate the issue to a human operator or higher authority.
    Escalate {
        task_id: Option<Uuid>,
        reason: String,
    },
    /// Terminate the task execution.
    Terminate { task_id: Uuid, reason: String },
}

/// An ordered plan of response actions with a justification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePlan {
    pub actions: Vec<ResponseAction>,
    pub justification: String,
}

impl ResponsePlan {
    pub fn new(justification: impl Into<String>) -> Self {
        Self {
            actions: Vec::new(),
            justification: justification.into(),
        }
    }

    pub fn with_action(mut self, action: ResponseAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn add_action(&mut self, action: ResponseAction) {
        self.actions.push(action);
    }
}
