//! Vox plugin: oratio
//!
//! Stub plugin that provides the AudioCapture extension point for the
//! Oratio speech-to-code pipeline. SP7 scaffold — actual extraction from
//! vox-oratio is deferred. See src/audio.rs for TODO(SP7-followup) markers.

mod audio;

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
    RString::from(r#"{"id":"oratio","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    audio::make_plugin(host)
}
