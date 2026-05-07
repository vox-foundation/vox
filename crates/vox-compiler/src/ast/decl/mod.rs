//! Top-level items in a `.vox` file: functions, data definitions, routes, and platform-specific UI.
//!
//! `Decl` is a flat enum: there is no nested module syntax in the AST. Import declarations are
//! included here so the parser can emit a single `Module` per file. Attributes and decorators
//! that the lexer/parser attach often land on fields inside each variant; mutators on `Decl`
//! exist for codegen and CLI tooling that patch metadata after parse.

/// `@config` blocks and typed configuration declarations.
pub mod config;
/// Relational table, collection, and index declarations.
pub mod db;
/// Effect annotations for the `uses` clause.
pub mod effect;
/// Functions, components, server handlers, MCP, hooks, and tests.
pub mod fundecl;
/// Actors, agents, workflows, activities, and HTTP routes.
pub mod logic;
/// State machine declarations (`state_machine Name { … }`).
pub mod state_machine;
/// ADTs, traits, impls, and type aliases.
pub mod typedef;
/// Client routing, layouts, themes, and SSG page metadata.
pub mod ui;
/// Typed URL path declarations (`url Name { … }`).
pub mod url;

pub use config::*;
pub use db::*;
pub use effect::*;
pub use fundecl::*;
pub use logic::*;
pub use state_machine::*;
pub use typedef::*;
pub use ui::*;
pub use url::*;

mod callable;
mod reactive;
mod types;

pub use types::{
    Decl, HttpMethod, ImportDecl, ImportPath, ImportPathKind, Module, PyImportDecl, RustCrateImport,
};
