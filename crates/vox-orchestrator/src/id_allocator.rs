//! Deterministic Entity ID Allocator (`VoxIdAllocatorV2` — `{ allocate, derive }`).
//!
//! Adapted from `packages/provider-core/src/IdAllocator.ts` (`IdAllocatorV2`):
//! - `allocate()`: generates a fresh root entity UUIDv4 (or deterministic seeded UUID in replay/test mode).
//! - `derive(namespace, parts)`: deterministically constructs a child entity ID from a namespace
//!   and parent/part segments using RFC 3986 unreserved percent-encoding joined with `:`.

use std::sync::atomic::{AtomicU64, Ordering};

/// RFC 3986 unreserved-set percent-encoder for deterministic entity ID segments.
/// Encodes `:`, `/`, `%`, whitespace, and non-unreserved bytes so `derive` is injective.
pub fn percent_encode_id_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for b in segment.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(
                    char::from_digit((b >> 4) as u32, 16)
                        .unwrap_or('0')
                        .to_ascii_uppercase(),
                );
                out.push(
                    char::from_digit((b & 0x0f) as u32, 16)
                        .unwrap_or('0')
                        .to_ascii_uppercase(),
                );
            }
        }
    }
    out
}

/// Deterministic and replay-safe entity ID allocator (`allocate` + `derive`).
pub trait VoxIdAllocatorV2: Send + Sync {
    /// Allocate a fresh root entity ID (UUIDv4 in production; deterministic sequence in replay tests).
    fn allocate(&self) -> String;

    /// Deterministically derive a child entity ID from `namespace` and `parts`.
    /// Each segment is URI-component encoded and joined by `:`.
    fn derive(&self, namespace: &str, parts: &[&str]) -> String {
        let mut out = percent_encode_id_segment(namespace);
        for part in parts {
            out.push(':');
            out.push_str(&percent_encode_id_segment(part));
        }
        out
    }
}

/// Production allocator using OS CSPRNG UUIDv4 for root allocations and deterministic `derive`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRandomIdAllocator;

impl VoxIdAllocatorV2 for SystemRandomIdAllocator {
    fn allocate(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

/// Seeded deterministic allocator for event-stream replay and unit tests.
#[derive(Debug)]
pub struct SeededDeterministicIdAllocator {
    prefix: String,
    counter: AtomicU64,
}

impl SeededDeterministicIdAllocator {
    /// Create a new seeded allocator with the given prefix and starting counter (`1`).
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            counter: AtomicU64::new(1),
        }
    }
}

impl VoxIdAllocatorV2 for SeededDeterministicIdAllocator {
    fn allocate(&self) -> String {
        let seq = self.counter.fetch_add(1, Ordering::SeqCst);
        format!("{}-{seq:08x}", self.prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SeededDeterministicIdAllocator, SystemRandomIdAllocator, VoxIdAllocatorV2,
        percent_encode_id_segment,
    };

    #[test]
    fn percent_encode_preserves_unreserved_and_escapes_delimiters() {
        assert_eq!(
            percent_encode_id_segment("thread-1_a.b~c"),
            "thread-1_a.b~c"
        );
        assert_eq!(percent_encode_id_segment("thread:1"), "thread%3A1");
        assert_eq!(percent_encode_id_segment("item/2:a"), "item%2F2%3Aa");
    }

    #[test]
    fn derive_produces_deterministic_injective_ids_across_replays() {
        let alloc = SystemRandomIdAllocator;
        let d1 = alloc.derive("turn", &["thread:1", "item/2:a"]);
        let d2 = alloc.derive("turn", &["thread:1", "item/2:a"]);
        assert_eq!(d1, "turn:thread%3A1:item%2F2%3Aa");
        assert_eq!(d1, d2);

        // Distinct boundary splits never collide because `:` inside a segment is encoded as `%3A`.
        let split_a = alloc.derive("turn", &["a:b", "c"]);
        let split_b = alloc.derive("turn", &["a", "b:c"]);
        assert_ne!(split_a, split_b);
    }

    #[test]
    fn seeded_allocator_replays_identical_sequence() {
        let a = SeededDeterministicIdAllocator::new("replay");
        let b = SeededDeterministicIdAllocator::new("replay");
        assert_eq!(a.allocate(), "replay-00000001");
        assert_eq!(a.allocate(), "replay-00000002");
        assert_eq!(b.allocate(), "replay-00000001");
        assert_eq!(b.allocate(), "replay-00000002");
    }
}
