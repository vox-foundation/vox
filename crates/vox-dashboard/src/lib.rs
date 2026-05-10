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
    /// Includes mesh routes and the oplog-at route (P4-T5).
    pub fn build_router_with_empty_mesh() -> Router<()> {
        let (registry, bus) = build_mesh_state();
        let state = MeshState { registry, bus };
        crate::api::mesh::mesh_router(state.clone())
            .merge(crate::api::oplog_at::oplog_router(state))
    }

    /// Build a router pre-populated with two nodes that have cost/cap data.
    ///
    /// Used by the P4-T6 budget route tests.
    pub async fn build_router_with_two_nodes_and_costs(
        a: (&str, f64, f64),
        b: (&str, f64, f64),
    ) -> Router<()> {
        use vox_orchestrator::mesh::{MeshNode, MeshNodeKind, MeshNodeStatus, MeshPrivacyClass};

        let registry = Arc::new(MeshRegistry::empty());
        let bus = Arc::new(EventBus::new(64));

        let make_node = |id: &str, used: f64, cap: f64| MeshNode {
            id: id.into(),
            kind: MeshNodeKind::Agent,
            status: MeshNodeStatus::Active,
            orchestrator: None,
            model: "test-model".into(),
            uptime_ms: 0,
            tokens_24h: 0,
            cost_usd_24h: used,
            cost_cap_usd_24h: cap,
            current_task: None,
            last_events: vec![],
            privacy_class: MeshPrivacyClass::Private,
            heartbeat_age_ms: 0,
        };

        registry.upsert_node(make_node(a.0, a.1, a.2)).await;
        registry.upsert_node(make_node(b.0, b.1, b.2)).await;

        let state = MeshState { registry, bus };
        crate::api::mesh::mesh_router(state.clone())
            .merge(crate::api::oplog_at::oplog_router(state))
    }
}
