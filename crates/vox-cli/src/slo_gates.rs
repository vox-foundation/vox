//! SLO gates and rollout criteria for VoxScript fast execution.
//!
//! Defines p50/p95 latency targets, cache hit goals, and phased rollout
//! criteria from sub-10s to sub-1s warm starts.

/// Phase 1: Warm pool established. p95 < 10s from POST to first output.
pub const PHASE1_P95_MS: u64 = 10_000;

/// Phase 2: Optimized. p50 < 1s for repeated/cache-hit runs.
pub const PHASE2_P50_WARM_MS: u64 = 1_000;

/// Target cache hit ratio for script hash cache (0.0–1.0).
pub const CACHE_HIT_TARGET: f64 = 0.8;

/// Check if a timing meets Phase 1 gate (p95 < 10s).
#[inline]
pub fn meets_phase1(total_ms: u64) -> bool {
    total_ms < PHASE1_P95_MS
}

/// Check if a timing meets Phase 2 gate (warm p50 < 1s).
#[inline]
pub fn meets_phase2_warm(total_ms: u64) -> bool {
    total_ms < PHASE2_P50_WARM_MS
}

/// Rollout phase based on current p95.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolloutPhase {
    /// Cold path only; p95 > 10s.
    Cold,
    /// Phase 1 done; p95 < 10s.
    Warm,
    /// Phase 2 done; warm p50 < 1s.
    Fast,
}

/// Determine rollout phase from p95 and warm p50.
pub fn rollout_phase(p95_ms: u64, warm_p50_ms: Option<u64>) -> RolloutPhase {
    if warm_p50_ms.is_some_and(|p| p < PHASE2_P50_WARM_MS) {
        RolloutPhase::Fast
    } else if p95_ms < PHASE1_P95_MS {
        RolloutPhase::Warm
    } else {
        RolloutPhase::Cold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase1_gate() {
        assert!(meets_phase1(9_999));
        assert!(!meets_phase1(10_000));
    }

    #[test]
    fn phase2_gate() {
        assert!(meets_phase2_warm(999));
        assert!(!meets_phase2_warm(1_000));
    }

    #[test]
    fn rollout_phase_cold_warm_fast() {
        assert_eq!(rollout_phase(15_000, None), RolloutPhase::Cold);
        assert_eq!(rollout_phase(5_000, None), RolloutPhase::Warm);
        assert_eq!(rollout_phase(5_000, Some(500)), RolloutPhase::Fast);
    }
}
