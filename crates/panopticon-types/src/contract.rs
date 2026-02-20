use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Payment conditions in a delegation contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentTerms {
    pub total_amount: f64,
    pub escrow_amount: f64,
    pub milestone_payments: Vec<MilestonePayment>,
    pub penalty_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MilestonePayment {
    pub milestone_id: String,
    pub amount: f64,
    pub paid: bool,
}

/// Monitoring conditions in a contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonitoringTerms {
    /// How often to check progress (in seconds).
    pub checkpoint_interval_secs: u64,
    /// Maximum allowed latency (in milliseconds).
    pub max_latency_ms: u64,
    /// Minimum quality score threshold.
    pub min_quality_score: f64,
    /// Maximum resource consumption.
    pub max_resource_budget: f64,
}

/// Dispute resolution configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisputeResolutionTerms {
    pub dispute_bond: f64,
    pub resolution_timeout_secs: u64,
    pub panel_size: u32,
    pub escalation_enabled: bool,
}

/// A delegation contract between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationContract {
    pub id: Uuid,
    pub task_id: Uuid,
    pub delegator_id: Uuid,
    pub delegatee_id: Uuid,
    pub payment: PaymentTerms,
    pub monitoring: MonitoringTerms,
    pub dispute_resolution: DisputeResolutionTerms,
    pub permitted_actions: Vec<String>,
    pub max_delegation_depth: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub signed_by_delegator: bool,
    pub signed_by_delegatee: bool,
}

impl DelegationContract {
    pub fn is_fully_signed(&self) -> bool {
        self.signed_by_delegator && self.signed_by_delegatee
    }
}

/// A link in the delegation chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationLink {
    pub from_agent_id: Uuid,
    pub to_agent_id: Uuid,
    pub contract_id: Uuid,
    pub task_id: Uuid,
    pub depth: u32,
    pub attestation: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}

/// Full delegation chain from original delegator to final executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationChain {
    pub links: Vec<DelegationLink>,
}

impl DelegationChain {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    pub fn depth(&self) -> u32 {
        self.links.len() as u32
    }

    pub fn add_link(&mut self, link: DelegationLink) {
        self.links.push(link);
    }

    /// Get the original delegator.
    pub fn origin(&self) -> Option<Uuid> {
        self.links.first().map(|l| l.from_agent_id)
    }

    /// Get the final delegatee.
    pub fn terminal(&self) -> Option<Uuid> {
        self.links.last().map(|l| l.to_agent_id)
    }
}

impl Default for DelegationChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delegation_chain() {
        let mut chain = DelegationChain::new();
        assert_eq!(chain.depth(), 0);
        assert!(chain.origin().is_none());

        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();
        let agent_c = Uuid::new_v4();

        chain.add_link(DelegationLink {
            from_agent_id: agent_a,
            to_agent_id: agent_b,
            contract_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            depth: 0,
            attestation: None,
            created_at: Utc::now(),
        });

        chain.add_link(DelegationLink {
            from_agent_id: agent_b,
            to_agent_id: agent_c,
            contract_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            depth: 1,
            attestation: None,
            created_at: Utc::now(),
        });

        assert_eq!(chain.depth(), 2);
        assert_eq!(chain.origin(), Some(agent_a));
        assert_eq!(chain.terminal(), Some(agent_c));
    }
}
