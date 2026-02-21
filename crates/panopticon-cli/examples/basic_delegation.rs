use panopticon_types::*;

fn main() {
    println!("=== Panopticon: Basic Delegation Example ===\n");

    // Create a task
    let mut task = Task::new(
        "Analyze market data",
        "Collect and analyze recent market trends for the tech sector",
    );
    task.characteristics = TaskCharacteristics {
        complexity: 0.6,
        criticality: 0.7,
        uncertainty: 0.4,
        verifiability: 0.8,
        reversibility: 0.9,
        time_sensitivity: 0.5,
        resource_intensity: 0.3,
        privacy_sensitivity: 0.2,
        human_interaction: 0.1,
        novelty: 0.3,
        interdependency: 0.2,
    };
    task.required_capabilities = vec!["data_analysis".into(), "market_research".into()];
    println!("Created task: {} (id: {})", task.name, task.id);
    println!("  State: {:?}", task.state);

    // Create agents
    let mut analyst = Agent::new("market-analyst");
    analyst.capabilities.capabilities.push(Capability {
        name: "data_analysis".into(),
        proficiency: 0.85,
        certified: true,
        last_verified: Some(chrono::Utc::now()),
    });
    analyst.capabilities.capabilities.push(Capability {
        name: "market_research".into(),
        proficiency: 0.9,
        certified: true,
        last_verified: Some(chrono::Utc::now()),
    });
    analyst.reputation = ReputationScore {
        completion: 0.9,
        quality: 0.85,
        reliability: 0.95,
        safety: 0.9,
        behavioral: 0.8,
    };
    println!("\nRegistered agent: {} (id: {})", analyst.name, analyst.id);
    println!(
        "  Reputation composite: {:.3}",
        analyst.reputation.composite()
    );

    // Walk through delegation lifecycle
    println!("\n--- Delegation Lifecycle ---");

    task.apply_event(TaskEvent::SkipDecomposition).unwrap();
    println!("  → {:?}", task.state);

    task.apply_event(TaskEvent::StartNegotiation).unwrap();
    println!("  → {:?}", task.state);

    task.apply_event(TaskEvent::NegotiationComplete).unwrap();
    println!("  → {:?}", task.state);

    task.apply_event(TaskEvent::StartExecution).unwrap();
    task.assigned_agent_id = Some(analyst.id);
    println!("  → {:?} (assigned to {})", task.state, analyst.name);

    task.apply_event(TaskEvent::ExecutionComplete).unwrap();
    println!("  → {:?}", task.state);

    task.apply_event(TaskEvent::VerificationPassed).unwrap();
    println!("  → {:?}", task.state);

    println!("\nDelegation completed successfully!");
}
