use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::error::PanopticonError;

/// State machine for dispute resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisputeState {
    Filed,
    BondDeposited,
    AlgorithmicResolution,
    PanelReview,
    Adjudicated,
    Settled,
}

/// Events that drive dispute state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisputeEvent {
    DepositBond,
    RunAlgorithm,
    RequestPanel,
    Adjudicate,
    Settle,
}

impl DisputeState {
    /// Attempt a state transition given a dispute event.
    pub fn transition(self, event: DisputeEvent) -> Result<DisputeState, PanopticonError> {
        match (self, event) {
            (DisputeState::Filed, DisputeEvent::DepositBond) => Ok(DisputeState::BondDeposited),
            (DisputeState::BondDeposited, DisputeEvent::RunAlgorithm) => {
                Ok(DisputeState::AlgorithmicResolution)
            }
            (DisputeState::AlgorithmicResolution, DisputeEvent::RequestPanel) => {
                Ok(DisputeState::PanelReview)
            }
            (DisputeState::AlgorithmicResolution, DisputeEvent::Settle) => {
                Ok(DisputeState::Settled)
            }
            (DisputeState::PanelReview, DisputeEvent::Adjudicate) => Ok(DisputeState::Adjudicated),
            (DisputeState::Adjudicated, DisputeEvent::Settle) => Ok(DisputeState::Settled),
            _ => Err(PanopticonError::DisputeError(format!(
                "Invalid dispute transition from {self:?} via {event:?}"
            ))),
        }
    }
}

/// How a dispute is resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DisputeResolution {
    InFavorOfComplainant,
    InFavorOfRespondent,
    /// Split: fraction going to complainant (0.0 to 1.0).
    Split(f64),
}

/// A dispute filed against a task result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    pub id: Uuid,
    pub task_id: Uuid,
    pub complainant_id: Uuid,
    pub respondent_id: Uuid,
    pub state: DisputeState,
    pub bond_amount: f64,
    pub resolution: Option<DisputeResolution>,
    pub created_at: DateTime<Utc>,
}

impl Dispute {
    pub fn new(task_id: Uuid, complainant_id: Uuid, respondent_id: Uuid, bond_amount: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id,
            complainant_id,
            respondent_id,
            state: DisputeState::Filed,
            bond_amount,
            resolution: None,
            created_at: Utc::now(),
        }
    }

    /// Apply a dispute event, transitioning the state machine.
    pub fn apply_event(&mut self, event: DisputeEvent) -> Result<(), PanopticonError> {
        self.state = self.state.transition(event)?;
        Ok(())
    }

    /// Resolve the dispute with a given resolution. Must be in Adjudicated state.
    pub fn resolve(&mut self, resolution: DisputeResolution) -> Result<(), PanopticonError> {
        if self.state != DisputeState::Adjudicated {
            return Err(PanopticonError::DisputeError(
                "Cannot resolve dispute: not in Adjudicated state".into(),
            ));
        }
        self.resolution = Some(resolution);
        self.state = self.state.transition(DisputeEvent::Settle)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispute_full_lifecycle() {
        let mut state = DisputeState::Filed;
        state = state.transition(DisputeEvent::DepositBond).unwrap();
        assert_eq!(state, DisputeState::BondDeposited);

        state = state.transition(DisputeEvent::RunAlgorithm).unwrap();
        assert_eq!(state, DisputeState::AlgorithmicResolution);

        state = state.transition(DisputeEvent::RequestPanel).unwrap();
        assert_eq!(state, DisputeState::PanelReview);

        state = state.transition(DisputeEvent::Adjudicate).unwrap();
        assert_eq!(state, DisputeState::Adjudicated);

        state = state.transition(DisputeEvent::Settle).unwrap();
        assert_eq!(state, DisputeState::Settled);
    }

    #[test]
    fn test_dispute_algorithmic_settle_shortcut() {
        let mut state = DisputeState::Filed;
        state = state.transition(DisputeEvent::DepositBond).unwrap();
        state = state.transition(DisputeEvent::RunAlgorithm).unwrap();
        // Can settle directly from algorithmic resolution without panel
        state = state.transition(DisputeEvent::Settle).unwrap();
        assert_eq!(state, DisputeState::Settled);
    }

    #[test]
    fn test_invalid_dispute_transition() {
        let state = DisputeState::Filed;
        let result = state.transition(DisputeEvent::Adjudicate);
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_struct_lifecycle() {
        let mut dispute = Dispute::new(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), 100.0);
        assert_eq!(dispute.state, DisputeState::Filed);
        assert!(dispute.resolution.is_none());

        dispute.apply_event(DisputeEvent::DepositBond).unwrap();
        dispute.apply_event(DisputeEvent::RunAlgorithm).unwrap();
        dispute.apply_event(DisputeEvent::RequestPanel).unwrap();
        dispute.apply_event(DisputeEvent::Adjudicate).unwrap();

        dispute
            .resolve(DisputeResolution::InFavorOfComplainant)
            .unwrap();
        assert_eq!(dispute.state, DisputeState::Settled);
        assert_eq!(
            dispute.resolution,
            Some(DisputeResolution::InFavorOfComplainant)
        );
    }

    #[test]
    fn test_resolve_requires_adjudicated() {
        let mut dispute = Dispute::new(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), 50.0);
        let result = dispute.resolve(DisputeResolution::Split(0.5));
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_resolution_variants() {
        let r1 = DisputeResolution::InFavorOfComplainant;
        let r2 = DisputeResolution::InFavorOfRespondent;
        let r3 = DisputeResolution::Split(0.6);
        assert_ne!(r1, r2);
        assert_ne!(r2, r3);
    }
}
