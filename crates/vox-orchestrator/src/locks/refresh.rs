use std::path::Path;

use crate::sync_lock;
use crate::types::AgentId;

use super::{FileLock, FileLockManager, LockConflict, LockEntry, LockKind};

impl FileLockManager {
    /// Forcefully release any locks held for longer than timeout_ms.
    /// Returns the number of disconnected stale locks.
    pub fn force_release_stale(&self, timeout_ms: u128) -> usize {
        let mut locks = sync_lock::rw_write(&*self.locks);
        let mut stale_count = 0;
        let mut to_remove = Vec::new();
        let mut to_update = Vec::new();

        for (path, entry) in locks.iter() {
            match entry {
                LockEntry::Exclusive(lock)
                    if lock.acquired_at.elapsed().as_millis() > timeout_ms =>
                {
                    to_remove.push(path.clone());
                    stale_count += 1;
                }
                LockEntry::SharedRead(readers) => {
                    let original_len = readers.len();
                    let remaining: Vec<FileLock> = readers
                        .iter()
                        .filter(|r| r.acquired_at.elapsed().as_millis() <= timeout_ms)
                        .cloned()
                        .collect();

                    stale_count += original_len - remaining.len();

                    if remaining.is_empty() {
                        to_remove.push(path.clone());
                    } else if remaining.len() != original_len {
                        to_update.push((path.clone(), remaining));
                    }
                }
                _ => {}
            }
        }

        for p in to_remove {
            locks.remove(&p);
        }
        for (p, arr) in to_update {
            locks.insert(p, LockEntry::SharedRead(arr));
        }

        stale_count
    }

    /// Upgrade a shared read lock to an exclusive write lock directly.
    pub fn escalate_read_to_write(&self, agent: AgentId, path: &Path) -> Result<(), LockConflict> {
        let mut locks = sync_lock::rw_write(&*self.locks);
        match locks.get(path) {
            Some(LockEntry::SharedRead(readers)) => {
                // If this agent is the ONLY reader, escalate safely.
                if readers.len() == 1 && readers[0].holder == agent {
                    locks.insert(
                        path.to_path_buf(),
                        LockEntry::Exclusive(FileLock {
                            path: path.to_path_buf(),
                            kind: LockKind::Exclusive,
                            holder: agent,
                            acquired_at: std::time::Instant::now(),
                        }),
                    );
                    Ok(())
                } else {
                    // Other readers exist, cannot elevate right now
                    Err(LockConflict::SharedReadersExist {
                        path: path.to_path_buf(),
                        readers: readers.iter().map(|r| r.holder).collect(),
                    })
                }
            }
            Some(LockEntry::Exclusive(lock)) if lock.holder == agent => {
                // Already have write access
                Ok(())
            }
            Some(LockEntry::Exclusive(lock)) => Err(LockConflict::ExclusivelyHeld {
                path: path.to_path_buf(),
                holder: lock.holder,
            }),
            None => {
                // Not locked at all... this shouldn't typically happen in elevate context,
                // but just acquire write broadly
                locks.insert(
                    path.to_path_buf(),
                    LockEntry::Exclusive(FileLock {
                        path: path.to_path_buf(),
                        kind: LockKind::Exclusive,
                        holder: agent,
                        acquired_at: std::time::Instant::now(),
                    }),
                );
                Ok(())
            }
        }
    }

    /// Add an agent to the wait queue for a file they cannot lock.
    pub fn queue_agent_for_lock(&self, agent: AgentId, path: &Path) {
        let mut q = sync_lock::rw_write(&*self.queue);
        let queue = q.entry(path.to_path_buf()).or_default();
        if !queue.contains(&agent) {
            queue.push_back(agent);
        }
    }

    /// Dequeue the next agent waiting for a lock, if any.
    pub fn dequeue_waiter(&self, path: &Path) -> Option<AgentId> {
        let mut q = crate::sync_lock::rw_write(&self.queue);
        if let Some(queue) = q.get_mut(path) {
            queue.pop_front()
        } else {
            None
        }
    }

    /// Total number of agents waiting for any lock.
    pub fn contention_count(&self) -> usize {
        let q = sync_lock::rw_read(&*self.queue);
        q.values().map(|v| v.len()).sum()
    }
}

impl Default for FileLockManager {
    fn default() -> Self {
        Self::new()
    }
}
