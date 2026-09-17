use std::sync::Arc;
use std::thread;
use vox_search::search_circuit_breaker::{
    ProviderCircuitBreaker, SearchProviderCircuitRegistry, SearchProviderId,
};

#[test]
fn test_circuit_breaker_cooldown_on_rate_limit() {
    let mut breaker = ProviderCircuitBreaker::default();
    assert!(breaker.is_available());

    // Record HTTP 429 rate limit failure
    breaker.record_failure(true);
    assert!(!breaker.is_available());

    // Reset on success
    breaker.record_success();
    assert!(breaker.is_available());
}

#[test]
fn test_successive_failure_backoff() {
    let mut breaker = ProviderCircuitBreaker::default();
    assert_eq!(breaker.consecutive_failures, 0);

    breaker.record_failure(false);
    assert_eq!(breaker.consecutive_failures, 1);
    assert!(!breaker.is_available());

    breaker.record_failure(false);
    assert_eq!(breaker.consecutive_failures, 2);

    breaker.record_failure(true);
    assert_eq!(breaker.consecutive_failures, 3);

    breaker.record_success();
    assert_eq!(breaker.consecutive_failures, 0);
    assert!(breaker.is_available());
}

#[test]
fn test_registry_availability_and_isolation() {
    let registry = SearchProviderCircuitRegistry::new();

    assert!(registry.is_available(SearchProviderId::Searxng));
    assert!(registry.is_available(SearchProviderId::Tavily));
    assert!(registry.is_available(SearchProviderId::DuckDuckGo));

    // Rate limit Tavily
    registry.record_failure(SearchProviderId::Tavily, true);
    assert!(registry.is_available(SearchProviderId::Searxng));
    assert!(!registry.is_available(SearchProviderId::Tavily));
    assert!(registry.is_available(SearchProviderId::DuckDuckGo));

    // Recover Tavily
    registry.record_success(SearchProviderId::Tavily);
    assert!(registry.is_available(SearchProviderId::Tavily));
}

#[test]
fn test_registry_thread_safety() {
    let registry = Arc::new(SearchProviderCircuitRegistry::new());
    let mut handles = Vec::new();

    for i in 0..10 {
        let reg = Arc::clone(&registry);
        handles.push(thread::spawn(move || {
            let provider = match i % 3 {
                0 => SearchProviderId::Searxng,
                1 => SearchProviderId::Tavily,
                _ => SearchProviderId::DuckDuckGo,
            };
            reg.record_failure(provider, false);
            let _ = reg.is_available(provider);
            reg.record_success(provider);
            assert!(reg.is_available(provider));
        }));
    }

    for handle in handles {
        handle.join().expect("thread joined successfully");
    }
}

#[tokio::test]
async fn test_dispatcher_skips_cooldown_provider() {
    use vox_search::policy::SearchPolicy;
    use vox_search::web_dispatcher::WebSearchDispatcher;

    let registry = SearchProviderCircuitRegistry::new();
    // Simulate SearXNG in cooldown
    registry.record_failure(SearchProviderId::Searxng, true);
    assert!(!registry.is_available(SearchProviderId::Searxng));

    let policy = SearchPolicy {
        searxng_url: Some("http://invalid.searxng.test".to_string()),
        duckduckgo_fallback_enabled: false,
        tavily_enabled: false,
        wikipedia_fallback_enabled: false,
        ..SearchPolicy::default()
    };

    // Because SearXNG is in cooldown and DDG/Tavily are disabled, it should return Ok(empty) immediately without network error
    let hits = WebSearchDispatcher::search_with_registry("query", &policy, &registry)
        .await
        .expect("dispatcher search");
    assert!(hits.is_empty());
}

#[tokio::test]
async fn test_dispatcher_records_failure_on_error() {
    use vox_search::policy::SearchPolicy;
    use vox_search::web_dispatcher::WebSearchDispatcher;

    let registry = SearchProviderCircuitRegistry::new();
    assert!(registry.is_available(SearchProviderId::Searxng));

    let policy = SearchPolicy {
        searxng_url: Some("http://127.0.0.1:9".to_string()),
        duckduckgo_fallback_enabled: false,
        tavily_enabled: false,
        ..SearchPolicy::default()
    };

    let _ = WebSearchDispatcher::search_with_registry("query", &policy, &registry).await;
    // Failure should have been recorded, putting SearXNG in cooldown
    assert!(!registry.is_available(SearchProviderId::Searxng));
}
