pub mod store;

pub use store::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::ledger::LedgerEntry;
use crate::reputation::AgentReputation;
use crate::types::{Agent, Task};

/// The top-level persisted state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub tasks: HashMap<Uuid, Task>,
    pub agents: HashMap<Uuid, Agent>,
    pub ledger_entries: Vec<LedgerEntry>,
    pub reputation_scores: HashMap<Uuid, AgentReputation>,
}
