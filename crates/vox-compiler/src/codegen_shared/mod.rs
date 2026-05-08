//! Codegen-shared utilities: structures and projections consumed by multiple
//! codegen backends (Rust, TypeScript) without belonging to any single emitter.

pub mod route_ir;

pub use route_ir::{RouteIR, RouteKind, RouteMethod, RouteParam, lower_module_routes};
