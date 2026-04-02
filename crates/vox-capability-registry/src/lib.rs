//! Capability registry: transport-independent SSOT in [`contracts/capability/capability-registry.yaml`](../../contracts/capability/capability-registry.yaml).
//!
//! - Mens chat uses [`default_registry`](crate::default_registry) (curated MCP rows).
//! - `vox ci command-compliance` validates cross-registry consistency.
//! - [`build_model_manifest`](crate::manifest::build_model_manifest) emits planner / external-model JSON.

mod command_registry;
mod document;
mod ids;
mod loader;
mod manifest;
mod openai;
mod registry;
mod types;
mod validate;

pub use command_registry::{
    active_vox_cli_paths_from_command_registry_yaml, COMMAND_REGISTRY_REL,
};
pub use document::{
    CapabilityRegistryDoc, CuratedCapability, Exemptions, RuntimeBuiltinMap,
};
pub use ids::{implicit_cli_capability_id, implicit_mcp_capability_id};
pub use loader::{load_document, CAPABILITY_REGISTRY_REL};
pub use manifest::{build_model_manifest, ModelCapabilityManifest};
pub use openai::{capability_to_openai_function, mens_chat_parameters};
pub use registry::{bundled_document, default_registry, registry_from_document};
pub use validate::validate_cross_registry;
pub use types::{CapabilityDescriptor, CapabilityRegistry, InvocationForms, PopuliExposure};
