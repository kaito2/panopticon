use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::Task;

/// How subtasks relate to each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionOrder {
    Sequential,
    Parallel,
    Hybrid,
}

/// A dependency edge in the subtask DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskDependency {
    pub from: Uuid,
    pub to: Uuid,
}

/// A proposal for decomposing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionProposal {
    pub parent_task_id: Uuid,
    pub subtasks: Vec<Task>,
    pub dependencies: Vec<SubtaskDependency>,
    pub execution_order: ExecutionOrder,
    pub estimated_total_cost: f64,
    pub estimated_total_duration_secs: u64,
    pub parallelism_factor: f64,
}

impl DecompositionProposal {
    pub fn new(parent_task_id: Uuid) -> Self {
        Self {
            parent_task_id,
            subtasks: Vec::new(),
            dependencies: Vec::new(),
            execution_order: ExecutionOrder::Sequential,
            estimated_total_cost: 0.0,
            estimated_total_duration_secs: 0,
            parallelism_factor: 1.0,
        }
    }

    pub fn add_subtask(&mut self, task: Task) {
        self.subtasks.push(task);
    }

    pub fn add_dependency(&mut self, from: Uuid, to: Uuid) {
        self.dependencies.push(SubtaskDependency { from, to });
    }

    /// Check if the DAG has cycles (simple DFS).
    pub fn is_acyclic(&self) -> bool {
        use std::collections::{HashMap, HashSet};

        let mut adj: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for dep in &self.dependencies {
            adj.entry(dep.from).or_default().push(dep.to);
        }

        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        fn dfs(
            node: Uuid,
            adj: &HashMap<Uuid, Vec<Uuid>>,
            visited: &mut HashSet<Uuid>,
            stack: &mut HashSet<Uuid>,
        ) -> bool {
            visited.insert(node);
            stack.insert(node);
            if let Some(neighbors) = adj.get(&node) {
                for &next in neighbors {
                    if !visited.contains(&next) {
                        if !dfs(next, adj, visited, stack) {
                            return false;
                        }
                    } else if stack.contains(&next) {
                        return false;
                    }
                }
            }
            stack.remove(&node);
            true
        }

        for task in &self.subtasks {
            if !visited.contains(&task.id) && !dfs(task.id, &adj, &mut visited, &mut stack) {
                return false;
            }
        }
        true
    }
}
