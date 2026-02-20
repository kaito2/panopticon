use serde::{Deserialize, Serialize};

/// Direction of optimization for an objective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationDirection {
    Minimize,
    Maximize,
}

/// A single objective with its weight and optimization direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub name: String,
    pub weight: f64,
    pub direction: OptimizationDirection,
}

/// Predefined objective names.
pub const OBJ_COST: &str = "cost";
pub const OBJ_QUALITY: &str = "quality";
pub const OBJ_LATENCY: &str = "latency";
pub const OBJ_UNCERTAINTY: &str = "uncertainty";
pub const OBJ_PRIVACY_RISK: &str = "privacy_risk";

/// Multi-objective function for evaluating delegation candidates.
#[derive(Debug, Clone)]
pub struct ObjectiveFunction {
    pub objectives: Vec<Objective>,
}

impl ObjectiveFunction {
    /// Create a default objective function with the five standard objectives.
    pub fn default_objectives() -> Self {
        Self {
            objectives: vec![
                Objective {
                    name: OBJ_COST.into(),
                    weight: 0.25,
                    direction: OptimizationDirection::Minimize,
                },
                Objective {
                    name: OBJ_QUALITY.into(),
                    weight: 0.30,
                    direction: OptimizationDirection::Maximize,
                },
                Objective {
                    name: OBJ_LATENCY.into(),
                    weight: 0.20,
                    direction: OptimizationDirection::Minimize,
                },
                Objective {
                    name: OBJ_UNCERTAINTY.into(),
                    weight: 0.15,
                    direction: OptimizationDirection::Minimize,
                },
                Objective {
                    name: OBJ_PRIVACY_RISK.into(),
                    weight: 0.10,
                    direction: OptimizationDirection::Minimize,
                },
            ],
        }
    }

    pub fn new(objectives: Vec<Objective>) -> Self {
        Self { objectives }
    }

    /// Evaluate a candidate solution against the objectives.
    ///
    /// `values` maps objective name to raw value. Returns a weighted composite score
    /// where Maximize objectives contribute positively and Minimize objectives contribute
    /// negatively (so higher composite = better).
    pub fn evaluate(&self, values: &std::collections::HashMap<String, f64>) -> f64 {
        self.objectives
            .iter()
            .map(|obj| {
                let raw = values.get(&obj.name).copied().unwrap_or(0.0);
                let directed = match obj.direction {
                    OptimizationDirection::Maximize => raw,
                    OptimizationDirection::Minimize => 1.0 - raw,
                };
                obj.weight * directed
            })
            .sum()
    }
}

/// Estimates the overhead cost of delegation (negotiation + contract + verification).
#[derive(Debug, Clone)]
pub struct DelegationOverhead {
    /// Fixed cost for the negotiation phase.
    pub negotiation_cost: f64,
    /// Fixed cost for contract creation and signing.
    pub contract_cost: f64,
    /// Cost per verification checkpoint.
    pub verification_cost_per_checkpoint: f64,
    /// Expected number of checkpoints.
    pub expected_checkpoints: u32,
}

impl Default for DelegationOverhead {
    fn default() -> Self {
        Self {
            negotiation_cost: 5.0,
            contract_cost: 2.0,
            verification_cost_per_checkpoint: 1.0,
            expected_checkpoints: 3,
        }
    }
}

impl DelegationOverhead {
    /// Total estimated overhead cost.
    pub fn total(&self) -> f64 {
        self.negotiation_cost
            + self.contract_cost
            + self.verification_cost_per_checkpoint * self.expected_checkpoints as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_default_objectives() {
        let obj_fn = ObjectiveFunction::default_objectives();
        assert_eq!(obj_fn.objectives.len(), 5);
        let total_weight: f64 = obj_fn.objectives.iter().map(|o| o.weight).sum();
        assert!((total_weight - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_evaluate_all_perfect() {
        let obj_fn = ObjectiveFunction::default_objectives();
        let mut values = HashMap::new();
        // For Minimize objectives, 0.0 is best (1.0 - 0.0 = 1.0 contribution)
        // For Maximize objectives, 1.0 is best (1.0 contribution)
        values.insert(OBJ_COST.into(), 0.0);
        values.insert(OBJ_QUALITY.into(), 1.0);
        values.insert(OBJ_LATENCY.into(), 0.0);
        values.insert(OBJ_UNCERTAINTY.into(), 0.0);
        values.insert(OBJ_PRIVACY_RISK.into(), 0.0);

        let score = obj_fn.evaluate(&values);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_evaluate_all_worst() {
        let obj_fn = ObjectiveFunction::default_objectives();
        let mut values = HashMap::new();
        values.insert(OBJ_COST.into(), 1.0);
        values.insert(OBJ_QUALITY.into(), 0.0);
        values.insert(OBJ_LATENCY.into(), 1.0);
        values.insert(OBJ_UNCERTAINTY.into(), 1.0);
        values.insert(OBJ_PRIVACY_RISK.into(), 1.0);

        let score = obj_fn.evaluate(&values);
        assert!(score.abs() < f64::EPSILON);
    }

    #[test]
    fn test_delegation_overhead() {
        let overhead = DelegationOverhead::default();
        // 5.0 + 2.0 + 1.0*3 = 10.0
        assert!((overhead.total() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_custom_overhead() {
        let overhead = DelegationOverhead {
            negotiation_cost: 10.0,
            contract_cost: 5.0,
            verification_cost_per_checkpoint: 2.0,
            expected_checkpoints: 5,
        };
        // 10 + 5 + 2*5 = 25
        assert!((overhead.total() - 25.0).abs() < f64::EPSILON);
    }
}
