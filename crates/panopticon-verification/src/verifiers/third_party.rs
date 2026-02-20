use async_trait::async_trait;

use panopticon_types::error::PanopticonError;
use panopticon_types::task::Task;

use crate::traits::{TaskResult, VerificationOutcome, Verifier};

/// Verifier where multiple auditors vote on the result.
/// Passes if the number of approvals meets the configurable quorum.
pub struct ThirdPartyAuditVerifier {
    /// Votes from auditors: true = approve, false = reject.
    pub votes: Vec<bool>,
    /// Minimum fraction of approvals required (0.0 to 1.0).
    pub quorum: f64,
}

impl ThirdPartyAuditVerifier {
    pub fn new(votes: Vec<bool>, quorum: f64) -> Self {
        Self { votes, quorum }
    }
}

#[async_trait]
impl Verifier for ThirdPartyAuditVerifier {
    async fn verify(
        &self,
        _task: &Task,
        _result: &TaskResult,
    ) -> Result<VerificationOutcome, PanopticonError> {
        if self.votes.is_empty() {
            return Ok(VerificationOutcome::Inconclusive);
        }

        let approvals = self.votes.iter().filter(|&&v| v).count();
        let approval_rate = approvals as f64 / self.votes.len() as f64;

        if approval_rate >= self.quorum {
            Ok(VerificationOutcome::Passed {
                confidence: approval_rate,
            })
        } else {
            Ok(VerificationOutcome::Failed {
                reason: format!(
                    "Quorum not met: {:.0}% approvals, {:.0}% required",
                    approval_rate * 100.0,
                    self.quorum * 100.0
                ),
            })
        }
    }

    fn name(&self) -> &str {
        "ThirdPartyAuditVerifier"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_task() -> Task {
        Task::new("test-task", "A test task")
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

    #[tokio::test]
    async fn test_majority_passes() {
        let verifier = ThirdPartyAuditVerifier::new(vec![true, true, false], 0.5);
        let outcome = verifier.verify(&make_task(), &make_result()).await.unwrap();
        assert!(matches!(outcome, VerificationOutcome::Passed { .. }));
    }

    #[tokio::test]
    async fn test_quorum_not_met() {
        let verifier = ThirdPartyAuditVerifier::new(vec![true, false, false, false], 0.75);
        let outcome = verifier.verify(&make_task(), &make_result()).await.unwrap();
        assert!(matches!(outcome, VerificationOutcome::Failed { .. }));
    }

    #[tokio::test]
    async fn test_no_votes_inconclusive() {
        let verifier = ThirdPartyAuditVerifier::new(vec![], 0.5);
        let outcome = verifier.verify(&make_task(), &make_result()).await.unwrap();
        assert_eq!(outcome, VerificationOutcome::Inconclusive);
    }

    #[tokio::test]
    async fn test_unanimous_passes() {
        let verifier = ThirdPartyAuditVerifier::new(vec![true, true, true], 1.0);
        let outcome = verifier.verify(&make_task(), &make_result()).await.unwrap();
        assert!(matches!(outcome, VerificationOutcome::Passed { .. }));
    }
}
