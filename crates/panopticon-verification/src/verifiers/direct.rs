use async_trait::async_trait;

use panopticon_types::error::PanopticonError;
use panopticon_types::task::Task;

use crate::traits::{TaskResult, VerificationOutcome, Verifier};

/// Verifier where the delegator evaluates the result directly,
/// checking output against expected schema/constraints.
pub struct DirectInspectionVerifier {
    /// Expected keys that must be present in the output JSON object.
    pub expected_keys: Vec<String>,
}

impl DirectInspectionVerifier {
    pub fn new(expected_keys: Vec<String>) -> Self {
        Self { expected_keys }
    }
}

#[async_trait]
impl Verifier for DirectInspectionVerifier {
    async fn verify(
        &self,
        _task: &Task,
        result: &TaskResult,
    ) -> Result<VerificationOutcome, PanopticonError> {
        let obj = match result.output.as_object() {
            Some(obj) => obj,
            None => {
                return Ok(VerificationOutcome::Failed {
                    reason: "Output is not a JSON object".to_string(),
                });
            }
        };

        let mut missing = Vec::new();
        for key in &self.expected_keys {
            if !obj.contains_key(key) {
                missing.push(key.clone());
            }
        }

        if missing.is_empty() {
            let confidence = 1.0;
            Ok(VerificationOutcome::Passed { confidence })
        } else {
            Ok(VerificationOutcome::Failed {
                reason: format!("Missing expected keys: {}", missing.join(", ")),
            })
        }
    }

    fn name(&self) -> &str {
        "DirectInspectionVerifier"
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

    fn make_result(output: serde_json::Value) -> TaskResult {
        TaskResult {
            task_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            output,
            completed_at: Utc::now(),
            resource_consumed: 1.0,
        }
    }

    #[tokio::test]
    async fn test_direct_inspection_passes() {
        let verifier =
            DirectInspectionVerifier::new(vec!["answer".to_string(), "score".to_string()]);
        let result = make_result(serde_json::json!({"answer": 42, "score": 0.95}));
        let outcome = verifier.verify(&make_task(), &result).await.unwrap();
        assert!(matches!(outcome, VerificationOutcome::Passed { .. }));
    }

    #[tokio::test]
    async fn test_direct_inspection_fails_missing_key() {
        let verifier =
            DirectInspectionVerifier::new(vec!["answer".to_string(), "score".to_string()]);
        let result = make_result(serde_json::json!({"answer": 42}));
        let outcome = verifier.verify(&make_task(), &result).await.unwrap();
        assert!(matches!(outcome, VerificationOutcome::Failed { .. }));
    }

    #[tokio::test]
    async fn test_direct_inspection_fails_not_object() {
        let verifier = DirectInspectionVerifier::new(vec!["answer".to_string()]);
        let result = make_result(serde_json::json!("just a string"));
        let outcome = verifier.verify(&make_task(), &result).await.unwrap();
        assert!(matches!(outcome, VerificationOutcome::Failed { .. }));
    }
}
