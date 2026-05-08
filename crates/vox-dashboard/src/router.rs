//! Axum sub-router: GET /dashboard/* → SPA shell + dashboard API routes
use crate::api::{mesh_router, models_router, runs_router, settings_router};
use crate::assets::serve_asset;
use axum::{Router, routing::get};

pub fn dashboard_router<S>(token: Option<String>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/dashboard", get(serve_asset))
        .route("/dashboard/{*path}", get(serve_asset))
        .merge(settings_router::<S>())
        .merge(mesh_router::<S>())
        .merge(models_router::<S>())
        .merge(runs_router::<S>())
        .layer(axum::extract::Extension(token))
}
