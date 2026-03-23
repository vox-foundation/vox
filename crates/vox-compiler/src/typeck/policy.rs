//! Type-checking policy hooks (rule packs, edition gates, optional Socrates integration).
//!
//! The Checker pipeline currently does not invoke this module; it exists as a stable
//! extension point and to satisfy `pub mod policy` in the crate root.

#![allow(dead_code)]

/// Placeholder policy handle until rules are attached to [`crate::typeck::checker::Checker`].
#[derive(Debug, Clone, Copy, Default)]
pub struct TypeckPolicy;

impl TypeckPolicy {
    #[must_use]
    pub fn default_policy() -> Self {
        Self
    }
}
