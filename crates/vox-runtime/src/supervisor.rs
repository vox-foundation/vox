use crate::process::ProcessHandle;
use crate::registry::RegistryError;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing;

/// Events emitted by the supervisor for graceful degradation or orchestrator reaction.
#[derive(Debug, Clone)]
pub enum SupervisorEvent {
    /// A child has exceeded its maximum restart count and will no longer be restarted.
    MaxRestartsExceeded { child_name: String },
}

/// Restart strategy for supervised processes.
#[derive(Debug, Clone, Copy)]
pub enum RestartStrategy {
    /// Restart only the crashed child.
    OneForOne,
    /// Restart all children if any crashes.
    OneForAll,
    /// Restart the crashed child and all children started after it.
    RestForOne,
}

/// Specification for a supervised child process.
pub struct ChildSpec {
    /// Child name for logging and restart correlation.
    pub name: String,
    /// Factory that spawns a fresh [`ProcessHandle`] when (re)starting the child.
    pub start: Box<dyn Fn() -> Result<ProcessHandle, RegistryError> + Send + Sync>,
}

/// A supervisor managing a set of child actor processes.
pub struct Supervisor {
    strategy: RestartStrategy,
    children: Arc<RwLock<Vec<ChildEntry>>>,
    max_restarts: u32,
    event_tx: mpsc::UnboundedSender<SupervisorEvent>,
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<SupervisorEvent>>>,
}

struct ChildEntry {
    name: String,
    handle: ProcessHandle,
    start: Arc<dyn Fn() -> Result<ProcessHandle, RegistryError> + Send + Sync>,
    restart_count: u32,
}

impl Supervisor {
    /// Creates a supervisor with the given restart strategy and default restart cap.
    pub fn new(strategy: RestartStrategy) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            strategy,
            children: Arc::new(RwLock::new(Vec::new())),
            max_restarts: 5,
            event_tx: tx,
            event_rx: Arc::new(RwLock::new(rx)),
        }
    }

    /// Take the event receiver (can only be done once) to monitor supervisor events.
    pub async fn take_events(&self) -> Option<mpsc::UnboundedReceiver<SupervisorEvent>> {
        let mut rx_guard = self.event_rx.write().await;
        // Swap with a dummy channel so we can return the real one
        let (_, dummy_rx) = mpsc::unbounded_channel();
        let current = std::mem::replace(&mut *rx_guard, dummy_rx);
        Some(current)
    }

    /// Sets how many consecutive restarts are allowed per child before giving up.
    pub fn max_restarts(mut self, count: u32) -> Self {
        self.max_restarts = count;
        self
    }

    /// Add a child specification and start the process.
    pub async fn add_child(&self, spec: ChildSpec) {
        let handle = match (spec.start)() {
            Ok(h) => h,
            Err(e) => {
                tracing::error!(
                    child = %spec.name,
                    error = %e,
                    "supervisor failed to start child (registry lock)"
                );
                return;
            }
        };
        let entry = ChildEntry {
            name: spec.name.clone(),
            handle,
            start: Arc::new(spec.start),
            restart_count: 0,
        };
        self.children.write().await.push(entry);
        tracing::info!("Supervisor started child: {}", spec.name);
    }

    /// Monitor all children and apply restart strategy when any dies.
    pub async fn monitor_loop(&self) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let mut children = self.children.write().await;
            let mut needs_restart = Vec::new();

            for (i, child) in children.iter().enumerate() {
                if !child.handle.is_alive() {
                    tracing::warn!("Child '{}' has died", child.name);
                    needs_restart.push(i);
                }
            }

            for idx in needs_restart.into_iter().rev() {
                let child = &mut children[idx];
                if child.restart_count >= self.max_restarts {
                    tracing::error!(
                        "Child '{}' exceeded max restarts ({})",
                        child.name,
                        self.max_restarts
                    );
                    let _ = self.event_tx.send(SupervisorEvent::MaxRestartsExceeded {
                        child_name: child.name.clone(),
                    });
                    continue;
                }

                match self.strategy {
                    RestartStrategy::OneForOne => {
                        tracing::info!(
                            "Restarting child '{}' (attempt {})",
                            child.name,
                            child.restart_count + 1
                        );
                        let new_handle = match (child.start)() {
                            Ok(h) => h,
                            Err(e) => {
                                tracing::error!(
                                    child = %child.name,
                                    error = %e,
                                    "supervisor restart failed (registry lock)"
                                );
                                continue;
                            }
                        };
                        child.handle = new_handle;
                        child.restart_count += 1;
                    }
                    RestartStrategy::OneForAll => {
                        tracing::info!("Restarting all children due to '{}' failure", child.name);
                        for c in children.iter_mut() {
                            let new_handle = match (c.start)() {
                                Ok(h) => h,
                                Err(e) => {
                                    tracing::error!(
                                        child = %c.name,
                                        error = %e,
                                        "supervisor mass-restart failed (registry lock)"
                                    );
                                    continue;
                                }
                            };
                            c.handle = new_handle;
                            c.restart_count += 1;
                        }
                        break;
                    }
                    RestartStrategy::RestForOne => {
                        tracing::info!("Restarting '{}' and subsequent children", child.name);
                        for c in children[idx..].iter_mut() {
                            let new_handle = match (c.start)() {
                                Ok(h) => h,
                                Err(e) => {
                                    tracing::error!(
                                        child = %c.name,
                                        error = %e,
                                        "supervisor rest-for-one restart failed (registry lock)"
                                    );
                                    continue;
                                }
                            };
                            c.handle = new_handle;
                            c.restart_count += 1;
                        }
                        break;
                    }
                }
            }
        }
    }
}
