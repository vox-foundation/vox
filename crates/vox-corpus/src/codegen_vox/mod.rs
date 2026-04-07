//! **Organic Vox code generator** — produces syntactically valid `.vox` programs
//! from the AST type definitions, verified by parser round-trip.
//!
//! ## Architecture
//! - **`VoxTypeGen`**: recursive type generator (all 5 `TypeExpr` variants)
//! - **`VoxExprGen`**: recursive expression generator (all 22 `Expr` variants)
//! - **`VoxDeclGen`**: per-construct generators consuming `TAXONOMY_FROM_AST`
//! - **Parser verification**: every emitted program runs through the parser
//!
//! ## Dynamic walking
//! This module consumes `TAXONOMY_FROM_AST` (auto-derived from `vox-compiler` `Decl` variants
//! at build time) so that when new language constructs are added, generators are
//! automatically flagged as missing by coverage analysis.

use serde_json::json;

// ── Build-time constants (dynamic, walked by build.rs) ───────────────────────
include!(concat!(env!("OUT_DIR"), "/dynamic_registry.rs"));

// ── Deterministic RNG ────────────────────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0xdeadbeef } else { seed })
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn usize(&mut self, max: usize) -> usize {
        self.next() as usize % max.max(1)
    }
    fn coin(&mut self) -> bool {
        self.next().is_multiple_of(2)
    }
}

// ── Word pools (used for identifier generation) ──────────────────────────────
// These are NOT language constructs — they're realistic domain names for
// generating variable names, function names, etc.

const NOUNS: &[&str] = &[
    "user", "order", "product", "session", "payment", "event", "metric", "config", "task",
    "report", "message", "record", "item", "entry", "account", "profile", "document", "request",
    "response", "result",
];

const VERBS: &[&str] = &[
    "process",
    "validate",
    "transform",
    "fetch",
    "store",
    "compute",
    "render",
    "parse",
    "encode",
    "notify",
    "schedule",
    "dispatch",
    "route",
    "check",
    "filter",
    "sort",
    "merge",
    "update",
    "create",
    "delete",
    "find",
    "search",
    "analyze",
    "generate",
    "format",
];

const FIELD_POOL: &[(&str, &str)] = &[
    ("id", "int"),
    ("name", "str"),
    ("email", "str"),
    ("count", "int"),
    ("active", "bool"),
    ("amount", "float"),
    ("status", "str"),
    ("created_at", "str"),
    ("data", "str"),
    ("score", "float"),
    ("label", "str"),
    ("value", "int"),
    ("title", "str"),
    ("description", "str"),
    ("priority", "int"),
    ("done", "bool"),
];

include!("part_01.rs");
include!("part_02.rs");
include!("part_03.rs");
