use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct UsageTracker {
    pub input_tokens: AtomicU64,
    pub output_tokens: AtomicU64,
    pub total_cost_usd_micros: AtomicU64,
}

impl UsageTracker {
    pub fn record(&self, input: u32, output: u32, cost_usd: f64) {
        self.input_tokens.fetch_add(input as u64, Ordering::Relaxed);
        self.output_tokens
            .fetch_add(output as u64, Ordering::Relaxed);
        let micros = (cost_usd * 1_000_000.0) as u64;
        self.total_cost_usd_micros
            .fetch_add(micros, Ordering::Relaxed);
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens.load(Ordering::Relaxed) + self.output_tokens.load(Ordering::Relaxed)
    }

    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_usd_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }
}
