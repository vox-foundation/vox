use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchProviderId {
    Searxng,
    Tavily,
    DuckDuckGo,
}

#[derive(Debug, Clone)]
pub struct ProviderCircuitBreaker {
    pub consecutive_failures: u32,
    pub cooldown_until: Option<Instant>,
}

impl Default for ProviderCircuitBreaker {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            cooldown_until: None,
        }
    }
}

impl ProviderCircuitBreaker {
    pub fn record_failure(&mut self, is_rate_limit: bool) {
        self.consecutive_failures += 1;
        let base_delay_secs = if is_rate_limit { 60 } else { 5 };
        let delay =
            Duration::from_secs(base_delay_secs * 2u64.pow(self.consecutive_failures.min(4)));
        self.cooldown_until = Some(Instant::now() + delay);
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.cooldown_until = None;
    }

    pub fn is_available(&self) -> bool {
        match self.cooldown_until {
            Some(expiry) => Instant::now() >= expiry,
            None => true,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SearchProviderCircuitRegistry {
    breakers: Arc<Mutex<HashMap<SearchProviderId, ProviderCircuitBreaker>>>,
}

impl SearchProviderCircuitRegistry {
    pub fn new() -> Self {
        Self {
            breakers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<SearchProviderCircuitRegistry> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    pub fn is_available(&self, provider: SearchProviderId) -> bool {
        let guard = match self.breakers.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard
            .get(&provider)
            .map(|b| b.is_available())
            .unwrap_or(true)
    }

    pub fn record_failure(&self, provider: SearchProviderId, is_rate_limit: bool) {
        let mut guard = match self.breakers.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard
            .entry(provider)
            .or_default()
            .record_failure(is_rate_limit);
    }

    pub fn record_success(&self, provider: SearchProviderId) {
        let mut guard = match self.breakers.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.entry(provider).or_default().record_success();
    }
}
