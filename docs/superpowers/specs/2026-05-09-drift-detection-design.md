---
title: "Drift Detection Design"
description: "Design spec for vox-drift-check: workspace-wide, multi-language, AST-backed drift and pattern-repetition detection."
category: "architecture"
status: "approved"
created: "2026-05-09"
---

# Drift Detection Design

## Problem Statement

A workspace audit identified 400+ locations across 98 crates where patterns are
repeated instead of consolidated: bypassed SSOT crates, raw path literals,
duplicated timeout values, copied serde-default functions, and more.  The
existing linter (`vox-code-audit`) is per-file only and cannot see cross-file
repetition.  No mechanism prevents these patterns from re-accumulating.

## Goals

1. Catch all patterns identified in the 2026-05-09 audit automatically.
2. Surface *new* repetition dynamically — string literals, numeric literals, and
   function body clones that appear N+ times workspace-wide.
3. First-class Rust, TypeScript, and Vox support via native parsers (no regex
   fallback for structural analysis).
4. Declarative config layer (`drift-patterns.toml`) so new rules can be added
   without recompiling.
5. Fast enough for pre-push hooks: cold <8 s, warm <1 s on the full workspace.

## Non-Goals

- ML/embedding semantic similarity.
- Auto-fix (`--fix` mode).
- LSP integration.
- Cross-language pattern equivalence (Rust ↔ TypeScript shape matching).
- Replacing `vox-code-audit` or `vox-arch-check`.

## Architecture

### Two-Phase Pipeline

```
Phase 1 (parallel, rayon)
  walkdir → per-file dispatch
    *.rs  → RustExtractor      (syn)
    *.ts  → TypeScriptExtractor (swc_ecma_parser)
    *.vox → VoxExtractor       (vox-compiler::parse)
  each extractor writes ExtractedFeatures into a channel

Phase 2 (single-threaded sweep)
  Aggregate all features into workspace index
  Run cross-file SweepRules  (threshold-based dedup)
  Run targeted DriftRules    (per-SSOT-violation)
  Emit Vec<Finding> → Reporter (Terminal / JSON / Markdown / SARIF)
```

### Crate Placement

New crate **`vox-drift-check`** at layer **L3** in `layers.toml`.

Justification:
- `vox-code-audit` is per-file; this crate is workspace-level.
- Three heavy parser dependencies (`syn` already exists; adds `swc_ecma_parser`,
  `vox-compiler`) deserve isolation.
- Mirrors `vox-arch-check` precedent of a dedicated binary for workspace checks.
- Reuses `vox-code-audit::{Finding, Severity}` without duplicating types.

## Module Layout

```
crates/vox-drift-check/
  Cargo.toml
  src/
    lib.rs                     # public API re-exports
    features.rs                # ExtractedFeatures schema
    extractor.rs               # LanguageExtractor trait + file dispatcher
    extractors/
      mod.rs
      rust.rs                  # syn — reuses vox-code-audit RustFileContext where possible
      typescript.rs            # swc_ecma_parser
      vox.rs                   # vox-compiler::parse
    engine.rs                  # orchestrator: walk → extract (parallel) → cache → sweep
    sweep/
      mod.rs                   # SweepRule trait
      literal_dedup.rs         # repeated string literals
      numeric_dedup.rs         # repeated numeric literals with unit-aware grouping
      body_hash.rs             # normalized AST body signatures
      call_shape.rs            # repeated call patterns (Module::fn/arity)
      import_drift.rs          # imports bypassing canonical SSOT modules
    rules/
      mod.rs                   # DriftRule trait
      reqwest_bypass.rs
      vox_path_literal.rs
      timeout_literal.rs
      serde_default_dup.rs
      version_string.rs
      bearer_header.rs
    config.rs                  # drift-patterns.toml loader + validation
    cache.rs                   # content-hash-keyed postcard cache
    report.rs                  # delegates to vox-code-audit::Reporter
    bin/
      vox_drift_check.rs       # standalone CLI entry point

drift-patterns.toml            # workspace-root declarative rule config
```

## Feature Vocabulary

The uniform language-agnostic schema that all three extractors populate:

```rust
pub struct ExtractedFeatures {
    pub file: PathBuf,
    pub language: Language,
    pub crate_name: Option<String>,
    pub string_literals: Vec<LiteralLoc>,
    pub numeric_literals: Vec<NumericLoc>,
    pub call_sites: Vec<CallSite>,
    pub body_signatures: Vec<BodySignature>,
    pub imports: Vec<ImportLoc>,
    pub fn_definitions: Vec<FnDef>,
}

pub struct LiteralLoc   { pub value: String,  pub span: Span, pub ctx: LiteralContext }
pub struct NumericLoc   { pub value: f64,     pub unit: Option<UnitHint>, pub span: Span }
pub struct CallSite     { pub path: Vec<String>, pub arity: u8, pub span: Span }
pub struct BodySignature{ pub hash: u64,      pub line_count: u32, pub parent_fn: Option<String>, pub span: Span }
pub struct ImportLoc    { pub path: Vec<String>, pub symbol: Option<String>, pub span: Span }
pub struct FnDef        { pub name: String,   pub body_hash: u64, pub sig_hash: u64, pub span: Span }

pub enum LiteralContext { Code, Test, Doc, ConstDecl }
pub enum UnitHint       { Millis, Seconds, Bytes, Count, Bare }
```

### Body Hash Normalization

1. Walk the native AST for the function body.
2. Alpha-rename all identifiers to positional placeholders (`α₀`, `α₁`, …)
   preserving binding structure but erasing names.
3. Strip all spans, comments, and whitespace tokens.
4. Sort statement-independent items (struct field order, use-list order).
5. Hash the canonical token sequence with **FxHash**.

`fn double(x: i32) -> i32 { x * 2 }` and `fn twice(n: i32) -> i32 { n * 2 }`
produce **the same hash** and appear as a dedup candidate.

## Cross-File Sweep Rules

| Rule | Mechanism | Default threshold | Severity |
|---|---|---|---|
| `sweep/duplicate-string-literal` | Group literals by exact string value; report groups ≥ threshold | 3 files | info |
| `sweep/duplicate-numeric-literal` | Group numeric values by (value, UnitHint); report groups ≥ threshold | 3 | warn |
| `sweep/duplicate-body` | Group fn body hashes; report groups ≥ threshold (min 5 lines) | 2 | warn |
| `sweep/duplicate-call-pattern` | Group `path + arity` call shapes; suggest helper | 5 | info |
| `sweep/import-bypass` | Config-driven: flag imports of forbidden prefixes | n/a | warn |

Each finding includes: all occurrence locations, suggested SSOT location
(heuristic: lowest-layer crate among occurrences), confidence score.

## Targeted Drift Rules

| Rule ID | What it catches | Language |
|---|---|---|
| `drift/reqwest-bypass` | `Client::new()` / `Client::builder()` outside `vox-reqwest-defaults` | Rust |
| `drift/vox-path-literal` | Raw `.vox/` / `.vox-cache/` strings outside `vox-config::paths` | Rust |
| `drift/timeout-literal` | `Duration::from_{secs,millis,nanos}` with a literal arg (not a const) whose value is in a common set | Rust |
| `drift/serde-default-dup` | `fn default_*()` with matching body hash defined in more than one crate | Rust |
| `drift/version-string` | String literals equal to `workspace.package.version` outside Cargo files | Rust, Vox |
| `drift/bearer-header-inline` | `HeaderValue::from_static("Bearer …")` literal | Rust |

```rust
pub trait DriftRule: Send + Sync {
    fn id(&self) -> &'static str;
    fn severity(&self) -> Severity;
    fn languages(&self) -> &[Language];
    fn check(&self, features: &ExtractedFeatures, ws: &WorkspaceContext) -> Vec<Finding>;
}
```

`WorkspaceContext` carries: crate-name → layer lookup, dep graph from
`cargo metadata`, workspace version, allowlists from config.

## Declarative Config — `drift-patterns.toml`

```toml
[meta]
version = 1

[[forbidden_call]]
id = "drift/reqwest-bypass"
match = ["reqwest::Client::new", "reqwest::Client::builder"]
allow_in_crate = ["vox-reqwest-defaults"]
allow_in_test = true
severity = "warn"
suggestion = "Use vox_reqwest_defaults::client_builder()"

[[forbidden_literal]]
id = "drift/vox-path-literal"
pattern = '^\.vox(-cache)?/'
allow_in_crate = ["vox-config"]
severity = "warn"
suggestion = "Use vox_config::paths::* constants"

[duplicated_literal]
threshold = 3
min_length = 8
ignore_in_paths = ["**/tests/**", "**/fixtures/**", "**/golden/**"]
severity = "info"

[duplicated_numeric]
threshold = 3
units = ["Millis", "Seconds", "Bytes"]
severity = "warn"

[duplicated_body]
threshold = 2
min_lines = 5
severity = "warn"

[[import_bypass]]
id = "drift/dei-shim-internals"
forbidden_prefix = "vox_orchestrator::dei_shim::"
canonical_prefix = "vox_orchestrator::"
severity = "warn"
```

Adding a new rule = edit TOML only. No recompile.

## CLI Surface

```
vox drift-check                               # full workspace scan
vox drift-check --rule drift/reqwest-bypass   # single rule
vox drift-check --languages rust,typescript   # restrict
vox drift-check --json | --sarif | --markdown
vox drift-check --severity error              # CI gate
vox drift-check --baseline .vox/cache/drift/baseline.json   # new findings only
vox drift-check --update-baseline             # write baseline
vox drift-check --suggest-config              # propose drift-patterns.toml additions
```

## Performance Plan

| Technique | Effect |
|---|---|
| `rayon::par_iter` over file list | N-core extraction |
| Drop native AST after extraction | Constant memory per file |
| Content-hash-keyed `postcard` cache at `.vox/cache/drift/` | Warm run re-parses only changed files |
| `FxHashMap<u64, SmallVec<[Loc;4]>>` for sweep indexes | Fast aggregation |
| Skip `target/`, `node_modules/`, `docs/src/archive/` | Avoid wasted work |

**Targets:** cold <8 s, warm <1 s on the full Vox workspace.

## Testing

| Layer | Approach |
|---|---|
| Per-extractor | `insta` golden snapshots: fixture file → assert `ExtractedFeatures` JSON |
| Per-rule | Fixture file with planted violation, assert `Finding` list |
| Sweep | Multi-file fixture workspace, assert dedup candidates |
| Integration | Run on Vox repo, assert finding count within baseline bounds |
| Performance | `criterion` bench; fail if >2× baseline |

## CI / Governance Integration

- New `lefthook.yml` pre-push step: `vox drift-check --severity warn`
- `vox doctor` drift-check section (findings summary, cache status)
- `layers.toml` entry for `vox-drift-check` at L3
- `where-things-live.md` row added

## Phased Rollout

| Phase | Deliverable |
|---|---|
| **P1** | Crate skeleton + features schema + Rust extractor + 3 sweep rules (string/numeric/body) + CLI |
| **P2** | 6 targeted Rust drift rules (reqwest, vox-path, timeout, serde-default, version-string, bearer-header) |
| **P3** | `drift-patterns.toml` config layer + `--suggest-config` mode |
| **P4** | TypeScript extractor (swc_ecma_parser) + cross-file sweep on `apps/` |
| **P5** | Vox extractor (vox-compiler) + cross-file sweep on `scripts/` |
| **P6** | Caching + `--baseline` mode + lefthook hook + `vox doctor` + docs |
