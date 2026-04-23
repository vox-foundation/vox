//! Stubs when `populi-transport` is disabled (no `vox-populi` HTTP client).

use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

/// No-op when Populi mesh transport is not compiled in.
pub fn spawn_populi_remote_result_poller(
    _orchestrator: Arc<crate::orchestrator::Orchestrator>,
    _join_slot: Arc<Mutex<Option<JoinHandle<()>>>>,
) {
    let _ = ();
}

/// No-op when Populi mesh transport is not compiled in.
pub async fn populi_remote_result_poll_once(_orchestrator: &crate::orchestrator::Orchestrator) {
    let _ = std::hint::black_box(_orchestrator as *const _ as usize);
}
