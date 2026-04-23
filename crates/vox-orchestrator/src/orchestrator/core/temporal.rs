impl crate::orchestrator::Orchestrator {
    /// Build temporal context string for system-prompt injection.
    pub fn build_temporal_context(
        session: &crate::session::Session,
        task: &crate::types::AgentTask,
    ) -> String {
        let mut base = session.temporal_summary();
        let elapsed_secs = task
            .created_at
            .map(|i| std::time::Instant::now().duration_since(i).as_secs())
            .unwrap_or_else(|| {
                crate::types::now_unix_ms().saturating_sub(task.created_at_ms) / 1000
            });
        base.push_str(&format!(" Task created: {}s ago.", elapsed_secs));
        base
    }
}
