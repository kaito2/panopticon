use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::error::PanopticonError;
use crate::types::task::Task;

use crate::verification::traits::{TaskResult, VerificationOutcome, Verifier};

/// An assessment submitted by an agent in the verification game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    pub agent_id: Uuid,
    /// true = agent assesses the result as correct
    pub approved: bool,
}

/// Schelling point based verification game.
/// Agents independently submit assessments. Those matching the consensus are rewarded.
pub struct GameTheoreticVerifier {
    pub assessments: Vec<Assessment>,
    /// Minimum fraction of assessments needed for a consensus (0.0 to 1.0).
    pub consensus_threshold: f64,
}

impl GameTheoreticVerifier {
    pub fn new(assessments: Vec<Assessment>, consensus_threshold: f64) -> Self {
        Self {
            assessments,
            consensus_threshold,
        }
    }

    /// Returns the IDs of agents whose assessment matched the consensus.
    pub fn rewarded_agents(&self) -> Vec<Uuid> {
        if self.assessments.is_empty() {
            return Vec::new();
        }
        let approvals = self.assessments.iter().filter(|a| a.approved).count();
        let total = self.assessments.len();
        let approval_rate = approvals as f64 / total as f64;
        let consensus_is_approve = approval_rate >= 0.5;

        self.assessments
            .iter()
            .filter(|a| a.approved == consensus_is_approve)
            .map(|a| a.agent_id)
            .collect()
    }
}

#[async_trait]
impl Verifier for GameTheoreticVerifier {
    async fn verify(
        &self,
        _task: &Task,
        _result: &TaskResult,
    ) -> Result<VerificationOutcome, PanopticonError> {
        if self.assessments.is_empty() {
            return Ok(VerificationOutcome::Inconclusive);
        }

        let approvals = self.assessments.iter().filter(|a| a.approved).count();
        let total = self.assessments.len();
        let approval_rate = approvals as f64 / total as f64;

        if approval_rate >= self.consensus_threshold {
            Ok(VerificationOutcome::Passed {
                confidence: approval_rate,
            })
        } else if (1.0 - approval_rate) >= self.consensus_threshold {
            Ok(VerificationOutcome::Failed {
                reason: format!(
                    "Consensus rejects result: {:.0}% disapproval",
                    (1.0 - approval_rate) * 100.0
                ),
            })
        } else {
            Ok(VerificationOutcome::Inconclusive)
        }
    }

    fn name(&self) -> &str {
        "GameTheoreticVerifier"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_task() -> Task {
        Task::new("test", "test task")
    }

    fn make_result() -> TaskResult {
        TaskResult {
            task_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            output: serde_json::json!({}),
            completed_at: Utc::now(),
            resource_consumed: 1.0,
        }
    }

    fn assessment(approved: bool) -> Assessment {
        Assessment {
            agent_id: Uuid::new_v4(),
            approved,
        }
    }

    #[tokio::test]
    async fn test_consensus_passes() {
        let verifier = GameTheoreticVerifier::new(
            vec![assessment(true), assessment(true), assessment(false)],
            0.6,
        );
        let outcome = verifier.verify(&make_task(), &make_result()).await.unwrap();
        assert!(matches!(
            outcome,
            VerificationOutcome::Passed { confidence } if (confidence - 2.0/3.0).abs() < 0.01
        ));
    }

    #[tokio::test]
    async fn test_consensus_rejects() {
        let verifier = GameTheoreticVerifier::new(
            vec![
                assessment(false),
                assessment(false),
                assessment(false),
                assessment(true),
            ],
            0.6,
        );
        let outcome = verifier.verify(&make_task(), &make_result()).await.unwrap();
        assert!(matches!(outcome, VerificationOutcome::Failed { .. }));
    }

    #[tokio::test]
    async fn test_inconclusive_no_assessments() {
        let verifier = GameTheoreticVerifier::new(vec![], 0.6);
        let outcome = verifier.verify(&make_task(), &make_result()).await.unwrap();
        assert_eq!(outcome, VerificationOutcome::Inconclusive);
    }

    #[tokio::test]
    async fn test_inconclusive_split_vote() {
        let verifier = GameTheoreticVerifier::new(vec![assessment(true), assessment(false)], 0.75);
        let outcome = verifier.verify(&make_task(), &make_result()).await.unwrap();
        assert_eq!(outcome, VerificationOutcome::Inconclusive);
    }

    #[tokio::test]
    async fn test_rewarded_agents() {
        let a1 = Uuid::new_v4();
        let a2 = Uuid::new_v4();
        let a3 = Uuid::new_v4();
        let verifier = GameTheoreticVerifier::new(
            vec![
                Assessment {
                    agent_id: a1,
                    approved: true,
                },
                Assessment {
                    agent_id: a2,
                    approved: true,
                },
                Assessment {
                    agent_id: a3,
                    approved: false,
                },
            ],
            0.6,
        );
        let rewarded = verifier.rewarded_agents();
        assert_eq!(rewarded.len(), 2);
        assert!(rewarded.contains(&a1));
        assert!(rewarded.contains(&a2));
    }
}
