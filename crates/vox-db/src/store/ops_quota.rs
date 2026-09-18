//! Provider quota tracking and persistence ops (`provider_quota_usage` table).

use crate::store::types::StoreError;
use turso::params;

/// Stored monthly quota state for an upstream provider (e.g. Tavily).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProviderQuotaUsage {
    pub provider: String,
    pub period_key: String,
    pub units_spent: i64,
    pub units_limit: i64,
    pub last_synced_at: String,
}

/// Formats the current UTC timestamp as a standard `'YYYY-MM'` period key.
pub fn current_period_key() -> String {
    chrono::Utc::now().format("%Y-%m").to_string()
}

/// Records or increments spent quota units for `(provider, period_key)`.
///
/// If a row does not exist yet for this period, it is created with a default limit of 1000.
pub async fn record_quota_spend(
    conn: &turso::Connection,
    provider: &str,
    period_key: &str,
    units: i64,
) -> Result<(), StoreError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO provider_quota_usage (provider, period_key, units_spent, units_limit, last_synced_at)
         VALUES (?1, ?2, ?3, 1000, ?4)
         ON CONFLICT(provider, period_key) DO UPDATE SET
             units_spent = provider_quota_usage.units_spent + excluded.units_spent,
             last_synced_at = excluded.last_synced_at",
        params![provider, period_key, units, now],
    )
    .await?;
    Ok(())
}

/// Retrieves stored quota usage for `(provider, period_key)` if present.
pub async fn get_quota_usage(
    conn: &turso::Connection,
    provider: &str,
    period_key: &str,
) -> Result<Option<ProviderQuotaUsage>, StoreError> {
    let mut rows = conn
        .query(
            "SELECT provider, period_key, units_spent, units_limit, last_synced_at
             FROM provider_quota_usage
             WHERE provider = ?1 AND period_key = ?2",
            params![provider, period_key],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        Ok(Some(ProviderQuotaUsage {
            provider: row.get(0)?,
            period_key: row.get(1)?,
            units_spent: row.get(2)?,
            units_limit: row.get(3)?,
            last_synced_at: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

/// Upserts full quota state after an upstream sync reconciliation.
pub async fn record_quota_sync(
    conn: &turso::Connection,
    provider: &str,
    period_key: &str,
    units_spent: i64,
    units_limit: i64,
) -> Result<(), StoreError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO provider_quota_usage (provider, period_key, units_spent, units_limit, last_synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(provider, period_key) DO UPDATE SET
             units_spent = excluded.units_spent,
             units_limit = excluded.units_limit,
             last_synced_at = excluded.last_synced_at",
        params![provider, period_key, units_spent, units_limit, now],
    )
    .await?;
    Ok(())
}

impl crate::VoxDb {
    /// Increment provider quota usage by `units` for `(provider, period_key)`.
    pub async fn record_quota_spend(
        &self,
        provider: &str,
        period_key: &str,
        units: i64,
    ) -> Result<(), StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        let provider = provider.to_string();
        let period_key = period_key.to_string();
        breaker
            .call(|| async move { record_quota_spend(&conn, &provider, &period_key, units).await })
            .await
    }

    /// Retrieve provider quota usage for `(provider, period_key)` if recorded.
    pub async fn get_quota_usage(
        &self,
        provider: &str,
        period_key: &str,
    ) -> Result<Option<ProviderQuotaUsage>, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        let provider = provider.to_string();
        let period_key = period_key.to_string();
        breaker
            .call(|| async move { get_quota_usage(&conn, &provider, &period_key).await })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DbConfig, VoxDb};

    #[test]
    fn test_current_period_key_format() {
        let key = current_period_key();
        assert_eq!(key.len(), 7);
        assert_eq!(&key[4..5], "-");
        let year: u32 = key[0..4].parse().expect("year");
        let month: u32 = key[5..7].parse().expect("month");
        assert!(year >= 2026);
        assert!((1..=12).contains(&month));
    }

    #[tokio::test]
    async fn test_quota_spend_and_sync_ops() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        let conn = db.connection();

        let initial = get_quota_usage(conn, "test_provider", "2026-09")
            .await
            .expect("query");
        assert!(initial.is_none());

        record_quota_spend(conn, "test_provider", "2026-09", 100)
            .await
            .expect("spend");
        let usage = get_quota_usage(conn, "test_provider", "2026-09")
            .await
            .expect("query")
            .expect("found");
        assert_eq!(usage.units_spent, 100);
        assert_eq!(usage.units_limit, 1000);

        // Sync updates
        record_quota_sync(conn, "test_provider", "2026-09", 350, 2000)
            .await
            .expect("sync");
        let synced = get_quota_usage(conn, "test_provider", "2026-09")
            .await
            .expect("query")
            .expect("found");
        assert_eq!(synced.units_spent, 350);
        assert_eq!(synced.units_limit, 2000);

        // VoxDb wrapper methods
        db.record_quota_spend("test_provider", "2026-09", 50)
            .await
            .expect("spend on db");
        let db_usage = db
            .get_quota_usage("test_provider", "2026-09")
            .await
            .expect("get on db")
            .expect("found");
        assert_eq!(db_usage.units_spent, 400);
    }
}
