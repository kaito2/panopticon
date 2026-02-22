use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use crate::ledger::{Ledger, LedgerEntry, LedgerEntryKind};
use crate::types::{PanopticonError, ReputationScore, TrustLevel};

use super::score::{AgentReputation, ReputationObservation};

/// EMA-based multi-dimensional reputation engine.
pub struct ReputationEngine {
    scores: DashMap<Uuid, AgentReputation>,
    ledger: Arc<dyn Ledger>,
}

impl ReputationEngine {
    pub fn new(ledger: Arc<dyn Ledger>) -> Self {
        Self {
            scores: DashMap::new(),
            ledger,
        }
    }

    /// Adaptive learning rate: agents with fewer observations have a higher alpha.
    /// `alpha = 1.0 / (1.0 + completed_tasks.sqrt())`
    fn adaptive_alpha(completed_tasks: u64) -> f64 {
        1.0 / (1.0 + (completed_tasks as f64).sqrt())
    }

    /// Apply a single observation using EMA:
    /// `new_score = alpha * observation + (1 - alpha) * old_score`
    /// The result is clamped to [0, 1].
    pub async fn update_reputation(
        &self,
        observation: ReputationObservation,
    ) -> Result<ReputationScore, PanopticonError> {
        let value = observation.value.clamp(0.0, 1.0);
        let agent_id = observation.agent_id;

        // Ensure an entry exists.
        if !self.scores.contains_key(&agent_id) {
            self.scores.insert(agent_id, AgentReputation::new(agent_id));
        }

        let new_score = {
            let mut rep = self
                .scores
                .get_mut(&agent_id)
                .expect("entry was just inserted");
            let alpha = Self::adaptive_alpha(rep.total_tasks);
            let dim = rep.dimension_mut(observation.dimension);
            dim.score = (alpha * value + (1.0 - alpha) * dim.score).clamp(0.0, 1.0);
            dim.observations += 1;
            dim.last_updated = observation.timestamp;
            rep.total_tasks += 1;
            rep.to_reputation_score()
        };

        // Record the update on the ledger.
        let previous_hash = self
            .ledger
            .latest_hash()
            .await
            .map_err(|e| PanopticonError::LedgerError(e.to_string()))?;

        let payload = serde_json::json!({
            "dimension": observation.dimension,
            "observed_value": observation.value,
            "new_score": new_score,
        });

        let entry = LedgerEntry::new(
            LedgerEntryKind::ReputationUpdated,
            agent_id,
            observation.task_id,
            payload,
            previous_hash,
        );

        self.ledger
            .append(entry)
            .await
            .map_err(|e| PanopticonError::LedgerError(e.to_string()))?;

        Ok(new_score)
    }

    /// Retrieve the current reputation score for an agent.
    pub fn get_reputation(&self, agent_id: Uuid) -> Option<ReputationScore> {
        self.scores
            .get(&agent_id)
            .map(|rep| rep.to_reputation_score())
    }

    /// Compute the weighted composite score for an agent.
    pub fn get_composite_score(&self, agent_id: Uuid) -> Option<f64> {
        self.get_reputation(agent_id).map(|score| score.composite())
    }

    /// Map a composite reputation score to a `TrustLevel`.
    pub fn compute_trust_level(composite: f64) -> TrustLevel {
        if composite < 0.2 {
            TrustLevel::Untrusted
        } else if composite < 0.4 {
            TrustLevel::Low
        } else if composite < 0.6 {
            TrustLevel::Medium
        } else if composite < 0.8 {
            TrustLevel::High
        } else {
            TrustLevel::Full
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reputation::score::ReputationDimension;
    use chrono::Utc;
    use crate::ledger::{LedgerEntry, LedgerEntryKind};
    use std::sync::Mutex;

    /// A trivial in-memory ledger for testing (no feature gate needed).
    struct FakeLedger {
        entries: Mutex<Vec<LedgerEntry>>,
    }

    impl FakeLedger {
        fn new() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl Ledger for FakeLedger {
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

    fn make_observation(
        agent_id: Uuid,
        task_id: Uuid,
        dimension: ReputationDimension,
        value: f64,
    ) -> ReputationObservation {
        ReputationObservation {
            agent_id,
            task_id,
            dimension,
            value,
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_ema_update_correctness() {
        let ledger = Arc::new(FakeLedger::new());
        let engine = ReputationEngine::new(ledger);
        let agent = Uuid::new_v4();
        let task = Uuid::new_v4();

        // First observation: alpha = 1/(1+sqrt(0)) = 1.0
        // new_score = 1.0 * 0.9 + 0.0 * 0.5 = 0.9
        let obs = make_observation(agent, task, ReputationDimension::Quality, 0.9);
        let score = engine.update_reputation(obs).await.unwrap();
        assert!(
            (score.quality - 0.9).abs() < 1e-9,
            "First observation with alpha=1.0 should set score directly, got {}",
            score.quality,
        );

        // Second observation: alpha = 1/(1+sqrt(1)) = 0.5
        // new_score = 0.5 * 0.7 + 0.5 * 0.9 = 0.8
        let obs2 = make_observation(agent, task, ReputationDimension::Quality, 0.7);
        let score2 = engine.update_reputation(obs2).await.unwrap();
        assert!(
            (score2.quality - 0.8).abs() < 1e-9,
            "Second observation with alpha=0.5 should give 0.8, got {}",
            score2.quality,
        );
    }

    #[tokio::test]
    async fn test_adaptive_learning_rate() {
        // alpha = 1/(1+sqrt(n))
        assert!((ReputationEngine::adaptive_alpha(0) - 1.0).abs() < 1e-9);
        assert!((ReputationEngine::adaptive_alpha(1) - 0.5).abs() < 1e-9);
        assert!((ReputationEngine::adaptive_alpha(4) - (1.0 / 3.0)).abs() < 1e-9);
        assert!((ReputationEngine::adaptive_alpha(9) - 0.25).abs() < 1e-9);

        // Verify that alpha decreases monotonically as tasks increase.
        let mut prev_alpha = 1.0;
        for n in 1..100 {
            let alpha = ReputationEngine::adaptive_alpha(n);
            assert!(alpha < prev_alpha, "alpha should decrease with more tasks");
            prev_alpha = alpha;
        }
    }

    #[tokio::test]
    async fn test_trust_level_thresholds() {
        assert_eq!(
            ReputationEngine::compute_trust_level(0.0),
            TrustLevel::Untrusted,
        );
        assert_eq!(
            ReputationEngine::compute_trust_level(0.19),
            TrustLevel::Untrusted,
        );
        assert_eq!(ReputationEngine::compute_trust_level(0.2), TrustLevel::Low);
        assert_eq!(ReputationEngine::compute_trust_level(0.39), TrustLevel::Low,);
        assert_eq!(
            ReputationEngine::compute_trust_level(0.4),
            TrustLevel::Medium,
        );
        assert_eq!(
            ReputationEngine::compute_trust_level(0.59),
            TrustLevel::Medium,
        );
        assert_eq!(ReputationEngine::compute_trust_level(0.6), TrustLevel::High,);
        assert_eq!(
            ReputationEngine::compute_trust_level(0.79),
            TrustLevel::High,
        );
        assert_eq!(ReputationEngine::compute_trust_level(0.8), TrustLevel::Full,);
        assert_eq!(ReputationEngine::compute_trust_level(1.0), TrustLevel::Full,);
    }

    #[tokio::test]
    async fn test_composite_score_calculation() {
        let ledger = Arc::new(FakeLedger::new());
        let engine = ReputationEngine::new(ledger);
        let agent = Uuid::new_v4();
        let task = Uuid::new_v4();

        // Set each dimension to a distinct value via first-observation (alpha=1.0 each time,
        // but total_tasks increments so alpha differs per subsequent call).
        // We manually compute the expected values with EMA.

        // Insert with known values: use a fresh agent for a clean test.
        // Because total_tasks increments across dimensions, we set up a scenario where we
        // know the exact scores by using a single observation per dimension.
        // Observation 0: completion=0.8, alpha=1/(1+0)=1.0 => score=0.8, total=1
        // Observation 1: quality=0.7,    alpha=1/(1+1)=0.5 => score=0.5*0.7+0.5*0.5=0.6, total=2
        // Observation 2: reliability=0.9, alpha=1/(1+sqrt(2))~0.414 => 0.414*0.9+0.586*0.5~0.666, total=3
        // Observation 3: safety=1.0,     alpha=1/(1+sqrt(3))~0.366 => 0.366*1.0+0.634*0.5~0.683, total=4
        // Observation 4: behavioral=0.6, alpha=1/(1+2)=1/3~0.333 => 0.333*0.6+0.667*0.5~0.533, total=5

        let dims_and_values = [
            (ReputationDimension::Completion, 0.8),
            (ReputationDimension::Quality, 0.7),
            (ReputationDimension::Reliability, 0.9),
            (ReputationDimension::Safety, 1.0),
            (ReputationDimension::Behavioral, 0.6),
        ];

        let mut expected_scores = [0.5_f64; 5]; // default scores
        let mut total_tasks = 0u64;

        for (i, &(dim, value)) in dims_and_values.iter().enumerate() {
            let alpha = 1.0 / (1.0 + (total_tasks as f64).sqrt());
            expected_scores[i] = alpha * value + (1.0 - alpha) * expected_scores[i];
            total_tasks += 1;

            let obs = make_observation(agent, task, dim, value);
            engine.update_reputation(obs).await.unwrap();
        }

        let composite = engine.get_composite_score(agent).unwrap();
        let expected_composite = expected_scores[0] * 0.4
            + expected_scores[1] * 0.3
            + expected_scores[2] * 0.15
            + expected_scores[3] * 0.1
            + expected_scores[4] * 0.05;

        assert!(
            (composite - expected_composite).abs() < 1e-9,
            "Composite should be {}, got {}",
            expected_composite,
            composite,
        );
    }

    #[tokio::test]
    async fn test_score_bounds() {
        let ledger = Arc::new(FakeLedger::new());
        let engine = ReputationEngine::new(ledger);
        let agent = Uuid::new_v4();
        let task = Uuid::new_v4();

        // Observation with a value above 1.0 should be clamped.
        let obs = make_observation(agent, task, ReputationDimension::Safety, 1.5);
        let score = engine.update_reputation(obs).await.unwrap();
        assert!(
            score.safety <= 1.0,
            "Score should be clamped to at most 1.0, got {}",
            score.safety,
        );
        assert!(
            score.safety >= 0.0,
            "Score should be non-negative, got {}",
            score.safety,
        );

        // Observation with a value below 0.0 should be clamped.
        let obs2 = make_observation(agent, task, ReputationDimension::Safety, -0.5);
        let score2 = engine.update_reputation(obs2).await.unwrap();
        assert!(
            score2.safety >= 0.0,
            "Score should be non-negative after negative observation, got {}",
            score2.safety,
        );
        assert!(
            score2.safety <= 1.0,
            "Score should be at most 1.0, got {}",
            score2.safety,
        );
    }

    #[tokio::test]
    async fn test_ledger_entries_recorded() {
        let ledger: Arc<dyn Ledger> = Arc::new(FakeLedger::new());
        let engine = ReputationEngine::new(ledger.clone());
        let agent = Uuid::new_v4();
        let task = Uuid::new_v4();

        let obs = make_observation(agent, task, ReputationDimension::Completion, 0.8);
        engine.update_reputation(obs).await.unwrap();

        let entries = ledger.all_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, LedgerEntryKind::ReputationUpdated);
        assert_eq!(entries[0].actor_id, agent);
        assert_eq!(entries[0].subject_id, task);
    }

    #[tokio::test]
    async fn test_get_reputation_unknown_agent() {
        let ledger = Arc::new(FakeLedger::new());
        let engine = ReputationEngine::new(ledger);
        assert!(engine.get_reputation(Uuid::new_v4()).is_none());
    }
}
