use crate::hir::{
    HirActivity, HirActor, HirAgent, HirFn, HirImport, HirMcpResource, HirMcpTool, HirRoute,
    HirRustImport, HirServerFn, HirTable, HirTypeDef, HirWorkflow,
};
use crate::web_ir::WebIrModule;
use serde::{Deserialize, Serialize};

/// The General Vox IR module structure, representing a machine-verifiable
/// and platform-agnostic view of a Vox program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxIrModule {
    pub version: String,
    pub metadata: VoxIrMetadata,
    pub module: VoxIrContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxIrMetadata {
    pub compiler_version: String,
    pub generated_at: String,
    pub source_hash: String,
}

/// The internal structure containing the lowered program logic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoxIrContent {
    pub imports: Vec<HirImport>,
    pub rust_imports: Vec<HirRustImport>,
    pub functions: Vec<HirFn>,
    pub types: Vec<HirTypeDef>,
    pub routes: Vec<HirRoute>,
    pub actors: Vec<HirActor>,
    pub workflows: Vec<HirWorkflow>,
    pub activities: Vec<HirActivity>,
    pub server_fns: Vec<HirServerFn>,
    pub query_fns: Vec<HirServerFn>,
    pub mutation_fns: Vec<HirServerFn>,
    pub tables: Vec<HirTable>,
    pub mcp_tools: Vec<HirMcpTool>,
    pub mcp_resources: Vec<HirMcpResource>,
    pub agents: Vec<HirAgent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_ir: Option<WebIrModule>,
}

pub mod lower;
