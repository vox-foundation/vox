//! MeshDriver extension point — for distributed mesh transport plugins.
//! Used by vox-plugin-populi-mesh to run the HTTP control plane.

use abi_stable::{sabi_trait, std_types::*};

pub const MESH_DRIVER_REVISION: u32 = 2;

#[sabi_trait]
pub trait MeshDriver: Send + Sync {
    fn revision(&self) -> u32 {
        MESH_DRIVER_REVISION
    }

    /// Boot the mesh transport using JSON-encoded MeshConfig. Idempotent.
    fn start_transport(&self, config_json: RStr<'_>) -> RResult<(), RBoxError>;

    /// Gracefully shut down the transport.
    fn stop_transport(&self) -> RResult<(), RBoxError>;

    /// Dispatch a request through the mesh. JSON-encoded; response shape
    /// is mesh-defined. Used by orchestrator A2A.
    fn dispatch(&self, request_json: RStr<'_>) -> RResult<RString, RBoxError>;

    /// Register a node in the local registry. Used by heartbeat handlers.
    fn node_join(&self, node_record_json: RStr<'_>) -> RResult<(), RBoxError>;

    /// List currently-registered nodes as a JSON array.
    fn list_nodes(&self) -> RResult<RString, RBoxError>;

    /// Relay an A2A message to a specific peer node URL. Used by clavis broadcasts
    /// and orchestrator A2A dispatch. Returns the peer's response JSON.
    fn relay_a2a(
        &self,
        target_url: RStr<'_>,
        request_json: RStr<'_>,
    ) -> RResult<RString, RBoxError> {
        let _ = (target_url, request_json);
        RResult::RErr(RBoxError::new(std::io::Error::other(
            "relay_a2a not implemented by this MeshDriver backend",
        )))
    }
}
