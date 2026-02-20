use async_trait::async_trait;
use panopticon_types::{PanopticonError, Task};

use crate::proposal::DecompositionProposal;

/// Strategy for decomposing tasks into subtasks.
#[async_trait]
pub trait DecompositionStrategy: Send + Sync {
    /// Decompose a task into a proposal of subtasks.
    async fn decompose(&self, task: &Task) -> Result<DecompositionProposal, PanopticonError>;

    /// Name of this strategy.
    fn name(&self) -> &str;
}
