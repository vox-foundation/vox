//! vox-plugin-populi-mesh — Populi mesh transport composite plugin.
//!
//! Code side (cdylib): MeshDriver impl. Skill side: populi.skill.md
//! describing the mesh tools to agents.
//!
//! Initial implementation stubs MeshDriver methods with "not yet ported"
//! errors. Follow-up batch will copy the actual mesh transport code from
//! vox-populi (currently behind feature `transport`).

mod mesh;

use abi_stable::{
    erased_types::TD_Opaque, export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn,
    std_types::*,
};
use vox_plugin_api::abi::{VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef};
use vox_plugin_api::host::VoxHost_TO;
use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;

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
    RString::from(r#"{"id":"populi-mesh","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = mesh::PopuliMeshPlugin::new();
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}
