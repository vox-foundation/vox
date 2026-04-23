//! Unified container sandboxing for community skill execution.
//!
//! Uses the existing `vox-container` infrastructure (the same Docker/Podman runtime
//! used for `.vox` application deployment) to sandbox untrusted community skills.
//!
//! ## Architecture
//!
//! ```text
//! ArsRuntime::run_skill()
//!     ├── ApprovalGuard::check()           ← Option C: explicit approval gate
//!     ├── policy::resolve_policy()         ← determine isolation tier
//!     └── SandboxedSkillRunner::run()      ← container execution via vox-container
//!             └── fallback: OpenClawSidecarSandbox  ← when no local runtime
//! ```
//!
//! All public items are re-exported from this module.

pub mod fallback;
pub mod image;
pub mod policy;
pub mod runner;

pub use policy::{ApprovalGuard, PolicyError, SandboxPolicy, resolve_policy};
pub use runner::{SandboxError, SandboxedSkillRunner, SkillOutput};
