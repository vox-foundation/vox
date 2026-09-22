//! Loose semver normalization and half-open interval claim validity checking.

use semver::{Version, VersionReq};

/// Normalizes loose semver strings into a strict [`semver::Version`].
///
/// Supports:
/// - Leading 'v', 'V', or '=' (e.g. "v1.25.0", "= 1.2.3")
/// - 1-part versions ("2" -> "2.0.0")
/// - 2-part versions ("1.0" -> "1.0.0")
/// - 3-part versions with optional prerelease and build metadata ("v2.1.0-alpha.1")
pub fn normalize_semver(s: &str) -> Option<Version> {
    let mut s = s.trim();
    if let Some(rest) = s.strip_prefix('=') {
        s = rest.trim();
    }
    if let Some(rest) = s.strip_prefix('v').or_else(|| s.strip_prefix('V')) {
        s = rest.trim();
    }
    if let Some(rest) = s.strip_prefix('=') {
        s = rest.trim();
    }

    if let Ok(v) = Version::parse(s) {
        return Some(v);
    }

    let (ver_core, extra) = if let Some(idx) = s.find(['-', '+']) {
        (&s[..idx], Some(&s[idx..]))
    } else {
        (s, None)
    };

    let parts: Vec<&str> = ver_core.split('.').collect();
    let padded = match parts.len() {
        1 if !parts[0].is_empty() => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        3 => format!("{}.{}.{}", parts[0], parts[1], parts[2]),
        _ => return None,
    };

    let reconstructed = match extra {
        Some(ext) => format!("{}{}", padded, ext),
        None => padded,
    };

    Version::parse(&reconstructed).ok()
}

/// Evaluates whether a claim is valid for a given target version according to:
/// 1. An optional semver requirement (e.g. "^1.2")
/// 2. A half-open interval `[valid_since, valid_until)`
///
/// Returns `Err(String)` if `target_version_str` or `version_req` cannot be parsed.
pub fn is_claim_valid_at_version(
    valid_since: Option<&str>,
    valid_until: Option<&str>,
    version_req: Option<&str>,
    target_version_str: &str,
) -> Result<bool, String> {
    let target = normalize_semver(target_version_str)
        .ok_or_else(|| format!("Invalid target version: '{target_version_str}'"))?;

    if let Some(req_str) = version_req {
        let req = VersionReq::parse(req_str)
            .map_err(|e| format!("Invalid version requirement '{req_str}': {e}"))?;
        if !req.matches(&target) {
            return Ok(false);
        }
    }

    if let Some(since_str) = valid_since
        && let Some(since) = normalize_semver(since_str)
        && target < since
    {
        return Ok(false);
    }

    if let Some(until_str) = valid_until
        && let Some(until) = normalize_semver(until_str)
        && target >= until
    {
        return Ok(false);
    }

    Ok(true)
}
