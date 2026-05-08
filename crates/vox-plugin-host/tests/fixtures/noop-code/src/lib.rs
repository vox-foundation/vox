//! Noop code plugin for SP2 host loader tests. Exports a VoxPluginRoot with
//! the matching ABI version; the VoxPlugin trait object reports id "noop-code"
//! and shuts down cleanly. No extension points provided.

use abi_stable::{
    export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, std_types::*,
};
use vox_plugin_api::abi::{
    VoxPlugin, VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef,
};
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
    RString::from(r#"{"id":"noop-code","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = NoopPlugin;
    let to = VoxPlugin_TO::from_value(plugin, abi_stable::erased_types::TD_Opaque);
    RResult::ROk(to)
}

struct NoopPlugin;

impl VoxPlugin for NoopPlugin {
    fn id(&self) -> RString {
        RString::from("noop-code")
    }
    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
}
