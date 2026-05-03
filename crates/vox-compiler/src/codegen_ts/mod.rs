//! TypeScript / React codegen for Vox web modules (components, routes, activities, etc.).
//!
//! Submodules map HIR/AST constructs to TS/JSX; this crate re-exports [`emitter::generate`].
#![allow(clippy::collapsible_if)]

/// Algebraic data types → TypeScript unions and helpers.
pub mod adt;
/// `@component` and related React component codegen.
pub mod component;
/// Main HIR → TypeScript emitter ([`generate`]).
pub mod emitter;
/// Shared HIR → TS emission (reactive, routes, activities).
pub mod hir_emit;
/// `@island` mount-point helpers (`data-vox-island`).
pub mod island_emit;
/// JSX lowering and attribute handling.
pub mod jsx;
/// Reactive components codegen (Path C).
pub mod reactive;
/// `routes.manifest.ts` (framework-agnostic `VoxRoute[]`).
pub mod route_manifest;
/// Segment-aware route-pattern parser and overlap detection (Phase C of the
/// Svelte-mineable features plan; not yet wired into [`routes`]).
pub mod route_pattern;
/// File-based routes → TS route tables.
pub mod routes;
/// One-time SPA / shadcn / Tailwind scaffold (user-owned files).
pub mod scaffold;
/// `@table` / VoxDB `schema.ts` generator ([`generate_voxdb_schema`]).
pub mod schema;
/// TanStack Query helper emission (`vox-tanstack-query.tsx`).
pub mod tanstack_query_emit;
/// `vox-client.ts` typed `fetch` SDK.
pub mod vox_client;
/// Design token CSS + TypeScript emit from vox.tokens.json.
pub mod tokens_emit;
/// `url` block TypeScript discriminated union + builder emit.
pub mod url_emit;
/// `state_machine` TypeScript discriminated union + reducer emit.
pub mod state_machine_emit;
/// `fragment` declaration → typed React function components in `fragments.tsx`
/// (Phase F of the Svelte-mineable features plan; per ADR-033).
pub mod fragment_emit;
/// Zod schema emission.
pub mod zod_emit;

pub use emitter::{CodegenOptions, generate, generate_with_options};
pub use schema::{generate_voxdb_schema, generate_voxdb_schema_from_hir};
