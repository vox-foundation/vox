//! PopuliMeshPlugin — composite plugin's code side.
//!
//! # Stub status
//!
//! All MeshDriver methods below are stubbed with "not yet implemented" errors.
//! The real implementation requires porting vox-populi's `transport` feature
//! module (~2000+ LOC: HTTP control plane, A2A dispatch, node registry, TLS,
//! federation). That work is deferred to a follow-up batch under
//! "vox-populi-as-plugin extraction" (see plugin-system-redesign-sp1-plan-2026.md).

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::abi::VoxPlugin;
use vox_plugin_api::extensions::mesh_driver::{MeshDriver, MeshDriver_TO};

#[derive(Clone)]
pub struct PopuliMeshPlugin;

impl PopuliMeshPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl VoxPlugin for PopuliMeshPlugin {
    fn id(&self) -> RString {
        RString::from("populi-mesh")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }

    fn as_mesh_driver(&self) -> ROption<MeshDriver_TO<'static, RBox<()>>> {
        ROption::RSome(MeshDriver_TO::from_value(self.clone(), TD_Opaque))
    }
}

impl MeshDriver for PopuliMeshPlugin {
    fn start_transport(&self, _config: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented (mesh code-motion deferred)",
        )))
    }

    fn stop_transport(&self) -> RResult<(), RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented",
        )))
    }

    fn dispatch(&self, _req: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented",
        )))
    }

    fn node_join(&self, _node: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "not yet implemented",
        )))
    }

    fn list_nodes(&self) -> RResult<RString, RBoxError> {
        // list_nodes returns an empty array rather than an error — callers
        // that just want to enumerate nodes get a safe empty result, while
        // callers that need live data will observe the empty list and can
        // degrade gracefully.
        RResult::ROk(RString::from("[]"))
    }
}
