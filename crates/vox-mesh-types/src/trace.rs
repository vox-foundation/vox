//! W3C `traceparent`-compatible mesh trace context.
//!
//! Provides a minimal, zero-external-dependency (uses `rand`) propagation
//! primitive that is forward-compatible with OpenTelemetry without pulling in
//! the OTel crate.  S2 will extend this to cross-node propagation; S1 wires
//! the local path only.
//!
//! # Wire format
//!
//! `00-{32 lowercase hex trace_id}-{16 lowercase hex span_id}-{2 hex flags}`
//!
//! Example: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`

use std::fmt;

use rand::RngCore;
use serde::{Deserialize, Serialize};

/// 16-byte trace identifier (128-bit, W3C `traceparent` field 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId([u8; 16]);

/// 8-byte span identifier (64-bit, W3C `traceparent` field 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId([u8; 8]);

impl TraceId {
    pub fn random() -> Self {
        let mut buf = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut buf);
        Self(buf)
    }

    pub fn to_hex(&self) -> String {
        hex_encode(&self.0)
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let bytes = hex_decode_fixed::<16>(s)?;
        if bytes == [0u8; 16] {
            return None; // W3C: all-zeros is invalid
        }
        Some(Self(bytes))
    }
}

impl SpanId {
    pub fn random() -> Self {
        let mut buf = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut buf);
        Self(buf)
    }

    pub fn to_hex(&self) -> String {
        hex_encode(&self.0)
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let bytes = hex_decode_fixed::<8>(s)?;
        if bytes == [0u8; 8] {
            return None; // W3C: all-zeros is invalid
        }
        Some(Self(bytes))
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Error returned by [`MeshTraceContext::from_traceparent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTraceparentError(String);

impl fmt::Display for ParseTraceparentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid traceparent: {}", self.0)
    }
}

impl std::error::Error for ParseTraceparentError {}

/// Minimal W3C-traceparent-compatible trace context.
///
/// Carries a `trace_id` (stable across the whole task), a `parent_span_id`
/// (identifies the producing span), and W3C `trace_flags` (bit 0 = sampled).
///
/// In S1 this context flows through the **local** path only:
/// orchestrator → populi A2A envelope → handler span.  Cross-node propagation
/// is wired in S2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshTraceContext {
    pub trace_id: TraceId,
    pub parent_span_id: SpanId,
    /// W3C trace flags byte (bit 0 = sampled).  Default `0x01` (always sample in S1).
    pub trace_flags: u8,
}

impl MeshTraceContext {
    /// Create a brand-new root context (fresh trace_id and span_id).
    pub fn new_root() -> Self {
        Self {
            trace_id: TraceId::random(),
            parent_span_id: SpanId::random(),
            trace_flags: 0x01,
        }
    }

    /// Parse a W3C traceparent string.
    ///
    /// Accepts version `00` only (the only currently defined version).
    pub fn from_traceparent(s: &str) -> Result<Self, ParseTraceparentError> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 4 {
            return Err(ParseTraceparentError(format!(
                "expected 4 dash-separated fields, got {}",
                parts.len()
            )));
        }
        if parts[0] != "00" {
            return Err(ParseTraceparentError(format!(
                "unsupported version {:?} (only '00' supported)",
                parts[0]
            )));
        }
        let trace_id = TraceId::from_hex(parts[1]).ok_or_else(|| {
            ParseTraceparentError(format!("invalid trace_id {:?}", parts[1]))
        })?;
        let parent_span_id = SpanId::from_hex(parts[2]).ok_or_else(|| {
            ParseTraceparentError(format!("invalid parent_span_id {:?}", parts[2]))
        })?;
        let flags_bytes =
            hex_decode_fixed::<1>(parts[3]).ok_or_else(|| {
                ParseTraceparentError(format!("invalid trace_flags {:?}", parts[3]))
            })?;
        Ok(Self {
            trace_id,
            parent_span_id,
            trace_flags: flags_bytes[0],
        })
    }

    /// Serialize to W3C traceparent string.
    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id.to_hex(),
            self.parent_span_id.to_hex(),
            self.trace_flags,
        )
    }

    /// Produce a child context: same `trace_id`, new random `parent_span_id`.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            parent_span_id: SpanId::random(),
            trace_flags: self.trace_flags,
        }
    }

    /// `trace_id` as a 32-char lowercase hex string (for span attributes).
    pub fn trace_id_hex(&self) -> String {
        self.trace_id.to_hex()
    }

    /// Whether the sampled bit (bit 0) is set.
    pub fn is_sampled(&self) -> bool {
        self.trace_flags & 0x01 != 0
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode_fixed<const N: usize>(s: &str) -> Option<[u8; N]> {
    if s.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    #[test]
    fn traceparent_round_trip() {
        let ctx = MeshTraceContext::from_traceparent(KNOWN).unwrap();
        assert_eq!(ctx.to_traceparent(), KNOWN);
        assert_eq!(ctx.trace_id_hex(), "4bf92f3577b34da6a3ce929d0e0e4736");
        assert!(ctx.is_sampled());
    }

    #[test]
    fn traceparent_rejects_malformed() {
        let cases = [
            "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01", // bad version
            "4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",    // missing version
            "00-ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ-00f067aa0ba902b7-01", // non-hex trace_id
            "00-00000000000000000000000000000000-00f067aa0ba902b7-01", // all-zero trace_id
            "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01", // all-zero span_id
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7",   // missing flags
            "",
        ];
        for bad in &cases {
            assert!(
                MeshTraceContext::from_traceparent(bad).is_err(),
                "should reject: {bad:?}"
            );
        }
    }

    #[test]
    fn child_preserves_trace_id_and_changes_span_id() {
        let parent = MeshTraceContext::new_root();
        let child = parent.child();
        assert_eq!(parent.trace_id, child.trace_id, "trace_id must be preserved");
        assert_ne!(
            parent.parent_span_id, child.parent_span_id,
            "span_id must change"
        );
        assert_eq!(parent.trace_flags, child.trace_flags);
    }

    #[test]
    fn new_root_produces_valid_traceparent() {
        let ctx = MeshTraceContext::new_root();
        let s = ctx.to_traceparent();
        let parsed = MeshTraceContext::from_traceparent(&s).unwrap();
        assert_eq!(ctx, parsed);
    }

    #[test]
    fn unsample_flag_preserved() {
        let s = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00";
        let ctx = MeshTraceContext::from_traceparent(s).unwrap();
        assert!(!ctx.is_sampled());
        assert_eq!(ctx.to_traceparent(), s);
    }
}
