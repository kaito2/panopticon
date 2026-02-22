use crate::decomposition::{HybridStrategy, traits::DecompositionStrategy};
use crate::types::*;
use anyhow::Result;

use crate::cli::state::AppState;

/// Run a full delegation lifecycle demo.
pub async fn run(state: &AppState) -> Result<()> {
    println!("=== Panopticon: Delegation Lifecycle Demo ===\n");

    // --- 1. Create a task ---
    let mut task = Task::new(
        "Analyze market data",
        "Collect and analyze recent market trends for the tech sector",
    );
    task.characteristics = TaskCharacteristics {
        complexity: 0.7,
        criticality: 0.6,
        uncertainty: 0.4,
        verifiability: 0.8,
        reversibility: 0.9,
        time_sensitivity: 0.5,
        resource_intensity: 0.3,
        privacy_sensitivity: 0.2,
        human_interaction: 0.1,
        novelty: 0.3,
        interdependency: 0.4,
    };
    task.required_capabilities = vec!["data_analysis".into(), "market_research".into()];
    println!("[1] Created task: {} ({})", task.name, task.id);
    println!("    State: {:?}\n", task.state);

    // --- 2. Decompose ---
    let strategy = HybridStrategy::default();
    let proposal = strategy.decompose(&task).await?;
    println!(
        "[2] Decomposed into {} subtasks ({:?})",
        proposal.subtasks.len(),
        proposal.execution_order,
    );
    for (i, sub) in proposal.subtasks.iter().enumerate() {
        println!("    [{i}] {}", sub.name);
    }
    println!();

    // --- 3. Register agents ---
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
    println!(
        "[3] Agent: {} ({}) -- reputation {:.3}",
        analyst.name,
        analyst.id,
        analyst.reputation.composite()
    );

    // --- 4. Capability check ---
    let has_caps = task
        .required_capabilities
        .iter()
        .all(|c| analyst.has_capability(c));
    println!("[4] Capability match: {has_caps}");

    // --- 5. Permission check ---
    let approval =
        crate::permissions::ApprovalRequirement::from_characteristics(&task.characteristics);
    println!("[5] Approval level: {:?}\n", approval.level);

    // --- 6. Walk through state machine ---
    println!("[6] Delegation lifecycle:");
    let events = [
        TaskEvent::SkipDecomposition,
        TaskEvent::StartNegotiation,
        TaskEvent::NegotiationComplete,
        TaskEvent::StartExecution,
        TaskEvent::ExecutionComplete,
        TaskEvent::VerificationPassed,
    ];
    for event in events {
        let prev = task.state;
        task.apply_event(event)?;
        println!("    {prev:?} --({event:?})--> {:?}", task.state);
    }

    println!("\n    Task completed successfully!");

    // Save to state
    state.tasks.insert(task.id, task);
    state.agents.insert(analyst.id, analyst);

    Ok(())
}
