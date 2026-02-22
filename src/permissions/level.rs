use serde::{Deserialize, Serialize};

use crate::types::TaskCharacteristics;

/// Approval level required for a task based on its characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovalLevel {
    /// Auto-approved based on standing permissions.
    StandingPermission,
    /// Requires contextual approval (1 approver).
    ContextualPermission,
    /// Requires just-in-time approval (2+ approvers + human).
    JustInTimePermission,
}

/// The approval requirement computed for a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequirement {
    pub level: ApprovalLevel,
    pub required_approvers: u32,
    pub human_required: bool,
}

impl ApprovalRequirement {
    /// Compute approval requirement from task characteristics using the
    /// criticality x reversibility matrix.
    ///
    /// - LOW criticality (< 0.4) + HIGH reversibility (>= 0.6) -> Standing (auto-approve)
    /// - HIGH criticality (>= 0.7) OR LOW reversibility (< 0.4) -> JustInTime (2+ approvers + human)
    /// - Otherwise -> Contextual (1 approver)
    pub fn from_characteristics(chars: &TaskCharacteristics) -> Self {
        let criticality = chars.criticality;
        let reversibility = chars.reversibility;

        if criticality >= 0.7 || reversibility < 0.4 {
            Self {
                level: ApprovalLevel::JustInTimePermission,
                required_approvers: 2,
                human_required: true,
            }
        } else if criticality < 0.4 && reversibility >= 0.6 {
            Self {
                level: ApprovalLevel::StandingPermission,
                required_approvers: 0,
                human_required: false,
            }
        } else {
            Self {
                level: ApprovalLevel::ContextualPermission,
                required_approvers: 1,
                human_required: false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_criticality_high_reversibility_is_standing() {
        let chars = TaskCharacteristics {
            criticality: 0.2,
            reversibility: 0.8,
            ..Default::default()
        };
        let req = ApprovalRequirement::from_characteristics(&chars);
        assert_eq!(req.level, ApprovalLevel::StandingPermission);
        assert_eq!(req.required_approvers, 0);
        assert!(!req.human_required);
    }

    #[test]
    fn test_high_criticality_is_jit() {
        let chars = TaskCharacteristics {
            criticality: 0.9,
            reversibility: 0.8,
            ..Default::default()
        };
        let req = ApprovalRequirement::from_characteristics(&chars);
        assert_eq!(req.level, ApprovalLevel::JustInTimePermission);
        assert!(req.required_approvers >= 2);
        assert!(req.human_required);
    }

    #[test]
    fn test_low_reversibility_is_jit() {
        let chars = TaskCharacteristics {
            criticality: 0.3,
            reversibility: 0.2,
            ..Default::default()
        };
        let req = ApprovalRequirement::from_characteristics(&chars);
        assert_eq!(req.level, ApprovalLevel::JustInTimePermission);
        assert!(req.required_approvers >= 2);
        assert!(req.human_required);
    }

    #[test]
    fn test_medium_is_contextual() {
        let chars = TaskCharacteristics {
            criticality: 0.5,
            reversibility: 0.5,
            ..Default::default()
        };
        let req = ApprovalRequirement::from_characteristics(&chars);
        assert_eq!(req.level, ApprovalLevel::ContextualPermission);
        assert_eq!(req.required_approvers, 1);
        assert!(!req.human_required);
    }

    #[test]
    fn test_boundary_high_criticality() {
        let chars = TaskCharacteristics {
            criticality: 0.7,
            reversibility: 0.9,
            ..Default::default()
        };
        let req = ApprovalRequirement::from_characteristics(&chars);
        assert_eq!(req.level, ApprovalLevel::JustInTimePermission);
    }

    #[test]
    fn test_boundary_low_reversibility() {
        let chars = TaskCharacteristics {
            criticality: 0.1,
            reversibility: 0.39,
            ..Default::default()
        };
        let req = ApprovalRequirement::from_characteristics(&chars);
        assert_eq!(req.level, ApprovalLevel::JustInTimePermission);
    }
}
