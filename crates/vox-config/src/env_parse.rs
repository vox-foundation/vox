//! Typed environment readers with defaults.
//!
//! **Secrets:** do not use this for API keys — resolve via `vox_clavis::resolve_secret` at the callsite.
//! This module is for numeric/timeouts and other non-secret operator tuning.

use std::time::Duration;

#[must_use]
pub fn parse_u64_opt(raw: Option<&str>, default: u64) -> u64 {
    raw.and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

#[must_use]
pub fn env_u64(name: &str, default: u64) -> u64 {
    parse_u64_opt(std::env::var(name).ok().as_deref(), default)
}

#[must_use]
pub fn parse_usize_opt(raw: Option<&str>, default: usize) -> usize {
    raw.and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

#[must_use]
pub fn env_usize(name: &str, default: usize) -> usize {
    parse_usize_opt(std::env::var(name).ok().as_deref(), default)
}

#[must_use]
pub fn env_duration_from_ms(name: &str, default_ms: u64) -> Duration {
    Duration::from_millis(env_u64(name, default_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_u64_trim_and_default() {
        assert_eq!(parse_u64_opt(None, 7), 7);
        assert_eq!(parse_u64_opt(Some(" 42 "), 7), 42);
        assert_eq!(parse_u64_opt(Some("nope"), 3), 3);
    }

    #[test]
    fn parse_usize_trim_and_default() {
        assert_eq!(parse_usize_opt(Some(" 9 "), 1), 9);
    }
}
