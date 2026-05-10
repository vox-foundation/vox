//! vox-dashboard: Axum router that serves the orchestration SPA.
//!
//! Mount via: `app.merge(vox_dashboard::router())`
pub mod api;
pub mod assets;
pub mod router;
pub use router::dashboard_router;

/// Test helpers for building an isolated dashboard router with an empty mesh.
pub mod test_support {
    use std::sync::Arc;

    use axum::Router;
    use vox_orchestrator::events::EventBus;
    use vox_orchestrator::mesh::MeshRegistry;

    use crate::api::MeshState;

    /// Build a `(MeshRegistry, EventBus)` pair backed by an empty registry.
    pub fn build_mesh_state() -> (Arc<MeshRegistry>, Arc<EventBus>) {
        let registry = Arc::new(MeshRegistry::empty());
        let bus = Arc::new(EventBus::new(64));
        (registry, bus)
    }

    /// Build a complete dashboard router against an empty mesh registry.
    ///
    /// Use this in integration tests that exercise mesh REST routes.
    pub fn build_router_with_empty_mesh() -> Router<()> {
        let (registry, bus) = build_mesh_state();
        let state = MeshState { registry, bus };
        crate::api::mesh::mesh_router(state)
    }
}
