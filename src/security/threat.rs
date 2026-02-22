use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Categories of threats in the delegation system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreatCategory {
    DataExfiltration,
    DataPoisoning,
    PromptInjection,
    HarmfulTask,
    VulnerabilityProbe,
    SybilAttack,
    Collusion,
}

/// Severity level of a detected threat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// An alert generated when a threat is detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAlert {
    pub id: Uuid,
    pub category: ThreatCategory,
    pub severity: ThreatSeverity,
    pub source_agent_id: Uuid,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl ThreatAlert {
    pub fn new(
        category: ThreatCategory,
        severity: ThreatSeverity,
        source_agent_id: Uuid,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            category,
            severity,
            source_agent_id,
            description: description.into(),
            detected_at: Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_severity_ordering() {
        assert!(ThreatSeverity::Low < ThreatSeverity::Medium);
        assert!(ThreatSeverity::Medium < ThreatSeverity::High);
        assert!(ThreatSeverity::High < ThreatSeverity::Critical);
    }

    #[test]
    fn test_threat_alert_creation() {
        let agent_id = Uuid::new_v4();
        let alert = ThreatAlert::new(
            ThreatCategory::SybilAttack,
            ThreatSeverity::High,
            agent_id,
            "Suspicious agent cluster detected",
        );
        assert_eq!(alert.category, ThreatCategory::SybilAttack);
        assert_eq!(alert.severity, ThreatSeverity::High);
        assert_eq!(alert.source_agent_id, agent_id);
    }
}
