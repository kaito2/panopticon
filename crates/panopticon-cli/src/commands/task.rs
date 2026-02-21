use anyhow::{Result, bail};
use panopticon_decomposition::{
    HybridStrategy, ParallelStrategy, SequentialStrategy, traits::DecompositionStrategy,
};
use panopticon_types::{Task, TaskCharacteristics, TaskEvent};

use crate::TaskAction;
use crate::state::AppState;

pub async fn handle(action: TaskAction, state: &AppState) -> Result<()> {
    match action {
        TaskAction::Create {
            name,
            description,
            complexity,
            criticality,
            verifiability,
            reversibility,
        } => {
            let mut task = Task::new(&name, &description);
            task.characteristics = TaskCharacteristics {
                complexity,
                criticality,
                verifiability,
                reversibility,
                ..TaskCharacteristics::default()
            };

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

fn print_task(t: &Task) {
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
