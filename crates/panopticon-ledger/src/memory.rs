use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::entry::{LedgerEntry, LedgerEntryKind};
use crate::traits::Ledger;
use panopticon_types::PanopticonError;

/// In-memory ledger implementation (default).
#[derive(Debug, Clone)]
pub struct InMemoryLedger {
    entries: Arc<RwLock<Vec<LedgerEntry>>>,
    index_by_id: Arc<DashMap<Uuid, usize>>,
    index_by_subject: Arc<DashMap<Uuid, Vec<usize>>>,
}

impl InMemoryLedger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            index_by_id: Arc::new(DashMap::new()),
            index_by_subject: Arc::new(DashMap::new()),
        }
    }
}

impl Default for InMemoryLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Ledger for InMemoryLedger {
    async fn append(&self, entry: LedgerEntry) -> Result<(), PanopticonError> {
        let mut entries = self.entries.write().await;
        let idx = entries.len();

        self.index_by_id.insert(entry.id, idx);
        self.index_by_subject
            .entry(entry.subject_id)
            .or_default()
            .push(idx);

        entries.push(entry);
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<LedgerEntry>, PanopticonError> {
        let entries = self.entries.read().await;
        Ok(self
            .index_by_id
            .get(&id)
            .and_then(|idx| entries.get(*idx).cloned()))
    }

    async fn latest_hash(&self) -> Result<Option<String>, PanopticonError> {
        let entries = self.entries.read().await;
        Ok(entries.last().map(|e| e.hash.clone()))
    }

    async fn query_by_subject(
        &self,
        subject_id: Uuid,
    ) -> Result<Vec<LedgerEntry>, PanopticonError> {
        let entries = self.entries.read().await;
        Ok(self
            .index_by_subject
            .get(&subject_id)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|i| entries.get(*i).cloned())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn query_by_kind(
        &self,
        kind: LedgerEntryKind,
    ) -> Result<Vec<LedgerEntry>, PanopticonError> {
        let entries = self.entries.read().await;
        Ok(entries.iter().filter(|e| e.kind == kind).cloned().collect())
    }

    async fn all_entries(&self) -> Result<Vec<LedgerEntry>, PanopticonError> {
        let entries = self.entries.read().await;
        Ok(entries.clone())
    }

    async fn verify_integrity(&self) -> Result<bool, PanopticonError> {
        let entries = self.entries.read().await;
        for (i, entry) in entries.iter().enumerate() {
            if i == 0 {
                if entry.previous_hash.is_some() {
                    return Ok(false);
                }
            } else if entry.previous_hash.as_ref() != Some(&entries[i - 1].hash) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_and_get() {
        let ledger = InMemoryLedger::new();
        let entry = LedgerEntry::new(
            LedgerEntryKind::TaskCreated,
            Uuid::new_v4(),
            Uuid::new_v4(),
            serde_json::json!({}),
            None,
        );
        let id = entry.id;
        ledger.append(entry).await.unwrap();

        let retrieved = ledger.get(id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_chain_integrity() {
        let ledger = InMemoryLedger::new();
        let subject = Uuid::new_v4();

        let entry1 = LedgerEntry::new(
            LedgerEntryKind::TaskCreated,
            Uuid::new_v4(),
            subject,
            serde_json::json!({}),
            None,
        );
        let hash1 = entry1.hash.clone();
        ledger.append(entry1).await.unwrap();

        let entry2 = LedgerEntry::new(
            LedgerEntryKind::TaskStateChanged,
            Uuid::new_v4(),
            subject,
            serde_json::json!({}),
            Some(hash1),
        );
        ledger.append(entry2).await.unwrap();

        assert!(ledger.verify_integrity().await.unwrap());
    }

    #[tokio::test]
    async fn test_query_by_subject() {
        let ledger = InMemoryLedger::new();
        let subject = Uuid::new_v4();
        let other = Uuid::new_v4();

        for _ in 0..3 {
            let entry = LedgerEntry::new(
                LedgerEntryKind::TaskCreated,
                Uuid::new_v4(),
                subject,
                serde_json::json!({}),
                ledger.latest_hash().await.unwrap(),
            );
            ledger.append(entry).await.unwrap();
        }

        let entry = LedgerEntry::new(
            LedgerEntryKind::TaskCreated,
            Uuid::new_v4(),
            other,
            serde_json::json!({}),
            ledger.latest_hash().await.unwrap(),
        );
        ledger.append(entry).await.unwrap();

        let results = ledger.query_by_subject(subject).await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
