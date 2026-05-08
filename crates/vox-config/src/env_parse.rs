//! Typed environment readers with defaults.
//!
//! **Secrets:** do not use this for API keys — resolve via `vox_secrets::resolve_secret` at the callsite.
//! This module is for numeric/timeouts and other non-secret operator tuning.

use std::time::Duration;

use crate::toml_config;

#[must_use]
pub fn parse_u64_opt(raw: Option<&str>, default: u64) -> u64 {
    raw.and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

/// Resolve a non-secret string config value with layered precedence:
/// 1. env var (highest — CI/override)
/// 2. `~/.vox/config.toml`
/// 3. compiled default
///
/// Do NOT use this for secrets — use `vox_secrets::resolve_secret`.
#[must_use]
pub fn resolve_config_str(name: &str, default: &str) -> String {
    if let Ok(v) = std::env::var(name)
        && !v.trim().is_empty()
    {
        return v;
    }
    if let Some(v) = toml_config::load_user_config().values.get(name) {
        if let Some(s) = v.as_str() {
            return s.to_string();
        } else if let Some(i) = v.as_integer() {
            return i.to_string();
        } else if let Some(f) = v.as_float() {
            return f.to_string();
        } else if let Some(b) = v.as_bool() {
            return b.to_string();
        }
    }
    default.to_string()
}

/// Resolve a u64 config value using layered precedence.
#[must_use]
pub fn resolve_config_u64(name: &str, default: u64) -> u64 {
    if let Ok(v) = std::env::var(name)
        && let Ok(parsed) = v.trim().parse::<u64>()
    {
        return parsed;
    }
    if let Some(v) = toml_config::load_user_config().values.get(name) {
        if let Some(i) = v.as_integer() {
            if i >= 0 {
                return i as u64;
            }
        } else if let Some(s) = v.as_str()
            && let Ok(parsed) = s.trim().parse::<u64>()
        {
            return parsed;
        }
    }
    default
}

/// Resolve a usize config value using layered precedence.
#[must_use]
pub fn resolve_config_usize(name: &str, default: usize) -> usize {
    if let Ok(v) = std::env::var(name)
        && let Ok(parsed) = v.trim().parse::<usize>()
    {
        return parsed;
    }
    if let Some(v) = toml_config::load_user_config().values.get(name) {
        if let Some(i) = v.as_integer() {
            if i >= 0 {
                return i as usize;
            }
        } else if let Some(s) = v.as_str()
            && let Ok(parsed) = s.trim().parse::<usize>()
        {
            return parsed;
        }
    }
    default
}

/// Resolve a bool config value using layered precedence.
#[must_use]
pub fn resolve_config_bool(name: &str, default: bool) -> bool {
    if let Ok(v) = std::env::var(name) {
        let t = v.trim().to_ascii_lowercase();
        if t == "1" || t == "true" || t == "yes" || t == "on" {
            return true;
        } else if t == "0" || t == "false" || t == "no" || t == "off" {
            return false;
        }
    }
    if let Some(v) = toml_config::load_user_config().values.get(name) {
        if let Some(b) = v.as_bool() {
            return b;
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_ascii_lowercase();
            if t == "1" || t == "true" || t == "yes" || t == "on" {
                return true;
            } else if t == "0" || t == "false" || t == "no" || t == "off" {
                return false;
            }
        }
    }
    default
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
