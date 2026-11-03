use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use vox_orchestrator_types::AgentId;

/// Kind of resource lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceLockKind {
    /// Exclusive access — only one agent at a time.
    Exclusive,
    /// Shared access — multiple agents can hold simultaneously.
    Shared,
}

/// A generic resource lock (e.g. database row, URI, logical semaphore).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLock {
    /// Unique resource identifier (e.g. `"db://users/1"`, `"http://api.com/resource"`).
    pub resource_id: String,
    pub kind: ResourceLockKind,
    pub holder: AgentId,
    /// Unix timestamp when the lock expires.
    pub expires_ms: u64,
}

/// Thread-safe resource lock manager with lease propagation support.
#[derive(Clone, Default)]
pub struct ResourceLockManager {
    locks: Arc<std::sync::RwLock<HashMap<String, ResourceLock>>>,
}

/// Purge all expired resource lock entries lazily (D-06).
///
/// Follows the stale-cleanup precedent established by `force_release_stale`.
fn sweep_expired(locks: &mut HashMap<String, ResourceLock>, now: u64) {
    locks.retain(|_, l| l.expires_ms > now);
}

impl ResourceLockManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to acquire a resource lock.
    pub fn try_acquire(
        &self,
        resource_id: &str,
        agent_id: AgentId,
        kind: ResourceLockKind,
        ttl_ms: u64,
    ) -> Result<ResourceLock, String> {
        let mut locks = self.locks.write().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        sweep_expired(&mut locks, now);

        if let Some(existing) = locks.get(resource_id)
            && existing.expires_ms > now
            && existing.holder != agent_id
        {
            return Err(format!(
                "Resource '{}' is already locked by agent {}",
                resource_id, existing.holder
            ));
        }
        // Re-entrant: extend TTL

        let lock = ResourceLock {
            resource_id: resource_id.to_string(),
            kind,
            holder: agent_id,
            expires_ms: now + ttl_ms,
        };
        locks.insert(resource_id.to_string(), lock.clone());
        Ok(lock)
    }

    /// Release a resource lock.
    pub fn release(&self, resource_id: &str, agent_id: AgentId) {
        let mut locks = self.locks.write().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        sweep_expired(&mut locks, now);
        if let Some(existing) = locks.get(resource_id)
            && existing.holder == agent_id
        {
            locks.remove(resource_id);
        }
    }

    /// Number of active locks.
    pub fn len(&self) -> usize {
        self.locks.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.locks.read().unwrap().is_empty()
    }

    /// Returns a snapshot of all active locks.
    pub fn snapshot(&self) -> Vec<ResourceLock> {
        self.locks.read().unwrap().values().cloned().collect()
    }

    /// Check if a resource is currently locked.
    ///
    /// Takes a write lock to purge expired leases lazily (D-06).
    /// No caller holds this manager's lock re-entrantly — the only production caller is `ResourceGate::park`.
    pub fn is_locked(&self, resource_id: &str) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let mut locks = self.locks.write().unwrap();
        sweep_expired(&mut locks, now);
        if let Some(lock) = locks.get(resource_id) {
            lock.expires_ms > now
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D_10MS: std::time::Duration = std::time::Duration::from_millis(10);

    #[test]
    fn acquire_sweeps_every_expired_entry() {
        let mgr = ResourceLockManager::new();
        mgr.try_acquire("res://a", AgentId(1), ResourceLockKind::Exclusive, 1)
            .unwrap();
        mgr.try_acquire("res://b", AgentId(2), ResourceLockKind::Exclusive, 1)
            .unwrap();
        std::thread::sleep(D_10MS);
        mgr.try_acquire("res://c", AgentId(3), ResourceLockKind::Exclusive, 60_000)
            .unwrap();
        assert_eq!(mgr.len(), 1);
        let snap = mgr.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].resource_id, "res://c");
    }

    #[test]
    fn is_locked_sweeps_expired_entries() {
        let mgr = ResourceLockManager::new();
        mgr.try_acquire("res://exp", AgentId(1), ResourceLockKind::Exclusive, 1)
            .unwrap();
        std::thread::sleep(D_10MS);
        assert!(!mgr.is_locked("res://other"));
        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn live_locks_survive_the_sweep() {
        let mgr = ResourceLockManager::new();
        mgr.try_acquire(
            "res://live",
            AgentId(1),
            ResourceLockKind::Exclusive,
            60_000,
        )
        .unwrap();
        mgr.try_acquire("res://dead", AgentId(2), ResourceLockKind::Exclusive, 1)
            .unwrap();
        std::thread::sleep(D_10MS);
        mgr.try_acquire("res://new", AgentId(3), ResourceLockKind::Exclusive, 60_000)
            .unwrap();
        assert_eq!(mgr.len(), 2);
        assert!(mgr.is_locked("res://live"));
        assert!(mgr.is_locked("res://new"));
        assert!(!mgr.is_locked("res://dead"));
    }

    #[test]
    fn release_sweeps_expired_entries() {
        let mgr = ResourceLockManager::new();
        mgr.try_acquire("res://dead", AgentId(1), ResourceLockKind::Exclusive, 1)
            .unwrap();
        mgr.try_acquire(
            "res://live",
            AgentId(2),
            ResourceLockKind::Exclusive,
            60_000,
        )
        .unwrap();
        std::thread::sleep(D_10MS);
        mgr.release("res://live", AgentId(2));
        assert_eq!(mgr.len(), 0);
    }
}
