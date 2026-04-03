//! Stubs when `populi-transport` is disabled.

use std::sync::{Arc, Mutex, RwLock};
use tokio::task::JoinHandle;
use vox_db::VoxDb;

use crate::config::OrchestratorConfig;
use crate::orchestrator::Orchestrator;
use crate::populi_federation::RemotePopuliSnapshot;

/// Background poll of populi control plane — disabled without `populi-transport`.
pub fn spawn_populi_federation_poller(
    _orchestrator_config: &OrchestratorConfig,
    _repository_id: String,
    _db: Option<Arc<VoxDb>>,
    _orchestrator: Arc<Orchestrator>,
    _snapshot: Arc<RwLock<RemotePopuliSnapshot>>,
    _join_slot: Arc<Mutex<Option<JoinHandle<()>>>>,
) {
}
