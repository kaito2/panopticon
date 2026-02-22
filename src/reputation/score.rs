use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::ReputationScore;

/// The dimensions along which an agent is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReputationDimension {
    Completion,
    Quality,
    Reliability,
    Safety,
    Behavioral,
}

/// A single dimension's score together with the number of observations that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub dimension: ReputationDimension,
    pub score: f64,
    pub observations: u64,
    pub last_updated: DateTime<Utc>,
}

impl DimensionScore {
    pub fn new(dimension: ReputationDimension) -> Self {
        Self {
            dimension,
            score: 0.5,
            observations: 0,
            last_updated: Utc::now(),
        }
    }
}

/// An observation recorded after a task completes (or a checkpoint is evaluated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationObservation {
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub dimension: ReputationDimension,
    /// The raw observed value in [0, 1].
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

/// Per-agent reputation state held by the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReputation {
    pub agent_id: Uuid,
    pub completion: DimensionScore,
    pub quality: DimensionScore,
    pub reliability: DimensionScore,
    pub safety: DimensionScore,
    pub behavioral: DimensionScore,
    pub total_tasks: u64,
}

impl AgentReputation {
    pub fn new(agent_id: Uuid) -> Self {
        Self {
            agent_id,
            completion: DimensionScore::new(ReputationDimension::Completion),
            quality: DimensionScore::new(ReputationDimension::Quality),
            reliability: DimensionScore::new(ReputationDimension::Reliability),
            safety: DimensionScore::new(ReputationDimension::Safety),
            behavioral: DimensionScore::new(ReputationDimension::Behavioral),
            total_tasks: 0,
        }
    }

    /// Return a mutable reference to the dimension score matching the given dimension.
    pub fn dimension_mut(&mut self, dim: ReputationDimension) -> &mut DimensionScore {
        match dim {
            ReputationDimension::Completion => &mut self.completion,
            ReputationDimension::Quality => &mut self.quality,
            ReputationDimension::Reliability => &mut self.reliability,
            ReputationDimension::Safety => &mut self.safety,
            ReputationDimension::Behavioral => &mut self.behavioral,
        }
    }

    /// Return an immutable reference to the dimension score matching the given dimension.
    pub fn dimension(&self, dim: ReputationDimension) -> &DimensionScore {
        match dim {
            ReputationDimension::Completion => &self.completion,
            ReputationDimension::Quality => &self.quality,
            ReputationDimension::Reliability => &self.reliability,
            ReputationDimension::Safety => &self.safety,
            ReputationDimension::Behavioral => &self.behavioral,
        }
    }

    /// Convert internal scores into the shared `ReputationScore` type.
    pub fn to_reputation_score(&self) -> ReputationScore {
        ReputationScore {
            completion: self.completion.score,
            quality: self.quality.score,
            reliability: self.reliability.score,
            safety: self.safety.score,
            behavioral: self.behavioral.score,
        }
    }
}
