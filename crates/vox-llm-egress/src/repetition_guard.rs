//! O(1) 1024-Slot Ring-Buffer `RepetitionGuard` (`T3-GUARD-REFUSAL-01`).
//!
//! Adapted from `shaltielshmid/NotT3Chat` `backend/Program.cs` (`RepetitionGuard` lines 884–941):
//! - Uses a power-of-2 ring buffer (`REPETITION_BUFFER_SIZE = 1024`, mask `1023`) of `u64` token hashes.
//! - Uses `vox_crypto::fast_hash` (XXH3) when the caller passes a hash or computes a deterministic
//!   64-bit hash via a caller-supplied closure / default `std::hash::Hasher` in pure-wire contexts,
//!   while `vox-orchestrator-mcp` wires `vox_crypto::fast_hash`.
//! - Detects:
//!   1. **Single-token degenerate repetition:** `>= MAX_SINGLE_TOKEN_REP` (`32`) consecutive identical non-whitespace chunks.
//!   2. **Multi-token n-gram loops:** Any period `p` in `MIN_NGRAM_PERIOD..=MAX_NGRAM_PERIOD` (`3..=12`)
//!      repeated `MIN_NGRAM_REPETITIONS` (`5`) consecutive times (`15..=60` chunks).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Power-of-2 ring buffer capacity (`1024` slots).
pub const REPETITION_BUFFER_SIZE: usize = 1024;
const REPETITION_BUFFER_MASK: usize = REPETITION_BUFFER_SIZE - 1;

/// Maximum consecutive identical non-whitespace token chunks before tripping.
pub const MAX_SINGLE_TOKEN_REP: usize = 32;

/// Minimum n-gram period (`3` tokens) checked for periodic loop detection.
pub const MIN_NGRAM_PERIOD: usize = 3;

/// Maximum n-gram period (`12` tokens) checked for periodic loop detection.
pub const MAX_NGRAM_PERIOD: usize = 12;

/// Number of consecutive period repetitions (`5`) required to trip an n-gram loop.
pub const MIN_NGRAM_REPETITIONS: usize = 5;

/// Reason a streaming generation was aborted by `RepetitionGuard`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepetitionTripped {
    /// A single non-whitespace token chunk repeated `count` consecutive times (`>= 32`).
    SingleToken { count: usize },
    /// A periodic n-gram of length `period` (`3..=12`) repeated `repetitions` (`5`) consecutive times.
    NGramLoop { period: usize, repetitions: usize },
}

/// O(1) per-token streaming loop breaker (`NotT3Chat` `RepetitionGuard` pattern).
#[derive(Debug, Clone)]
pub struct RepetitionGuard {
    ring: [u64; REPETITION_BUFFER_SIZE],
    write_pos: usize,
    total_written: usize,
    last_hash: u64,
    consecutive_same: usize,
}

impl Default for RepetitionGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl RepetitionGuard {
    /// Construct a fresh `RepetitionGuard` with a zeroed 1024-slot ring buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ring: [0u64; REPETITION_BUFFER_SIZE],
            write_pos: 0,
            total_written: 0,
            last_hash: 0,
            consecutive_same: 0,
        }
    }

    /// Observe a pre-hashed token chunk (`token_hash`, e.g. from `vox_crypto::fast_hash(chunk.as_bytes())`)
    /// after whitespace filtering.
    pub fn observe_hash(&mut self, token_hash: u64) -> Option<RepetitionTripped> {
        if self.total_written > 0 && token_hash == self.last_hash {
            self.consecutive_same += 1;
            if self.consecutive_same >= MAX_SINGLE_TOKEN_REP {
                return Some(RepetitionTripped::SingleToken {
                    count: self.consecutive_same,
                });
            }
        } else {
            self.last_hash = token_hash;
            self.consecutive_same = 1;
        }

        self.ring[self.write_pos] = token_hash;
        self.write_pos = (self.write_pos + 1) & REPETITION_BUFFER_MASK;
        self.total_written += 1;

        for period in MIN_NGRAM_PERIOD..=MAX_NGRAM_PERIOD {
            let window_len = period * MIN_NGRAM_REPETITIONS;
            if self.total_written < window_len {
                continue;
            }
            if self.matches_periodic_tail(period, MIN_NGRAM_REPETITIONS) {
                return Some(RepetitionTripped::NGramLoop {
                    period,
                    repetitions: MIN_NGRAM_REPETITIONS,
                });
            }
        }
        None
    }

    /// Observe a raw text chunk. Pure-whitespace chunks (`" "`, `"\n"`, indentation) are ignored
    /// so markdown tables and indented code blocks never false-trip.
    pub fn observe_chunk_with_hasher<F>(
        &mut self,
        chunk: &str,
        hash_fn: F,
    ) -> Option<RepetitionTripped>
    where
        F: FnOnce(&[u8]) -> u64,
    {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            return None;
        }
        let h = hash_fn(trimmed.as_bytes());
        self.observe_hash(h)
    }

    /// Observe a raw text chunk using the default 64-bit hasher.
    pub fn observe_chunk(&mut self, chunk: &str) -> Option<RepetitionTripped> {
        self.observe_chunk_with_hasher(chunk, |bytes| {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            hasher.finish()
        })
    }

    fn ring_at_offset_from_head(&self, back_offset: usize) -> u64 {
        let idx = (self.write_pos + REPETITION_BUFFER_SIZE - back_offset) & REPETITION_BUFFER_MASK;
        self.ring[idx]
    }

    fn matches_periodic_tail(&self, period: usize, repetitions: usize) -> bool {
        // Ensure the pattern isn't just a single token repeated (already handled by MAX_SINGLE_TOKEN_REP).
        let first = self.ring_at_offset_from_head(1);
        let mut all_identical = true;
        for offset in 1..=period {
            if self.ring_at_offset_from_head(offset) != first {
                all_identical = false;
                break;
            }
        }
        if all_identical {
            return false;
        }

        for i in 1..=period {
            let expected = self.ring_at_offset_from_head(i);
            for rep in 1..repetitions {
                let candidate = self.ring_at_offset_from_head(i + rep * period);
                if candidate != expected {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_SINGLE_TOKEN_REP, MIN_NGRAM_REPETITIONS, RepetitionGuard, RepetitionTripped};

    #[test]
    fn single_token_repetition_trips_at_32_and_ignores_whitespace() {
        let mut guard = RepetitionGuard::new();
        for _ in 0..100 {
            assert_eq!(guard.observe_chunk("   \n\t  "), None);
        }
        for i in 1..MAX_SINGLE_TOKEN_REP {
            assert_eq!(
                guard.observe_chunk("stuck"),
                None,
                "should not trip at iteration {i}"
            );
        }
        assert_eq!(
            guard.observe_chunk("stuck"),
            Some(RepetitionTripped::SingleToken {
                count: MAX_SINGLE_TOKEN_REP
            })
        );
    }

    #[test]
    fn ngram_loop_trips_at_5_repetitions_for_periods_3_to_12() {
        let mut guard = RepetitionGuard::new();
        let pattern = ["alpha", "beta", "gamma", "delta"];
        let mut tripped = None;
        for _ in 0..MIN_NGRAM_REPETITIONS {
            for tok in pattern {
                if let Some(t) = guard.observe_chunk(tok) {
                    tripped = Some(t);
                    break;
                }
            }
        }
        assert_eq!(
            tripped,
            Some(RepetitionTripped::NGramLoop {
                period: 4,
                repetitions: MIN_NGRAM_REPETITIONS
            })
        );
    }
}
