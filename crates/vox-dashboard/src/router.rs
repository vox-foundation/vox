//! Axum sub-router: GET /dashboard/* → SPA shell + dashboard API routes
use std::sync::Arc;

use axum::{Router, routing::get};
use vox_orchestrator::events::EventBus;
use vox_orchestrator::mesh::MeshRegistry;

use crate::api::{MeshState, mesh_router, models_router, oplog_router, runs_router, settings_router};
use crate::assets::serve_asset;
use crate::audit_log::AuditWriter;

pub fn dashboard_router<S>(token: Option<String>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // Build a default empty MeshRegistry. In production, the orchestrator binary
    // injects the shared registry via `Arc` before serving.
    let registry = Arc::new(MeshRegistry::empty());
    let bus = Arc::new(EventBus::new(512));
    let mesh_state = MeshState {
        registry,
        bus,
        audit: Arc::new(AuditWriter::ephemeral()),
    };

    Router::new()
        .route("/dashboard", get(serve_asset))
        .route("/dashboard/{*path}", get(serve_asset))
        .merge(settings_router::<S>())
        .merge(mesh_router(mesh_state.clone()))
        .merge(oplog_router(mesh_state))
        .merge(models_router::<S>())
        .merge(runs_router::<S>())
        .layer(axum::extract::Extension(token))
}
