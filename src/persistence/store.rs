use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::PersistedState;

/// File-based state store with atomic writes.
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    pub fn new(state_dir: &Path) -> Self {
        Self {
            path: state_dir.join("state.json"),
        }
    }

    /// Default state directory: `~/.panopticon/` or `$PANOPTICON_STATE_DIR`.
    pub fn default_state_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("PANOPTICON_STATE_DIR") {
            PathBuf::from(dir)
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".panopticon")
        }
    }

    /// Load state from disk. Returns default state if file doesn't exist.
    pub fn load(&self) -> Result<PersistedState> {
        if !self.path.exists() {
            return Ok(PersistedState::default());
        }
        let content =
            std::fs::read_to_string(&self.path).context("Failed to read state file")?;
        let state: PersistedState =
            serde_json::from_str(&content).context("Failed to parse state file")?;
        Ok(state)
    }

    /// Save state to disk using atomic write (.tmp → rename).
    pub fn save(&self, state: &PersistedState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create state directory")?;
        }
        let tmp_path = self.path.with_extension("json.tmp");
        let content =
            serde_json::to_string_pretty(state).context("Failed to serialize state")?;
        std::fs::write(&tmp_path, content).context("Failed to write temp state file")?;
        std::fs::rename(&tmp_path, &self.path).context("Failed to rename temp state file")?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Task;
    use tempfile::tempdir;

    #[test]
    fn test_load_nonexistent_returns_default() {
        let dir = tempdir().unwrap();
        let store = FileStore::new(dir.path());
        let state = store.load().unwrap();
        assert!(state.tasks.is_empty());
        assert!(state.agents.is_empty());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let store = FileStore::new(dir.path());

        let mut state = PersistedState::default();
        let task = Task::new("test-task", "a description");
        state.tasks.insert(task.id, task.clone());

        store.save(&state).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[&task.id].name, "test-task");
    }
}
