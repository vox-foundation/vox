use crate::StoreError;
use crate::VoxDb;
use turso::params;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryEntry {
    pub discovery_id: String,
    pub session_key: String,
    pub repository_id: String,
    pub title: String,
    pub summary: String,
    pub claims_json: String,
    pub confidence_score: f64,
}

impl VoxDb {
    /// Retrieve discoveries with 'pending' human-gate status.
    pub async fn query_pending_discoveries(&self) -> Result<Vec<DiscoveryEntry>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT discovery_id, session_key, repository_id, title, summary, claims_json, confidence_score 
             FROM scientia_discoveries WHERE human_gate_status = 'pending' ORDER BY created_at_ms ASC",
            ()
        ).await.map_err(|e| StoreError::Db(e.to_string()))?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| StoreError::Db(e.to_string()))?
        {
            out.push(DiscoveryEntry {
                discovery_id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                session_key: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                repository_id: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                title: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                summary: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                claims_json: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                confidence_score: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    /// Mark a discovery as approved and initiate publication tracking.
    pub async fn approve_discovery(
        &self,
        discovery_id: &str,
        reason: Option<&str>,
    ) -> Result<(), StoreError> {
        let now = crate::now_unix_ms();
        self.conn.execute(
            "UPDATE scientia_discoveries SET human_gate_status = 'approved', human_gate_reason = ?1, updated_at_ms = ?2
             WHERE discovery_id = ?3",
            params![reason, now as i64, discovery_id]
        ).await.map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(())
    }

    /// Reject a discovery based on validation failure.
    pub async fn reject_discovery(
        &self,
        discovery_id: &str,
        reason: &str,
    ) -> Result<(), StoreError> {
        let now = crate::now_unix_ms();
        self.conn.execute(
            "UPDATE scientia_discoveries SET human_gate_status = 'rejected', human_gate_reason = ?1, updated_at_ms = ?2
             WHERE discovery_id = ?3",
            params![reason, now as i64, discovery_id]
        ).await.map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(())
    }
}
