use serde_json::Value;
use turso::params;

use crate::VoxDb;
use crate::store::StoreError;

impl VoxDb {
    /// Ordered steps for a canonical journey id (e.g. `canonical_journey.v1.greenfield_vox_mens_devloop`).
    pub async fn list_developer_journey_steps(
        &self,
        journey_id: &str,
    ) -> Result<Vec<Value>, StoreError> {
        let journey_id = journey_id.to_string();
        let mut rows = self
            .conn
            .query(
                "SELECT step_json FROM developer_journey_steps WHERE journey_id = ?1 ORDER BY ordinal ASC",
                params![journey_id.as_str()],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            let s: String = r.get(0)?;
            let v: Value =
                serde_json::from_str(&s).map_err(|e| StoreError::Serialization(e.to_string()))?;
            out.push(v);
        }
        Ok(out)
    }
}
