use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request for Proposal — issued when a task needs an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFP {
    pub task_id: Uuid,
    pub required_capabilities: Vec<String>,
    pub max_cost: f64,
    pub deadline: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl RFP {
    pub fn new(task_id: Uuid, required_capabilities: Vec<String>, max_cost: f64) -> Self {
        Self {
            task_id,
            required_capabilities,
            max_cost,
            deadline: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }
}

/// A bid submitted by an agent in response to an RFP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub proposed_cost: f64,
    pub proposed_duration_secs: u64,
    pub confidence_score: f64,
    pub created_at: DateTime<Utc>,
}

impl Bid {
    pub fn new(
        agent_id: Uuid,
        task_id: Uuid,
        proposed_cost: f64,
        proposed_duration_secs: u64,
        confidence_score: f64,
    ) -> Self {
        Self {
            agent_id,
            task_id,
            proposed_cost,
            proposed_duration_secs,
            confidence_score,
            created_at: Utc::now(),
        }
    }
}

/// A scored bid produced by the evaluator.
#[derive(Debug, Clone)]
pub struct ScoredBid {
    pub bid: Bid,
    pub total_score: f64,
    pub cost_score: f64,
    pub quality_score: f64,
    pub confidence_component: f64,
}

/// Evaluates and ranks bids based on cost, predicted quality, and confidence.
pub struct BidEvaluator {
    /// Weight for cost component (lower cost = higher score).
    pub cost_weight: f64,
    /// Weight for predicted quality (from reputation).
    pub quality_weight: f64,
    /// Weight for the agent's self-reported confidence.
    pub confidence_weight: f64,
}

impl Default for BidEvaluator {
    fn default() -> Self {
        Self {
            cost_weight: 0.4,
            quality_weight: 0.4,
            confidence_weight: 0.2,
        }
    }
}

impl BidEvaluator {
    pub fn new(cost_weight: f64, quality_weight: f64, confidence_weight: f64) -> Self {
        Self {
            cost_weight,
            quality_weight,
            confidence_weight,
        }
    }

    /// Evaluate a set of bids given a max cost budget and per-agent quality predictions.
    ///
    /// `quality_predictor` maps agent_id to a predicted quality score in [0, 1].
    pub fn evaluate(
        &self,
        bids: &[Bid],
        max_cost: f64,
        quality_predictor: &dyn Fn(Uuid) -> f64,
    ) -> Vec<ScoredBid> {
        let mut scored: Vec<ScoredBid> = bids
            .iter()
            .filter(|b| b.proposed_cost <= max_cost)
            .map(|b| {
                let cost_score = if max_cost > 0.0 {
                    1.0 - (b.proposed_cost / max_cost)
                } else {
                    0.0
                };
                let quality_score = quality_predictor(b.agent_id);
                let confidence_component = b.confidence_score;
                let total_score = self.cost_weight * cost_score
                    + self.quality_weight * quality_score
                    + self.confidence_weight * confidence_component;

                ScoredBid {
                    bid: b.clone(),
                    total_score,
                    cost_score,
                    quality_score,
                    confidence_component,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfp_creation() {
        let task_id = Uuid::new_v4();
        let rfp = RFP::new(task_id, vec!["nlp".into()], 100.0);
        assert_eq!(rfp.task_id, task_id);
        assert_eq!(rfp.max_cost, 100.0);
        assert!(rfp.deadline.is_none());
    }

    #[test]
    fn test_bid_creation() {
        let bid = Bid::new(Uuid::new_v4(), Uuid::new_v4(), 50.0, 3600, 0.8);
        assert_eq!(bid.proposed_cost, 50.0);
        assert_eq!(bid.proposed_duration_secs, 3600);
    }

    #[test]
    fn test_bid_evaluation_ranking() {
        let evaluator = BidEvaluator::default();
        let task_id = Uuid::new_v4();
        let agent_cheap = Uuid::new_v4();
        let agent_expensive = Uuid::new_v4();

        let bids = vec![
            Bid::new(agent_expensive, task_id, 90.0, 3600, 0.9),
            Bid::new(agent_cheap, task_id, 30.0, 7200, 0.7),
        ];

        // Both agents have equal quality
        let scored = evaluator.evaluate(&bids, 100.0, &|_| 0.8);

        assert_eq!(scored.len(), 2);
        // Cheaper bid should score higher (cost_score is higher)
        assert_eq!(scored[0].bid.agent_id, agent_cheap);
    }

    #[test]
    fn test_bids_over_budget_excluded() {
        let evaluator = BidEvaluator::default();
        let task_id = Uuid::new_v4();

        let bids = vec![
            Bid::new(Uuid::new_v4(), task_id, 150.0, 3600, 0.9),
            Bid::new(Uuid::new_v4(), task_id, 50.0, 3600, 0.7),
        ];

        let scored = evaluator.evaluate(&bids, 100.0, &|_| 0.8);
        assert_eq!(scored.len(), 1);
        assert!(scored[0].bid.proposed_cost <= 100.0);
    }

    #[test]
    fn test_quality_prediction_affects_ranking() {
        let evaluator = BidEvaluator::new(0.2, 0.6, 0.2);
        let task_id = Uuid::new_v4();
        let agent_low_quality = Uuid::new_v4();
        let agent_high_quality = Uuid::new_v4();

        let bids = vec![
            Bid::new(agent_low_quality, task_id, 50.0, 3600, 0.8),
            Bid::new(agent_high_quality, task_id, 50.0, 3600, 0.8),
        ];

        let scored = evaluator.evaluate(&bids, 100.0, &|id| {
            if id == agent_high_quality { 0.95 } else { 0.3 }
        });

        assert_eq!(scored[0].bid.agent_id, agent_high_quality);
    }
}
