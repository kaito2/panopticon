use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A checkpoint reported by an agent during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub task_id: Uuid,
    pub agent_id: Uuid,
    pub timestamp: DateTime<Utc>,
    /// Progress percentage (0.0 - 1.0).
    pub progress_pct: f64,
    /// Resource units consumed so far.
    pub resource_consumed: f64,
    /// Human-readable status message.
    pub status_message: String,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
}

impl Checkpoint {
    pub fn new(task_id: Uuid, agent_id: Uuid) -> Self {
        Self {
            task_id,
            agent_id,
            timestamp: Utc::now(),
            progress_pct: 0.0,
            resource_consumed: 0.0,
            status_message: String::new(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_progress(mut self, pct: f64) -> Self {
        self.progress_pct = pct.clamp(0.0, 1.0);
        self
    }

    pub fn with_resource_consumed(mut self, amount: f64) -> Self {
        self.resource_consumed = amount;
        self
    }

    pub fn with_status(mut self, msg: impl Into<String>) -> Self {
        self.status_message = msg.into();
        self
    }

    pub fn with_metadata(mut self, meta: serde_json::Value) -> Self {
        self.metadata = meta;
        self
    }
}
