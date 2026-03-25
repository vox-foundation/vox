//! Corpus staleness detection, auto-regeneration preflight, and supplementary
//! pair generators that address the critical gaps identified in the gap analysis:
//!
//! - **Compact Vox syntax** (minified, single-line — the canonical serializable form)
//! - **Multi-turn conversation pairs** (user iterates on code across turns)
//! - **Error → fix pairs** (broken Vox code + explanation + corrected version)
//! - **Architectural Q&A pairs** (when to use actor vs workflow, etc.)
//! - **Explain-from-code pairs** (code → English prose explanation)
//!
//! ## Staleness detection
//! Uses xxhash-rust (xxh3_64) to fingerprint all AST source files and generator
//! source files. If any watched file changes, the corpus is considered stale.
//! The fingerprint is stored in Arca V18 `corpus_snapshots` table.

use anyhow::Result;
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::xxh3_64;

// ── Watched file list ────────────────────────────────────────────────────────

/// Files whose changes invalidate the corpus fingerprint.
/// Ordered by importance — AST definitions first, generators second.
const WATCHED_FILES: &[&str] = &[
    "crates/vox-ast/src/expr.rs",
    "crates/vox-ast/src/types.rs",
    "crates/vox-ast/src/pattern.rs",
    "crates/vox-ast/src/stmt.rs",
    "crates/vox-ast/src/decl/mod.rs",
    "crates/vox-cli/src/lib.rs",
    "crates/vox-corpus/src/codegen_vox.rs",
    "crates/vox-corpus/src/synthetic_gen/mod.rs",
    "crates/vox-corpus/src/corpus/preflight/mod.rs",
    "crates/vox-corpus/src/corpus/augment.rs",
    "Cargo.toml",
    "mens/config/templates.yaml",
    "mens/config/mix.yaml",
];

include!("preflight_part1.rs");
include!("preflight_part2.rs");
