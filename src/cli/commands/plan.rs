use anyhow::Result;
use std::io::{self, Write};
use uuid::Uuid;

use crate::cli::state::AppState;
use crate::executor::{ClaudeExecutor, AgentExecutor, ExecutionContext};
use crate::types::{Task, TaskCharacteristics};

/// Handle the `plan` command: decompose a goal into subtasks using Claude.
pub async fn handle(goal: &str, model: &str, state: &AppState) -> Result<()> {
    println!("Planning with Claude ({model})...");

    let executor = ClaudeExecutor::default().with_model(model);

    // Check if claude CLI is available.
    match executor.health_check().await {
        Ok(true) => {}
        _ => {
            anyhow::bail!(
                "claude CLI is not available. Install it from https://claude.ai/code\n\
                 Falling back is not supported yet."
            );
        }
    }

    // Create a planning meta-task.
    let planning_task = Task::new(
        "Plan decomposition",
        format!(
            "You are a task planning assistant. Given a goal, decompose it into concrete subtasks.\n\n\
             Goal: {goal}\n\n\
             Respond with a JSON object with this exact structure:\n\
             {{\n  \
               \"subtasks\": [\n    \
                 {{\n      \
                   \"name\": \"short task name\",\n      \
                   \"description\": \"detailed description of what to do\",\n      \
                   \"complexity\": 0.5,\n      \
                   \"criticality\": 0.5,\n      \
                   \"verifiability\": 0.5,\n      \
                   \"reversibility\": 0.5,\n      \
                   \"capabilities\": [\"cap1\", \"cap2\"]\n    \
                 }}\n  \
               ],\n  \
               \"dependencies\": [[0, 1], [1, 2]]\n\
             }}\n\n\
             The dependencies array contains [from_index, to_index] pairs meaning \
             subtask at from_index must complete before subtask at to_index can start.\n\
             Keep the number of subtasks between 2 and 8."
        ),
    );

    let ctx = ExecutionContext::default();
    let result = executor
        .execute(&planning_task, &ctx)
        .await
        .map_err(|e| anyhow::anyhow!("Planning failed: {e}"))?;

    // Parse the structured result.
    let subtasks_data = result
        .output
        .get("subtasks")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Claude did not return a valid subtask array"))?;

    let dependencies = result
        .output
        .get("dependencies")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Create a parent task.
    let mut parent = Task::new(goal, goal);
    let parent_id = parent.id;
    let mut subtask_ids = Vec::new();

    println!(
        "Decomposed into {} subtasks:",
        subtasks_data.len()
    );

    for (i, st) in subtasks_data.iter().enumerate() {
        let name = st
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unnamed");
        let desc = st
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let complexity = st
            .get("complexity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let criticality = st
            .get("criticality")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let verifiability = st
            .get("verifiability")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let reversibility = st
            .get("reversibility")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let mut task = Task::new(name, desc);
        task.parent_id = Some(parent_id);
        task.characteristics = TaskCharacteristics {
            complexity,
            criticality,
            verifiability,
            reversibility,
            ..TaskCharacteristics::default()
        };

        if let Some(caps) = st.get("capabilities").and_then(|v| v.as_array()) {
            task.required_capabilities = caps
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }

        println!(
            "  [{}] {} (complexity={:.1}, criticality={:.1})",
            i + 1,
            name,
            complexity,
            criticality,
        );

        subtask_ids.push(task.id);
        state.tasks.insert(task.id, task);
    }

    if !dependencies.is_empty() {
        let dep_strs: Vec<String> = dependencies
            .iter()
            .filter_map(|d| {
                let arr = d.as_array()?;
                let from = arr.first()?.as_u64()? as usize;
                let to = arr.get(1)?.as_u64()? as usize;
                Some(format!("[{}] -> [{}]", from + 1, to + 1))
            })
            .collect();
        println!("\nDependencies: {}", dep_strs.join(", "));
    }

    // Ask for confirmation.
    print!("\nProceed? [Y/n] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if !input.is_empty() && !input.eq_ignore_ascii_case("y") && !input.eq_ignore_ascii_case("yes")
    {
        println!("Cancelled.");
        // Clean up subtasks.
        for id in &subtask_ids {
            state.tasks.remove(id);
        }
        return Ok(());
    }

    // Store dependency info in parent metadata.
    parent.subtask_ids = subtask_ids;
    parent.metadata = serde_json::json!({
        "dependencies": dependencies,
        "goal": goal,
    });

    println!("\nCreated task tree: {} ({} subtasks)", parent_id, parent.subtask_ids.len());
    state.tasks.insert(parent_id, parent);

    Ok(())
}

/// Plan without Claude — use built-in decomposition strategies.
/// This is a fallback when Claude is unavailable.
#[allow(dead_code)]
pub async fn plan_offline(
    goal: &str,
    state: &AppState,
) -> Result<Uuid> {
    use crate::decomposition::{HybridStrategy, traits::DecompositionStrategy};

    let mut parent = Task::new(goal, goal);
    let parent_id = parent.id;

    let strategy = HybridStrategy::default();
    let proposal = strategy.decompose(&parent).await
        .map_err(|e| anyhow::anyhow!("Decomposition failed: {e}"))?;

    println!(
        "Decomposed into {} subtasks ({:?})",
        proposal.subtasks.len(),
        proposal.execution_order,
    );

    for (i, sub) in proposal.subtasks.iter().enumerate() {
        println!("  [{}] {}", i + 1, sub.name);
        parent.subtask_ids.push(sub.id);
        state.tasks.insert(sub.id, sub.clone());
    }

    state.tasks.insert(parent_id, parent);
    Ok(parent_id)
}
