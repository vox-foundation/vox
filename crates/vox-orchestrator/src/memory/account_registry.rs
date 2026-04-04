use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::config::MemoryConfig;
use super::error::MemoryError;
use super::manager::MemoryManager;

/// Lazily-initialized per-account memory manager registry.
///
/// Holds one [`MemoryManager`] per `account_id`, shared across all agents within
/// the same account. Agents in distinct accounts cannot access each other's files
/// because each `MemoryManager` is constructed with a scoped [`MemoryConfig`].
///
/// ## Thread safety
/// Internally protected by a `Mutex`; `get_or_create` briefly locks, then returns
/// a cloned `Arc`. Long-lived callers hold the `Arc`, not the lock.
pub struct AccountMemoryRegistry {
    managers: Mutex<HashMap<String, Arc<MemoryManager>>>,
    base_dir: PathBuf,
}

impl AccountMemoryRegistry {
    /// Create a new registry rooted at `base_dir`.
    ///
    /// `base_dir` is typically `.vox/memory` inside the workspace root.  Each
    /// account gets a subdirectory: `{base_dir}/{account_id}/`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            managers: Mutex::new(HashMap::new()),
            base_dir: base_dir.into(),
        }
    }

    /// Return (or lazily create) the [`MemoryManager`] for `account_id`.
    ///
    /// On first call for a given `account_id` the manager is constructed and
    /// stored; subsequent calls return the same `Arc`.
    pub fn get_or_create(&self, account_id: &str) -> Result<Arc<MemoryManager>, MemoryError> {
        let mut map = self
            .managers
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        if let Some(existing) = map.get(account_id) {
            return Ok(Arc::clone(existing));
        }

        let config = MemoryConfig::for_account(account_id, &self.base_dir);
        let manager = Arc::new(MemoryManager::new(config)?);
        map.insert(account_id.to_string(), Arc::clone(&manager));
        Ok(manager)
    }

    /// Return the `account_id` strings for all accounts that already have an
    /// active `MemoryManager` in this registry.
    pub fn active_accounts(&self) -> Vec<String> {
        self.managers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    /// Scan `base_dir` on disk and return all subdirectory names that look like
    /// valid account ids (i.e. a `MEMORY.md` or `logs/` child is present).
    ///
    /// This is a best-effort scan and may return stale entries if directories
    /// were removed externally.
    pub fn list_accounts_on_disk(&self) -> Vec<String> {
        let Ok(read_dir) = std::fs::read_dir(&self.base_dir) else {
            return vec![];
        };
        let mut accounts = Vec::new();
        for entry in read_dir.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().into_owned();
                let memory_md = entry.path().join("MEMORY.md");
                let logs_dir = entry.path().join("logs");
                if memory_md.exists() || logs_dir.exists() {
                    accounts.push(name);
                }
            }
        }
        accounts
    }
}
