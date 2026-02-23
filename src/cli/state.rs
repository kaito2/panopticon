use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use uuid::Uuid;

use crate::ledger::{InMemoryLedger, Ledger, LedgerEntry};
use crate::persistence::{FileStore, PersistedState};
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

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
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

    /// Load state from a `FileStore`, populating ledger, reputation, tasks, and agents.
    pub async fn load_from(store: &FileStore) -> Result<Self> {
        let persisted = store.load()?;

        let ledger = Arc::new(InMemoryLedger::new());

        // Replay ledger entries to rebuild indices.
        for entry in &persisted.ledger_entries {
            ledger.append(entry.clone()).await.map_err(|e| anyhow::anyhow!("{e}"))?;
        }

        let reputation_engine = Arc::new(ReputationEngine::new(ledger.clone()));

        // Restore reputation scores.
        reputation_engine.load_scores(&persisted.reputation_scores);

        let tasks = DashMap::new();
        for (id, task) in persisted.tasks {
            tasks.insert(id, task);
        }

        let agents = DashMap::new();
        for (id, agent) in persisted.agents {
            agents.insert(id, agent);
        }

        Ok(Self {
            ledger,
            reputation_engine,
            tasks,
            agents,
        })
    }

    /// Dump current state into a `PersistedState` and save via `FileStore`.
    pub async fn save_to(&self, store: &FileStore) -> Result<()> {
        let ledger_entries: Vec<LedgerEntry> = self
            .ledger
            .all_entries()
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let reputation_scores = self.reputation_engine.dump_scores();

        let mut tasks = std::collections::HashMap::new();
        for entry in self.tasks.iter() {
            tasks.insert(*entry.key(), entry.value().clone());
        }

        let mut agents = std::collections::HashMap::new();
        for entry in self.agents.iter() {
            agents.insert(*entry.key(), entry.value().clone());
        }

        let persisted = PersistedState {
            tasks,
            agents,
            ledger_entries,
            reputation_scores,
        };

        store.save(&persisted)?;
        Ok(())
    }
}
