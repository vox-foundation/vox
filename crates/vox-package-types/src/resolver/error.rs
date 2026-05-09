//! Resolver error types.

use std::fmt;

#[derive(Debug)]
pub enum ResolverError {
    InvalidVersion(String),
    InvalidVersionReq(String),
    PackageNotFound(String),
    NoMatchingVersion(String, String),
    Conflict(String, String, String),
    CycleDetected(Vec<String>),
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersion(v) => write!(f, "Invalid version: {v}"),
            Self::InvalidVersionReq(r) => write!(f, "Invalid version requirement: {r}"),
            Self::PackageNotFound(n) => write!(f, "Package not found: {n}"),
            Self::NoMatchingVersion(n, r) => {
                write!(f, "No version of {n} matches requirement {r}")
            }
            Self::Conflict(n, v1, v2) => {
                write!(f, "Version conflict for {n}: {v1} vs {v2}")
            }
            Self::CycleDetected(path) => {
                write!(f, "Dependency cycle detected: {}", path.join(" -> "))
            }
        }
    }
}

impl std::error::Error for ResolverError {}
