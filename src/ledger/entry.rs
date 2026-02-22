use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types of ledger entries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LedgerEntryKind {
    TaskCreated,
    TaskStateChanged,
    AgentRegistered,
    DelegationRequested,
    BidSubmitted,
    ContractCreated,
    ContractSigned,
    CheckpointRecorded,
    VerificationResult,
    DisputeOpened,
    DisputeResolved,
    ReputationUpdated,
    PermissionGranted,
    PermissionRevoked,
    SecurityAlert,
    PaymentProcessed,
}

/// An immutable ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub kind: LedgerEntryKind,
    pub timestamp: DateTime<Utc>,
    pub actor_id: Uuid,
    pub subject_id: Uuid,
    pub payload: serde_json::Value,
    pub previous_hash: Option<String>,
    pub hash: String,
}

impl LedgerEntry {
    pub fn new(
        kind: LedgerEntryKind,
        actor_id: Uuid,
        subject_id: Uuid,
        payload: serde_json::Value,
        previous_hash: Option<String>,
    ) -> Self {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();

        let hash_input = format!(
            "{}:{}:{:?}:{}:{}:{}:{}",
            id,
            timestamp.timestamp_nanos_opt().unwrap_or(0),
            kind,
            actor_id,
            subject_id,
            payload,
            previous_hash.as_deref().unwrap_or("genesis"),
        );

        // Simple hash using std — feature-gated crates provide stronger hashing.
        let hash = format!("{:x}", md5_like_hash(hash_input.as_bytes()));

        Self {
            id,
            kind,
            timestamp,
            actor_id,
            subject_id,
            payload,
            previous_hash,
            hash,
        }
    }
}

/// A simple non-cryptographic hash for the default (non-merkle) ledger.
fn md5_like_hash(data: &[u8]) -> u128 {
    let mut h: u128 = 0xcbf29ce484222325;
    for &b in data {
        h = h.wrapping_mul(0x100000001b3);
        h ^= b as u128;
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_entry_creation() {
        let entry = LedgerEntry::new(
            LedgerEntryKind::TaskCreated,
            Uuid::new_v4(),
            Uuid::new_v4(),
            serde_json::json!({"name": "test"}),
            None,
        );
        assert!(!entry.hash.is_empty());
        assert!(entry.previous_hash.is_none());
    }

    #[test]
    fn test_chained_entries() {
        let first = LedgerEntry::new(
            LedgerEntryKind::TaskCreated,
            Uuid::new_v4(),
            Uuid::new_v4(),
            serde_json::json!({}),
            None,
        );
        let second = LedgerEntry::new(
            LedgerEntryKind::TaskStateChanged,
            Uuid::new_v4(),
            Uuid::new_v4(),
            serde_json::json!({}),
            Some(first.hash.clone()),
        );
        assert_eq!(second.previous_hash.as_ref().unwrap(), &first.hash);
    }
}
