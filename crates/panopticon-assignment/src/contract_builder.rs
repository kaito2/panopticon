use chrono::{DateTime, Utc};
use panopticon_types::{DelegationContract, DisputeResolutionTerms, MonitoringTerms, PaymentTerms};
use uuid::Uuid;

/// Errors specific to contract building.
#[derive(Debug, thiserror::Error)]
pub enum ContractBuildError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("invalid value: {0}")]
    InvalidValue(String),
}

/// Builder pattern for constructing a `DelegationContract`.
#[derive(Debug, Default)]
pub struct ContractBuilder {
    task_id: Option<Uuid>,
    delegator_id: Option<Uuid>,
    delegatee_id: Option<Uuid>,
    payment: Option<PaymentTerms>,
    monitoring: Option<MonitoringTerms>,
    dispute_resolution: Option<DisputeResolutionTerms>,
    permitted_actions: Vec<String>,
    max_delegation_depth: u32,
    expires_at: Option<DateTime<Utc>>,
}

impl ContractBuilder {
    pub fn new() -> Self {
        Self {
            max_delegation_depth: 1,
            ..Default::default()
        }
    }

    pub fn task_id(mut self, id: Uuid) -> Self {
        self.task_id = Some(id);
        self
    }

    pub fn delegator_id(mut self, id: Uuid) -> Self {
        self.delegator_id = Some(id);
        self
    }

    pub fn delegatee_id(mut self, id: Uuid) -> Self {
        self.delegatee_id = Some(id);
        self
    }

    pub fn payment_terms(mut self, terms: PaymentTerms) -> Self {
        self.payment = Some(terms);
        self
    }

    pub fn monitoring_terms(mut self, terms: MonitoringTerms) -> Self {
        self.monitoring = Some(terms);
        self
    }

    pub fn dispute_resolution_terms(mut self, terms: DisputeResolutionTerms) -> Self {
        self.dispute_resolution = Some(terms);
        self
    }

    pub fn permitted_actions(mut self, actions: Vec<String>) -> Self {
        self.permitted_actions = actions;
        self
    }

    pub fn max_delegation_depth(mut self, depth: u32) -> Self {
        self.max_delegation_depth = depth;
        self
    }

    pub fn expires_at(mut self, dt: DateTime<Utc>) -> Self {
        self.expires_at = Some(dt);
        self
    }

    /// Validate completeness and build the contract.
    pub fn build(self) -> Result<DelegationContract, ContractBuildError> {
        let task_id = self
            .task_id
            .ok_or(ContractBuildError::MissingField("task_id"))?;
        let delegator_id = self
            .delegator_id
            .ok_or(ContractBuildError::MissingField("delegator_id"))?;
        let delegatee_id = self
            .delegatee_id
            .ok_or(ContractBuildError::MissingField("delegatee_id"))?;
        let payment = self
            .payment
            .ok_or(ContractBuildError::MissingField("payment_terms"))?;
        let monitoring = self
            .monitoring
            .ok_or(ContractBuildError::MissingField("monitoring_terms"))?;
        let dispute_resolution = self
            .dispute_resolution
            .ok_or(ContractBuildError::MissingField("dispute_resolution_terms"))?;

        if payment.total_amount < 0.0 {
            return Err(ContractBuildError::InvalidValue(
                "total_amount must be non-negative".into(),
            ));
        }
        if delegator_id == delegatee_id {
            return Err(ContractBuildError::InvalidValue(
                "delegator and delegatee must be different agents".into(),
            ));
        }

        Ok(DelegationContract {
            id: Uuid::new_v4(),
            task_id,
            delegator_id,
            delegatee_id,
            payment,
            monitoring,
            dispute_resolution,
            permitted_actions: self.permitted_actions,
            max_delegation_depth: self.max_delegation_depth,
            created_at: Utc::now(),
            expires_at: self.expires_at,
            signed_by_delegator: false,
            signed_by_delegatee: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use panopticon_types::MilestonePayment;

    fn default_payment() -> PaymentTerms {
        PaymentTerms {
            total_amount: 100.0,
            escrow_amount: 50.0,
            milestone_payments: vec![MilestonePayment {
                milestone_id: "m1".into(),
                amount: 100.0,
                paid: false,
            }],
            penalty_rate: 0.1,
        }
    }

    fn default_monitoring() -> MonitoringTerms {
        MonitoringTerms {
            checkpoint_interval_secs: 60,
            max_latency_ms: 5000,
            min_quality_score: 0.7,
            max_resource_budget: 500.0,
        }
    }

    fn default_dispute() -> DisputeResolutionTerms {
        DisputeResolutionTerms {
            dispute_bond: 10.0,
            resolution_timeout_secs: 3600,
            panel_size: 3,
            escalation_enabled: true,
        }
    }

    #[test]
    fn test_build_complete_contract() {
        let delegator = Uuid::new_v4();
        let delegatee = Uuid::new_v4();
        let task_id = Uuid::new_v4();

        let contract = ContractBuilder::new()
            .task_id(task_id)
            .delegator_id(delegator)
            .delegatee_id(delegatee)
            .payment_terms(default_payment())
            .monitoring_terms(default_monitoring())
            .dispute_resolution_terms(default_dispute())
            .permitted_actions(vec!["read".into(), "write".into()])
            .max_delegation_depth(2)
            .build();

        let contract = contract.expect("should build successfully");
        assert_eq!(contract.task_id, task_id);
        assert_eq!(contract.delegator_id, delegator);
        assert_eq!(contract.delegatee_id, delegatee);
        assert!(!contract.signed_by_delegator);
        assert!(!contract.signed_by_delegatee);
    }

    #[test]
    fn test_missing_task_id() {
        let result = ContractBuilder::new()
            .delegator_id(Uuid::new_v4())
            .delegatee_id(Uuid::new_v4())
            .payment_terms(default_payment())
            .monitoring_terms(default_monitoring())
            .dispute_resolution_terms(default_dispute())
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("task_id"));
    }

    #[test]
    fn test_missing_payment_terms() {
        let result = ContractBuilder::new()
            .task_id(Uuid::new_v4())
            .delegator_id(Uuid::new_v4())
            .delegatee_id(Uuid::new_v4())
            .monitoring_terms(default_monitoring())
            .dispute_resolution_terms(default_dispute())
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("payment_terms"));
    }

    #[test]
    fn test_same_delegator_delegatee_rejected() {
        let same_id = Uuid::new_v4();
        let result = ContractBuilder::new()
            .task_id(Uuid::new_v4())
            .delegator_id(same_id)
            .delegatee_id(same_id)
            .payment_terms(default_payment())
            .monitoring_terms(default_monitoring())
            .dispute_resolution_terms(default_dispute())
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("different agents"));
    }

    #[test]
    fn test_negative_payment_rejected() {
        let mut payment = default_payment();
        payment.total_amount = -10.0;

        let result = ContractBuilder::new()
            .task_id(Uuid::new_v4())
            .delegator_id(Uuid::new_v4())
            .delegatee_id(Uuid::new_v4())
            .payment_terms(payment)
            .monitoring_terms(default_monitoring())
            .dispute_resolution_terms(default_dispute())
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-negative"));
    }
}
