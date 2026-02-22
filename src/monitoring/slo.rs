use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::checkpoint::Checkpoint;

/// Comparison operator for SLO threshold checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparison {
    LessThan,
    GreaterThan,
}

/// A service-level objective definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloDefinition {
    pub metric_name: String,
    pub threshold: f64,
    pub comparison: Comparison,
    /// Time window in seconds over which the metric is evaluated.
    pub window_secs: u64,
}

/// A detected SLO violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloViolation {
    pub definition: SloDefinition,
    pub actual_value: f64,
    pub detected_at: DateTime<Utc>,
}

/// Checks checkpoints against a set of SLO definitions.
pub struct SloChecker {
    definitions: Vec<SloDefinition>,
}

impl SloChecker {
    pub fn new(definitions: Vec<SloDefinition>) -> Self {
        Self { definitions }
    }

    /// Check a checkpoint against all SLO definitions and return any violations.
    pub fn check(&self, checkpoint: &Checkpoint) -> Vec<SloViolation> {
        let mut violations = Vec::new();
        for def in &self.definitions {
            let actual_value = match def.metric_name.as_str() {
                "progress_pct" => checkpoint.progress_pct,
                "resource_consumed" => checkpoint.resource_consumed,
                _ => continue,
            };

            let violated = match def.comparison {
                Comparison::LessThan => actual_value >= def.threshold,
                Comparison::GreaterThan => actual_value <= def.threshold,
            };

            if violated {
                violations.push(SloViolation {
                    definition: def.clone(),
                    actual_value,
                    detected_at: Utc::now(),
                });
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_slo_no_violation() {
        let defs = vec![SloDefinition {
            metric_name: "resource_consumed".into(),
            threshold: 100.0,
            comparison: Comparison::LessThan,
            window_secs: 300,
        }];
        let checker = SloChecker::new(defs);
        let cp = Checkpoint::new(Uuid::new_v4(), Uuid::new_v4()).with_resource_consumed(50.0);
        let violations = checker.check(&cp);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_slo_violation_resource_exceeded() {
        let defs = vec![SloDefinition {
            metric_name: "resource_consumed".into(),
            threshold: 100.0,
            comparison: Comparison::LessThan,
            window_secs: 300,
        }];
        let checker = SloChecker::new(defs);
        let cp = Checkpoint::new(Uuid::new_v4(), Uuid::new_v4()).with_resource_consumed(150.0);
        let violations = checker.check(&cp);
        assert_eq!(violations.len(), 1);
        assert!((violations[0].actual_value - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_slo_violation_progress_too_low() {
        let defs = vec![SloDefinition {
            metric_name: "progress_pct".into(),
            threshold: 0.5,
            comparison: Comparison::GreaterThan,
            window_secs: 600,
        }];
        let checker = SloChecker::new(defs);
        let cp = Checkpoint::new(Uuid::new_v4(), Uuid::new_v4()).with_progress(0.2);
        let violations = checker.check(&cp);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_slo_unknown_metric_ignored() {
        let defs = vec![SloDefinition {
            metric_name: "unknown_metric".into(),
            threshold: 1.0,
            comparison: Comparison::LessThan,
            window_secs: 60,
        }];
        let checker = SloChecker::new(defs);
        let cp = Checkpoint::new(Uuid::new_v4(), Uuid::new_v4());
        let violations = checker.check(&cp);
        assert!(violations.is_empty());
    }
}
