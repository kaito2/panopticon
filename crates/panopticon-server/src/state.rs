use std::sync::Arc;

use panopticon_ledger::{InMemoryLedger, Ledger};
use panopticon_reputation::ReputationEngine;

use dashmap::DashMap;
use uuid::Uuid;

use panopticon_types::{Agent, Task};

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub ledger: Arc<dyn Ledger>,
    pub reputation_engine: Arc<ReputationEngine>,
    pub tasks: Arc<DashMap<Uuid, Task>>,
    pub agents: Arc<DashMap<Uuid, Agent>>,
}

impl AppState {
    pub fn new() -> Self {
        let ledger = Arc::new(InMemoryLedger::new());
        let reputation_engine = Arc::new(ReputationEngine::new(ledger.clone()));

        Self {
            ledger,
            reputation_engine,
            tasks: Arc::new(DashMap::new()),
            agents: Arc::new(DashMap::new()),
        }
    }
}
