use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trust level derived from reputation and context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    Untrusted,
    Low,
    Medium,
    High,
    Full,
}

/// Multi-dimensional reputation score.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReputationScore {
    pub completion: f64,
    pub quality: f64,
    pub reliability: f64,
    pub safety: f64,
    pub behavioral: f64,
}

impl Default for ReputationScore {
    fn default() -> Self {
        Self {
            completion: 0.5,
            quality: 0.5,
            reliability: 0.5,
            safety: 0.5,
            behavioral: 0.5,
        }
    }
}

impl ReputationScore {
    /// Weighted composite score from the paper.
    /// completion(0.4) + quality(0.3) + reliability(0.15) + safety(0.1) + behavioral(0.05)
    pub fn composite(&self) -> f64 {
        self.completion * 0.4
            + self.quality * 0.3
            + self.reliability * 0.15
            + self.safety * 0.1
            + self.behavioral * 0.05
    }
}

/// Permission set for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionSet {
    pub allowed_actions: Vec<String>,
    pub max_delegation_depth: u32,
    pub max_cost_budget: f64,
    pub allowed_data_classifications: Vec<String>,
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self {
            allowed_actions: Vec::new(),
            max_delegation_depth: 1,
            max_cost_budget: 100.0,
            allowed_data_classifications: Vec::new(),
        }
    }
}

impl PermissionSet {
    /// Check if this permission set is a subset of another (for privilege attenuation).
    pub fn is_subset_of(&self, parent: &PermissionSet) -> bool {
        self.max_delegation_depth <= parent.max_delegation_depth
            && self.max_cost_budget <= parent.max_cost_budget
            && self
                .allowed_actions
                .iter()
                .all(|a| parent.allowed_actions.contains(a))
            && self
                .allowed_data_classifications
                .iter()
                .all(|d| parent.allowed_data_classifications.contains(d))
    }
}

/// Capability registry for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityRegistry {
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capability {
    pub name: String,
    pub proficiency: f64,
    pub certified: bool,
    pub last_verified: Option<DateTime<Utc>>,
}

/// An AI agent that can participate in delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub capabilities: CapabilityRegistry,
    pub reputation: ReputationScore,
    pub trust_level: TrustLevel,
    pub permissions: PermissionSet,
    pub available: bool,
    pub current_load: f64,
    pub max_concurrent_tasks: u32,
    pub active_task_ids: Vec<Uuid>,
    pub registered_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

impl Agent {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            capabilities: CapabilityRegistry {
                capabilities: Vec::new(),
            },
            reputation: ReputationScore::default(),
            trust_level: TrustLevel::Low,
            permissions: PermissionSet::default(),
            available: true,
            current_load: 0.0,
            max_concurrent_tasks: 3,
            active_task_ids: Vec::new(),
            registered_at: now,
            last_active_at: now,
        }
    }

    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities
            .capabilities
            .iter()
            .any(|c| c.name == name)
    }

    pub fn capability_proficiency(&self, name: &str) -> f64 {
        self.capabilities
            .capabilities
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.proficiency)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_composite() {
        let score = ReputationScore {
            completion: 1.0,
            quality: 1.0,
            reliability: 1.0,
            safety: 1.0,
            behavioral: 1.0,
        };
        let composite = score.composite();
        assert!((composite - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reputation_weighted() {
        let score = ReputationScore {
            completion: 0.8,
            quality: 0.7,
            reliability: 0.6,
            safety: 0.9,
            behavioral: 0.5,
        };
        let expected = 0.8 * 0.4 + 0.7 * 0.3 + 0.6 * 0.15 + 0.9 * 0.1 + 0.5 * 0.05;
        assert!((score.composite() - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_permission_subset() {
        let parent = PermissionSet {
            allowed_actions: vec!["read".into(), "write".into(), "execute".into()],
            max_delegation_depth: 3,
            max_cost_budget: 1000.0,
            allowed_data_classifications: vec!["public".into(), "internal".into()],
        };
        let child = PermissionSet {
            allowed_actions: vec!["read".into()],
            max_delegation_depth: 1,
            max_cost_budget: 100.0,
            allowed_data_classifications: vec!["public".into()],
        };
        assert!(child.is_subset_of(&parent));
        assert!(!parent.is_subset_of(&child));
    }

    #[test]
    fn test_agent_capabilities() {
        let mut agent = Agent::new("test-agent");
        agent.capabilities.capabilities.push(Capability {
            name: "nlp".into(),
            proficiency: 0.9,
            certified: true,
            last_verified: Some(Utc::now()),
        });
        assert!(agent.has_capability("nlp"));
        assert!(!agent.has_capability("vision"));
        assert!((agent.capability_proficiency("nlp") - 0.9).abs() < f64::EPSILON);
    }
}
