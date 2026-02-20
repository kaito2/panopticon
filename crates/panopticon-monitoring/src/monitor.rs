use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use panopticon_ledger::{Ledger, LedgerEntry, LedgerEntryKind};

use crate::checkpoint::Checkpoint;
use crate::slo::{SloChecker, SloViolation};

/// Events emitted by the monitoring loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringEvent {
    CheckpointReceived {
        task_id: Uuid,
        agent_id: Uuid,
        progress_pct: f64,
    },
    SloViolation {
        task_id: Uuid,
        agent_id: Uuid,
        violation: SloViolation,
    },
    AgentUnresponsive {
        agent_id: Uuid,
        last_seen: DateTime<Utc>,
    },
    TaskTimeout {
        task_id: Uuid,
    },
}

/// Configuration for the monitoring loop.
pub struct MonitoringConfig {
    /// How long an agent can go without a checkpoint before being considered unresponsive.
    pub heartbeat_timeout: std::time::Duration,
    /// How often to run the heartbeat check.
    pub heartbeat_check_interval: std::time::Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout: std::time::Duration::from_secs(60),
            heartbeat_check_interval: std::time::Duration::from_secs(10),
        }
    }
}

/// The monitoring loop that receives checkpoints, checks SLOs, records to the ledger,
/// and emits monitoring events.
pub struct MonitoringLoop {
    checkpoint_rx: mpsc::Receiver<Checkpoint>,
    event_tx: mpsc::Sender<MonitoringEvent>,
    shutdown_rx: watch::Receiver<bool>,
    slo_checker: SloChecker,
    ledger: Arc<dyn Ledger>,
    config: MonitoringConfig,
    /// Tracks last checkpoint time per agent.
    agent_heartbeats: HashMap<Uuid, DateTime<Utc>>,
}

impl MonitoringLoop {
    pub fn new(
        checkpoint_rx: mpsc::Receiver<Checkpoint>,
        event_tx: mpsc::Sender<MonitoringEvent>,
        shutdown_rx: watch::Receiver<bool>,
        slo_checker: SloChecker,
        ledger: Arc<dyn Ledger>,
        config: MonitoringConfig,
    ) -> Self {
        Self {
            checkpoint_rx,
            event_tx,
            shutdown_rx,
            slo_checker,
            ledger,
            config,
            agent_heartbeats: HashMap::new(),
        }
    }

    /// Run the monitoring loop until shutdown is signalled.
    pub async fn run(mut self) {
        let mut heartbeat_interval = tokio::time::interval(self.config.heartbeat_check_interval);

        loop {
            tokio::select! {
                Some(checkpoint) = self.checkpoint_rx.recv() => {
                    self.handle_checkpoint(checkpoint).await;
                }
                _ = heartbeat_interval.tick() => {
                    self.check_heartbeats().await;
                }
                Ok(()) = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        tracing::info!("Monitoring loop shutting down");
                        break;
                    }
                }
            }
        }
    }

    async fn handle_checkpoint(&mut self, checkpoint: Checkpoint) {
        // Update heartbeat
        self.agent_heartbeats
            .insert(checkpoint.agent_id, checkpoint.timestamp);

        // Emit checkpoint received event
        let _ = self
            .event_tx
            .send(MonitoringEvent::CheckpointReceived {
                task_id: checkpoint.task_id,
                agent_id: checkpoint.agent_id,
                progress_pct: checkpoint.progress_pct,
            })
            .await;

        // Check SLOs
        let violations = self.slo_checker.check(&checkpoint);
        for violation in violations {
            let _ = self
                .event_tx
                .send(MonitoringEvent::SloViolation {
                    task_id: checkpoint.task_id,
                    agent_id: checkpoint.agent_id,
                    violation,
                })
                .await;
        }

        // Record to ledger
        let payload = serde_json::to_value(&checkpoint).unwrap_or_default();
        let prev_hash = self.ledger.latest_hash().await.unwrap_or(None);
        let entry = LedgerEntry::new(
            LedgerEntryKind::CheckpointRecorded,
            checkpoint.agent_id,
            checkpoint.task_id,
            payload,
            prev_hash,
        );
        if let Err(e) = self.ledger.append(entry).await {
            tracing::error!("Failed to record checkpoint to ledger: {}", e);
        }
    }

    async fn check_heartbeats(&mut self) {
        let now = Utc::now();
        let timeout = chrono::Duration::from_std(self.config.heartbeat_timeout)
            .unwrap_or(chrono::Duration::seconds(60));

        let unresponsive: Vec<(Uuid, DateTime<Utc>)> = self
            .agent_heartbeats
            .iter()
            .filter(|(_, last_seen)| now - **last_seen > timeout)
            .map(|(id, ts)| (*id, *ts))
            .collect();

        for (agent_id, last_seen) in unresponsive {
            let _ = self
                .event_tx
                .send(MonitoringEvent::AgentUnresponsive {
                    agent_id,
                    last_seen,
                })
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slo::{Comparison, SloDefinition};
    use async_trait::async_trait;
    use panopticon_ledger::LedgerEntry;
    use panopticon_types::PanopticonError;
    use std::sync::Mutex;

    /// In-memory ledger for testing.
    struct MockLedger {
        entries: Mutex<Vec<LedgerEntry>>,
    }

    impl MockLedger {
        fn new() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl Ledger for MockLedger {
        async fn append(&self, entry: LedgerEntry) -> Result<(), PanopticonError> {
            self.entries.lock().unwrap().push(entry);
            Ok(())
        }

        async fn get(&self, id: Uuid) -> Result<Option<LedgerEntry>, PanopticonError> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .find(|e| e.id == id)
                .cloned())
        }

        async fn latest_hash(&self) -> Result<Option<String>, PanopticonError> {
            Ok(self.entries.lock().unwrap().last().map(|e| e.hash.clone()))
        }

        async fn query_by_subject(
            &self,
            subject_id: Uuid,
        ) -> Result<Vec<LedgerEntry>, PanopticonError> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.subject_id == subject_id)
                .cloned()
                .collect())
        }

        async fn query_by_kind(
            &self,
            kind: LedgerEntryKind,
        ) -> Result<Vec<LedgerEntry>, PanopticonError> {
            Ok(self
                .entries
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.kind == kind)
                .cloned()
                .collect())
        }

        async fn all_entries(&self) -> Result<Vec<LedgerEntry>, PanopticonError> {
            Ok(self.entries.lock().unwrap().clone())
        }

        async fn verify_integrity(&self) -> Result<bool, PanopticonError> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_monitoring_loop_receives_checkpoint() {
        let (cp_tx, cp_rx) = mpsc::channel(16);
        let (evt_tx, mut evt_rx) = mpsc::channel(16);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let slo_checker = SloChecker::new(vec![]);
        let ledger: Arc<dyn Ledger> = Arc::new(MockLedger::new());

        let monitor = MonitoringLoop::new(
            cp_rx,
            evt_tx,
            shutdown_rx,
            slo_checker,
            ledger,
            MonitoringConfig {
                heartbeat_timeout: std::time::Duration::from_secs(60),
                heartbeat_check_interval: std::time::Duration::from_secs(600),
            },
        );

        tokio::spawn(monitor.run());

        let task_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let cp = Checkpoint::new(task_id, agent_id).with_progress(0.5);
        cp_tx.send(cp).await.unwrap();

        let event = evt_rx.recv().await.unwrap();
        match event {
            MonitoringEvent::CheckpointReceived {
                task_id: tid,
                agent_id: aid,
                progress_pct,
            } => {
                assert_eq!(tid, task_id);
                assert_eq!(aid, agent_id);
                assert!((progress_pct - 0.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected CheckpointReceived event"),
        }

        let _ = shutdown_tx.send(true);
    }

    #[tokio::test]
    async fn test_monitoring_loop_detects_slo_violation() {
        let (cp_tx, cp_rx) = mpsc::channel(16);
        let (evt_tx, mut evt_rx) = mpsc::channel(16);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let slo_defs = vec![SloDefinition {
            metric_name: "resource_consumed".into(),
            threshold: 50.0,
            comparison: Comparison::LessThan,
            window_secs: 300,
        }];
        let slo_checker = SloChecker::new(slo_defs);
        let ledger: Arc<dyn Ledger> = Arc::new(MockLedger::new());

        let monitor = MonitoringLoop::new(
            cp_rx,
            evt_tx,
            shutdown_rx,
            slo_checker,
            ledger,
            MonitoringConfig {
                heartbeat_timeout: std::time::Duration::from_secs(60),
                heartbeat_check_interval: std::time::Duration::from_secs(600),
            },
        );

        tokio::spawn(monitor.run());

        let cp = Checkpoint::new(Uuid::new_v4(), Uuid::new_v4()).with_resource_consumed(100.0);
        cp_tx.send(cp).await.unwrap();

        // First event: CheckpointReceived
        let event1 = evt_rx.recv().await.unwrap();
        assert!(matches!(event1, MonitoringEvent::CheckpointReceived { .. }));

        // Second event: SloViolation
        let event2 = evt_rx.recv().await.unwrap();
        assert!(matches!(event2, MonitoringEvent::SloViolation { .. }));

        let _ = shutdown_tx.send(true);
    }

    #[tokio::test]
    async fn test_monitoring_loop_shutdown() {
        let (_cp_tx, cp_rx) = mpsc::channel::<Checkpoint>(16);
        let (evt_tx, _evt_rx) = mpsc::channel(16);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let slo_checker = SloChecker::new(vec![]);
        let ledger: Arc<dyn Ledger> = Arc::new(MockLedger::new());

        let monitor = MonitoringLoop::new(
            cp_rx,
            evt_tx,
            shutdown_rx,
            slo_checker,
            ledger,
            MonitoringConfig {
                heartbeat_timeout: std::time::Duration::from_secs(60),
                heartbeat_check_interval: std::time::Duration::from_secs(600),
            },
        );

        let handle = tokio::spawn(monitor.run());

        // Signal shutdown
        let _ = shutdown_tx.send(true);

        // The loop should exit
        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("Monitoring loop should shut down within timeout")
            .expect("Task should not panic");
    }
}
