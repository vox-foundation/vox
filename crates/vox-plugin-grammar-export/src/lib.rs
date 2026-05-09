//! Grammar Export plugin — exposes `vox-grammar-export` via the Vox plugin ABI.
//!
//! Existing consumers (`vox-cli`, `vox-compiler`, `vox-constrained-gen`,
//! `vox-orchestrator`, `vox-populi`) can continue to depend on the library
//! crate directly. This plugin allows the same capability to be installed and
//! dispatched through the plugin host for tooling that prefers runtime extension.
//!
//! Public re-exports let host code call grammar export without a direct library
//! dependency once the plugin is loaded:
//!
//! ```ignore
//! use vox_plugin_grammar_export::{export, GrammarExportConfig, GrammarFormat};
//! ```

use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, std_types::*};
use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;
use vox_plugin_api::abi::{VoxPlugin, VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef};
use vox_plugin_api::host::VoxHost_TO;

// Re-export the full public surface of the grammar-export library so that
// callers loading this plugin can reach it without a separate direct dep.
pub use vox_grammar_export::{
    GrammarExportConfig, GrammarExportResult, GrammarFormat, export,
    grammar_version_matches_compiler,
};

#[export_root_module]
fn root_module() -> VoxPluginRootRef {
    VoxPluginRoot {
        abi_version: VOX_PLUGIN_ABI_VERSION,
        manifest_json,
        init,
    }
    .leak_into_prefix()
}

#[sabi_extern_fn]
fn manifest_json() -> RString {
    RString::from(r#"{"id":"grammar-export","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    tracing::debug!("vox-plugin-grammar-export: initialising");
    let plugin = GrammarExportPlugin;
    let to = VoxPlugin_TO::from_value(plugin, abi_stable::erased_types::TD_Opaque);
    RResult::ROk(to)
}

struct GrammarExportPlugin;

impl VoxPlugin for GrammarExportPlugin {
    fn id(&self) -> RString {
        RString::from("grammar-export")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        tracing::debug!("vox-plugin-grammar-export: shutting down");
        RResult::ROk(())
    }
}
