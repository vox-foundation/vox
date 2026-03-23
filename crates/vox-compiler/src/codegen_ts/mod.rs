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
/// JSX lowering and attribute handling.
pub mod jsx;
/// File-based routes → TS route tables.
pub mod routes;
/// `@table` / VoxDB `schema.ts` generator ([`generate_voxdb_schema`]).
pub mod schema;

pub use emitter::{generate, generate_with_options, CodegenOptions};
pub use schema::generate_voxdb_schema;
