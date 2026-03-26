//! TypeScript / React codegen for Vox web modules (components, routes, activities, etc.).
//!
//! Submodules map HIR/AST constructs to TS/JSX; this crate re-exports [`emitter::generate`].
#![allow(clippy::collapsible_if)]

/// Vox activity / durable step TypeScript emission.
pub mod activity;
/// Algebraic data types → TypeScript unions and helpers.
pub mod adt;
/// `@component` and related React component codegen.
pub mod component;
/// Main HIR → TypeScript emitter ([`generate`]).
pub mod emitter;
/// Shared HIR → TS emission (reactive, routes, activities).
pub mod hir_emit;
/// JSX lowering and attribute handling.
pub mod jsx;
/// Reactive components codegen (Path C).
pub mod reactive;
/// File-based routes → TS route tables.
pub mod routes;
/// `@table` / VoxDB `schema.ts` generator ([`generate_voxdb_schema`]).
pub mod schema;
/// TanStack Start server-fn emission constants.
pub mod tanstack_start;

pub use emitter::{CodegenOptions, generate, generate_with_options};
pub use schema::{generate_voxdb_schema, generate_voxdb_schema_from_hir};
