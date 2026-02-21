use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use panopticon_ledger::{InMemoryLedger, Ledger};
use panopticon_reputation::ReputationEngine;
use panopticon_types::{Agent, Task};

/// Shared application state for CLI commands.
#[allow(dead_code)]
pub struct AppState {
    pub ledger: Arc<dyn Ledger>,
    pub reputation_engine: Arc<ReputationEngine>,
    pub tasks: DashMap<Uuid, Task>,
    pub agents: DashMap<Uuid, Agent>,
}

impl AppState {
    pub fn new() -> Self {
        let ledger = Arc::new(InMemoryLedger::new());
        let reputation_engine = Arc::new(ReputationEngine::new(ledger.clone()));

        Self {
            ledger,
            reputation_engine,
            tasks: DashMap::new(),
            agents: DashMap::new(),
        }
    }
}
