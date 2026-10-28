# Phase 83: Core Crate - Parser, IR & Diagnostics - Pattern Map

**Mapped:** 2026-10-16
**Files analyzed:** ~40 (new crate; registration edits to 5 existing files)
**Analogs found:** scaffolding/registration/test-style exact; parser/lexer/fmt/fixes/fence are greenfield (RESEARCH.md designs are the spec)

All analog paths verified git-tracked (`git ls-files`) or read from the worktree checkout; none are gitignored mirrors.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/vox-diagram/Cargo.toml` | config | n/a | `crates/vox-kb-facets/Cargo.toml` | exact |
| `crates/vox-diagram/src/lib.rs` | module root | n/a | `crates/vox-kb-facets/src/lib.rs` | exact |
| `src/{limits,codes}.rs` | utility (consts) | n/a | none (see RESEARCH Structure) | no analog |
| `src/{span,diag,ir,sequence_ir}.rs` | model (serde) | transform | `vox-kb-facets/src/lib.rs` serde derives | role-match |
| `src/lexer.rs`, `src/parser/*`, `src/resolve.rs`, `src/print.rs`, `src/fixes.rs` | parser/transform | transform | none in workspace (vox-compiler is the vox-lang parser, different shape) | no analog |
| `src/fence.rs` | utility | transform | `crates/vox-doc-pipeline/src/claims/extractor.rs:61-72` (pulldown-cmark offset iter) | role-match |
| `src/card.rs`, `grammar-card.txt` | const/asset | n/a | none | no analog |
| `tests/*.rs` insta | test | snapshot | `crates/vox-compiler/tests/diagnostic_snapshots.rs` | exact |
| `tests/fmt_props.rs`, proptest modules | test | property | `crates/vox-db-types/src/kb_text.rs:741-762` | exact |
| `tests/proptest-regressions/` | test data | n/a | `crates/vox-db-types/proptest-regressions/kb_text.txt` | exact |
| root `Cargo.toml` (workspace dep row) | config | n/a | line 101 `vox-kb-facets` / line 194 `vox-similarity` | exact |
| `docs/src/architecture/layers.toml` (+row) | config | n/a | line 187 `vox-similarity`, line 227 `vox-oauth-pkce` (orphan_exempt) | exact |
| `docs/src/architecture/where-things-live.md` (+rows) | docs | n/a | lines 112 (L2 table), 230 (concept row) | exact |
| `docs/src/api/vox-diagram.generated.md` | generated | n/a | `docs/src/api/vox-kb-facets.generated.md` (commit 79d9cd454) | exact (run `vox ci crate-api-sync --write`) |
| `contracts/ci/crate-graph.v1.json` | generated | n/a | regen via `vox ci affected-crates --regen --out contracts/ci/crate-graph.v1.json` | exact |

## Pattern Assignments

### `crates/vox-diagram/Cargo.toml` (config)
**Analog:** `crates/vox-kb-facets/Cargo.toml` (use workspace edition, NOT vox-similarity's `edition = "2024"` literal)
```toml
[package]
name = "vox-kb-facets"
description = "..."            # mandatory (arch-check Rule 6)
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
workspace-hack = { version = "0.6", path = "../workspace-hack" }

[lints]
workspace = true
```
Add per RESEARCH: `pulldown-cmark`, `strsim` (workspace), dev-deps `insta` (json), `proptest`, `serde_json`. No `blake3`/`sha2` (vox-similarity has blake3; do NOT copy that).

### `src/lib.rs`
**Analog:** `crates/vox-kb-facets/src/lib.rs:1-30`: `//!` first line (L2 strict), `pub mod` list then `pub use` re-exports, no fn bodies in lib.rs.
```rust
//! L0 KB facet contract (KB Phase 4, D-01).
//!
//! Pure: no I/O and no hashing. ...
pub mod backlog;
...
pub use backlog::{BACKLOG_REL, Backlog, BacklogRow};
```
Add `/// doc` on each `pub mod` (crate-api generator reads lib.rs only). DSL examples in `//!` use ```` ```text ````.

### `src/fence.rs` (utility, transform)
**Analog:** `crates/vox-doc-pipeline/src/claims/extractor.rs:61-72`
```rust
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
let parser = Parser::new_ext(content, Options::all()).into_offset_iter();
for (event, range) in parser {
    match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
            let info_str = info.as_ref().trim().to_lowercase();
```
Differences: use exact `info.trim() == info` match (no lowercasing), concatenate `Event::Text`, track `body_range` and `closed`. Do NOT copy `mermaid.rs` (uncommitted, forbidden dependency).

### Tests: insta snapshots (`tests/graph_constructs.rs` etc.)
**Analog:** `crates/vox-compiler/tests/diagnostic_snapshots.rs:7`: `insta::assert_json_snapshot!(diags);` Use `assert_json_snapshot!(report.ir)`; `*.snap.new` is gitignored; never commit.

### Tests: proptest (`tests/fmt_props.rs`, S1-S4)
**Analog:** `crates/vox-db-types/src/kb_text.rs:741-762`
```rust
proptest! {
    #[test]
    fn sanitize_is_idempotent(s in fragments()) {
        let once = sanitize(&s);
        prop_assert_eq!(sanitize(&once), once);
    }
    #[test]
    fn arbitrary_unicode_is_idempotent_and_valid(s in "\\PC{0,64}") { ... }
}
```
Same shape for `fmt(fmt(x)) == fmt(x)`. Locked proptest is 1.11.0 `default-features=false, features=["std"]` (no timeout/fork). Commit generated `proptest-regressions/` like `crates/vox-db-types/proptest-regressions/kb_text.txt`. Public-fn inline tests must live in the same `src/` file (tdd-guard is file-level; `tests/` does not satisfy it).

### Registration rows (copy exact format)
- `layers.toml` after line 187: `vox-diagram          = { layer = 2, max_loc = 12_000, orphan_exempt = true }   # L2 pure VoxDiagram core ...; first consumer lands Phase 87 which removes orphan_exempt` (formats from line 187 and 227).
- root `Cargo.toml` `[workspace.dependencies]`: `vox-diagram               = { path = "crates/vox-diagram" }` (style of line 101/194). `members = ["crates/*", ...]` already covers it.
- `where-things-live.md`: L2 table row in the style of line 112, `| [`vox-diagram`](../../../crates/vox-diagram/) | Pure VoxDiagram core ... |`; concept rows in the style of line 230 containing the substring `crates/vox-diagram/`.

## Shared Patterns

- **Same-file tests for every public `pub fn`**: internals `pub(crate)`; inline `#[cfg(test)] mod tests` asserting values (weak_test rejects `.is_ok()`-only).
- **Never panic on input**: `str::get` via `Span::slice`; crate attrs `#![forbid(unsafe_code)]`, `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]`.
- **Determinism**: Vec/String only in IR; guard test builds needles with `concat!("Hash","Map")`.
- **File size**: keep files under 400 non-blank lines; no `utils.rs`/`helpers.rs`/`common.rs`.
- **Docs**: rustdoc DSL examples ```` ```text ````; `rustdocflags -D warnings`.
- **Verification**: only `cargo check -p vox-diagram` and `cargo nextest run -p vox-diagram` locally, backgrounded; one PR (Cargo.lock/Cargo.toml sentinel triggers full CI); run `vox ci crate-api-sync --write` and `cargo hakari generate --diff` (expect empty).

## No Analog Found

| File | Role | Reason |
|---|---|---|
| lexer/parser/resolve/print/fixes | parser pipeline | No hand-written recovering parser exists to copy; follow RESEARCH.md EBNF, recovery rules, statement-loop skeleton, `LineIndex` snippet |
| limits.rs, codes.rs, diag.rs | consts/model | New; code catalogue is in RESEARCH "Code catalogue" |
| grammar-card.txt / card.rs | asset | New; draft in RESEARCH |
| sequence_ir.rs | model | New serde types, no parser |

## Metadata

**Analog search scope:** `crates/vox-kb-facets`, `crates/vox-similarity`, `crates/vox-doc-pipeline/src/claims`, `crates/vox-db-types`, `crates/vox-compiler/tests`, `docs/src/architecture/{layers.toml,where-things-live.md}`, root `Cargo.toml`
**Pattern extraction date:** 2026-10-16
