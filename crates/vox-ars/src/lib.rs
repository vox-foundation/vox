//! Agent Runtime System (ARS) — sandbox isolation, trust classification,
//! and OpenClaw skill marketplace integration.
//!
//! # Migration note (CC-1)
//!
//! This crate is a thin re-export facade over [`vox_skills::ars_shim`].
//! The physical files live in `crates/vox-skills/src/ars_shim/` until
//! the `ars_shim` types are fully decoupled from the vox-skills crate root
//! (`SkillRegistry`, `install_builtins`, `parse_skill_md` cross-references).
//!
//! Consumers should import from `vox_ars` rather than `vox_skills::ars_shim`.
//! The underlying modules are re-exported here for full path compatibility:
//!
//! ```rust,ignore
//! // Old (deprecated path):
//! use vox_skills::ars_shim::OpenClawClient;
//!
//! // New (preferred):
//! use vox_ars::OpenClawClient;
//! ```
//!
//! See: `docs/src/architecture/vox-populi-extraction-followup-plan-2026.md` (CC-1)

pub use vox_skills::ars_shim::context;
pub use vox_skills::ars_shim::domain;
pub use vox_skills::ars_shim::executor;
pub use vox_skills::ars_shim::hooks;
pub use vox_skills::ars_shim::manifest;
pub use vox_skills::ars_shim::openclaw;
pub use vox_skills::ars_shim::openclaw_adapter;
pub use vox_skills::ars_shim::openclaw_discovery;
pub use vox_skills::ars_shim::openclaw_gateway_ws;
pub use vox_skills::ars_shim::openclaw_protocol;
pub use vox_skills::ars_shim::runtime;

// Flat re-exports mirroring the ars_shim mod.rs surface:
pub use vox_skills::ars_shim::ArsSkill;
pub use vox_skills::ars_shim::DefaultOpenClawRuntimeAdapter;
pub use vox_skills::ars_shim::OpenClawAdapterConfig;
pub use vox_skills::ars_shim::OpenClawAdapterError;
pub use vox_skills::ars_shim::OpenClawClient;
pub use vox_skills::ars_shim::OpenClawConnectionOverrides;
pub use vox_skills::ars_shim::OpenClawDiscoveryOverrides;
pub use vox_skills::ars_shim::OpenClawError;
pub use vox_skills::ars_shim::OpenClawGatewayWsClient;
pub use vox_skills::ars_shim::OpenClawGatewayWsConfig;
pub use vox_skills::ars_shim::OpenClawGatewayWsError;
pub use vox_skills::ars_shim::OpenClawRemoteConfig;
pub use vox_skills::ars_shim::OpenClawRuntimeAdapter;
pub use vox_skills::ars_shim::OpenClawSkillSpec;
pub use vox_skills::ars_shim::PublishResult;
pub use vox_skills::ars_shim::SkillCategory;
pub use vox_skills::ars_shim::SkillManifest;
pub use vox_skills::ars_shim::SkillPermission;
pub use vox_skills::ars_shim::SkillRegistry;
pub use vox_skills::ars_shim::adapter_config_with_token_override;
pub use vox_skills::ars_shim::connect_default_runtime_adapter;
pub use vox_skills::ars_shim::connect_runtime_adapter_with_overrides;
pub use vox_skills::ars_shim::install_builtins;
pub use vox_skills::ars_shim::resolve_adapter_config;
pub use vox_skills::ars_shim::resolve_openclaw_endpoints;

// Re-export discovery constants
pub use vox_skills::ars_shim::DEFAULT_HTTP_GATEWAY_URL;
pub use vox_skills::ars_shim::DEFAULT_WS_GATEWAY_URL;
pub use vox_skills::ars_shim::OpenClawResolvedEndpoints;
