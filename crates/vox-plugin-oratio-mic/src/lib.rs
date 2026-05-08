//! Vox plugin: oratio-mic
//!
//! Stub plugin that provides the AudioCapture extension point for low-level
//! microphone device access. SP7 scaffold — actual extraction from vox-oratio
//! is deferred. See src/mic.rs for TODO(SP7-followup) markers.

mod mic;

use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, std_types::*};
use vox_plugin_api::abi::{VoxPluginRef, VoxPluginRoot, VoxPluginRootRef};
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
    RString::from(r#"{"id":"oratio-mic","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    mic::make_plugin(host)
}
