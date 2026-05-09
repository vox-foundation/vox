//! Semantic versions and dependency resolution helpers.

mod error;
mod resolve;
mod semver;
mod version_req;

pub use error::ResolverError;
pub use resolve::{AvailablePackage, ResolvedDep, Resolver};
pub use semver::SemVer;
pub use version_req::VersionReq;

#[cfg(test)]
mod tests;
