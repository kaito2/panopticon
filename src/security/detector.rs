use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::types::Result;

use super::threat::{ThreatAlert, ThreatCategory, ThreatSeverity};

/// Context provided to threat detectors for analysis.
#[derive(Debug, Clone)]
pub struct ThreatContext {
    pub agent_id: Uuid,
    pub action_description: String,
    pub resource_access_patterns: Vec<ResourceAccess>,
    pub bid_patterns: Vec<BidRecord>,
    pub registered_at: Option<DateTime<Utc>>,
    pub capabilities: Vec<String>,
}

/// A record of a resource access by an agent.
#[derive(Debug, Clone)]
pub struct ResourceAccess {
    pub resource_name: String,
    pub access_type: String,
    pub timestamp: DateTime<Utc>,
}

/// A record of a bid placed by an agent.
#[derive(Debug, Clone)]
pub struct BidRecord {
    pub task_id: Uuid,
    pub agent_id: Uuid,
    pub bid_amount: f64,
    pub timestamp: DateTime<Utc>,
}

/// Trait for threat detection strategies.
#[async_trait]
pub trait ThreatDetector: Send + Sync {
    async fn detect(&self, context: &ThreatContext) -> Result<Vec<ThreatAlert>>;
}

/// Detects Sybil attacks by identifying clusters of agents with similar
/// capabilities registered at similar times.
pub struct SybilDetector {
    /// Time window in seconds within which registrations are considered "similar".
    pub registration_window_secs: i64,
    /// Minimum capability overlap ratio to flag as suspicious.
    pub capability_overlap_threshold: f64,
    /// Known agent registrations for comparison.
    pub known_agents: Vec<AgentRecord>,
}

/// A summary record of an agent used by detectors.
#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub id: Uuid,
    pub capabilities: Vec<String>,
    pub registered_at: DateTime<Utc>,
}

impl SybilDetector {
    pub fn new(registration_window_secs: i64, capability_overlap_threshold: f64) -> Self {
        Self {
            registration_window_secs,
            capability_overlap_threshold,
            known_agents: Vec::new(),
        }
    }

    fn capability_overlap(&self, a: &[String], b: &[String]) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 0.0;
        }
        let intersection = a.iter().filter(|cap| b.contains(cap)).count();
        let union = {
            let mut all: Vec<&String> = a.iter().chain(b.iter()).collect();
            all.sort();
            all.dedup();
            all.len()
        };
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }
}

#[async_trait]
impl ThreatDetector for SybilDetector {
    async fn detect(&self, context: &ThreatContext) -> Result<Vec<ThreatAlert>> {
        let mut alerts = Vec::new();
        let registered_at = match context.registered_at {
            Some(t) => t,
            None => return Ok(alerts),
        };

        for agent in &self.known_agents {
            if agent.id == context.agent_id {
                continue;
            }
            let time_diff = (registered_at - agent.registered_at).num_seconds().abs();
            if time_diff > self.registration_window_secs {
                continue;
            }
            let overlap = self.capability_overlap(&context.capabilities, &agent.capabilities);
            if overlap >= self.capability_overlap_threshold {
                alerts.push(ThreatAlert::new(
                    ThreatCategory::SybilAttack,
                    ThreatSeverity::High,
                    context.agent_id,
                    format!(
                        "Agent registered within {}s of agent {} with {:.0}% capability overlap",
                        time_diff,
                        agent.id,
                        overlap * 100.0,
                    ),
                ));
            }
        }

        Ok(alerts)
    }
}

/// Detects collusion by identifying agents that consistently bid in coordination.
pub struct CollusionDetector {
    /// Minimum number of co-occurring bids to flag as suspicious.
    pub min_co_bid_count: usize,
    /// Time window in seconds within which bids on the same task are considered coordinated.
    pub bid_window_secs: i64,
}

impl CollusionDetector {
    pub fn new(min_co_bid_count: usize, bid_window_secs: i64) -> Self {
        Self {
            min_co_bid_count,
            bid_window_secs,
        }
    }
}

#[async_trait]
impl ThreatDetector for CollusionDetector {
    async fn detect(&self, context: &ThreatContext) -> Result<Vec<ThreatAlert>> {
        let mut alerts = Vec::new();

        // Group bid patterns by task to find co-bidders.
        let agent_bids: Vec<&BidRecord> = context
            .bid_patterns
            .iter()
            .filter(|b| b.agent_id == context.agent_id)
            .collect();

        let other_bids: Vec<&BidRecord> = context
            .bid_patterns
            .iter()
            .filter(|b| b.agent_id != context.agent_id)
            .collect();

        // Count how many times each other agent bids on the same task within the window.
        let mut co_bid_counts: std::collections::HashMap<Uuid, usize> =
            std::collections::HashMap::new();

        for my_bid in &agent_bids {
            for other_bid in &other_bids {
                if my_bid.task_id != other_bid.task_id {
                    continue;
                }
                let time_diff = (my_bid.timestamp - other_bid.timestamp).num_seconds().abs();
                if time_diff <= self.bid_window_secs {
                    *co_bid_counts.entry(other_bid.agent_id).or_insert(0) += 1;
                }
            }
        }

        for (other_agent_id, count) in co_bid_counts {
            if count >= self.min_co_bid_count {
                alerts.push(ThreatAlert::new(
                    ThreatCategory::Collusion,
                    ThreatSeverity::Medium,
                    context.agent_id,
                    format!(
                        "Agent co-bid with agent {} on {} tasks within {}s windows",
                        other_agent_id, count, self.bid_window_secs,
                    ),
                ));
            }
        }

        Ok(alerts)
    }
}

/// Detects anomalous agent behavior such as sudden capability claims
/// or unusual resource access patterns.
pub struct BehavioralDetector {
    /// Maximum number of new capabilities an agent can claim at once.
    pub max_new_capabilities: usize,
    /// Maximum number of distinct resources accessed in a short period.
    pub max_resource_accesses: usize,
    /// Time window in seconds for resource access analysis.
    pub resource_window_secs: i64,
}

impl BehavioralDetector {
    pub fn new(
        max_new_capabilities: usize,
        max_resource_accesses: usize,
        resource_window_secs: i64,
    ) -> Self {
        Self {
            max_new_capabilities,
            max_resource_accesses,
            resource_window_secs,
        }
    }
}

#[async_trait]
impl ThreatDetector for BehavioralDetector {
    async fn detect(&self, context: &ThreatContext) -> Result<Vec<ThreatAlert>> {
        let mut alerts = Vec::new();

        // Check for sudden capability claims.
        if context.capabilities.len() > self.max_new_capabilities {
            alerts.push(ThreatAlert::new(
                ThreatCategory::VulnerabilityProbe,
                ThreatSeverity::Medium,
                context.agent_id,
                format!(
                    "Agent claims {} capabilities, exceeding threshold of {}",
                    context.capabilities.len(),
                    self.max_new_capabilities,
                ),
            ));
        }

        // Check for unusual resource access patterns within time window.
        let now = Utc::now();
        let recent_accesses: Vec<&ResourceAccess> = context
            .resource_access_patterns
            .iter()
            .filter(|a| (now - a.timestamp).num_seconds().abs() <= self.resource_window_secs)
            .collect();

        let distinct_resources: std::collections::HashSet<&str> = recent_accesses
            .iter()
            .map(|a| a.resource_name.as_str())
            .collect();

        if distinct_resources.len() > self.max_resource_accesses {
            alerts.push(ThreatAlert::new(
                ThreatCategory::DataExfiltration,
                ThreatSeverity::High,
                context.agent_id,
                format!(
                    "Agent accessed {} distinct resources in {}s, exceeding threshold of {}",
                    distinct_resources.len(),
                    self.resource_window_secs,
                    self.max_resource_accesses,
                ),
            ));
        }

        Ok(alerts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_sybil_detector_flags_similar_agents() {
        let now = Utc::now();
        let target_id = Uuid::new_v4();
        let similar_id = Uuid::new_v4();

        let mut detector = SybilDetector::new(60, 0.5);
        detector.known_agents.push(AgentRecord {
            id: similar_id,
            capabilities: vec!["nlp".into(), "vision".into(), "reasoning".into()],
            registered_at: now - Duration::seconds(30),
        });

        let context = ThreatContext {
            agent_id: target_id,
            action_description: "register".into(),
            resource_access_patterns: vec![],
            bid_patterns: vec![],
            registered_at: Some(now),
            capabilities: vec!["nlp".into(), "vision".into()],
        };

        let alerts = detector.detect(&context).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].category, ThreatCategory::SybilAttack);
    }

    #[tokio::test]
    async fn test_sybil_detector_ignores_distant_registration() {
        let now = Utc::now();
        let target_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();

        let mut detector = SybilDetector::new(60, 0.5);
        detector.known_agents.push(AgentRecord {
            id: other_id,
            capabilities: vec!["nlp".into(), "vision".into()],
            registered_at: now - Duration::seconds(3600),
        });

        let context = ThreatContext {
            agent_id: target_id,
            action_description: "register".into(),
            resource_access_patterns: vec![],
            bid_patterns: vec![],
            registered_at: Some(now),
            capabilities: vec!["nlp".into(), "vision".into()],
        };

        let alerts = detector.detect(&context).await.unwrap();
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_collusion_detector_flags_coordinated_bids() {
        let now = Utc::now();
        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();
        let task1 = Uuid::new_v4();
        let task2 = Uuid::new_v4();
        let task3 = Uuid::new_v4();

        let bids = vec![
            BidRecord {
                task_id: task1,
                agent_id: agent_a,
                bid_amount: 10.0,
                timestamp: now,
            },
            BidRecord {
                task_id: task1,
                agent_id: agent_b,
                bid_amount: 12.0,
                timestamp: now + Duration::seconds(5),
            },
            BidRecord {
                task_id: task2,
                agent_id: agent_a,
                bid_amount: 20.0,
                timestamp: now,
            },
            BidRecord {
                task_id: task2,
                agent_id: agent_b,
                bid_amount: 22.0,
                timestamp: now + Duration::seconds(3),
            },
            BidRecord {
                task_id: task3,
                agent_id: agent_a,
                bid_amount: 15.0,
                timestamp: now,
            },
            BidRecord {
                task_id: task3,
                agent_id: agent_b,
                bid_amount: 17.0,
                timestamp: now + Duration::seconds(2),
            },
        ];

        let detector = CollusionDetector::new(3, 10);
        let context = ThreatContext {
            agent_id: agent_a,
            action_description: "bid".into(),
            resource_access_patterns: vec![],
            bid_patterns: bids,
            registered_at: None,
            capabilities: vec![],
        };

        let alerts = detector.detect(&context).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].category, ThreatCategory::Collusion);
    }

    #[tokio::test]
    async fn test_behavioral_detector_flags_excessive_capabilities() {
        let detector = BehavioralDetector::new(3, 10, 300);

        let context = ThreatContext {
            agent_id: Uuid::new_v4(),
            action_description: "register".into(),
            resource_access_patterns: vec![],
            bid_patterns: vec![],
            registered_at: None,
            capabilities: vec![
                "nlp".into(),
                "vision".into(),
                "reasoning".into(),
                "coding".into(),
                "planning".into(),
            ],
        };

        let alerts = detector.detect(&context).await.unwrap();
        assert!(!alerts.is_empty());
        assert!(
            alerts
                .iter()
                .any(|a| a.category == ThreatCategory::VulnerabilityProbe)
        );
    }

    #[tokio::test]
    async fn test_behavioral_detector_flags_resource_access() {
        let now = Utc::now();
        let detector = BehavioralDetector::new(100, 2, 300);

        let accesses = (0..5)
            .map(|i| ResourceAccess {
                resource_name: format!("resource_{}", i),
                access_type: "read".into(),
                timestamp: now - Duration::seconds(10),
            })
            .collect();

        let context = ThreatContext {
            agent_id: Uuid::new_v4(),
            action_description: "access".into(),
            resource_access_patterns: accesses,
            bid_patterns: vec![],
            registered_at: None,
            capabilities: vec![],
        };

        let alerts = detector.detect(&context).await.unwrap();
        assert!(!alerts.is_empty());
        assert!(
            alerts
                .iter()
                .any(|a| a.category == ThreatCategory::DataExfiltration)
        );
    }
}
