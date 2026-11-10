//! Workspace tools: architect, audit, search, compact, clean.

/// Architect subcommand: validates workspace layout and god-object analysis (`vox architect`).
pub mod architect;
/// Audit subcommand: audits dependencies for security and drift.
pub mod audit;
/// Clean subcommand: removes build artifacts and temporary files.
pub mod clean;
/// Compact subcommand: compacts Vox source files.
pub mod compact;
/// Search subcommand: searches for packages in the registry.
pub mod search;
