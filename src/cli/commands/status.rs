use anyhow::Result;

use crate::cli::state::AppState;
use crate::reputation::ReputationEngine;
use crate::types::TaskState;

/// Handle the `status` command: show a dashboard of task/agent state.
pub async fn handle(state: &AppState) -> Result<()> {
    // Task statistics.
    let total = state.tasks.len();
    let mut completed = 0;
    let mut in_progress = 0;
    let mut pending = 0;
    let mut failed = 0;
    let mut other = 0;

    for entry in state.tasks.iter() {
        match entry.value().state {
            TaskState::Completed => completed += 1,
            TaskState::InProgress => in_progress += 1,
            TaskState::Pending => pending += 1,
            TaskState::Failed => failed += 1,
            _ => other += 1,
        }
    }

    println!("Tasks: {} total ({} completed, {} in-progress, {} pending, {} failed{})",
        total,
        completed,
        in_progress,
        pending,
        failed,
        if other > 0 { format!(", {} other", other) } else { String::new() },
    );

    // Agent statistics.
    let agent_count = state.agents.len();
    if agent_count > 0 {
        print!("Agents: {} registered (", agent_count);
        let mut first = true;
        for entry in state.agents.iter() {
            let a = entry.value();
            let composite = state
                .reputation_engine
                .get_composite_score(a.id)
                .unwrap_or(0.5);
            let trust = ReputationEngine::compute_trust_level(composite);
            if !first {
                print!(", ");
            }
            print!("{} reputation: {:.3} [{:?}]", a.name, composite, trust);
            first = false;
        }
        println!(")");
    } else {
        println!("Agents: none registered");
    }

    // Ledger statistics.
    let entries = state
        .ledger
        .all_entries()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let integrity = state
        .ledger
        .verify_integrity()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "Ledger: {} entries, integrity: {}",
        entries.len(),
        if integrity { "OK" } else { "CORRUPTED" }
    );

    // Show recent tasks.
    if total > 0 {
        println!("\nRecent tasks:");
        let mut tasks: Vec<_> = state.tasks.iter().map(|e| e.value().clone()).collect();
        tasks.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        for t in tasks.iter().take(10) {
            println!(
                "  {} {:?}  {}{}",
                &t.id.to_string()[..8],
                t.state,
                t.name,
                if let Some(agent) = t.assigned_agent_id {
                    format!(" [agent: {}]", &agent.to_string()[..8])
                } else {
                    String::new()
                }
            );
        }
    }

    Ok(())
}
