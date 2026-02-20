use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use panopticon_types::{PanopticonError, PermissionSet, ReputationScore, Result};

/// State of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Normal operation — agent is allowed to execute tasks.
    Closed,
    /// Agent is blocked from executing tasks.
    Open,
    /// Agent is allowed a single probe task to demonstrate recovery.
    HalfOpen,
}

/// Circuit breaker for an individual agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub threshold: u32,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub cooldown_secs: i64,
    pub reputation_threshold: f64,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, cooldown_secs: i64, reputation_threshold: f64) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            threshold,
            last_failure_at: None,
            cooldown_secs,
            reputation_threshold,
        }
    }

    /// Record a failure. If the failure count exceeds the threshold, trip the breaker open.
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_at = Some(Utc::now());
        if self.failure_count >= self.threshold {
            self.state = CircuitBreakerState::Open;
        }
    }

    /// Check if the breaker should transition from Open to HalfOpen based on cooldown.
    pub fn check_cooldown(&mut self) {
        if self.state != CircuitBreakerState::Open {
            return;
        }
        if let Some(last_failure) = self.last_failure_at {
            let elapsed = (Utc::now() - last_failure).num_seconds();
            if elapsed >= self.cooldown_secs {
                self.state = CircuitBreakerState::HalfOpen;
            }
        }
    }

    /// Record a successful probe in HalfOpen state, closing the breaker.
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::HalfOpen => {
                self.state = CircuitBreakerState::Closed;
                self.failure_count = 0;
                self.last_failure_at = None;
            }
            CircuitBreakerState::Closed => {
                // Reset failure count on success in closed state.
                if self.failure_count > 0 {
                    self.failure_count = self.failure_count.saturating_sub(1);
                }
            }
            CircuitBreakerState::Open => {
                // No-op while open.
            }
        }
    }

    /// Check reputation and trip the breaker if composite score is below threshold.
    pub fn check_reputation(&mut self, reputation: &ReputationScore) {
        if reputation.composite() < self.reputation_threshold {
            self.state = CircuitBreakerState::Open;
            self.last_failure_at = Some(Utc::now());
        }
    }

    /// Check whether the agent is allowed to proceed.
    pub fn is_allowed(&self) -> bool {
        matches!(
            self.state,
            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen
        )
    }
}

/// Registry of circuit breakers for all agents.
pub struct CircuitBreakerRegistry {
    breakers: DashMap<Uuid, CircuitBreaker>,
    default_threshold: u32,
    default_cooldown_secs: i64,
    default_reputation_threshold: f64,
}

impl CircuitBreakerRegistry {
    pub fn new(
        default_threshold: u32,
        default_cooldown_secs: i64,
        default_reputation_threshold: f64,
    ) -> Self {
        Self {
            breakers: DashMap::new(),
            default_threshold,
            default_cooldown_secs,
            default_reputation_threshold,
        }
    }

    /// Get or create a circuit breaker for an agent.
    pub fn get_or_create(&self, agent_id: Uuid) -> CircuitBreaker {
        self.breakers
            .entry(agent_id)
            .or_insert_with(|| {
                CircuitBreaker::new(
                    self.default_threshold,
                    self.default_cooldown_secs,
                    self.default_reputation_threshold,
                )
            })
            .clone()
    }

    /// Record a failure for an agent and return the revoked permissions if the breaker opens.
    pub fn record_failure(&self, agent_id: Uuid) -> Option<CircuitBreakerState> {
        let mut entry = self.breakers.entry(agent_id).or_insert_with(|| {
            CircuitBreaker::new(
                self.default_threshold,
                self.default_cooldown_secs,
                self.default_reputation_threshold,
            )
        });
        let was_open = entry.state == CircuitBreakerState::Open;
        entry.record_failure();
        if !was_open && entry.state == CircuitBreakerState::Open {
            Some(CircuitBreakerState::Open)
        } else {
            None
        }
    }

    /// Record a success for an agent.
    pub fn record_success(&self, agent_id: Uuid) {
        if let Some(mut breaker) = self.breakers.get_mut(&agent_id) {
            breaker.record_success();
        }
    }

    /// Check reputation for an agent and trip the breaker if needed.
    /// Returns revoked permission set if the breaker trips.
    pub fn check_reputation(
        &self,
        agent_id: Uuid,
        reputation: &ReputationScore,
        permissions: &PermissionSet,
    ) -> Option<PermissionSet> {
        let mut entry = self.breakers.entry(agent_id).or_insert_with(|| {
            CircuitBreaker::new(
                self.default_threshold,
                self.default_cooldown_secs,
                self.default_reputation_threshold,
            )
        });
        let was_open = entry.state == CircuitBreakerState::Open;
        entry.check_reputation(reputation);
        if !was_open && entry.state == CircuitBreakerState::Open {
            // Return the revoked permissions.
            Some(permissions.clone())
        } else {
            None
        }
    }

    /// Check if an agent is allowed to proceed.
    pub fn check_agent(&self, agent_id: Uuid) -> Result<()> {
        if let Some(mut breaker) = self.breakers.get_mut(&agent_id) {
            breaker.check_cooldown();
            if !breaker.is_allowed() {
                return Err(PanopticonError::CircuitBreakerOpen(agent_id));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed_by_default() {
        let cb = CircuitBreaker::new(3, 60, 0.3);
        assert_eq!(cb.state, CircuitBreakerState::Closed);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_trips_on_threshold() {
        let mut cb = CircuitBreaker::new(3, 60, 0.3);
        cb.record_failure();
        assert_eq!(cb.state, CircuitBreakerState::Closed);
        cb.record_failure();
        assert_eq!(cb.state, CircuitBreakerState::Closed);
        cb.record_failure();
        assert_eq!(cb.state, CircuitBreakerState::Open);
        assert!(!cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_halfopen_after_cooldown() {
        let mut cb = CircuitBreaker::new(1, 0, 0.3);
        cb.record_failure();
        assert_eq!(cb.state, CircuitBreakerState::Open);
        // With cooldown_secs = 0, check_cooldown should immediately transition.
        cb.check_cooldown();
        assert_eq!(cb.state, CircuitBreakerState::HalfOpen);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_closes_on_success_from_halfopen() {
        let mut cb = CircuitBreaker::new(1, 0, 0.3);
        cb.record_failure();
        cb.check_cooldown();
        assert_eq!(cb.state, CircuitBreakerState::HalfOpen);
        cb.record_success();
        assert_eq!(cb.state, CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count, 0);
    }

    #[test]
    fn test_circuit_breaker_trips_on_reputation() {
        let mut cb = CircuitBreaker::new(10, 60, 0.3);
        let bad_reputation = ReputationScore {
            completion: 0.1,
            quality: 0.1,
            reliability: 0.1,
            safety: 0.1,
            behavioral: 0.1,
        };
        cb.check_reputation(&bad_reputation);
        assert_eq!(cb.state, CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_stays_closed_on_good_reputation() {
        let mut cb = CircuitBreaker::new(10, 60, 0.3);
        let good_reputation = ReputationScore {
            completion: 0.8,
            quality: 0.8,
            reliability: 0.8,
            safety: 0.8,
            behavioral: 0.8,
        };
        cb.check_reputation(&good_reputation);
        assert_eq!(cb.state, CircuitBreakerState::Closed);
    }

    #[test]
    fn test_registry_check_agent_blocks_when_open() {
        let registry = CircuitBreakerRegistry::new(1, 300, 0.3);
        let agent_id = Uuid::new_v4();
        registry.record_failure(agent_id);
        let result = registry.check_agent(agent_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_check_agent_allows_when_closed() {
        let registry = CircuitBreakerRegistry::new(3, 60, 0.3);
        let agent_id = Uuid::new_v4();
        let result = registry.check_agent(agent_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_registry_reputation_trip_returns_permissions() {
        let registry = CircuitBreakerRegistry::new(10, 60, 0.3);
        let agent_id = Uuid::new_v4();
        let bad_reputation = ReputationScore {
            completion: 0.1,
            quality: 0.1,
            reliability: 0.1,
            safety: 0.1,
            behavioral: 0.1,
        };
        let permissions = PermissionSet {
            allowed_actions: vec!["read".into(), "write".into()],
            max_delegation_depth: 2,
            max_cost_budget: 500.0,
            allowed_data_classifications: vec!["public".into()],
        };
        let revoked = registry.check_reputation(agent_id, &bad_reputation, &permissions);
        assert!(revoked.is_some());
        assert_eq!(revoked.unwrap(), permissions);
    }
}
