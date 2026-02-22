use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::PanopticonError;

/// 11-dimensional task characteristics from the paper.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskCharacteristics {
    /// Computational/cognitive complexity (0.0 - 1.0)
    pub complexity: f64,
    /// Business criticality (0.0 - 1.0)
    pub criticality: f64,
    /// Degree of uncertainty in the task (0.0 - 1.0)
    pub uncertainty: f64,
    /// How easily the result can be verified (0.0 - 1.0)
    pub verifiability: f64,
    /// Whether the task outcome can be reversed (0.0 - 1.0)
    pub reversibility: f64,
    /// Time sensitivity (0.0 - 1.0)
    pub time_sensitivity: f64,
    /// Required resource intensity (0.0 - 1.0)
    pub resource_intensity: f64,
    /// Privacy sensitivity of data involved (0.0 - 1.0)
    pub privacy_sensitivity: f64,
    /// Degree of human interaction required (0.0 - 1.0)
    pub human_interaction: f64,
    /// How novel/unprecedented the task is (0.0 - 1.0)
    pub novelty: f64,
    /// Degree of interdependency with other tasks (0.0 - 1.0)
    pub interdependency: f64,
}

impl Default for TaskCharacteristics {
    fn default() -> Self {
        Self {
            complexity: 0.5,
            criticality: 0.5,
            uncertainty: 0.5,
            verifiability: 0.5,
            reversibility: 0.5,
            time_sensitivity: 0.5,
            resource_intensity: 0.5,
            privacy_sensitivity: 0.5,
            human_interaction: 0.5,
            novelty: 0.5,
            interdependency: 0.5,
        }
    }
}

/// Task state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    Decomposing,
    AwaitingAssignment,
    Negotiating,
    Contracted,
    InProgress,
    AwaitingVerification,
    Completed,
    Failed,
    Disputed,
}

/// Events that drive task state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskEvent {
    StartDecomposition,
    DecompositionComplete,
    SkipDecomposition,
    StartNegotiation,
    NegotiationComplete,
    ContractSigned,
    StartExecution,
    ExecutionComplete,
    VerificationPassed,
    VerificationFailed,
    DisputeRaised,
    DisputeResolved,
    TaskFailed,
    Retry,
}

impl TaskState {
    /// Attempt a state transition given an event.
    /// Returns the new state or an error if the transition is invalid.
    pub fn transition(self, event: TaskEvent) -> super::error::Result<TaskState> {
        match (self, event) {
            // From Pending
            (TaskState::Pending, TaskEvent::StartDecomposition) => Ok(TaskState::Decomposing),
            (TaskState::Pending, TaskEvent::SkipDecomposition) => Ok(TaskState::AwaitingAssignment),

            // From Decomposing
            (TaskState::Decomposing, TaskEvent::DecompositionComplete) => {
                Ok(TaskState::AwaitingAssignment)
            }

            // From AwaitingAssignment
            (TaskState::AwaitingAssignment, TaskEvent::StartNegotiation) => {
                Ok(TaskState::Negotiating)
            }

            // From Negotiating
            (TaskState::Negotiating, TaskEvent::NegotiationComplete) => Ok(TaskState::Contracted),

            // From Contracted
            (TaskState::Contracted, TaskEvent::StartExecution) => Ok(TaskState::InProgress),

            // From InProgress
            (TaskState::InProgress, TaskEvent::ExecutionComplete) => {
                Ok(TaskState::AwaitingVerification)
            }
            (TaskState::InProgress, TaskEvent::TaskFailed) => Ok(TaskState::Failed),

            // From AwaitingVerification
            (TaskState::AwaitingVerification, TaskEvent::VerificationPassed) => {
                Ok(TaskState::Completed)
            }
            (TaskState::AwaitingVerification, TaskEvent::VerificationFailed) => {
                Ok(TaskState::Failed)
            }
            (TaskState::AwaitingVerification, TaskEvent::DisputeRaised) => Ok(TaskState::Disputed),

            // From Disputed
            (TaskState::Disputed, TaskEvent::DisputeResolved) => Ok(TaskState::Completed),
            (TaskState::Disputed, TaskEvent::TaskFailed) => Ok(TaskState::Failed),

            // From Failed — allow retry
            (TaskState::Failed, TaskEvent::Retry) => Ok(TaskState::Pending),

            // All other transitions are invalid
            (state, event) => Err(PanopticonError::InvalidStateTransition { from: state, event }),
        }
    }
}

/// A delegation task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub state: TaskState,
    pub characteristics: TaskCharacteristics,
    pub required_capabilities: Vec<String>,
    pub assigned_agent_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub subtask_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

impl Task {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            parent_id: None,
            name: name.into(),
            description: description.into(),
            state: TaskState::Pending,
            characteristics: TaskCharacteristics::default(),
            required_capabilities: Vec::new(),
            assigned_agent_id: None,
            contract_id: None,
            subtask_ids: Vec::new(),
            created_at: now,
            updated_at: now,
            deadline: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_characteristics(mut self, chars: TaskCharacteristics) -> Self {
        self.characteristics = chars;
        self
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.required_capabilities = caps;
        self
    }

    /// Apply a state transition event.
    pub fn apply_event(&mut self, event: TaskEvent) -> super::error::Result<()> {
        self.state = self.state.transition(event)?;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_full_lifecycle() {
        let mut state = TaskState::Pending;
        let transitions = [
            TaskEvent::StartDecomposition,
            TaskEvent::DecompositionComplete,
            TaskEvent::StartNegotiation,
            TaskEvent::NegotiationComplete,
            TaskEvent::StartExecution,
            TaskEvent::ExecutionComplete,
            TaskEvent::VerificationPassed,
        ];
        for event in transitions {
            state = state.transition(event).unwrap();
        }
        assert_eq!(state, TaskState::Completed);
    }

    #[test]
    fn test_skip_decomposition() {
        let state = TaskState::Pending;
        let state = state.transition(TaskEvent::SkipDecomposition).unwrap();
        assert_eq!(state, TaskState::AwaitingAssignment);
    }

    #[test]
    fn test_invalid_transition() {
        let state = TaskState::Pending;
        let result = state.transition(TaskEvent::VerificationPassed);
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_flow() {
        let state = TaskState::AwaitingVerification;
        let state = state.transition(TaskEvent::DisputeRaised).unwrap();
        assert_eq!(state, TaskState::Disputed);
        let state = state.transition(TaskEvent::DisputeResolved).unwrap();
        assert_eq!(state, TaskState::Completed);
    }

    #[test]
    fn test_retry_from_failed() {
        let state = TaskState::InProgress;
        let state = state.transition(TaskEvent::TaskFailed).unwrap();
        assert_eq!(state, TaskState::Failed);
        let state = state.transition(TaskEvent::Retry).unwrap();
        assert_eq!(state, TaskState::Pending);
    }

    #[test]
    fn test_task_builder() {
        let task = Task::new("test", "a test task").with_capabilities(vec!["nlp".to_string()]);
        assert_eq!(task.state, TaskState::Pending);
        assert_eq!(task.required_capabilities, vec!["nlp".to_string()]);
    }
}
