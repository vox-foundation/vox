use crate::types::AgentId;

impl crate::orchestrator::Orchestrator {
    /// Check if the orchestrator is in an emergency stop state.
    pub fn is_stopped(&self) -> bool {
        self.stop_flag.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Trigger a global emergency stop across the orchestrator.
    pub fn emergency_stop(&self, reason: Option<String>) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.event_bus
            .emit(crate::events::AgentEventKind::EmergencyStop { reason });
        tracing::warn!("Orchestrator emergency stop triggered.");
    }

    /// Access the underlying database handle if connected.
    pub fn db(&self) -> Option<std::sync::Arc<vox_db::VoxDb>> {
        crate::sync_lock::rw_read(&*self.db).clone()
    }

    /// Access the internal context store.
    pub fn context_store(&self) -> std::sync::Arc<std::sync::RwLock<crate::context::ContextStore>> {
        self.context_store.clone()
    }

    /// Update the global activity timestamp.
    pub fn record_activity(&self) {
        self.last_activity_ms.store(
            crate::types::now_unix_ms(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Get the global last activity timestamp in milliseconds.
    pub fn last_activity_ms(&self) -> u64 {
        self.last_activity_ms
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
