use crate::types::{Agent, PanopticonError, Result, Task, TrustLevel};

use super::level::{ApprovalLevel, ApprovalRequirement};

/// Evaluates whether an agent can perform a task given its permissions,
/// task characteristics, and trust level.
pub struct PermissionEvaluator;

impl PermissionEvaluator {
    /// Check whether an agent has permission to execute a task and return
    /// the approval requirement (potentially reduced by trust level).
    pub fn check_permission(agent: &Agent, task: &Task) -> Result<ApprovalRequirement> {
        // Check that the agent has the required capabilities.
        for cap in &task.required_capabilities {
            if !agent.has_capability(cap) {
                return Err(PanopticonError::CapabilityMismatch(cap.clone()));
            }
        }

        // Check that the agent's permission set allows the required actions.
        // Task required_capabilities map to actions the agent must be permitted to perform.
        for cap in &task.required_capabilities {
            if !agent.permissions.allowed_actions.contains(cap) {
                return Err(PanopticonError::PermissionDenied(format!(
                    "Agent lacks permission for action: {}",
                    cap
                )));
            }
        }

        // Compute base approval requirement from task characteristics.
        let base = ApprovalRequirement::from_characteristics(&task.characteristics);

        // Higher trust level can reduce approval requirements,
        // but never below JIT for critical tasks.
        let adjusted = Self::adjust_for_trust(base, agent.trust_level, &task.characteristics);

        Ok(adjusted)
    }

    /// Adjust approval requirements based on trust level.
    /// Higher trust reduces requirements, but JIT is never reduced for critical tasks
    /// (criticality >= 0.7).
    fn adjust_for_trust(
        base: ApprovalRequirement,
        trust_level: TrustLevel,
        characteristics: &crate::types::TaskCharacteristics,
    ) -> ApprovalRequirement {
        let is_critical = characteristics.criticality >= 0.7;

        // If the task is critical, JIT cannot be reduced regardless of trust.
        if is_critical && base.level == ApprovalLevel::JustInTimePermission {
            return base;
        }

        match trust_level {
            TrustLevel::Full => {
                // Full trust: reduce JIT to Contextual, Contextual to Standing.
                match base.level {
                    ApprovalLevel::JustInTimePermission => ApprovalRequirement {
                        level: ApprovalLevel::ContextualPermission,
                        required_approvers: 1,
                        human_required: false,
                    },
                    ApprovalLevel::ContextualPermission => ApprovalRequirement {
                        level: ApprovalLevel::StandingPermission,
                        required_approvers: 0,
                        human_required: false,
                    },
                    ApprovalLevel::StandingPermission => base,
                }
            }
            TrustLevel::High => {
                // High trust: reduce JIT to Contextual, keep others.
                match base.level {
                    ApprovalLevel::JustInTimePermission => ApprovalRequirement {
                        level: ApprovalLevel::ContextualPermission,
                        required_approvers: 1,
                        human_required: false,
                    },
                    _ => base,
                }
            }
            TrustLevel::Medium | TrustLevel::Low | TrustLevel::Untrusted => {
                // No reduction for medium trust and below.
                base
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Agent, Capability, PermissionSet, TaskCharacteristics};

    fn make_agent(trust: TrustLevel, actions: Vec<&str>, capabilities: Vec<&str>) -> Agent {
        let mut agent = Agent::new("test-agent");
        agent.trust_level = trust;
        agent.permissions = PermissionSet {
            allowed_actions: actions.into_iter().map(String::from).collect(),
            max_delegation_depth: 3,
            max_cost_budget: 1000.0,
            allowed_data_classifications: vec!["public".into()],
        };
        agent.capabilities.capabilities = capabilities
            .into_iter()
            .map(|name| Capability {
                name: name.into(),
                proficiency: 0.8,
                certified: true,
                last_verified: None,
            })
            .collect();
        agent
    }

    fn make_task(criticality: f64, reversibility: f64, capabilities: Vec<&str>) -> Task {
        let mut task = Task::new("test-task", "a test task");
        task.characteristics = TaskCharacteristics {
            criticality,
            reversibility,
            ..Default::default()
        };
        task.required_capabilities = capabilities.into_iter().map(String::from).collect();
        task
    }

    #[test]
    fn test_permission_check_succeeds() {
        let agent = make_agent(TrustLevel::Medium, vec!["nlp"], vec!["nlp"]);
        let task = make_task(0.2, 0.8, vec!["nlp"]);
        let result = PermissionEvaluator::check_permission(&agent, &task);
        assert!(result.is_ok());
    }

    #[test]
    fn test_permission_denied_missing_capability() {
        let agent = make_agent(TrustLevel::Medium, vec!["nlp"], vec!["nlp"]);
        let task = make_task(0.2, 0.8, vec!["vision"]);
        let result = PermissionEvaluator::check_permission(&agent, &task);
        assert!(result.is_err());
    }

    #[test]
    fn test_permission_denied_missing_action() {
        let agent = make_agent(TrustLevel::Medium, vec![], vec!["nlp"]);
        let task = make_task(0.2, 0.8, vec!["nlp"]);
        let result = PermissionEvaluator::check_permission(&agent, &task);
        assert!(result.is_err());
    }

    #[test]
    fn test_high_trust_reduces_jit_for_non_critical() {
        let agent = make_agent(TrustLevel::High, vec!["nlp"], vec!["nlp"]);
        // Non-critical task that would normally be JIT due to low reversibility
        let task = make_task(0.5, 0.2, vec!["nlp"]);
        let result = PermissionEvaluator::check_permission(&agent, &task).unwrap();
        assert_eq!(result.level, ApprovalLevel::ContextualPermission);
    }

    #[test]
    fn test_high_trust_does_not_reduce_jit_for_critical() {
        let agent = make_agent(TrustLevel::High, vec!["nlp"], vec!["nlp"]);
        // Critical task: JIT should not be reduced.
        let task = make_task(0.9, 0.8, vec!["nlp"]);
        let result = PermissionEvaluator::check_permission(&agent, &task).unwrap();
        assert_eq!(result.level, ApprovalLevel::JustInTimePermission);
    }

    #[test]
    fn test_full_trust_reduces_contextual_to_standing() {
        let agent = make_agent(TrustLevel::Full, vec!["nlp"], vec!["nlp"]);
        // Medium criticality, medium reversibility -> Contextual base
        let task = make_task(0.5, 0.5, vec!["nlp"]);
        let result = PermissionEvaluator::check_permission(&agent, &task).unwrap();
        assert_eq!(result.level, ApprovalLevel::StandingPermission);
    }

    #[test]
    fn test_untrusted_no_reduction() {
        let agent = make_agent(TrustLevel::Untrusted, vec!["nlp"], vec!["nlp"]);
        let task = make_task(0.5, 0.5, vec!["nlp"]);
        let result = PermissionEvaluator::check_permission(&agent, &task).unwrap();
        assert_eq!(result.level, ApprovalLevel::ContextualPermission);
    }

    #[test]
    fn test_no_required_capabilities_succeeds() {
        let agent = make_agent(TrustLevel::Low, vec![], vec![]);
        let task = make_task(0.2, 0.8, vec![]);
        let result = PermissionEvaluator::check_permission(&agent, &task);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().level, ApprovalLevel::StandingPermission);
    }
}
