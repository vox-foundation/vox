//! Schema Digest Generator — the LLM Context Engine for VoxDB.
//!
//! Walks `Module` AST declarations and produces a structured `SchemaDigest`
//! that makes the database fully self-describing for AI models.
//!
//! This is the **core differentiator** that makes VoxDB "LLM-first":
//! AI coding assistants using VoxDB always know the exact database shape,
//! field types, relationships, indexes, and can generate accurate queries
//! without guessing.

mod api;
mod digest_types;
mod helpers;

pub use api::{digest_to_json, format_llm_context, generate_schema_digest};
pub use digest_types::*;
