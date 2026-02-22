use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use crate::ledger::{InMemoryLedger, Ledger};
use crate::reputation::ReputationEngine;
use crate::types::{Agent, Task};

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
