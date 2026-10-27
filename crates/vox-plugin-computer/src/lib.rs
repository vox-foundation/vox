//! vox-plugin-computer — desktop computer use behind the `ComputerControl`
//! extension point: accessibility snapshots with refs, per-window screenshots,
//! and mouse/keyboard input delivered in the background where the OS allows.
//!
//! Input tiers, cheapest first: `ax` (accessibility actions), `event` (input
//! posted to the target process; the user's cursor and focus stay put), and
//! `activate` (target raised, input sent, previous app restored). `auto`
//! tries `ax` then `event` and never activates on its own.

mod computer;
mod model;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod unsupported;

use std::sync::OnceLock;

use abi_stable::{
    erased_types::TD_Opaque, export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn,
    std_types::*,
};
use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;
use vox_plugin_api::abi::{VoxPlugin, VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef};
use vox_plugin_api::extensions::computer_control::{ComputerControl, ComputerControl_TO};
use vox_plugin_api::host::VoxHost_TO;

use computer::Computer;

#[cfg(target_os = "macos")]
type Platform = macos::Mac;
#[cfg(not(target_os = "macos"))]
type Platform = unsupported::Unsupported;

fn computer() -> &'static Computer<Platform> {
    static COMPUTER: OnceLock<Computer<Platform>> = OnceLock::new();
    COMPUTER.get_or_init(|| Computer::new(Platform::default()))
}

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
    RString::from(concat!(
        r#"{"id":"computer","version":""#,
        env!("CARGO_PKG_VERSION"),
        r#""}"#
    ))
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    RResult::ROk(VoxPlugin_TO::from_value(ComputerPluginHost, TD_Opaque))
}

#[derive(Clone)]
struct ComputerPluginHost;

impl VoxPlugin for ComputerPluginHost {
    fn id(&self) -> RString {
        RString::from("computer")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }

    fn as_computer_control(&self) -> ROption<ComputerControl_TO<'static, RBox<()>>> {
        ROption::RSome(ComputerControl_TO::from_value(ComputerPlugin, TD_Opaque))
    }
}

/// Stateless facade; snapshot refs and screenshot frames live in the
/// process-wide [`Computer`] so they survive across extension handles.
#[derive(Clone)]
struct ComputerPlugin;

fn reply(value: serde_json::Value) -> RResult<RString, RBoxError> {
    RResult::ROk(RString::from(value.to_string()))
}

impl ComputerControl for ComputerPlugin {
    fn permissions(&self) -> RResult<RString, RBoxError> {
        reply(computer().permissions())
    }

    fn list_apps(&self) -> RResult<RString, RBoxError> {
        reply(computer().list_apps())
    }

    fn list_windows(&self, options_json: RStr<'_>) -> RResult<RString, RBoxError> {
        reply(computer().list_windows(options_json.as_str()))
    }

    fn snapshot(&self, options_json: RStr<'_>) -> RResult<RString, RBoxError> {
        reply(computer().snapshot(options_json.as_str()))
    }

    fn screenshot(&self, options_json: RStr<'_>) -> RResult<RString, RBoxError> {
        reply(computer().screenshot(options_json.as_str()))
    }

    fn act(&self, action_json: RStr<'_>) -> RResult<RString, RBoxError> {
        reply(computer().act(action_json.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_names_the_plugin_and_crate_version() {
        let m: serde_json::Value = serde_json::from_str(manifest_json().as_str()).unwrap();
        assert_eq!(m["id"], "computer");
        assert_eq!(m["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(ComputerPluginHost.id().as_str(), "computer");
    }

    #[test]
    fn host_exposes_the_computer_control_extension() {
        let ext = ComputerPluginHost.as_computer_control();
        let ROption::RSome(ext) = ext else {
            panic!("ComputerControl not exposed")
        };
        let report: serde_json::Value =
            serde_json::from_str(ext.permissions().unwrap().as_str()).unwrap();
        assert!(report["platform"].is_string(), "{report}");
        let bad: serde_json::Value =
            serde_json::from_str(ext.act("{\"kind\":\"warp\"}".into()).unwrap().as_str()).unwrap();
        assert_eq!(bad["reason"], "invalid_request");
    }
}
