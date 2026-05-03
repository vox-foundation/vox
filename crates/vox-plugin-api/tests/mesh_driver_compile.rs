//! Compile-only test that the MeshDriver trait shape is sabi-stable.
//! Runtime behavior will be exercised in vox-plugin-populi-mesh's tests
//! once the actual mesh code-motion completes.

use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::mesh_driver::{MeshDriver, MeshDriver_TO, MESH_DRIVER_REVISION};

#[test]
fn revision_constant_is_one() {
    assert_eq!(MESH_DRIVER_REVISION, 1);
}

struct DummyMesh;

impl MeshDriver for DummyMesh {
    fn start_transport(&self, _config: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
    fn stop_transport(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
    fn dispatch(&self, _req: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("{}"))
    }
    fn node_join(&self, _node: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
    fn list_nodes(&self) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("[]"))
    }
}

#[test]
fn dummy_mesh_constructs() {
    let _: MeshDriver_TO<'static, RBox<()>> = MeshDriver_TO::from_value(DummyMesh, TD_Opaque);
}
