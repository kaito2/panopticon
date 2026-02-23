use anyhow::{Result, bail};
use chrono::Utc;
use std::io::Write;
use uuid::Uuid;

use crate::cli::state::AppState;
use crate::executor::{AgentExecutor, ClaudeExecutor, ExecutionContext};
use crate::reputation::score::{ReputationDimension, ReputationObservation};
use crate::types::{Agent, Capability, TaskEvent, TaskState};
use crate::verification::{VerificationOutcome, Verifier, verifiers::DirectInspectionVerifier};

/// Handle the `execute` command.
pub async fn handle(
    id: Option<Uuid>,
    all: bool,
    model: &str,
    state: &AppState,
) -> Result<()> {
    let executor = ClaudeExecutor::default().with_model(model);

    // Check health.
    match executor.health_check().await {
        Ok(true) => {}
        _ => bail!("claude CLI is not available."),
    }

    // Ensure a default Claude agent exists.
    let agent_id = ensure_default_agent(state, model);

    if let Some(task_id) = id {
        // Execute a single task (which may be a parent with subtasks).
        let task = state
            .tasks
            .get(&task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {task_id}"))?
            .clone();

        if task.subtask_ids.is_empty() {
            // Leaf task — execute directly.
            execute_single_task(task_id, agent_id, &executor, state).await?;
        } else {
            // Parent task — execute all subtasks in dependency order.
            execute_task_tree(&task, agent_id, &executor, state).await?;
        }
    } else if all {
        // Execute all pending root tasks.
        let root_tasks: Vec<Uuid> = state
            .tasks
            .iter()
            .filter(|e| e.value().parent_id.is_none() && e.value().state == TaskState::Pending)
            .map(|e| *e.key())
            .collect();

        if root_tasks.is_empty() {
            println!("No pending tasks to execute.");
            return Ok(());
        }

        for task_id in root_tasks {
            let task = state.tasks.get(&task_id).unwrap().clone();
            if task.subtask_ids.is_empty() {
                execute_single_task(task_id, agent_id, &executor, state).await?;
            } else {
                execute_task_tree(&task, agent_id, &executor, state).await?;
            }
        }
    } else {
        bail!("Specify a task ID or use --all");
    }

    Ok(())
}

/// Execute subtasks of a parent task in dependency order.
async fn execute_task_tree(
    parent: &crate::types::Task,
    agent_id: Uuid,
    executor: &ClaudeExecutor,
    state: &AppState,
) -> Result<()> {
    let subtask_ids = &parent.subtask_ids;
    let total = subtask_ids.len();

    // Parse dependencies from parent metadata.
    let deps: Vec<(usize, usize)> = parent
        .metadata
        .get("dependencies")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    let a = d.as_array()?;
                    Some((a.first()?.as_u64()? as usize, a.get(1)?.as_u64()? as usize))
                })
                .collect()
        })
        .unwrap_or_default();

    // Simple topological execution: track which indices are completed.
    let mut completed = vec![false; total];
    let mut execution_order = Vec::new();

    // Kahn's algorithm for topological sort.
    let mut in_degree = vec![0usize; total];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); total];
    for &(from, to) in &deps {
        if from < total && to < total {
            adj[from].push(to);
            in_degree[to] += 1;
        }
    }

    let mut queue: Vec<usize> = (0..total).filter(|&i| in_degree[i] == 0).collect();
    while let Some(idx) = queue.pop() {
        execution_order.push(idx);
        for &next in &adj[idx] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push(next);
            }
        }
    }

    // Add any remaining (in case of cycles or disconnected nodes).
    for i in 0..total {
        if !execution_order.contains(&i) {
            execution_order.push(i);
        }
    }

    let mut all_passed = true;

    for (step, &idx) in execution_order.iter().enumerate() {
        if idx >= subtask_ids.len() {
            continue;
        }
        let task_id = subtask_ids[idx];
        let task_name = state
            .tasks
            .get(&task_id)
            .map(|t| t.value().name.clone())
            .unwrap_or_default();

        println!(
            "\nExecuting subtask [{}/{}]: {}",
            step + 1,
            total,
            task_name
        );

        match execute_single_task(task_id, agent_id, executor, state).await {
            Ok(()) => {
                completed[idx] = true;
            }
            Err(e) => {
                println!("  Failed: {e}");
                all_passed = false;
                // Don't continue with dependents.
                break;
            }
        }
    }

    if all_passed {
        println!("\nAll subtasks completed successfully.");
    } else {
        println!("\nSome subtasks failed. Use `panopticon status` to review.");
    }

    Ok(())
}

/// Execute a single leaf task through the full lifecycle.
async fn execute_single_task(
    task_id: Uuid,
    agent_id: Uuid,
    executor: &ClaudeExecutor,
    state: &AppState,
) -> Result<()> {
    // Get agent info for display.
    let agent_name = state
        .agents
        .get(&agent_id)
        .map(|a| a.value().name.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let reputation = state
        .reputation_engine
        .get_composite_score(agent_id)
        .unwrap_or(0.5);

    println!("  Agent: {} (reputation: {:.3})", agent_name, reputation);

    // Walk through state machine.
    // Pending → AwaitingAssignment (skip decomposition for leaf tasks).
    {
        let mut entry = state
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {task_id}"))?;
        if entry.state == TaskState::Pending {
            entry.apply_event(TaskEvent::SkipDecomposition)?;
        }
    }

    // AwaitingAssignment → Negotiating → Contracted → InProgress
    {
        let mut entry = state.tasks.get_mut(&task_id).unwrap();
        entry.assigned_agent_id = Some(agent_id);
        if entry.state == TaskState::AwaitingAssignment {
            entry.apply_event(TaskEvent::StartNegotiation)?;
        }
    }
    {
        let mut entry = state.tasks.get_mut(&task_id).unwrap();
        if entry.state == TaskState::Negotiating {
            entry.apply_event(TaskEvent::NegotiationComplete)?;
        }
    }
    // ContractSigned to get to Contracted is not needed — NegotiationComplete already goes to Contracted.
    {
        let mut entry = state.tasks.get_mut(&task_id).unwrap();
        if entry.state == TaskState::Contracted {
            entry.apply_event(TaskEvent::StartExecution)?;
        }
    }

    // Actually execute.
    let task = state.tasks.get(&task_id).unwrap().clone();
    let ctx = ExecutionContext::default();

    print!("  Running...");
    let _ = std::io::stdout().flush();
    let start = std::time::Instant::now();
    let exec_result = executor.execute(&task, &ctx).await;
    let elapsed = start.elapsed();

    match exec_result {
        Ok(mut result) => {
            result.agent_id = agent_id;
            println!(" done ({:.1}s)", elapsed.as_secs_f64());

            // ExecutionComplete.
            {
                let mut entry = state.tasks.get_mut(&task_id).unwrap();
                entry.metadata = result.output.clone();
                entry.apply_event(TaskEvent::ExecutionComplete)?;
            }

            // Verify.
            let verifier = DirectInspectionVerifier::new(vec!["result".to_string()]);
            let outcome = verifier.verify(&task, &result).await.map_err(|e| anyhow::anyhow!("{e}"))?;

            match &outcome {
                VerificationOutcome::Passed { confidence } => {
                    println!("  Verification: Passed (confidence: {:.1})", confidence);

                    let mut entry = state.tasks.get_mut(&task_id).unwrap();
                    entry.apply_event(TaskEvent::VerificationPassed)?;

                    // Update reputation positively.
                    let obs = ReputationObservation {
                        agent_id,
                        task_id,
                        dimension: ReputationDimension::Completion,
                        value: 1.0,
                        timestamp: Utc::now(),
                    };
                    let _ = state.reputation_engine.update_reputation(obs).await;

                    let obs = ReputationObservation {
                        agent_id,
                        task_id,
                        dimension: ReputationDimension::Quality,
                        value: *confidence,
                        timestamp: Utc::now(),
                    };
                    let _ = state.reputation_engine.update_reputation(obs).await;
                }
                VerificationOutcome::Failed { reason } => {
                    println!("  Verification: Failed ({})", reason);

                    let mut entry = state.tasks.get_mut(&task_id).unwrap();
                    entry.apply_event(TaskEvent::VerificationFailed)?;

                    // Update reputation negatively.
                    let obs = ReputationObservation {
                        agent_id,
                        task_id,
                        dimension: ReputationDimension::Completion,
                        value: 0.0,
                        timestamp: Utc::now(),
                    };
                    let _ = state.reputation_engine.update_reputation(obs).await;

                    println!("  Hint: Use `panopticon task transition {} --event Retry` to retry", task_id);
                }
                VerificationOutcome::Inconclusive => {
                    println!("  Verification: Inconclusive");

                    let mut entry = state.tasks.get_mut(&task_id).unwrap();
                    entry.apply_event(TaskEvent::VerificationPassed)?;
                }
            }
        }
        Err(e) => {
            println!(" failed ({:.1}s)", elapsed.as_secs_f64());
            println!("  Error: {e}");

            let mut entry = state.tasks.get_mut(&task_id).unwrap();
            entry.apply_event(TaskEvent::TaskFailed)?;
        }
    }

    Ok(())
}

/// Ensure a default Claude agent is registered in the state.
fn ensure_default_agent(state: &AppState, model: &str) -> Uuid {
    let agent_name = format!("claude-{model}");

    // Check if already registered.
    for entry in state.agents.iter() {
        if entry.value().name == agent_name {
            return *entry.key();
        }
    }

    // Register a new default Claude agent.
    let mut agent = Agent::new(&agent_name);
    agent.capabilities.capabilities.push(Capability {
        name: "general".to_string(),
        proficiency: 0.8,
        certified: false,
        last_verified: Some(Utc::now()),
    });
    agent.capabilities.capabilities.push(Capability {
        name: "code".to_string(),
        proficiency: 0.9,
        certified: false,
        last_verified: Some(Utc::now()),
    });
    agent.capabilities.capabilities.push(Capability {
        name: "analysis".to_string(),
        proficiency: 0.85,
        certified: false,
        last_verified: Some(Utc::now()),
    });

    let id = agent.id;
    state.agents.insert(id, agent);
    println!("Auto-registered agent: {} ({})", agent_name, id);
    id
}
