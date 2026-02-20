use panopticon_types::*;

/// Single-hop delegation: A → B
#[test]
fn test_single_hop_delegation() {
    // Create a task
    let mut task = Task::new("Translate document", "Translate from English to Japanese");
    task.characteristics.complexity = 0.3;
    task.characteristics.criticality = 0.5;
    task.characteristics.verifiability = 0.8;
    assert_eq!(task.state, TaskState::Pending);

    // Create agents
    let agent_a = Agent::new("delegator-a");
    let mut agent_b = Agent::new("delegatee-b");
    agent_b.capabilities.capabilities.push(Capability {
        name: "translation".into(),
        proficiency: 0.9,
        certified: true,
        last_verified: Some(chrono::Utc::now()),
    });

    // Verify agent B has required capability
    assert!(agent_b.has_capability("translation"));

    // Walk through the state machine
    task.apply_event(TaskEvent::SkipDecomposition).unwrap();
    assert_eq!(task.state, TaskState::AwaitingAssignment);

    task.apply_event(TaskEvent::StartNegotiation).unwrap();
    assert_eq!(task.state, TaskState::Negotiating);

    task.apply_event(TaskEvent::NegotiationComplete).unwrap();
    assert_eq!(task.state, TaskState::Contracted);

    task.apply_event(TaskEvent::StartExecution).unwrap();
    assert_eq!(task.state, TaskState::InProgress);

    task.apply_event(TaskEvent::ExecutionComplete).unwrap();
    assert_eq!(task.state, TaskState::AwaitingVerification);

    task.apply_event(TaskEvent::VerificationPassed).unwrap();
    assert_eq!(task.state, TaskState::Completed);
}

/// Multi-hop chain: A → B → C
#[test]
fn test_multi_hop_delegation_chain() {
    let agent_a = Agent::new("origin");
    let agent_b = Agent::new("intermediary");
    let agent_c = Agent::new("executor");

    let mut chain = DelegationChain::new();
    chain.add_link(DelegationLink {
        from_agent_id: agent_a.id,
        to_agent_id: agent_b.id,
        contract_id: uuid::Uuid::new_v4(),
        task_id: uuid::Uuid::new_v4(),
        depth: 0,
        attestation: None,
        created_at: chrono::Utc::now(),
    });
    chain.add_link(DelegationLink {
        from_agent_id: agent_b.id,
        to_agent_id: agent_c.id,
        contract_id: uuid::Uuid::new_v4(),
        task_id: uuid::Uuid::new_v4(),
        depth: 1,
        attestation: None,
        created_at: chrono::Utc::now(),
    });

    assert_eq!(chain.depth(), 2);
    assert_eq!(chain.origin().unwrap(), agent_a.id);
    assert_eq!(chain.terminal().unwrap(), agent_c.id);
}

/// Permission attenuation in delegation chain
#[test]
fn test_permission_attenuation_chain() {
    let parent_perms = PermissionSet {
        allowed_actions: vec!["read".into(), "write".into(), "execute".into()],
        max_delegation_depth: 3,
        max_cost_budget: 1000.0,
        allowed_data_classifications: vec!["public".into(), "internal".into()],
    };

    let child_perms = PermissionSet {
        allowed_actions: vec!["read".into(), "write".into()],
        max_delegation_depth: 2,
        max_cost_budget: 500.0,
        allowed_data_classifications: vec!["public".into()],
    };

    assert!(child_perms.is_subset_of(&parent_perms));

    // Grandchild must be even more restricted
    let grandchild_perms = PermissionSet {
        allowed_actions: vec!["read".into()],
        max_delegation_depth: 1,
        max_cost_budget: 100.0,
        allowed_data_classifications: vec!["public".into()],
    };

    assert!(grandchild_perms.is_subset_of(&child_perms));
    assert!(grandchild_perms.is_subset_of(&parent_perms));
}

/// Task failure and retry
#[test]
fn test_failure_and_retry() {
    let mut task = Task::new("risky task", "might fail");

    task.apply_event(TaskEvent::SkipDecomposition).unwrap();
    task.apply_event(TaskEvent::StartNegotiation).unwrap();
    task.apply_event(TaskEvent::NegotiationComplete).unwrap();
    task.apply_event(TaskEvent::StartExecution).unwrap();

    // Task fails
    task.apply_event(TaskEvent::TaskFailed).unwrap();
    assert_eq!(task.state, TaskState::Failed);

    // Retry
    task.apply_event(TaskEvent::Retry).unwrap();
    assert_eq!(task.state, TaskState::Pending);

    // Second attempt succeeds
    task.apply_event(TaskEvent::SkipDecomposition).unwrap();
    task.apply_event(TaskEvent::StartNegotiation).unwrap();
    task.apply_event(TaskEvent::NegotiationComplete).unwrap();
    task.apply_event(TaskEvent::StartExecution).unwrap();
    task.apply_event(TaskEvent::ExecutionComplete).unwrap();
    task.apply_event(TaskEvent::VerificationPassed).unwrap();
    assert_eq!(task.state, TaskState::Completed);
}

/// Dispute lifecycle
#[test]
fn test_dispute_lifecycle() {
    let mut task = Task::new("disputed task", "will be disputed");

    task.apply_event(TaskEvent::SkipDecomposition).unwrap();
    task.apply_event(TaskEvent::StartNegotiation).unwrap();
    task.apply_event(TaskEvent::NegotiationComplete).unwrap();
    task.apply_event(TaskEvent::StartExecution).unwrap();
    task.apply_event(TaskEvent::ExecutionComplete).unwrap();

    assert_eq!(task.state, TaskState::AwaitingVerification);

    // Dispute raised
    task.apply_event(TaskEvent::DisputeRaised).unwrap();
    assert_eq!(task.state, TaskState::Disputed);

    // Dispute resolved
    task.apply_event(TaskEvent::DisputeResolved).unwrap();
    assert_eq!(task.state, TaskState::Completed);
}
