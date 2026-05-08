use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use vox_secrets::{SecretId, resolve_secret};

/// Thread-safe atomic credit counter for one MCP/CLI session.
#[derive(Debug, Clone)]
pub struct TavilySessionBudget {
    remaining: Arc<AtomicUsize>,
}

impl TavilySessionBudget {
    pub fn new(limit: usize) -> Self {
        Self {
            remaining: Arc::new(AtomicUsize::new(limit)),
        }
    }

    /// Returns `false` and does NOT decrement if already at zero.
    pub fn try_consume(&self, cost: usize) -> bool {
        let mut current = self.remaining.load(Ordering::SeqCst);
        loop {
            if current < cost {
                return false;
            }
            match self.remaining.compare_exchange_weak(
                current,
                current - cost,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(val) => current = val,
            }
        }
    }

    pub fn remaining(&self) -> usize {
        self.remaining.load(Ordering::SeqCst)
    }
}

#[cfg(feature = "tavily")]
use tavily::{SearchRequest, Tavily};

#[cfg(feature = "tavily")]
pub struct TavilySearchClient {
    inner: Tavily,
}

#[cfg(feature = "tavily")]
impl TavilySearchClient {
    pub fn from_env() -> Option<Self> {
        let binding = resolve_secret(SecretId::TavilyApiKey);
        let key_str = binding.expose()?;
        let client = Tavily::builder(key_str)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;
        Some(Self { inner: client })
    }

    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
        depth: &str,
    ) -> Result<Vec<TavilyHit>, String> {
        let req = SearchRequest::new(
            // key is internally stored in client
            "", query,
        )
        .search_depth(depth)
        .max_results(max_results as i32);

        let resp = self
            .inner
            .call(&req)
            .await
            .map_err(|e| format!("tavily_search_failed:{e}"))?;

        Ok(resp
            .results
            .into_iter()
            .map(|r| TavilyHit {
                url: r.url,
                title: r.title,
                content: r.content,
                score: r.score,
            })
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct TavilyHit {
    pub url: String,
    pub title: String,
    pub content: String,
    pub score: f32,
}

#[cfg(test)]
mod budget_tests {
    use super::TavilySessionBudget;

    #[test]
    fn tavily_session_budget_exhausts_after_limit() {
        let b = TavilySessionBudget::new(2);
        assert!(b.try_consume(1));
        assert!(b.try_consume(1));
        assert!(!b.try_consume(1));
        assert_eq!(b.remaining(), 0);
    }
}
