use crate::StoreError;
use crate::paths::local_user_id;

impl crate::VoxDb {
    /// Register the current machine directory as a known Vox project (`components` + path key).
    ///
    /// The `user_preferences` path write is **best-effort**: failures are ignored so component
    /// registration still succeeds (check logs if paths do not persist).
    pub async fn register_local_project(
        &self,
        name: &str,
        path: &std::path::Path,
    ) -> Result<(), StoreError> {
        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = abs_path.to_string_lossy();

        self.register_component(
            name,
            "local", // namespace for local projects
            None,    // schema_hash not needed for projects
            Some(&format!("Local project at {}", path_str)),
            "1.0.0",
        )
        .await?;

        // Also store the path in user_preferences as a 'known_project'
        let user_id = local_user_id();
        let key = format!("project.{}.path", name);
        let value = path_str.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        let _ = breaker
            .call(|| async move {
                conn.execute(
                    "INSERT OR REPLACE INTO user_preferences (user_id, key, value) VALUES (?1, ?2, ?3)",
                    (user_id, key, value),
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await;

        Ok(())
    }

    /// Return true if the given activity was completed in the specified workflow run.
    pub async fn is_workflow_activity_completed(
        &self,
        run_id: &str,
        workflow_name: &str,
        activity_id: &str,
    ) -> Result<bool, StoreError> {
        let row = self.query_all(
            "SELECT 1 FROM workflow_activity_log WHERE run_id = ?1 AND workflow_name = ?2 AND activity_id = ?3 AND status = 'completed'",
            (run_id.to_string(), workflow_name.to_string(), activity_id.to_string())
        ).await?;
        Ok(!row.is_empty())
    }

    /// Record that an activity has started in the durable journal.
    pub async fn record_workflow_activity_started(
        &self,
        run_id: &str,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let run_id = run_id.to_string();
        let workflow_name = workflow_name.to_string();
        let activity_name = activity_name.to_string();
        let activity_id = activity_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
            "INSERT OR IGNORE INTO workflow_activity_log (run_id, workflow_name, activity_name, activity_id, status, recorded_at_ms) VALUES (?1, ?2, ?3, ?4, 'started', ?5)",
            (run_id, workflow_name, activity_name, activity_id, now)
        ).await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Record that an activity has successfully completed in the durable journal.
    pub async fn record_workflow_activity_completed(
        &self,
        run_id: &str,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let run_id = run_id.to_string();
        let workflow_name = workflow_name.to_string();
        let activity_name = activity_name.to_string();
        let activity_id = activity_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
            "INSERT OR REPLACE INTO workflow_activity_log (run_id, workflow_name, activity_name, activity_id, status, recorded_at_ms) VALUES (?1, ?2, ?3, ?4, 'completed', ?5)",
            (run_id, workflow_name, activity_name, activity_id, now)
        ).await?;
                Ok::<(), StoreError>(())
            })
            .await
    }
}
