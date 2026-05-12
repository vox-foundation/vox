//! Maps MCP tool names to coarse `mutation_kind` strings matching ACI contract enums.
//!
//! Implementation lives in [`vox_primitives::agentos_mutation`] (SSOT for orchestrator, MCP, and `std.agentos` in Vox).

pub use vox_primitives::agentos_mutation::mutation_kind_for_tool;
