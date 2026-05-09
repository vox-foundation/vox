//! Parsed semantic versions.

use std::cmp::Ordering;
use std::fmt;

use super::error::ResolverError;

/// A parsed semantic version: major.minor.patch with optional pre-release.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

impl SemVer {
    /// Parse a version string like `"1.2.3"` or `"1.2.3-beta.1"`.
    pub fn parse(s: &str) -> Result<Self, ResolverError> {
        let s = s.trim().trim_start_matches('v');
        let (version_part, pre) = if let Some(idx) = s.find('-') {
            (&s[..idx], Some(s[idx + 1..].to_string()))
        } else {
            (s, None)
        };
        // Strip build metadata after +
        let version_part = version_part.split('+').next().unwrap_or(version_part);
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return Err(ResolverError::InvalidVersion(s.to_string()));
        }
        let major = parts[0]
            .parse()
            .map_err(|_| ResolverError::InvalidVersion(s.to_string()))?;
        let minor = if parts.len() > 1 {
            parts[1]
                .parse()
                .map_err(|_| ResolverError::InvalidVersion(s.to_string()))?
        } else {
            0
        };
        let patch = if parts.len() > 2 {
            parts[2]
                .parse()
                .map_err(|_| ResolverError::InvalidVersion(s.to_string()))?
        } else {
            0
        };
        Ok(Self {
            major,
            minor,
            patch,
            pre,
        })
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (Some(_), None) => Ordering::Less, // pre-release < release
                (None, Some(_)) => Ordering::Greater,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        Ok(())
    }
}
