//! Version requirement parsing and matching.

use std::fmt;

use super::error::ResolverError;
use super::semver::SemVer;

/// A version requirement/range.
#[derive(Debug, Clone)]
pub enum VersionReq {
    /// `*` — any version
    Any,
    /// `^1.2.3` — compatible (Cargo-style default)
    Caret(SemVer),
    /// `~1.2.3` — approximately (allows patch bumps)
    Tilde(SemVer),
    /// `=1.2.3` — exact match
    Exact(SemVer),
    /// `>=1.2.3`
    Gte(SemVer),
    /// `>1.2.3`
    Gt(SemVer),
    /// `<=1.2.3`
    Lte(SemVer),
    /// `<1.2.3`
    Lt(SemVer),
    /// Intersection of multiple requirements, e.g. `>=1.0, <2.0`
    And(Vec<VersionReq>),
}

impl VersionReq {
    /// Parse a version requirement string.
    pub fn parse(s: &str) -> Result<Self, ResolverError> {
        let s = s.trim();
        if s == "*" {
            return Ok(Self::Any);
        }
        // Handle comma-separated compound requirements
        if s.contains(',') {
            let parts: Result<Vec<VersionReq>, _> =
                s.split(',').map(|p| VersionReq::parse(p.trim())).collect();
            return Ok(Self::And(parts?));
        }
        if let Some(rest) = s.strip_prefix("^") {
            return Ok(Self::Caret(SemVer::parse(rest)?));
        }
        if let Some(rest) = s.strip_prefix("~") {
            return Ok(Self::Tilde(SemVer::parse(rest)?));
        }
        if let Some(rest) = s.strip_prefix(">=") {
            return Ok(Self::Gte(SemVer::parse(rest)?));
        }
        if let Some(rest) = s.strip_prefix('>') {
            return Ok(Self::Gt(SemVer::parse(rest)?));
        }
        if let Some(rest) = s.strip_prefix("<=") {
            return Ok(Self::Lte(SemVer::parse(rest)?));
        }
        if let Some(rest) = s.strip_prefix('<') {
            return Ok(Self::Lt(SemVer::parse(rest)?));
        }
        if let Some(rest) = s.strip_prefix('=') {
            return Ok(Self::Exact(SemVer::parse(rest)?));
        }
        // Default: treat bare version as caret (Cargo convention)
        Ok(Self::Caret(SemVer::parse(s)?))
    }

    /// Check if a version matches this requirement.
    pub fn matches(&self, version: &SemVer) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(v) => version == v,
            Self::Caret(v) => {
                if v.major == 0 {
                    if v.minor == 0 {
                        // ^0.0.x — only exact patch
                        version.major == 0 && version.minor == 0 && version.patch == v.patch
                    } else {
                        // ^0.y.z — same minor
                        version.major == 0 && version.minor == v.minor && version.patch >= v.patch
                    }
                } else {
                    // ^x.y.z — same major, >= minor.patch
                    version.major == v.major && version >= v
                }
            }
            Self::Tilde(v) => {
                // ~x.y.z — same major.minor, >= patch
                version.major == v.major && version.minor == v.minor && version.patch >= v.patch
            }
            Self::Gte(v) => version >= v,
            Self::Gt(v) => version > v,
            Self::Lte(v) => version <= v,
            Self::Lt(v) => version < v,
            Self::And(reqs) => reqs.iter().all(|r| r.matches(version)),
        }
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => write!(f, "*"),
            Self::Caret(v) => write!(f, "^{v}"),
            Self::Tilde(v) => write!(f, "~{v}"),
            Self::Exact(v) => write!(f, "={v}"),
            Self::Gte(v) => write!(f, ">={v}"),
            Self::Gt(v) => write!(f, ">{v}"),
            Self::Lte(v) => write!(f, "<={v}"),
            Self::Lt(v) => write!(f, "<{v}"),
            Self::And(reqs) => {
                for (i, r) in reqs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{r}")?;
                }
                Ok(())
            }
        }
    }
}
