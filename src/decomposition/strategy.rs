use crate::types::{PanopticonError, Task, TaskCharacteristics};
use async_trait::async_trait;

use super::proposal::{DecompositionProposal, ExecutionOrder};
use super::traits::DecompositionStrategy;

/// Verifiability threshold below which a task should be further decomposed.
const VERIFIABILITY_THRESHOLD: f64 = 0.3;

/// Sequential decomposition — subtasks must execute in order.
pub struct SequentialStrategy {
    pub min_subtasks: usize,
    pub max_subtasks: usize,
}

impl Default for SequentialStrategy {
    fn default() -> Self {
        Self {
            min_subtasks: 2,
            max_subtasks: 5,
        }
    }
}

#[async_trait]
impl DecompositionStrategy for SequentialStrategy {
    async fn decompose(&self, task: &Task) -> Result<DecompositionProposal, PanopticonError> {
        let num_subtasks = compute_subtask_count(&task.characteristics, self.max_subtasks);
        let num_subtasks = num_subtasks.max(self.min_subtasks).min(self.max_subtasks);

        let mut proposal = DecompositionProposal::new(task.id);
        proposal.execution_order = ExecutionOrder::Sequential;

        let mut prev_id = None;
        for i in 0..num_subtasks {
            let mut subtask =
                Task::new(format!("{} - step {}", task.name, i + 1), &task.description);
            subtask.parent_id = Some(task.id);
            subtask.characteristics =
                distribute_characteristics(&task.characteristics, i, num_subtasks);
            subtask.required_capabilities = task.required_capabilities.clone();

            if let Some(prev) = prev_id {
                proposal.add_dependency(prev, subtask.id);
            }
            prev_id = Some(subtask.id);
            proposal.add_subtask(subtask);
        }

        proposal.parallelism_factor = 1.0;
        Ok(proposal)
    }

    fn name(&self) -> &str {
        "sequential"
    }
}

/// Parallel decomposition — subtasks can execute concurrently.
pub struct ParallelStrategy {
    pub max_subtasks: usize,
}

impl Default for ParallelStrategy {
    fn default() -> Self {
        Self { max_subtasks: 8 }
    }
}

#[async_trait]
impl DecompositionStrategy for ParallelStrategy {
    async fn decompose(&self, task: &Task) -> Result<DecompositionProposal, PanopticonError> {
        let num_subtasks = compute_subtask_count(&task.characteristics, self.max_subtasks).max(2);

        let mut proposal = DecompositionProposal::new(task.id);
        proposal.execution_order = ExecutionOrder::Parallel;

        for i in 0..num_subtasks {
            let mut subtask = Task::new(
                format!("{} - partition {}", task.name, i + 1),
                &task.description,
            );
            subtask.parent_id = Some(task.id);
            subtask.characteristics =
                distribute_characteristics(&task.characteristics, i, num_subtasks);
            subtask.required_capabilities = task.required_capabilities.clone();
            proposal.add_subtask(subtask);
        }

        proposal.parallelism_factor = num_subtasks as f64;
        Ok(proposal)
    }

    fn name(&self) -> &str {
        "parallel"
    }
}

/// Hybrid decomposition — some subtasks are sequential, some parallel.
pub struct HybridStrategy {
    verifiability_threshold: f64,
}

impl Default for HybridStrategy {
    fn default() -> Self {
        Self {
            verifiability_threshold: VERIFIABILITY_THRESHOLD,
        }
    }
}

#[async_trait]
impl DecompositionStrategy for HybridStrategy {
    async fn decompose(&self, task: &Task) -> Result<DecompositionProposal, PanopticonError> {
        let mut proposal = DecompositionProposal::new(task.id);
        proposal.execution_order = ExecutionOrder::Hybrid;

        // Phase 1: Preparation (sequential)
        let mut prep = Task::new(format!("{} - prepare", task.name), "Preparation phase");
        prep.parent_id = Some(task.id);
        prep.characteristics = task.characteristics.clone();
        prep.characteristics.complexity *= 0.3;

        // Phase 2: Execution (parallel workers)
        let num_workers = (task.characteristics.complexity * 4.0).ceil() as usize;
        let num_workers = num_workers.clamp(2, 6);

        let mut worker_ids = Vec::new();
        for i in 0..num_workers {
            let mut worker = Task::new(
                format!("{} - worker {}", task.name, i + 1),
                "Parallel execution",
            );
            worker.parent_id = Some(task.id);
            worker.characteristics =
                distribute_characteristics(&task.characteristics, i, num_workers);
            worker.required_capabilities = task.required_capabilities.clone();

            proposal.add_dependency(prep.id, worker.id);
            worker_ids.push(worker.id);
            proposal.add_subtask(worker);
        }

        // Phase 3: Aggregation (sequential after all workers)
        let mut agg = Task::new(format!("{} - aggregate", task.name), "Aggregation phase");
        agg.parent_id = Some(task.id);
        for &wid in &worker_ids {
            proposal.add_dependency(wid, agg.id);
        }

        // Re-decompose if verifiability is low
        if task.characteristics.verifiability < self.verifiability_threshold {
            let mut verify = Task::new(
                format!("{} - extra verification", task.name),
                "Additional verification step for low-verifiability task",
            );
            verify.parent_id = Some(task.id);
            verify.characteristics.verifiability = 0.8;
            proposal.add_dependency(agg.id, verify.id);
            proposal.add_subtask(verify);
        }

        proposal.add_subtask(prep);
        proposal.add_subtask(agg);
        proposal.parallelism_factor = num_workers as f64;

        Ok(proposal)
    }

    fn name(&self) -> &str {
        "hybrid"
    }
}

fn compute_subtask_count(chars: &TaskCharacteristics, max: usize) -> usize {
    let score = chars.complexity * 0.4 + chars.uncertainty * 0.3 + chars.interdependency * 0.3;
    ((score * max as f64).ceil() as usize).max(2).min(max)
}

fn distribute_characteristics(
    parent: &TaskCharacteristics,
    index: usize,
    total: usize,
) -> TaskCharacteristics {
    let fraction = 1.0 / total as f64;
    TaskCharacteristics {
        complexity: parent.complexity * fraction,
        criticality: parent.criticality,
        uncertainty: parent.uncertainty * (1.0 - (index as f64 / total as f64) * 0.2),
        verifiability: (parent.verifiability + 0.1).min(1.0),
        reversibility: parent.reversibility,
        time_sensitivity: parent.time_sensitivity,
        resource_intensity: parent.resource_intensity * fraction,
        privacy_sensitivity: parent.privacy_sensitivity,
        human_interaction: parent.human_interaction * fraction,
        novelty: parent.novelty * (1.0 - (index as f64 / total as f64) * 0.1),
        interdependency: parent.interdependency * 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sequential_decomposition() {
        let strategy = SequentialStrategy::default();
        let task = Task::new("test task", "test description");
        let proposal = strategy.decompose(&task).await.unwrap();

        assert!(proposal.subtasks.len() >= 2);
        assert_eq!(proposal.execution_order, ExecutionOrder::Sequential);
        assert!(proposal.is_acyclic());
        // Sequential: n-1 dependencies
        assert_eq!(proposal.dependencies.len(), proposal.subtasks.len() - 1);
    }

    #[tokio::test]
    async fn test_parallel_decomposition() {
        let strategy = ParallelStrategy::default();
        let task = Task::new("test task", "test description");
        let proposal = strategy.decompose(&task).await.unwrap();

        assert!(proposal.subtasks.len() >= 2);
        assert_eq!(proposal.execution_order, ExecutionOrder::Parallel);
        assert!(proposal.is_acyclic());
        // Parallel: no dependencies
        assert!(proposal.dependencies.is_empty());
    }

    #[tokio::test]
    async fn test_hybrid_decomposition() {
        let strategy = HybridStrategy::default();
        let mut task = Task::new("test task", "test description");
        task.characteristics.complexity = 0.8;
        task.characteristics.verifiability = 0.2; // below threshold

        let proposal = strategy.decompose(&task).await.unwrap();
        assert_eq!(proposal.execution_order, ExecutionOrder::Hybrid);
        assert!(proposal.is_acyclic());
        assert!(proposal.subtasks.len() >= 4); // prep + workers + agg + extra verify
    }

    #[tokio::test]
    async fn test_proposal_acyclicity() {
        let mut proposal = DecompositionProposal::new(uuid::Uuid::new_v4());
        let t1 = Task::new("t1", "");
        let t2 = Task::new("t2", "");
        let t3 = Task::new("t3", "");

        let id1 = t1.id;
        let id2 = t2.id;
        let id3 = t3.id;

        proposal.add_subtask(t1);
        proposal.add_subtask(t2);
        proposal.add_subtask(t3);

        proposal.add_dependency(id1, id2);
        proposal.add_dependency(id2, id3);
        assert!(proposal.is_acyclic());

        // Add cycle
        proposal.add_dependency(id3, id1);
        assert!(!proposal.is_acyclic());
    }
}
