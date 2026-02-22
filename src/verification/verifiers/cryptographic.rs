use async_trait::async_trait;

use crate::types::error::PanopticonError;
use crate::types::task::Task;

use crate::verification::traits::{TaskResult, VerificationOutcome, Verifier};

/// Stub verifier for zero-knowledge proof based verification.
/// Currently always returns Passed as the full ZK implementation is pending.
pub struct CryptographicVerifier;

impl CryptographicVerifier {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CryptographicVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Verifier for CryptographicVerifier {
    async fn verify(
        &self,
        _task: &Task,
        _result: &TaskResult,
    ) -> Result<VerificationOutcome, PanopticonError> {
        // TODO: Implement full ZK proof verification.
        // For now, return Passed with moderate confidence as a stub.
        Ok(VerificationOutcome::Passed { confidence: 0.5 })
    }

    fn name(&self) -> &str {
        "CryptographicVerifier"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_cryptographic_verifier_stub() {
        let verifier = CryptographicVerifier::new();
        let task = Task::new("test", "test task");
        let result = TaskResult {
            task_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            output: serde_json::json!({}),
            completed_at: Utc::now(),
            resource_consumed: 1.0,
        };
        let outcome = verifier.verify(&task, &result).await.unwrap();
        assert!(matches!(
            outcome,
            VerificationOutcome::Passed { confidence } if (confidence - 0.5).abs() < f64::EPSILON
        ));
    }
}
