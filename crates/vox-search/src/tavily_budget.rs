//! Session-scoped Tavily credit budget and upstream usage reconciliation.

use serde::Deserialize;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

static GLOBAL_DB: OnceLock<Arc<vox_db::Codex>> = OnceLock::new();

/// Sets the optional database handle used for recording quota spend.
pub fn set_budget_db(db: Arc<vox_db::Codex>) {
    let _ = GLOBAL_DB.set(db);
}

/// Retrieves the database handle, checking local cell then `ProviderSafetyGovernor`.
pub fn get_budget_db() -> Option<Arc<vox_db::Codex>> {
    GLOBAL_DB
        .get()
        .cloned()
        .or_else(|| crate::safety_governor::ProviderSafetyGovernor::global().db())
}

/// Formats the current UTC period key ('YYYY-MM').
pub fn period_key() -> String {
    vox_db::store::ops_quota::current_period_key()
}

#[derive(Debug, Deserialize)]
struct UpstreamUsageResponse {
    #[serde(alias = "monthly_limit", alias = "limit")]
    monthly_limit: Option<usize>,
    #[serde(alias = "monthly_usage", alias = "usage")]
    monthly_usage: Option<usize>,
}

/// Thread-safe atomic credit counter for one MCP/CLI session.
#[derive(Debug, Clone)]
pub struct TavilySessionBudget {
    limit: Arc<AtomicUsize>,
    used: Arc<AtomicUsize>,
    remaining: Arc<AtomicUsize>,
}

impl TavilySessionBudget {
    /// New budget with `limit` remaining credits.
    pub fn new(limit: usize) -> Self {
        Self {
            limit: Arc::new(AtomicUsize::new(limit)),
            used: Arc::new(AtomicUsize::new(0)),
            remaining: Arc::new(AtomicUsize::new(limit)),
        }
    }

    /// Returns `(used, remaining)` credits.
    pub fn usage_and_remaining(&self) -> (usize, usize) {
        (
            self.used.load(Ordering::SeqCst),
            self.remaining.load(Ordering::SeqCst),
        )
    }

    /// Returns `false` and does NOT decrement if already at zero or remaining < cost.
    ///
    /// On success, also triggers best-effort asynchronous persistence of quota spend to SQLite.
    pub fn try_consume(&self, cost: usize) -> bool {
        let mut current = self.remaining.load(Ordering::SeqCst);
        loop {
            if current == 0 || current < cost {
                return false;
            }
            match self.remaining.compare_exchange_weak(
                current,
                current - cost,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    self.used.fetch_add(cost, Ordering::SeqCst);
                    let cost_i64 = cost as i64;
                    if let Some(db) = get_budget_db() {
                        let period = period_key();
                        if let Ok(handle) = tokio::runtime::Handle::try_current() {
                            handle.spawn(async move {
                                let conn = db.connection();
                                let _ = vox_db::store::ops_quota::record_quota_spend(
                                    conn, "tavily", &period, cost_i64,
                                )
                                .await;
                            });
                        }
                    }
                    return true;
                }
                Err(val) => current = val,
            }
        }
    }

    /// Remaining credits (best-effort; concurrent consumers may race).
    pub fn remaining(&self) -> usize {
        self.remaining.load(Ordering::SeqCst)
    }

    /// Syncs local budget limits and usage with the upstream Tavily `/usage` endpoint.
    pub async fn sync_with_upstream(
        &self,
        api_key: &str,
        base_url: Option<&str>,
    ) -> anyhow::Result<(usize, usize)> {
        let base = base_url
            .unwrap_or("https://api.tavily.com")
            .trim_end_matches('/');
        let url = format!("{base}/usage");
        let client = vox_http_client::client_builder()
            .timeout(vox_config::timeouts::D_10S)
            .build()?;
        let res = client
            .get(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("Tavily usage request failed with status: {}", res.status());
        }

        let body = res.json::<UpstreamUsageResponse>().await?;
        let limit = body.monthly_limit.unwrap_or(1000);
        let used = body.monthly_usage.unwrap_or(0);
        let remaining = limit.saturating_sub(used);

        self.limit.store(limit, Ordering::SeqCst);
        self.used.store(used, Ordering::SeqCst);
        self.remaining.store(remaining, Ordering::SeqCst);

        Ok((used, remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_exhausts() {
        let b = TavilySessionBudget::new(2);
        assert!(b.try_consume(1));
        assert!(b.try_consume(1));
        assert!(!b.try_consume(1));
        assert_eq!(b.remaining(), 0);
        let (used, remaining) = b.usage_and_remaining();
        assert_eq!(used, 2);
        assert_eq!(remaining, 0);
    }
}
