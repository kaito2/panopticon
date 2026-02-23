use crate::decomposition::{
    HybridStrategy, ParallelStrategy, SequentialStrategy, traits::DecompositionStrategy,
};
use crate::types::{Task, TaskCharacteristics, TaskEvent};
use crate::verification::{TaskResult, Verifier, verifiers::DirectInspectionVerifier};
use anyhow::{Result, bail};
use chrono::Utc;

use crate::cli::TaskAction;
use crate::cli::state::AppState;

pub async fn handle(action: TaskAction, state: &AppState) -> Result<()> {
    match action {
        TaskAction::Create {
            name,
            description,
            complexity,
            criticality,
            verifiability,
            reversibility,
            capabilities,
        } => {
            let mut task = Task::new(&name, &description);
            task.characteristics = TaskCharacteristics {
                complexity,
                criticality,
                verifiability,
                reversibility,
                ..TaskCharacteristics::default()
            };

            if let Some(caps) = capabilities {
                task.required_capabilities = caps
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }

            let id = task.id;
            println!("Created task: {} ({})", name, id);
            print_task(&task);
            state.tasks.insert(id, task);
        }

        TaskAction::List => {
            if state.tasks.is_empty() {
                println!("No tasks.");
                return Ok(());
            }
            for entry in state.tasks.iter() {
                let t = entry.value();
                println!("  {} {:?}  {}", t.id, t.state, t.name);
            }
        }

        TaskAction::Get { id } => match state.tasks.get(&id) {
            Some(entry) => print_task(entry.value()),
            None => bail!("Task not found: {id}"),
        },

        TaskAction::Transition { id, event } => {
            let event = parse_event(&event)?;
            let mut entry = state
                .tasks
                .get_mut(&id)
                .ok_or_else(|| anyhow::anyhow!("Task not found: {id}"))?;
            let prev = entry.state;
            entry.apply_event(event)?;
            println!("{:?} -> {:?}", prev, entry.state);
        }

        TaskAction::Decompose {
            id,
            strategy: strategy_name,
        } => {
            let task = state
                .tasks
                .get(&id)
                .ok_or_else(|| anyhow::anyhow!("Task not found: {id}"))?
                .clone();

            let strategy: Box<dyn DecompositionStrategy> = match strategy_name.as_str() {
                "sequential" => Box::new(SequentialStrategy::default()),
                "parallel" => Box::new(ParallelStrategy::default()),
                "hybrid" => Box::new(HybridStrategy::default()),
                other => bail!("Unknown strategy: {other} (use sequential, parallel, hybrid)"),
            };

            let proposal = strategy.decompose(&task).await?;

            println!(
                "Decomposed into {} subtasks ({:?}, parallelism={:.1})",
                proposal.subtasks.len(),
                proposal.execution_order,
                proposal.parallelism_factor,
            );
            for (i, sub) in proposal.subtasks.iter().enumerate() {
                println!(
                    "  [{}] {} (complexity={:.2})",
                    i, sub.name, sub.characteristics.complexity
                );
                state.tasks.insert(sub.id, sub.clone());
            }
            println!("{} dependencies", proposal.dependencies.len());
        }

        TaskAction::Assign { id, agent } => {
            // Verify both exist
            if !state.agents.contains_key(&agent) {
                bail!("Agent not found: {agent}");
            }
            let mut entry = state
                .tasks
                .get_mut(&id)
                .ok_or_else(|| anyhow::anyhow!("Task not found: {id}"))?;
            entry.assigned_agent_id = Some(agent);
            println!("Assigned task {} to agent {}", id, agent);
        }

        TaskAction::Verify { id } => {
            let task = state
                .tasks
                .get(&id)
                .ok_or_else(|| anyhow::anyhow!("Task not found: {id}"))?
                .clone();

            let agent_id = task
                .assigned_agent_id
                .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

            // Build a TaskResult from task metadata
            let result = TaskResult {
                task_id: task.id,
                agent_id,
                output: task.metadata.clone(),
                completed_at: Utc::now(),
                resource_consumed: 0.0,
            };

            let verifier = DirectInspectionVerifier::new(vec!["result".to_string()]);
            let outcome = verifier.verify(&task, &result).await.map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Verification result: {:?}", outcome);
        }
    }
    Ok(())
}

fn parse_event(s: &str) -> Result<TaskEvent> {
    match s {
        "StartDecomposition" => Ok(TaskEvent::StartDecomposition),
        "DecompositionComplete" => Ok(TaskEvent::DecompositionComplete),
        "SkipDecomposition" => Ok(TaskEvent::SkipDecomposition),
        "StartNegotiation" => Ok(TaskEvent::StartNegotiation),
        "NegotiationComplete" => Ok(TaskEvent::NegotiationComplete),
        "ContractSigned" => Ok(TaskEvent::ContractSigned),
        "StartExecution" => Ok(TaskEvent::StartExecution),
        "ExecutionComplete" => Ok(TaskEvent::ExecutionComplete),
        "VerificationPassed" => Ok(TaskEvent::VerificationPassed),
        "VerificationFailed" => Ok(TaskEvent::VerificationFailed),
        "DisputeRaised" => Ok(TaskEvent::DisputeRaised),
        "DisputeResolved" => Ok(TaskEvent::DisputeResolved),
        "TaskFailed" => Ok(TaskEvent::TaskFailed),
        "Retry" => Ok(TaskEvent::Retry),
        other => bail!(
            "Unknown event: {other}\nValid events: StartDecomposition, DecompositionComplete, \
             SkipDecomposition, StartNegotiation, NegotiationComplete, ContractSigned, \
             StartExecution, ExecutionComplete, VerificationPassed, VerificationFailed, \
             DisputeRaised, DisputeResolved, TaskFailed, Retry"
        ),
    }
}

pub fn print_task(t: &Task) {
    println!("  ID:          {}", t.id);
    println!("  Name:        {}", t.name);
    println!("  State:       {:?}", t.state);
    println!("  Complexity:  {:.2}", t.characteristics.complexity);
    println!("  Criticality: {:.2}", t.characteristics.criticality);
    println!("  Verifiab.:   {:.2}", t.characteristics.verifiability);
    println!("  Reversib.:   {:.2}", t.characteristics.reversibility);
    if !t.required_capabilities.is_empty() {
        println!("  Capabilities: {}", t.required_capabilities.join(", "));
    }
    if let Some(agent) = t.assigned_agent_id {
        println!("  Assigned to: {agent}");
    }
}
