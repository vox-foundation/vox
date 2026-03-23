//! `vox-forge` — Platform-agnostic Git forge integration for Vox.
//!
//! Abstracts **GitHub** and **GitLab** behind a single trait.
//! All forge-specific API logic lives in the per-platform modules below;
//! callers only depend on [`GitForgeProvider`].
//!
//! ## Forge coverage
//! | Forge    | Feature flag | API basis        | Self-hostable |
//! |----------|-------------|------------------|---------------|
//! | GitHub   | `github`    | REST + GraphQL   | Enterprise only |
//! | GitLab   | `gitlab`    | REST             | ✅ CE (free) |
//!
//! ## Platform independence
//! All internal Vox code uses `ChangeRequest` instead of "PR" or "MR".

/// Error types for forge HTTP/auth/parse failures.
pub mod error;
/// [`GitForgeProvider`](provider::GitForgeProvider) trait and registry.
pub mod provider;
/// Forge-neutral DTOs (change requests, labels, webhooks, …).
pub mod types;

// Platform implementations — compiled only when the relevant feature is enabled.
/// GitHub REST (`api.github.com` or Enterprise base URL).
#[cfg(feature = "github")]
pub mod github;
/// GitLab REST API.
#[cfg(feature = "gitlab")]
pub mod gitlab;

pub use error::ForgeError;
pub use provider::GitForgeProvider;
pub use types::{
    ChangeRequest, ChangeRequestId, ChangeRequestState, ChangeRequestStatus, ForgeRepoInfo,
    ForgeUser, Label, NewChangeRequest, Review, ReviewState, WebhookEvent,
};
