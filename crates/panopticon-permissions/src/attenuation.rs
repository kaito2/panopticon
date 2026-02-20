use panopticon_types::{PanopticonError, PermissionSet, Result};

/// Attenuate privileges when re-delegating from parent to child.
///
/// Guarantees:
/// - Child permissions are a strict subset of parent permissions.
/// - `max_delegation_depth` is reduced by 1.
/// - `max_cost_budget` is reduced proportionally (child cannot exceed parent).
/// - `allowed_actions` is the intersection of parent and child request.
/// - `allowed_data_classifications` is the intersection of parent and child request.
pub fn attenuate(parent: &PermissionSet, child_request: &PermissionSet) -> Result<PermissionSet> {
    // Cannot delegate if parent has no delegation depth left.
    if parent.max_delegation_depth == 0 {
        return Err(PanopticonError::PermissionDenied(
            "Parent has no delegation depth remaining".into(),
        ));
    }

    // Compute intersection of allowed actions.
    let allowed_actions: Vec<String> = child_request
        .allowed_actions
        .iter()
        .filter(|a| parent.allowed_actions.contains(a))
        .cloned()
        .collect();

    // Compute intersection of data classifications.
    let allowed_data_classifications: Vec<String> = child_request
        .allowed_data_classifications
        .iter()
        .filter(|d| parent.allowed_data_classifications.contains(d))
        .cloned()
        .collect();

    // Reduce delegation depth by 1.
    let max_delegation_depth = parent.max_delegation_depth - 1;

    // Cost budget: child cannot exceed parent; take the minimum.
    let max_cost_budget = child_request.max_cost_budget.min(parent.max_cost_budget);

    let attenuated = PermissionSet {
        allowed_actions,
        max_delegation_depth,
        max_cost_budget,
        allowed_data_classifications,
    };

    // Final validation: attenuated must be a subset of parent.
    if !attenuated.is_subset_of(parent) {
        return Err(PanopticonError::PermissionDenied(
            "Attenuated permissions are not a subset of parent".into(),
        ));
    }

    Ok(attenuated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent_permissions() -> PermissionSet {
        PermissionSet {
            allowed_actions: vec!["read".into(), "write".into(), "execute".into()],
            max_delegation_depth: 3,
            max_cost_budget: 1000.0,
            allowed_data_classifications: vec!["public".into(), "internal".into()],
        }
    }

    #[test]
    fn test_attenuate_basic() {
        let parent = parent_permissions();
        let child_request = PermissionSet {
            allowed_actions: vec!["read".into(), "write".into()],
            max_delegation_depth: 5, // Will be capped.
            max_cost_budget: 500.0,
            allowed_data_classifications: vec!["public".into()],
        };

        let result = attenuate(&parent, &child_request).unwrap();
        assert_eq!(
            result.allowed_actions,
            vec!["read".to_string(), "write".to_string()]
        );
        assert_eq!(result.max_delegation_depth, 2); // parent(3) - 1
        assert!((result.max_cost_budget - 500.0).abs() < f64::EPSILON);
        assert_eq!(
            result.allowed_data_classifications,
            vec!["public".to_string()]
        );
    }

    #[test]
    fn test_attenuate_filters_unauthorized_actions() {
        let parent = parent_permissions();
        let child_request = PermissionSet {
            allowed_actions: vec!["read".into(), "delete".into()], // "delete" not in parent
            max_delegation_depth: 1,
            max_cost_budget: 100.0,
            allowed_data_classifications: vec!["public".into()],
        };

        let result = attenuate(&parent, &child_request).unwrap();
        assert_eq!(result.allowed_actions, vec!["read".to_string()]);
    }

    #[test]
    fn test_attenuate_caps_cost_budget() {
        let parent = parent_permissions();
        let child_request = PermissionSet {
            allowed_actions: vec!["read".into()],
            max_delegation_depth: 1,
            max_cost_budget: 5000.0, // Exceeds parent's 1000.0
            allowed_data_classifications: vec!["public".into()],
        };

        let result = attenuate(&parent, &child_request).unwrap();
        assert!((result.max_cost_budget - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_attenuate_reduces_delegation_depth() {
        let parent = parent_permissions();
        let child_request = PermissionSet {
            allowed_actions: vec!["read".into()],
            max_delegation_depth: 10,
            max_cost_budget: 100.0,
            allowed_data_classifications: vec!["public".into()],
        };

        let result = attenuate(&parent, &child_request).unwrap();
        assert_eq!(result.max_delegation_depth, 2); // 3 - 1
    }

    #[test]
    fn test_attenuate_fails_when_no_depth_remaining() {
        let parent = PermissionSet {
            allowed_actions: vec!["read".into()],
            max_delegation_depth: 0,
            max_cost_budget: 100.0,
            allowed_data_classifications: vec!["public".into()],
        };
        let child_request = PermissionSet {
            allowed_actions: vec!["read".into()],
            max_delegation_depth: 1,
            max_cost_budget: 50.0,
            allowed_data_classifications: vec!["public".into()],
        };

        let result = attenuate(&parent, &child_request);
        assert!(result.is_err());
    }

    #[test]
    fn test_attenuate_result_is_subset_of_parent() {
        let parent = parent_permissions();
        let child_request = PermissionSet {
            allowed_actions: vec!["read".into(), "write".into()],
            max_delegation_depth: 1,
            max_cost_budget: 500.0,
            allowed_data_classifications: vec!["public".into(), "internal".into()],
        };

        let result = attenuate(&parent, &child_request).unwrap();
        assert!(result.is_subset_of(&parent));
    }

    #[test]
    fn test_attenuate_chained_delegation() {
        let parent = parent_permissions();
        let child1_request = PermissionSet {
            allowed_actions: vec!["read".into(), "write".into()],
            max_delegation_depth: 5,
            max_cost_budget: 800.0,
            allowed_data_classifications: vec!["public".into(), "internal".into()],
        };
        let child1 = attenuate(&parent, &child1_request).unwrap();
        assert_eq!(child1.max_delegation_depth, 2);

        let child2_request = PermissionSet {
            allowed_actions: vec!["read".into()],
            max_delegation_depth: 5,
            max_cost_budget: 400.0,
            allowed_data_classifications: vec!["public".into()],
        };
        let child2 = attenuate(&child1, &child2_request).unwrap();
        assert_eq!(child2.max_delegation_depth, 1);
        assert!(child2.is_subset_of(&child1));
        assert!(child2.is_subset_of(&parent));

        let child3_request = PermissionSet {
            allowed_actions: vec!["read".into()],
            max_delegation_depth: 5,
            max_cost_budget: 200.0,
            allowed_data_classifications: vec!["public".into()],
        };
        let child3 = attenuate(&child2, &child3_request).unwrap();
        assert_eq!(child3.max_delegation_depth, 0);

        // Further delegation should fail.
        let child4_request = PermissionSet {
            allowed_actions: vec!["read".into()],
            max_delegation_depth: 1,
            max_cost_budget: 100.0,
            allowed_data_classifications: vec!["public".into()],
        };
        let result = attenuate(&child3, &child4_request);
        assert!(result.is_err());
    }

    #[test]
    fn test_attenuate_empty_intersection() {
        let parent = parent_permissions();
        let child_request = PermissionSet {
            allowed_actions: vec!["delete".into(), "admin".into()],
            max_delegation_depth: 1,
            max_cost_budget: 100.0,
            allowed_data_classifications: vec!["secret".into()],
        };

        let result = attenuate(&parent, &child_request).unwrap();
        assert!(result.allowed_actions.is_empty());
        assert!(result.allowed_data_classifications.is_empty());
    }
}
