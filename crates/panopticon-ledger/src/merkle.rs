use async_trait::async_trait;
use rs_merkle::{Hasher, MerkleTree, algorithms::Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::entry::{LedgerEntry, LedgerEntryKind};
use crate::traits::Ledger;
use panopticon_types::PanopticonError;

/// Merkle tree-backed ledger for cryptographic integrity.
#[derive(Clone)]
pub struct MerkleLedger {
    entries: Arc<RwLock<Vec<LedgerEntry>>>,
    tree: Arc<RwLock<MerkleTree<Sha256>>>,
}

impl MerkleLedger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            tree: Arc::new(RwLock::new(MerkleTree::<Sha256>::new())),
        }
    }

    /// Get the Merkle root hash.
    pub async fn root_hex(&self) -> Option<String> {
        let tree = self.tree.read().await;
        tree.root_hex()
    }

    /// Generate a proof for entry at given index.
    pub async fn proof(&self, index: usize) -> Option<Vec<u8>> {
        let tree = self.tree.read().await;
        let entries = self.entries.read().await;
        if index >= entries.len() {
            return None;
        }
        let proof = tree.proof(&[index]);
        Some(proof.to_bytes())
    }
}

impl Default for MerkleLedger {
    fn default() -> Self {
        Self::new()
    }
}

fn entry_to_leaf(entry: &LedgerEntry) -> [u8; 32] {
    let data = format!("{}:{}", entry.id, entry.hash);
    Sha256::hash(data.as_bytes())
}

#[async_trait]
impl Ledger for MerkleLedger {
    async fn append(&self, entry: LedgerEntry) -> Result<(), PanopticonError> {
        let leaf = entry_to_leaf(&entry);

        let mut entries = self.entries.write().await;
        let mut tree = self.tree.write().await;

        tree.insert(leaf);
        tree.commit();
        entries.push(entry);
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<LedgerEntry>, PanopticonError> {
        let entries = self.entries.read().await;
        Ok(entries.iter().find(|e| e.id == id).cloned())
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
        Ok(entries
            .iter()
            .filter(|e| e.subject_id == subject_id)
            .cloned()
            .collect())
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
        let tree = self.tree.read().await;
        let entries = self.entries.read().await;

        // Verify Merkle tree leaves match entries
        let expected_leaves: Vec<[u8; 32]> = entries.iter().map(entry_to_leaf).collect();
        let indices: Vec<usize> = (0..entries.len()).collect();

        if entries.is_empty() {
            return Ok(true);
        }

        if let Some(root) = tree.root() {
            let proof = tree.proof(&indices);
            Ok(proof.verify(root, &indices, &expected_leaves, entries.len()))
        } else {
            Ok(entries.is_empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_merkle_append_and_root() {
        let ledger = MerkleLedger::new();
        assert!(ledger.root_hex().await.is_none());

        let entry = LedgerEntry::new(
            LedgerEntryKind::TaskCreated,
            Uuid::new_v4(),
            Uuid::new_v4(),
            serde_json::json!({}),
            None,
        );
        ledger.append(entry).await.unwrap();

        assert!(ledger.root_hex().await.is_some());
    }

    #[tokio::test]
    async fn test_merkle_integrity() {
        let ledger = MerkleLedger::new();

        for i in 0..5 {
            let prev = ledger.latest_hash().await.unwrap();
            let entry = LedgerEntry::new(
                LedgerEntryKind::TaskCreated,
                Uuid::new_v4(),
                Uuid::new_v4(),
                serde_json::json!({"index": i}),
                prev,
            );
            ledger.append(entry).await.unwrap();
        }

        assert!(ledger.verify_integrity().await.unwrap());
    }

    #[tokio::test]
    async fn test_merkle_proof() {
        let ledger = MerkleLedger::new();

        for _ in 0..3 {
            let prev = ledger.latest_hash().await.unwrap();
            let entry = LedgerEntry::new(
                LedgerEntryKind::TaskCreated,
                Uuid::new_v4(),
                Uuid::new_v4(),
                serde_json::json!({}),
                prev,
            );
            ledger.append(entry).await.unwrap();
        }

        let proof = ledger.proof(1).await;
        assert!(proof.is_some());
    }
}
