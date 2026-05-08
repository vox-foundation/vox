use crate::pid::Pid;
use crate::process::ProcessHandle;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Lock acquisition failed because another thread panicked while holding the registry mutex.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("process registry lock poisoned ({operation})")]
    LockPoisoned { operation: &'static str },
}

type PidMap = HashMap<Pid, ProcessHandle>;
type NameMap = HashMap<String, Pid>;

fn write_pid<'a>(
    lock: &'a RwLock<PidMap>,
    operation: &'static str,
) -> Result<RwLockWriteGuard<'a, PidMap>, RegistryError> {
    lock.write()
        .map_err(|_| RegistryError::LockPoisoned { operation })
}

fn write_names<'a>(
    lock: &'a RwLock<NameMap>,
    operation: &'static str,
) -> Result<RwLockWriteGuard<'a, NameMap>, RegistryError> {
    lock.write()
        .map_err(|_| RegistryError::LockPoisoned { operation })
}

fn read_pid<'a>(
    lock: &'a RwLock<PidMap>,
    operation: &'static str,
) -> Result<RwLockReadGuard<'a, PidMap>, RegistryError> {
    lock.read()
        .map_err(|_| RegistryError::LockPoisoned { operation })
}

fn read_names<'a>(
    lock: &'a RwLock<NameMap>,
    operation: &'static str,
) -> Result<RwLockReadGuard<'a, NameMap>, RegistryError> {
    lock.read()
        .map_err(|_| RegistryError::LockPoisoned { operation })
}

/// Global registry mapping Pids and names to live process handles.
#[derive(Clone)]
pub struct ProcessRegistry {
    by_pid: Arc<RwLock<PidMap>>,
    by_name: Arc<RwLock<NameMap>>,
}

impl ProcessRegistry {
    /// Returns an empty registry.
    pub fn new() -> Self {
        Self {
            by_pid: Arc::new(RwLock::new(HashMap::new())),
            by_name: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a process handle.
    pub fn register(&self, handle: ProcessHandle) -> Result<(), RegistryError> {
        let pid = handle.pid;
        write_pid(&self.by_pid, "register(insert by_pid)")?.insert(pid, handle);
        Ok(())
    }

    /// Register a named process.
    pub fn register_name(&self, name: String, handle: ProcessHandle) -> Result<(), RegistryError> {
        let pid = handle.pid;
        let replaced_pid =
            write_names(&self.by_name, "register_name(insert by_name)")?.insert(name, pid);
        let mut pid_map = write_pid(&self.by_pid, "register_name(insert by_pid)")?;
        if let Some(old_pid) = replaced_pid
            && old_pid != pid
        {
            pid_map.remove(&old_pid);
        }
        pid_map.insert(pid, handle);
        Ok(())
    }

    /// Lookup a process by Pid.
    pub fn lookup(&self, pid: &Pid) -> Result<Option<ProcessHandle>, RegistryError> {
        Ok(read_pid(&self.by_pid, "lookup(get by_pid)")?
            .get(pid)
            .cloned())
    }

    /// Lookup a process by name.
    pub fn lookup_name(&self, name: &str) -> Result<Option<ProcessHandle>, RegistryError> {
        let pid = read_names(&self.by_name, "lookup_name(get by_name)")?
            .get(name)
            .copied();
        let Some(pid) = pid else {
            return Ok(None);
        };
        self.lookup(&pid)
    }

    /// Remove a process from the registry.
    pub fn unregister(&self, pid: &Pid) -> Result<(), RegistryError> {
        write_pid(&self.by_pid, "unregister(remove by_pid)")?.remove(pid);
        // Also remove from name registry
        let mut names = write_names(&self.by_name, "unregister(retain by_name)")?;
        names.retain(|_, v| v != pid);
        Ok(())
    }

    /// Count of registered processes.
    pub fn len(&self) -> Result<usize, RegistryError> {
        Ok(read_pid(&self.by_pid, "len(by_pid)")?.len())
    }

    /// Returns true when no [`ProcessHandle`]s are registered.
    pub fn is_empty(&self) -> Result<bool, RegistryError> {
        Ok(self.len()? == 0)
    }
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::spawn_process;

    #[tokio::test]
    async fn test_register_and_lookup() -> Result<(), RegistryError> {
        let registry = ProcessRegistry::new();
        let handle = spawn_process(|_ctx| async {});
        let pid = handle.pid;
        registry.register(handle)?;
        assert!(registry.lookup(&pid)?.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn test_named_lookup() -> Result<(), RegistryError> {
        let registry = ProcessRegistry::new();
        let handle = spawn_process(|_ctx| async {});
        registry.register_name("worker".to_string(), handle)?;
        assert!(registry.lookup_name("worker")?.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn test_register_name_replaces_prior_pid_entry() -> Result<(), RegistryError> {
        let registry = ProcessRegistry::new();
        let first = spawn_process(|_ctx| async {});
        let first_pid = first.pid;
        registry.register_name("worker".to_string(), first)?;
        assert!(registry.lookup(&first_pid)?.is_some());
        assert_eq!(registry.len()?, 1);

        let second = spawn_process(|_ctx| async {});
        let second_pid = second.pid;
        registry.register_name("worker".to_string(), second)?;

        assert!(registry.lookup(&first_pid)?.is_none());
        assert!(registry.lookup(&second_pid)?.is_some());
        assert_eq!(registry.len()?, 1);
        Ok(())
    }
}
