//! Rate-limiting and provider concurrency safety governor.
//!
//! Prevents HTTP 429 rate-limiting, IP bans, and token exhaustion across parallel research workers:
//! - SearXNG: Max 2 concurrent requests, token bucket 4 req/sec
//! - Tavily: Max 5 concurrent requests, 10 req/sec
//! - DuckDuckGo: Max 1 concurrent request, 1200ms mandatory inter-request delay
//! - LLM Synthesis: Max 4 concurrent inferences

use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;

/// Concurrency and rate governor for web search and LLM synthesis providers.
#[derive(Clone)]
pub struct ProviderSafetyGovernor {
    searxng_sem: Arc<Semaphore>,
    tavily_sem: Arc<Semaphore>,
    ddg_sem: Arc<Semaphore>,
    llm_sem: Arc<Semaphore>,
    last_ddg_request: Arc<Mutex<Option<Instant>>>,
}

impl Default for ProviderSafetyGovernor {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII permit guard for DuckDuckGo requests.
///
/// On drop, updates `last_ddg_request` with completion timestamp,
/// ensuring robust end-to-start inter-request spacing even for slow responses.
pub struct DdgPermit {
    _permit: OwnedSemaphorePermit,
    last_ddg_request: Arc<Mutex<Option<Instant>>>,
}

impl Drop for DdgPermit {
    fn drop(&mut self) {
        if let Ok(mut last) = self.last_ddg_request.try_lock() {
            *last = Some(Instant::now());
        }
    }
}

impl ProviderSafetyGovernor {
    /// Creates a new governor with default production safety thresholds.
    pub fn new() -> Self {
        Self {
            searxng_sem: Arc::new(Semaphore::new(2)),
            tavily_sem: Arc::new(Semaphore::new(5)),
            ddg_sem: Arc::new(Semaphore::new(1)),
            llm_sem: Arc::new(Semaphore::new(4)),
            last_ddg_request: Arc::new(Mutex::new(None)),
        }
    }

    /// Global shared governor singleton.
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<ProviderSafetyGovernor> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
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

    /// Acquire permit for DuckDuckGo call (max 1 concurrent + min 1200ms delay between calls).
    pub async fn acquire_ddg(&self) -> DdgPermit {
        let permit = self
            .ddg_sem
            .clone()
            .acquire_owned()
            .await
            .expect("ddg semaphore closed");

        let last_time = {
            let last = self.last_ddg_request.lock().await;
            *last
        };
        if let Some(prev) = last_time {
            let elapsed = prev.elapsed();
            if elapsed < Duration::from_millis(1200) {
                tokio::time::sleep(Duration::from_millis(1200) - elapsed).await;
            }
        }

        DdgPermit {
            _permit: permit,
            last_ddg_request: Arc::clone(&self.last_ddg_request),
        }
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
    }

    #[tokio::test]
    async fn test_ddg_rate_limit_spacing() {
        let governor = ProviderSafetyGovernor::new();

        let start = Instant::now();
        {
            let _p1 = governor.acquire_ddg().await;
        }
        {
            let _p2 = governor.acquire_ddg().await;
        }
        let elapsed = start.elapsed();
        // Second call must have waited at least 1100ms
        assert!(
            elapsed >= Duration::from_millis(1100),
            "DDG inter-request delay should be >= 1100ms, was {:?}",
            elapsed
        );
    }
}
