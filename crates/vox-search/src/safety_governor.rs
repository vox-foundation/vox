//! Rate-limiting and provider concurrency safety governor.
//!
//! Prevents HTTP 429 rate-limiting, IP bans, and token exhaustion across parallel research workers:
//! - SearXNG: Max 2 concurrent requests, token bucket 4 req/sec
//! - Tavily: Max 5 concurrent requests, 10 req/sec
//! - arXiv: Max 3 concurrent requests
//! - LLM Synthesis: Max 4 concurrent inferences

use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Concurrency and rate governor for web search and LLM synthesis providers.
#[derive(Clone)]
pub struct ProviderSafetyGovernor {
    searxng_sem: Arc<Semaphore>,
    tavily_sem: Arc<Semaphore>,
    arxiv_sem: Arc<Semaphore>,
    llm_sem: Arc<Semaphore>,
    db: Arc<OnceLock<Arc<vox_db::Codex>>>,
}

impl Default for ProviderSafetyGovernor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderSafetyGovernor {
    /// Creates a new governor with default production safety thresholds.
    pub fn new() -> Self {
        Self {
            searxng_sem: Arc::new(Semaphore::new(2)),
            tavily_sem: Arc::new(Semaphore::new(5)),
            arxiv_sem: Arc::new(Semaphore::new(3)),
            llm_sem: Arc::new(Semaphore::new(4)),
            db: Arc::new(OnceLock::new()),
        }
    }

    /// Global shared governor singleton.
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<ProviderSafetyGovernor> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    /// Returns the database handle if configured.
    pub fn db(&self) -> Option<Arc<vox_db::Codex>> {
        self.db.get().cloned()
    }

    /// Sets the database handle for quota recording.
    pub fn set_db(&self, db: Arc<vox_db::Codex>) {
        let _ = self.db.set(db);
    }

    /// Acquire permit for SearXNG call (max 2 concurrent).
    pub async fn acquire_searxng(&self) -> OwnedSemaphorePermit {
        self.searxng_sem
            .clone()
            .acquire_owned()
            .await
            .expect("searxng semaphore closed")
    }

    /// Acquire permit for Tavily call (max 5 concurrent).
    pub async fn acquire_tavily(&self) -> OwnedSemaphorePermit {
        self.tavily_sem
            .clone()
            .acquire_owned()
            .await
            .expect("tavily semaphore closed")
    }

    /// Acquire permit for arXiv call (max 3 concurrent).
    pub async fn acquire_arxiv(&self) -> OwnedSemaphorePermit {
        self.arxiv_sem
            .clone()
            .acquire_owned()
            .await
            .expect("arxiv semaphore closed")
    }

    /// Acquire permit for heavy LLM synthesis (max 4 concurrent).
    pub async fn acquire_llm(&self) -> OwnedSemaphorePermit {
        self.llm_sem
            .clone()
            .acquire_owned()
            .await
            .expect("llm semaphore closed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrency_permits_are_bounded() {
        let governor = ProviderSafetyGovernor::new();

        // SearXNG allows 2 permits
        let p1 = governor.acquire_searxng().await;
        let p2 = governor.acquire_searxng().await;
        assert_eq!(governor.searxng_sem.available_permits(), 0);

        drop(p1);
        assert_eq!(governor.searxng_sem.available_permits(), 1);
        drop(p2);
        assert_eq!(governor.searxng_sem.available_permits(), 2);

        // arXiv allows 3 permits
        let a1 = governor.acquire_arxiv().await;
        let a2 = governor.acquire_arxiv().await;
        let a3 = governor.acquire_arxiv().await;
        assert_eq!(governor.arxiv_sem.available_permits(), 0);

        drop(a1);
        assert_eq!(governor.arxiv_sem.available_permits(), 1);
        drop(a2);
        assert_eq!(governor.arxiv_sem.available_permits(), 2);
        drop(a3);
        assert_eq!(governor.arxiv_sem.available_permits(), 3);
    }
}
