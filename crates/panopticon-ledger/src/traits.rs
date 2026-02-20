use async_trait::async_trait;
use uuid::Uuid;

use crate::entry::{LedgerEntry, LedgerEntryKind};
use panopticon_types::PanopticonError;

/// Core ledger trait — all ledger implementations must satisfy this.
#[async_trait]
pub trait Ledger: Send + Sync {
    /// Append an entry to the ledger.
    async fn append(&self, entry: LedgerEntry) -> Result<(), PanopticonError>;

    /// Get an entry by its ID.
    async fn get(&self, id: Uuid) -> Result<Option<LedgerEntry>, PanopticonError>;

    /// Get the latest entry hash (for chaining).
    async fn latest_hash(&self) -> Result<Option<String>, PanopticonError>;

    /// Query entries by subject.
    async fn query_by_subject(&self, subject_id: Uuid)
    -> Result<Vec<LedgerEntry>, PanopticonError>;

    /// Query entries by kind.
    async fn query_by_kind(
        &self,
        kind: LedgerEntryKind,
    ) -> Result<Vec<LedgerEntry>, PanopticonError>;

    /// Get all entries (for auditing).
    async fn all_entries(&self) -> Result<Vec<LedgerEntry>, PanopticonError>;

    /// Verify the chain integrity.
    async fn verify_integrity(&self) -> Result<bool, PanopticonError>;
}
