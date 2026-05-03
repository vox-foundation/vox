//! ABI surface for Vox code plugins. Each plugin dylib exports a single
//! root symbol of type `VoxPluginRootRef`. The host reads `abi_version`,
//! calls `init` to obtain a `VoxPluginRef`, and interacts with the trait
//! object thereafter.
//!
//! SP2 ships only the `VoxPlugin` root with `id` + `shutdown`. Per-extension
//! `as_*` accessors are added in SP3+ as their respective traits land.

use crate::host::VoxHost_TO;
use abi_stable::{
    StableAbi,
    library::RootModule,
    package_version_strings,
    sabi_trait,
    sabi_types::VersionStrings,
    std_types::*,
};

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = VoxPluginRootRef)))]
#[sabi(missing_field(panic))]
pub struct VoxPluginRoot {
    pub abi_version: u32,
    #[sabi(last_prefix_field)]
    pub manifest_json: extern "C" fn() -> RString,
    pub init: extern "C" fn(host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError>,
}

impl RootModule for VoxPluginRootRef {
    abi_stable::declare_root_module_statics! {VoxPluginRootRef}
    const BASE_NAME: &'static str = "vox_plugin";
    const NAME: &'static str = "vox_plugin";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}

#[sabi_trait]
pub trait VoxPlugin: Send + Sync {
    fn id(&self) -> RString;
    fn shutdown(&self) -> RResult<(), RBoxError>;
}

pub type VoxPluginRef = VoxPlugin_TO<'static, RBox<()>>;
