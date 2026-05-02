//! vox-dashboard: Axum router that serves the orchestration SPA.
//!
//! Mount via: `app.merge(vox_dashboard::router())`
pub(crate) mod api;
pub mod assets;
pub mod router;
pub use router::dashboard_router;
