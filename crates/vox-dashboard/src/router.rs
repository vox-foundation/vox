//! Axum sub-router: GET /dashboard/* → SPA shell
use axum::{Router, routing::get};
use crate::assets::serve_asset;

pub fn dashboard_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/dashboard", get(serve_asset))
        .route("/dashboard/{*path}", get(serve_asset))
}
