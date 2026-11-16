# Phase 83: Core Crate - Parser, IR & Diagnostics - Research

**Researched:** 2026-10-16
**Domain:** Hand-written lexer + error-recovering recursive-descent parser, typed AST -> resolved serde IR, stable diagnostics with machine-applicable fixes, canonical formatter, input caps, CommonMark fence scanner, all inside a new governance-clean L2 Rust crate (`vox-diagram`)
**Confidence:** MEDIUM-HIGH (repo registration facts read from source this session = HIGH; parser/recovery/Mermaid-ism/proptest design = MEDIUM, it is design guidance not library lookup)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**DSL surface syntax**
- Header: `graph { direction: right ... }`; direction values `down` | `right` | `up` | `left` (plain words, no Mermaid `TD/LR` tokens).
- Node declaration: `node api: "API Gateway"` with optional attribute block `{ shape: cylinder }`. Shapes: `rect` (default), `rounded`, `diamond`, `cylinder`, `start`, `end`.
- Edges: `a -> b` and `a -> b: "label"`; self-loops allowed; `//` line comments. All labels are double-quoted strings with escapes (`\"`, `\\`, `\n`).
- Edge to an undeclared id is an error `diagram/undeclared-node` with a fix inserting `node x: "x"` (no implicit declaration).
- Containers: `container backend: "Orchestrator" { ... }`; ids resolve in the innermost container first, outer scopes after; dotted refs `backend.gateway` address nested ids from outside.

**Diagnostics & fixes**
- Flat, stable kebab codes `diagram/<name>` (e.g. `diagram/unterminated-string`, `diagram/undeclared-node`, `diagram/container-endpoint`); codes are never renamed, only added.
- Error recovery re-syncs at statement boundaries (newline or `}`); all independent errors reported; cap 100 diagnostics.
- `fixes[]` (`{span, replacement}`) only on machine-applicable remedies (quote a label, close a brace, declare a node, Mermaid-ism rewrite) — applying them must always be safe.
- Mermaid-isms (`-->`, `A[Label]`, `subgraph … end`, etc.) parse leniently and yield `diagram/mermaid-syntax` with the canonical rewrite as a fix — the primary mitigation for LLMs with no training data on the DSL.

**Limits, IR & formatting**
- Caps (each a `diagram/limit-*` error, span-precise): 64 KiB source, 500 nodes, 1,000 edges, container depth 4, label 200 chars. Edge endpoint that is a container → `diagram/container-endpoint`. Adversarial input must terminate in bounded time with no panic.
- IR: serde, `Vec`s in declaration order (no `HashMap`, no sorted maps), spans as byte offsets + line/col; JSON byte-identical across runs.
- `fmt`: 4-space indent, one statement per line, declaration order preserved (never sorted), `//` comments kept attached to their statement; `fmt(fmt(x)) == fmt(x)` and `parse(fmt(x)) == parse(x)` (proptest).
- `sequence` IR types (actors, messages incl. dashed return, `note over`) defined now, no parser (Phase 86).

### Claude's Discretion
- Internal module layout under the 500-line file cap; lexer token set; exact diagnostic message wording; `max_loc` value in layers.toml; proptest generators.

### Deferred Ideas (OUT OF SCOPE)
- Edges to containers via hidden-anchor lowering (v2).
- Published JSON Schema for the IR (v2).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DGM-01 | Author can write a `graph` diagram with `direction`, quoted `node` labels (escapes), `container` nesting, `a -> b` / `a -> b: "label"` edges, dotted refs, six shapes, self-loops, `//` comments | EBNF + lexical rules (Section "DSL grammar"), scope resolution algorithm, IR design, canonical example as golden fixture |
| DGM-03 | A single check reports every error (parser recovery), each with stable `diagram/*` code, severity, exact span | Recovery strategy (declare-then-diagnose, sync sets, fuel), diagnostic catalogue, ordering/cap rules, cascade-suppression tests |
| DGM-04 | Diagnostics carry machine-applicable `fixes[]` (`{span, replacement}`) | Fix model (atomic per-diagnostic edit sets), `apply_fixes`/`repair` algorithm, Mermaid-ism rewrite catalogue, mutate-and-repair proptest |
| DGM-05 | Canonical idempotent `fmt` | AST entry model for comments/blank lines, printer rules, 3 proptest properties + oracle |
| DGM-06 | Oversized / over-nested input and container-endpoint edges rejected span-precisely, no hang | Limits table with enforcement points, bounded-time argument (fuel + iterative depth skip + amortized scan-ahead), adversarial corpus |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

Directives the plan must honor (same authority as locked decisions):

- **Test-first, file-level:** every new `pub fn` in `crates/*/src/**` needs an inline `#[test]`/`#[cfg(test)] mod tests` in the SAME file (see Pitfall 1 for the exact detector semantics). `tdd-guard` blocks the commit.
- **No direct `blake3`/`sha2`/`sha3`** (crypto policy; detector `vox/crypto/banned-crate-import` also scans `Cargo.toml`). This crate needs no hashing at all.
- **Dependency Discipline:** no new workspace-crate edge in this phase (the only dep on a workspace member is `workspace-hack`, exempt). Never write a `crate-edges` `exceptions` entry; never regenerate `crate-edges.allow.v1.json`/`fan-in-snapshot.v1.json`. Consumer edges (`vox-doc-pipeline`, `vox-orchestrator-mcp`) are pre-authorized but land in Phases 87/88, not here.
- **New crate** must get a `layers.toml [crates]` row at creation, a `where-things-live.md` row, a `description`, and a `//!` lib docstring.
- **Hosted CI is the gate.** Locally only `cargo check -p vox-diagram` and `cargo nextest run -p vox-diagram` (never workspace clippy/test). Background every cargo invocation >~60 s (memory: never block foreground on cargo; GSD lean verification: mutation proofs stay).
- **Formatting:** never `cargo fmt --all`; use `cargo fmt -p vox-diagram` or `rustfmt --config-path . <files>`.
- **Plain `cargo` on PATH** (build-broker shim at `~/.vox/build-broker/bin/cargo`); never an explicit `~/.cargo/bin/cargo` path.
- **VoxScript-first glue:** any generator/fixture tool is `.vox` run via `vox run`, never new `.py`/`.sh`. Fixtures are plain files; CRLF test inputs are built in code (`.replace('\n', "\r\n")`), never committed (`vox ci line-endings` is a forward-only LF gate).
- **Docs:** any page created under `docs/src/` needs full frontmatter + taxonomy facets + `<!-- kb:summary -->`; `docs/src/api/vox-diagram.generated.md` is generated (do not hand-edit). Rustdoc examples of the DSL MUST be fenced ```` ```text ```` (bare fences compile as Rust doctests; `.cargo/config.toml` sets `rustdocflags = ["-D", "warnings"]`).
- **LLM boundary / secrets / retired surfaces:** not touched (crate makes no LLM calls, reads no env).
- **Archive protocol:** never read or modify `docs/src/archive/`.
- **Perennial bug pattern:** after resolving any merge in Rust files, verify each item is defined once (`grep -c 'fn <name>'`).

## Summary

`vox-diagram` is a pure std+serde L2 library. Phase 83 builds its front half for the `graph` kind: a byte-offset lexer, an error-recovering recursive-descent parser that "declares then diagnoses" (so one broken statement never cascades into false `undeclared-node` errors), a hoisting scope resolver (declarations first, then edges, so forward references work), a serde IR made only of `Vec`s/`String`s, a diagnostic model whose `fixes[]` are atomic per-diagnostic edit sets with a deterministic `apply_fixes`/`repair` loop, a canonical printer driven by an AST that keeps comments and single blank lines as ordered entries (which makes idempotence almost structural), a CommonMark-correct `fence` scanner built on the already-locked `pulldown-cmark`, and `sequence` IR types with no parser. Adversarial-input safety comes from four cheap mechanisms: an up-front 64 KiB reject, a parser fuel counter, an iterative (non-recursive) skip for over-deep containers, and a rule that every scan-ahead either consumes what it scanned or ends the statement.

Registration is the surprising half of this phase. A new crate PR touches `Cargo.toml`, `Cargo.lock` and `contracts/ci/crate-graph.v1.json`, all CI "sentinel" paths, so hosted CI will run the **full** workspace test suite once (batch the whole phase into one PR). The generated crate API page `docs/src/api/vox-diagram.generated.md` is verified by `ssot-drift` but is **not** in `scripts/ssot-regen.vox`, so the `ssot-autoregen` bot will not create it: the executor must run `vox ci crate-api-sync --write` and commit the result (and re-run it after any later change to `lib.rs` top-level `pub` items or the Cargo `description`).

Two upstream premises are wrong in this tree and must be surfaced to the user before Phases 84/87/88 (details in Open Questions): (1) `crates/vox-doc-pipeline/src/pipeline/mermaid.rs`, `extract_mermaid_blocks`, `validate_and_repair_mermaid` and `vox_validate_mermaid` do not exist in any committed history here; the milestone research read them from uncommitted work-in-progress in the primary checkout (`/Users/brbrainerd/dev/vox`), so the `fence` module is greenfield; (2) the committed corpus is **33 fences / 21 files** (excluding `archive/`), not 41/26.

**Primary recommendation:** Build `vox-diagram` as one crate with ~14 small modules (all files <=400 non-blank lines), parse leniently but report Mermaid-isms as `Error` severity with exact fixes, keep every internal fn `pub(crate)` so only the thin public API needs same-file tests, drive correctness from three test families (fixture+insta, AST-first noisy-print proptests, mutate-and-repair proptest), and land crate registration plus `vox ci crate-api-sync --write` output as the first plan so hosted CI proves the governance gates early.

## Architectural Responsibility Map

This phase is a library, not a multi-tier app; "tier" below means crate/layer ownership.

| Capability | Primary Owner | Secondary | Rationale |
|------------|--------------|-----------|-----------|
| Lex / parse / resolve / IR / fmt / diagnostics / fixes | `vox-diagram` (L2, pure) | — | Pure functions over `&str`; zero I/O so every later consumer can share one parser (ARCHITECTURE.md headline decision 1) |
| Markdown fence extraction | `vox-diagram::fence` | — | One scanner for lint, render manifest, CLI, MCP; swap of consumers happens in Phase 87 |
| File walking, manifests, JSON shaping, clap | consumers (`vox-doc-pipeline`, MCP, CLI) | — | Not this phase; no edges added now |
| Governance registration | `layers.toml`, `where-things-live.md`, generated API page | `ssot-autoregen` bot for the other generated artifacts | `MissingLayer`/WTL/orphan rules are build-time gates |
| Determinism guarantee | IR types (Vec only) + grep guard test | Phase 85 SVG writer | HashMap-free from the first commit (PITFALLS #3) |

## Critical Findings That Change Planning (read first)

1. **`mermaid.rs` and friends are not in git.** [VERIFIED: `git ls-tree --name-only origin/main crates/vox-doc-pipeline/src/pipeline/` lists only `anchors.rs doctest.rs facet_backlog.rs facets.rs facets_tests.rs lint.rs mod.rs types.rs`; `git log --all -- crates/vox-doc-pipeline/src/pipeline/mermaid.rs` is empty; `git grep -il "vox_validate_mermaid"` hits only `.planning/` and the advisory doc.] The files exist as uncommitted work in `/Users/brbrainerd/dev/vox` (`ls` shows `mermaid.rs` 18726 bytes and `openwiki.rs`; `git status` there shows `M crates/vox-doc-pipeline/src/pipeline/{lint,mod,types}.rs`, `?? .../claims/consolidation.rs`). Consequence for Phase 83: the `fence` module is written from scratch (CONTEXT's "generalizes the existing `extract_mermaid_blocks`" does not hold). Consequence for later phases: REND-06's "existing validator as oracle", DOCS-01's seed, and TOOL-01's "mirror `vox_validate_mermaid`" depend on that WIP landing. Needs a user/orchestrator decision; not a Phase 83 blocker.
2. **Corpus count differs.** [VERIFIED: fence-aware scan of `docs/src` excluding `archive/` at HEAD 38bc7de26 printed `33 21`] vs the milestone's 41/26 (read from the dirty primary checkout). REQUIREMENTS MIG-03/DOCS-01 quote 41/26. Re-baseline at Phase 87/90 time.
3. **Registration PR runs the full CI suite.** [VERIFIED: `crates/vox-cli-ci/src/affected.rs:5-12`: `pub const SENTINEL_EXACT: &[&str] = &[ "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".config/hakari.toml", CRATE_GRAPH_SENTINEL, ];` and lines 151-153 `if changed_files.iter().any(|f| is_sentinel(f)) { return Affected::Full; }`.] One PR for the whole phase pays this once.
4. **API page is hand-run.** [VERIFIED: `scripts/ssot-regen.vox:26-43` lists operations-sync, command-sync, plugin-*, capability-*, gui-*, `affected-crates --regen`, `crate-build-map-parity`, `test-inventory`, `config-gui-codegen` and no `crate-api-sync`; `run_ssot_drift` in `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs:651-654` calls `vox_cli_ci::crate_api_docs::run(root, false)` (verify mode).] Precedent: commit `79d9cd454 chore(ssot): sync crate api docs` added `docs/src/api/vox-kb-facets.generated.md` after the crate landed.
5. **Locked proptest is 1.11.0**, not 1.12.0 as STACK.md says. [VERIFIED: `grep -n 'name = "proptest"' -A1 Cargo.lock` -> `version = "1.11.0"`.] `unicode-width` is NOT needed in Phase 83 (label cap counts `chars()`); it belongs to Phase 85.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde` (derive) | `1` / locked 1.0.228 | `Serialize` (+`Deserialize` for round-trip tests) on IR, `Diagnostic`, `CheckReport`, sequence IR | Workspace pin `serde = { version = "1", features = ["derive"] }` [VERIFIED: Cargo.toml:207] |
| `pulldown-cmark` | `0.13` / locked 0.13.4 | `fence` module (CommonMark-correct fenced-block extraction with byte ranges) | Workspace pin `pulldown-cmark     = "0.13"` [VERIFIED: Cargo.toml:412]; already unified in `workspace-hack`; agrees with micromark (Astro/remark) on indentation stripping, which Phase 89's exact-source manifest key depends on |
| `strsim` | `0.11` / locked 0.11.1 | `levenshtein` for `help` did-you-mean text only | Workspace pin `strsim             = "0.11"` [VERIFIED: Cargo.toml:392]; precedent L2 consumer `vox-capability-registry` [VERIFIED: crates/vox-capability-registry/Cargo.toml:14] |

### Supporting (dev-dependencies)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `insta` | `1` + `json` / locked 1.48.0 | Golden IR JSON, diagnostics JSON, canonical text | `insta = { version = "1", features = ["json"] }` [VERIFIED: Cargo.toml:265]; ~10-15 goldens only |
| `proptest` | `1`, `default-features=false, features=["std"]` / locked 1.11.0 | fmt idempotence/round-trip, never-panics, mutate-and-repair | [VERIFIED: Cargo.toml:400] `proptest           = { version = "1", default-features = false, features = ["std"] }`; no `fork`/`timeout` features, so no per-case timeouts |
| `serde_json` | `1` / locked | Dev-only: JSON byte-identity tests and snapshots | Consumers (MCP) own runtime JSON; keep it out of `[dependencies]` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `pulldown-cmark` fence scan | ~100-line hand-rolled line scanner (as `lint.rs`/`doctest.rs` do) | Zero dep, but diverges from micromark on list/blockquote indentation; Phase 89 keys the manifest by exact normalized body, so divergence = build failure. Fall back to hand-rolled only if the planner wants literally zero third-party deps (then replicate CommonMark rules from the spec below) |
| `strsim` | 15-line local Levenshtein | Avoids one dep edge for help text; precedent exists in-repo (`vox-codegen/src/web_ir/validate_palette.rs:80`). Either is fine; recommend `strsim` (already L2 precedent) |
| `thiserror` | — | Not needed: diagnostics are data, there is no error enum to derive |
| `miette` | — | Rejected by STACK/CONTEXT (no `miette` in the library) |
| `chumsky`/`winnow`/`logos` | — | Rejected: small grammar, span-precise recovery needs hand control (`logos = "0.16"` is a workspace dep, Cargo.toml:204, still not used) |

**Installation (Cargo.toml for the new crate):**
```toml
[package]
name = "vox-diagram"
description = "Pure VoxDiagram core: graph DSL lexer, error-recovering parser, resolved IR, diagnostics with machine-applicable fixes, canonical formatter, and markdown fence scanner."
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
pulldown-cmark = { workspace = true }
strsim = { workspace = true }
workspace-hack = { version = "0.6", path = "../workspace-hack" }

[dev-dependencies]
insta = { workspace = true }
proptest = { workspace = true }
serde_json = { workspace = true }

[lints]
workspace = true
```
(Shape mirrors `vox-kb-facets`: [VERIFIED: crates/vox-kb-facets/Cargo.toml] `description = "L0 KB facet contract: ..."`, `version.workspace = true`, `edition.workspace = true`, `license.workspace = true`, `workspace-hack = { version = "0.6", path = "../workspace-hack" }`, `[lints] workspace = true`.)

**Version verification:** versions above come from `Cargo.lock` (no network needed; nothing new enters the lock except the `vox-diagram` package itself). `cargo-hakari 0.9.39` is installed; `.config/hakari.toml` unifies `serde`, `pulldown-cmark`, `strsim` already (they are existing workspace deps), so `cargo hakari generate --diff` is expected to be empty.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| pulldown-cmark | crates.io | 11 yrs (2015-06) | ~4.4M/wk | github.com/raphlinus/pulldown-cmark | OK | Approved (already locked 0.13.4) |
| strsim | crates.io | 11 yrs (2015-02) | ~19M/wk | github.com/rapidfuzz/strsim-rs | OK | Approved (already locked 0.11.1) |
| serde | crates.io | 11 yrs | ~28M/wk | — | OK | Approved (existing) |
| insta | crates.io | 7 yrs | ~2.5M/wk | — | OK | Approved (existing dev-dep) |
| proptest | crates.io | 9 yrs | ~4.5M/wk | — | OK | Approved (existing dev-dep) |

Source: `gsd-tools query package-legitimacy check --ecosystem crates ...` run this session; all returned `OK`. **Packages removed (SLOP):** none. **Flagged (SUS):** none. No package is new to the workspace; `[VERIFIED: Cargo.lock]` for all versions.

## Architecture Patterns

### System Architecture Diagram

```
source &str ──► limits gate (len > 65_536 B?) ──► YES: one `diagram/limit-source`, STOP (no parse)
                       │ no
                       ▼
                 lexer  (single pass, byte offsets, char-boundary safe)
                  │  tokens (incl. Mermaid-ish tokens) + comment/blank trivia list + lex diagnostics
                  ▼
            recursive-descent parser  (fuel counter, diag cap 100, iterative skip past depth 4)
                  │   statement error ─► declare-then-diagnose ─► sync at NL / ';' / '}'
                  │   Mermaid-ism   ─► lenient AST node + `diagram/mermaid-syntax` + Fix edits
                  ▼
               AST (Document → Block{entries: Comment | Item}) + syntax diagnostics
                  │
          ┌───────┴────────┐
          ▼                ▼
   resolver (pass A: declare all names per scope; pass B: resolve edge refs)
          │  semantic diagnostics (duplicate/undeclared/container-endpoint/limits/...)
          ▼
   sort+cap diagnostics ──►  CheckReport { schema_version, valid, diagnostics[], stats, ir?: Ir, normalized_source?: String }
          │                                   ir / normalized_source present iff zero Error diagnostics
          ├─► print::to_source(AST)  = `fmt`  (idempotent, comments + single blank lines kept)
          └─► fixes::apply_fixes / repair  (deterministic, bounded rounds)

markdown &str ──► fence::fences / fence::extract(md, "vox-diagram") ──► Fence{info, body, spans, closed}
sequence IR types (serde only)  ── defined, no parser (Phase 86)
```

### Recommended Project Structure

All files stay <=~400 non-blank lines (hard Error at 500; Warning at 400), directories <=20 files, no generic names (`utils.rs`, `helpers.rs`, `misc.rs`, `common.rs` are forbidden [CITED: docs/agents/governance.md "Naming Enforcement"]).

```
crates/vox-diagram/
├── Cargo.toml
├── src/
│   ├── lib.rs            # `//!` first (strict for L2), `#![forbid(unsafe_code)]`, mod decls, `pub use`; NO fn bodies
│   ├── limits.rs         # named consts: MAX_SOURCE_BYTES=65_536, MAX_NODES=500, MAX_EDGES=1_000, MAX_DEPTH=4, MAX_LABEL_CHARS=200, MAX_DIAGNOSTICS=100
│   ├── codes.rs          # `pub const UNDECLARED_NODE: &str = "diagram/undeclared-node";` ... + ALL_CODES
│   ├── span.rs           # Span{start,end:u32}, LineIndex (binary-searched line starts), Span::slice via `get`
│   ├── diag.rs           # Diagnostic, Severity, Fix, Related, Stats, CheckReport
│   ├── lexer.rs          # Token, TokenKind, lex()
│   ├── ast.rs            # Document, Block, Entry, Item, NodeDecl, ContainerDecl, EdgeStmt, Ref, Label, Comment
│   ├── parser/           # mod.rs (driver, fuel, cap), stmt.rs, label.rs (string/label recovery), recover.rs, mermaid.rs
│   ├── resolve.rs        # scope tree, hoisted declarations, ref resolution -> Ir
│   ├── ir.rs             # GraphIr, IrNode, IrContainer, IrEdge, Shape, Direction (serde)
│   ├── sequence_ir.rs    # SequenceIr, Actor, SeqEvent (serde only, no parser)
│   ├── print.rs          # canonical printer (fmt)
│   ├── fixes.rs          # apply_fixes, repair
│   ├── fence.rs          # pulldown-cmark based extraction, normalize
│   └── card.rs           # `pub const GRAMMAR_CARD: &str = include_str!("../grammar-card.txt");`
├── grammar-card.txt      # <=~600-token card (draft below)
└── tests/
    ├── fixtures/{valid,broken,mermaid,adversarial}/*.vdg   # `.vdg` is a tentative internal extension (A7)
    ├── snapshots/        # insta
    ├── graph_constructs.rs  diagnostics_all_errors.rs  fixes_repair.rs  fmt_props.rs
    ├── limits_adversarial.rs  fence_scan.rs  determinism_guard.rs  sequence_ir.rs
    └── (proptest-regressions/ committed if proptest writes any)
```

### Public API (thin; everything else `pub(crate)`)

```rust
pub fn check(source: &str) -> CheckReport;                 // lex+parse+resolve; never panics
pub fn parse(source: &str) -> Parsed;                      // best-effort AST + diagnostics (consumers rarely need it)
pub fn fmt(source: &str) -> Option<String>;                // Some(canonical) iff zero Error diagnostics
pub fn apply_fixes(source: &str, diags: &[Diagnostic]) -> FixOutcome;   // one round
pub fn repair(source: &str, max_rounds: usize) -> Repaired;             // check -> apply_fixes loop, bounded
pub mod fence { pub fn fences(md: &str) -> Vec<Fence>; pub fn extract<'a>(md: &'a str, info: &str) -> Vec<Fence<'a>>; pub fn normalize(body: &str) -> String; }
pub const GRAMMAR_CARD: &str;
```
Keep these definitions in modules that carry their own `#[cfg(test)] mod tests`; `lib.rs` only re-exports (a `pub use` is not a `pub fn` definition).

### Pattern 1: Declare-then-diagnose recovery
**What:** When a statement fails after its head keyword and id were read, still push the declaration into the AST (placeholder label = best-effort text or the id) and mark `recovered: true`; then emit the diagnostic and sync. **Why:** otherwise every later edge to that id yields a false `undeclared-node` (cascade), violating "all independent errors reported" exactly-once expectations. **Test:** a fixture with 6 independent mistakes on separate lines must yield exactly 6 diagnostics.

### Pattern 2: Lenient parse, strict report
Mermaid-isms produce the same AST as their canonical form plus an `Error` diagnostic carrying the rewrite. `check().ir` is `None` while any Error exists, so nothing downstream renders a non-canonical document, but `repair` converges in 1-3 rounds.

### Pattern 3: Hoisted two-pass resolution
Pass A builds all scopes (names -> entries) from the AST; pass B resolves edge refs. Forward references and fix insertion position are then order-independent.

### Pattern 4: Ordered-entry AST for trivia
`Block.entries: Vec<Entry>` where `Entry::Comment(Comment{text, blank_before})` | `Entry::Item(Item{..., blank_before, trailing: Option<Comment>})`. Own-line comments stay in sequence (adjacent = "attached"); only same-line comments need explicit attachment (`Item.trailing`, `ContainerDecl.header_trailing`, `ContainerDecl.close_trailing`). The printer just walks entries; idempotence follows from printing a function of the AST.

### Anti-Patterns to Avoid
- **`HashMap`/`HashSet` anywhere in the crate** (even internally): use `Vec` + `BTreeMap<&str, usize>`; enforced by a grep test (below).
- **Slicing source with `&src[a..b]`:** panics on non-char-boundary. Always `src.get(a..b)` via `Span::slice`.
- **Recursion depth driven by input:** only containers recurse, and recursion stops at depth 4 (see Limits).
- **Per-diagnostic `O(n)` line/col computation:** build `LineIndex` once.
- **Regex-based lexing/parsing** (the failure mode the milestone exists to fix).
- **Dedup by "identical edits" inside `apply_fixes`:** breaks multiple `}` insertions at EOF; dedup at emission time instead (first diagnostic per `(scope,id)` carries the declare-fix).

## DSL Grammar (EBNF for the decided syntax)

```text
document    = { trivia } , "graph" , "{" , block , "}" , { trivia } , EOF ;      (* Phase 86 adds an alternative: "sequence" "{" ... "}" *)
block       = { NL | comment | stmt } ;
stmt        = ( direction | node | container | edge ) , stmt_end ;
stmt_end    = NL | comment | EOF | lookahead("}") ;
direction   = "direction" , ":" , ( "down" | "right" | "up" | "left" ) ;          (* once per graph block; not inside a container *)
node        = "node" , ident , ":" , string , [ attrs ] ;
attrs       = "{" , { NL } , [ attr , { ( "," | NL ) , { NL } , attr } ] , { NL } , "}" ;
attr        = "shape" , ":" , shape ;
shape       = "rect" | "rounded" | "diamond" | "cylinder" | "start" | "end" ;
container   = "container" , ident , ":" , string , "{" , block , "}" ;
edge        = ref , "->" , ref , [ ":" , string ] ;
ref         = ident , { "." , ident } ;
ident       = ( letter | "_" ) , { letter | digit | "_" | "-" } ;                 (* ASCII; a "-" is part of the ident only if the next char is not ">" and not "-" *)
string      = '"' , { chr | escape } , '"' ;                                      (* single line; chr excludes '"', '\', NL, forbidden controls *)
escape      = "\" , ( '"' | "\" | "n" ) ;
comment     = "//" , { any - NL } ;
```

Lexical decisions (Claude's-discretion area; all [ASSUMED] unless noted, planner confirms):
- **Statement terminator:** newline (LF or CRLF). `;` is NOT in the grammar; it is lexed as `Semi` and handled as a Mermaid-ism (below). Blank lines and BOM at offset 0 are trivia. Tabs/`\r` are whitespace.
- **Keywords are contextual, not globally reserved:** a statement-head ident `node`/`container`/`direction` followed by `ident`/`:` starts that declaration; followed by `->` or `.` it begins an edge. Declaring an id equal to `graph sequence direction node container shape` -> `diagram/reserved-word` (no fix). `start`/`end`/`rect`... are shape *values*, never reserved as ids.
- **Idents:** ASCII `[A-Za-z_][A-Za-z0-9_-]*` with the `-`/`->` lookahead rule so `a->b` and `api-gw -> db` both lex; leading digit or non-ASCII -> `diagram/invalid-identifier` (no fix). Hyphen support (kebab ids) is a deliberate LLM-friction reducer [ASSUMED A1].
- **Strings:** single-line; escapes `\"`, `\\`, `\n` only. Other `\x` -> `diagram/invalid-escape` (fix: double the backslash, which preserves the literal text). Raw control chars U+0000-U+0008, U+000B-U+000C, U+000E-U+001F, U+007F, and bidi overrides U+202A-U+202E / U+2066-U+2069 inside strings -> `diagram/invalid-character` (fix: delete). Tab (U+0009) allowed. This makes the IR's label text XML-safe for the Phase 85 writer (PITFALLS #12 control-character trap) [ASSUMED A8].
- **Spans:** `Span{start,end}` as `u32` byte offsets; diagnostics additionally carry 1-based `line`/`col`/`end_line`/`end_col` (col = count of Unicode scalar values from line start). CRLF is lexed as one newline but offsets stay in original bytes.
- **`direction`:** statement in the graph block (position preserved, never hoisted by `fmt`); absent -> `down`; a second one -> `diagram/duplicate-direction`; inside a container -> `diagram/direction-in-container`.
- **Shape:** `shape: rect` explicit is normalized to the default at parse time (AST stores `shape: Shape`, `Rect` = default) so `fmt` omits `{ shape: rect }` and AST equality holds.
- **Attr block newlines:** newlines inside `{ ... }` of a `node` are allowed (D2-style multi-line attr block is a likely LLM form); canonical output is single-line `{ shape: cylinder }`. On a malformed attr block, skip to the first `}` but stop (do not consume) at a line-leading `node`/`container` keyword or EOF.

## Parser and Error Recovery

**Strategy:** panic-mode recovery with synchronizing tokens (standard for hand-written RD parsers) plus three refinements.

1. **Sync set:** `Newline`, `Semi`, `RBrace` (not consumed, belongs to the enclosing block), `Eof`. `skip_to_stmt_end()` advances to the first sync token at brace-depth 0 relative to the skipped region.
2. **Declare-then-diagnose** (Pattern 1) for `node`, `container` headers; an edge whose either endpoint cannot be parsed is dropped (no cascade because edges declare nothing).
3. **Container header recovery:** if the label is missing/garbled but a `{` exists on the same line, parse the body as a container with placeholder label; if no `{` on the line, record `diagram/unexpected-token`, create the container with an empty body, and let the following `}` surface as `diagram/trailing-content` (acceptable cascade; document it). Never swallow more than the header line to find `{`.
4. **Lexer-level string recovery:** a string reaching raw newline/EOF -> `Str{terminated:false, value: text to EOL}` + `diagram/unterminated-string` (fix: insert `"` at end of line after trimming trailing whitespace). The parser then treats it as a normal label, so no cascade.
5. **EOF with open scopes:** one `diagram/unclosed-brace` per open container/graph block at its opening `{` span; each carries a fix inserting `\n}` at EOF (identical text, applied in diagnostic order; `apply_fixes` must NOT dedup these).
6. **Progress/fuel guarantee:** `fuel = 4 * token_count + 1_000`, decremented once per parser loop iteration and per scan-ahead step; on exhaustion emit `diagram/limit-diagnostics` (see cap below) and stop. Additionally `debug_assert!(pos > last_pos)` at each statement loop head. Together with the invariant "every scan-ahead either consumes what it scanned or ends the statement" this yields O(n) total work (without it, a 64 KiB line of `[` characters, each scanning ahead for `]`, is O(n^2)).
7. **Cap:** at most 99 real diagnostics + one `diagram/limit-diagnostics` (Warning) as the 100th; after the cap the parser stops emitting and parsing. Final list sorted by `(span.start, code)`; exact duplicates (same code+span) removed.
8. **Depth cap without recursion:** on opening a container at depth > 4, emit `diagram/limit-depth` once (span = that container header), then skip its body iteratively with a brace counter instead of recursing, so 65 000 `{` cannot overflow the stack.

## Scope Resolution (nested containers + dotted refs)

Data: `scopes: Vec<Scope>` (root = graph block); `Scope{ parent: Option<usize>, path: String, names: BTreeMap<&str, Entity> }`, `Entity = Node(idx) | Container(idx, child_scope)`. Nodes and containers share one namespace per scope (Mermaid-compatible).

- **Pass A (declare), AST order:** insert each name; existing name in the same scope -> `diagram/duplicate-id` (primary span = the later id; `related` = first declaration; keep the first). A name that also exists in any ancestor scope -> `diagram/shadowed-id` (Warning; innermost-wins is locked, this just makes silent mis-links visible). Reserved word -> `diagram/reserved-word`.
- **Pass B (resolve), edge order:** first segment: search current scope, then each ancestor outward; the first scope containing it wins. Each later segment: lookup only among the children of the container resolved so far (no outward search). Outcomes: unknown first segment -> `diagram/undeclared-node` (fix only if the ref is a single segment; dotted with unknown prefix -> no fix); intermediate segment resolves to a node -> `diagram/not-a-container` (span = the remaining segments); unknown later segment -> `diagram/undeclared-node` with fix declaring it inside that container; final entity is a container -> `diagram/container-endpoint` (no fix; `help` names the container's first child as `container.child`). Stop at the first failing segment (keeps per-ref work O(depth * log n)).
- **IR ids:** full path joined by `.` (`backend.gateway`), unique by construction because idents cannot contain `.`. Nodes/containers appear in DFS pre-order of declaration; edges in source order.
- **Declare-fix position (Rule R):** insert inside the target scope at the end of the line that holds its opening `{` (replacement `"\n" + indent + 4 spaces + "node x: \"x\""`), because declarations-first matches the canonical example's shape and all inserts for one scope share one offset (applied in diagnostic order). If the scope's closing `}` is on that same line (single-line scope), insert immediately after the `{` with replacement `"\n" + indent + 4 + decl + "\n"`. Only the FIRST diagnostic per `(scope,id)` carries the fix. If the scope's `{` is absent (Mermaid header case) no fix this round; the header fix runs first and round 2 produces it.

## Diagnostics and Fixes

### Model
```rust
pub struct Diagnostic { pub code: &'static str, pub severity: Severity /* Error | Warning */, pub message: String,
    pub span: Span /* +line,col,end_line,end_col on the serialized form */, pub help: Option<String>,
    pub fixes: Vec<Fix>, pub related: Vec<Related> }
pub struct Fix { pub span: Span, pub replacement: String }   // matches the locked `{span, replacement}`
```
Semantics (the clarification the locked text leaves open [ASSUMED A11]): **all `fixes` of one diagnostic are one atomic remedy** (multi-edit rewrites like `subgraph..end` need >1 edit); fixes of different diagnostics never overlap by construction except zero-width inserts at the same offset, which apply in diagnostic order. `valid` = no `Error`. Warnings never carry fixes. JSON uses `skip_serializing_if` for empty `help/fixes/related` to save agent tokens; add `"schema_version": 1` on `CheckReport` only [ASSUMED A13].

### `apply_fixes(source, diags) -> FixOutcome{text, applied, skipped}`
1. Walk diagnostics in order; for each non-empty `fixes` group, accept it only if every edit lies on char boundaries within `0..=len` and no edit overlaps an already-accepted edit (overlap = intersecting non-empty ranges, or a zero-width insert strictly inside a replaced range; equal-offset inserts are fine).
2. Sort accepted edits by `(start, end, acceptance_seq)` (stable) and build the output in one forward pass (O(n + k log k)).
3. Never panic: a bad span skips its group.
`repair(source, max_rounds=5)`: `check` -> `apply_fixes` -> repeat until no fix-bearing diagnostics or no text change; return text, rounds, remaining diagnostics. Used by the "seeded broken set" criterion and the mutate-and-repair property.

### Code catalogue (stable; add only) [mix of CONTEXT-locked names and recommended additions]
| Code | Sev | Fix? | Trigger |
|------|-----|------|---------|
| `diagram/unterminated-string` | Error | insert `"` at EOL | string hits newline/EOF |
| `diagram/invalid-escape` | Error | double the backslash | `\t`, `\x`, ... |
| `diagram/invalid-character` | Error | delete char | forbidden control/bidi char in string |
| `diagram/unexpected-token` | Error | — | generic "expected X, found Y" (names both) |
| `diagram/expected-quoted-label` | Error | quote + escape | unquoted or single-quoted label after `:` |
| `diagram/unclosed-brace` | Error | insert `\n}` at EOF | open `{` never closed |
| `diagram/trailing-content` | Error | — | tokens after the closing `}` of the graph block (incl. stray `}`) |
| `diagram/unknown-diagram-kind` | Error | — | header is not `graph` (`sequence` becomes valid in Phase 86) |
| `diagram/invalid-identifier` | Error | — | leading digit / non-ASCII |
| `diagram/reserved-word` | Error | — | id equals a keyword |
| `diagram/invalid-direction` | Error | — (help: did-you-mean) | unknown direction word |
| `diagram/duplicate-direction` | Error | — | second `direction` |
| `diagram/direction-in-container` | Error | — | `direction` inside container |
| `diagram/invalid-shape` | Error | case-fold exact match only | unknown shape word |
| `diagram/unknown-attribute` | Error | — | key other than `shape` |
| `diagram/duplicate-id` | Error | — (`related` = first) | same scope |
| `diagram/undeclared-node` | Error | declare-fix (Rule R) | unresolved ref |
| `diagram/not-a-container` | Error | — | dotted path through a node |
| `diagram/container-endpoint` | Error | — | edge endpoint resolves to a container |
| `diagram/empty-label` | Error | `""` -> `"<id>"`; edge `: ""` -> delete | empty quoted label |
| `diagram/shadowed-id` | Warning | — | inner id hides outer id |
| `diagram/mermaid-syntax` | Error | when exact (catalogue below) | any Mermaid-ism |
| `diagram/chained-edge` | Error | split into two edges | `a -> b -> c` (P2) |
| `diagram/limit-source` | Error | — | > 65_536 bytes (only diagnostic; no parse) |
| `diagram/limit-nodes` / `-edges` / `-depth` | Error | — | first item past 500 / 1_000 / depth 4; one diagnostic per kind, message carries the total |
| `diagram/limit-label` | Error | — | decoded label > 200 `chars()` |
| `diagram/limit-diagnostics` | Warning | — | 100th-slot sentinel |

A unit test asserts every code string emitted anywhere is in `codes::ALL_CODES`, and a golden test asserts the committed sorted code list is a subset of the current list (never renamed, only added).

## Mermaid-ism Detection and Canonical Rewrites

All are `diagram/mermaid-syntax`, `Error`, parsed leniently into the canonical AST. **P1** = required this phase (the CONTEXT names `-->`, `A[Label]`, `subgraph … end`, "etc."); **P2** = nice-to-have, safe to defer. Corpus facts: only the arrow `-->` occurs (279/279 edges), 26 labelled edges in three spellings, shapes rect/diamond/rounded/cylinder, 25 subgraphs, 0 style/click usage [CITED: .planning/workstreams/voxdiagram/research/FEATURES.md section 0.3].

| # | Pri | Trigger (lexer/parser) | Lenient AST | Fix edits (all in ONE diagnostic) |
|---|-----|------------------------|-------------|-----------------------------------|
| M1 | P1 | arrow token `-{2,}>` between refs | Edge | replace token with `->`. `==>`, `-.->`, `---`, `--x`, `--o`: diagnostic WITHOUT fix (style would be lost) |
| M2 | P1 | `-->\|text\|` or `-- text -->` | Edge with label | (a) replace `[arrow..closing pipe]` / `[-- .. -->]` with `->`; (b) insert `: "text"` after the target ref. Text: trim, strip surrounding quotes, `<br>`/`<br/>` -> `\n`, then escape `\` and `"` |
| M3 | P1 | ident immediately followed by `[` `(` `{` | implicit `NodeDecl` in current scope | statement-start form: replace `A[Label]` with `node a: "Label"` (+ ` { shape: ... }` if not rect). Inside an edge: (a) replace `A[Label]` with `A`; (b) insert `node A: "Label"` via Rule R. Exact shape map: `[ ]` rect, `( )` rounded, `{ }` diamond, `[( )]` cylinder. `(( ))`, `([ ])`, `[[ ]]`, `{{ }}`, `>..]` -> diagnostic without fix. Repeated bracket form for an id already declared: same label+shape -> only replace with bare id; different -> `diagram/duplicate-id` |
| M4 | P1 | statement-head `subgraph` ... matching `end` | Container (frame kind `MermaidSubgraph`) | `subgraph ID` / `subgraph ID[Title]` / `ID["Title"]` -> header replaced by `container id: "Title" {` (title defaults to id) AND the `end` line replaced by `}` — one diagnostic, two edits (create it at the header, append the `end` edit when the frame closes). Title-only form `subgraph My Title`: no fix. `end` counts as a close only inside a MermaidSubgraph frame and only when it is the whole statement |
| M5 | P1 | first line `graph`/`flowchart` + optional `TD\|TB\|BT\|LR\|RL` + optional `;`, no `{` | Document, body parsed to EOF | replace header with `graph {\n    direction: <down\|right\|up\|left>` and insert `\n}\n` at EOF (TD,TB->down, LR->right, RL->left, BT->up) |
| M6 | P1 | `direction LR` (no colon) or `direction: LR` | DirectionStmt | replace with `direction: right` (mapped) |
| M7 | P1 | `%%` comment | Comment | replace `%%` with `//` (no fix for `%%{` directives) |
| M8 | P1 | `;` | terminator | trailing: delete `;`; mid-line: replace `;` with newline + the statement's indent |
| M9 | P1 | statement-head `style`, `classDef`, `class`, `linkStyle`, `click`, `callback`, `accTitle`, `accDescr`, suffix `:::name` | skipped to EOL | none; message: "styling is theme-controlled; VoxDiagram has no equivalent" |
| M10 | P2 | `A & B --> C`, `a -> b -> c` | expand | replace statement with the expanded edge lines (`diagram/chained-edge` for the `->` chain) |

Also DSL-native (non-Mermaid) lenient forms: unquoted or `'single'`-quoted label -> `diagram/expected-quoted-label` with quote+escape fix (unquoted text runs to end of line, or to the first ` {` on `node`/`container` lines).

**Fix-convergence requirement:** M1 on `A --> B` with undeclared A, B converges in 2 rounds (round 1 arrow, round 2 two declare-fixes). M3 declares in pass A so it does not also trigger `undeclared-node`.

## Formatter (`fmt`) and Comment Attachment

Canonical form (all [CITED: CONTEXT.md] except where marked):
- 4-space indent per nesting level; one statement per line; declaration order preserved (never sorted, `direction` not hoisted); LF only; exactly one trailing `\n`; no trailing whitespace.
- `graph {` ... `}`; `container id: "Label" {` ... `}` (empty container prints `{` and `}` on separate lines); `node id: "Label"` and `node id: "Label" { shape: cylinder }` (block omitted when `rect`); `a -> b` / `a -> b: "label"` with single spaces; refs printed dotted.
- Labels re-escaped: `\` -> `\\`, `"` -> `\"`, U+000A -> `\n`; tab printed raw.
- **Blank lines [ASSUMED A3]:** collapse runs to one; none at block start/end. Rationale: the CONTEXT canonical example has blank-line grouping and must itself be a fixed point (`fmt(example) == example` is a required golden test).
- **Comments:** own-line comments remain `Entry::Comment` in sequence (blank-before preserved as above); same-line trailing comment printed after one space (`node a: "A" // note`); comment on a container header line prints after `{`; comment on a closing `}` line prints after `}`; comments before `graph` / after the final `}` live on `Document.leading/trailing`; comment text is preserved verbatim after `//` (only trailing whitespace trimmed, no space inserted) so `//x` stays `//x` (idempotent).
- `fmt` returns `None` when any `Error` exists (including Mermaid-isms; use `apply_fixes`/`repair` first).

AST equality for the round-trip property compares span-stripped AST including comment text and `blank_before` flags (fmt keeps them, so equality holds) AND the span-stripped IR.

## IR

- `Ir` is an enum `Graph(GraphIr) | Sequence(SequenceIr)`, serde `#[serde(tag = "kind", rename_all = "kebab-case")]` (no `schema_version` here; it lives on `CheckReport` only).
- `GraphIr { direction, nodes: Vec<IrNode>, containers: Vec<IrContainer>, edges: Vec<IrEdge> }`; `IrNode { id, local_id, label, shape, container: Option<String>, span }`; `IrContainer { id, local_id, label, parent: Option<String>, depth, span }`; `IrEdge { from, to, label: Option<String>, span }` with `from`/`to` = full path ids. Spans: byte offsets plus `line`/`col`.
- `Shape` / `Direction` serialize lowercase (`rect rounded diamond cylinder start end`, `down right up left`).
- `SequenceIr { actors: Vec<Actor{id,label,span}>, events: Vec<SeqEvent> }`, `SeqEvent = Message{from,to,text,dashed:bool,span} | Note{over:Vec<String> /*1..=2*/,text,span}`; derive both `Serialize` and `Deserialize`; one serde round-trip test; no parser, no `pub fn` (so no tdd-guard exposure).
- Determinism: only `Vec`/`String`/enums/`u32`/`bool`; no floats; derive field order is the JSON order. Guard test: scan `src/**/*.rs` for the needles built via `concat!("Hash", "Map")`, `concat!("Hash", "Set")` (so the test file does not match itself) and assert none.

## Limits and Bounded Time (DGM-06)

| Cap | Const | Enforced at | Diagnostic span | Behavior after trip |
|-----|-------|-------------|-----------------|---------------------|
| Source 64 KiB | `MAX_SOURCE_BYTES = 65_536` | entry of `check`/`parse`/`fmt`, before lexing | first byte past the cap to end (char-boundary adjusted) | only this diagnostic; no parse (O(1)) |
| Nodes 500 | `MAX_NODES` | parser on node #501 | that node decl | one diagnostic per kind (message carries the total); items still parsed for other errors but not added to IR |
| Edges 1,000 | `MAX_EDGES` | parser on edge #1001 | that edge stmt | same |
| Depth 4 | `MAX_DEPTH` | container open at depth 5 | container header | iterative skip of body |
| Label 200 | `MAX_LABEL_CHARS` (`chars().count()` of the decoded value) | lexer/parser per label | the string token | per-label error; capped by the 100 rule |

Bounded-time argument to put in the module docs: lexer O(n); parser O(tokens) by fuel; resolver O(edges * depth * log nodes); printer O(n); `apply_fixes` O(n + k log k); `LineIndex` O(n) build, O(log n) lookup. Test with a loose in-test hang detector (e.g. 10 s) plus nextest's own `slow-timeout = { period = "60s", terminate-after = 3 }` [VERIFIED: .config/nextest.toml:4]; do not assert tight wall-clock numbers (flaky on a loaded machine).

## Fence Module (generalizes nothing: greenfield)

Use `pulldown_cmark::Parser::new_ext(md, Options::ENABLE_YAML_STYLE_METADATA_BLOCKS).into_offset_iter()`; collect `Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))` ... `Event::Text` ... `Event::End(TagEnd::CodeBlock)`; the idiom is already used in-repo [VERIFIED: crates/vox-doc-pipeline/src/claims/extractor.rs:3,61-70: `use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};`, `Parser::new_ext(content, Options::all()).into_offset_iter()`, `Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))`].

```rust
pub struct Fence<'a> { pub info: &'a str /* trimmed */, pub body: String /* concatenated Text events = indentation-stripped content */,
    pub open_line: u32 /* 1-based line of the opening fence */, pub body_range: Range<usize>, pub closed: bool }
pub fn extract<'a>(md: &'a str, info: &str) -> Vec<Fence<'a>>  // exact: info.trim() == info (matches doctest.rs semantics; `vox-diagram,foo` is NOT a match)
pub fn normalize(body: &str) -> String                         // CRLF/CR -> LF, trim_end; the Rust side of Phase 89's JS key normalization
```
CommonMark rules the scanner inherits [CITED: spec.commonmark.org/0.31.2 fenced code blocks]: fence = >=3 backticks or tildes; opening may be indented up to 3 spaces; "If the info string comes after a backtick fence, it may not contain any backtick characters"; closing fence same character and "at least as many" as the opener, only spaces after; unclosed fence runs to end of containing block/document; up to N spaces of opening indentation are stripped from content lines. Test fixtures: prose that merely mentions the fence text, a 4-backtick outer fence containing a 3-backtick inner example (inner is body text), tilde fences, a fence inside a list item and a blockquote, an unclosed fence at EOF, `\r\n` input, `.mdx` with frontmatter and an `import` line, `vox` vs `vox-diagram` vs `vox-diagram,x` info strings. Limitation to document: pulldown-cmark treats MDX JSX as HTML blocks, same as every other scanner; JSX-nested fences are best-effort.

## Grammar Card Draft (<= ~600 tokens; finalized in Phase 88)

File `crates/vox-diagram/grammar-card.txt` (plain text, no ``` fences so it can be embedded later):

```text
VoxDiagram graph. Put it in a ```vox-diagram fence. EVERY label is "double quoted".

graph {
    direction: right                 // down | right | up | left (optional, default down)

    node client: "Desktop Client"
    node db: "Task DB" { shape: cylinder }   // rect (default) rounded diamond cylinder start end

    container backend: "Orchestrator" {      // containers nest, max depth 4
        node gateway: "Gateway"
        node engine: "Agent Runtime"
        gateway -> engine: "dispatch"        // edge label optional
        engine -> engine: "retry"            // self-loop ok
    }

    client -> backend.gateway: "TLS"         // dotted path reaches inside a container
    backend.engine -> db
}

Rules
1. Declare every node: node id: "Label". An edge never creates a node.
2. The arrow is exactly ->. Endpoints are nodes, never containers (use container.node).
3. Ids: letters, digits, _ and -; start with a letter or _. Reserved: graph node container direction shape.
4. One statement per line. Comments are // only. Escapes in labels: \" \\ \n.
5. Limits: 64 KiB, 500 nodes, 1000 edges, depth 4, labels 200 chars.

Do NOT write Mermaid:
  A[Label] --> B           use  node a: "Label"  then  a -> b
  subgraph X ... end       use  container x: "X" { ... }
  graph TD / flowchart LR  use  graph { direction: down }
  -->|text| or -- text --> use  a -> b: "text"
  %% comment, trailing ;   use  // comment, no semicolon
  style / classDef / click not supported

Run vox_validate_diagram; apply every fixes[] entry; repeat until valid.
```
Token estimate ~520 at 4 bytes/token [ASSUMED A6: no tokenizer available; the test uses `len() <= 2_600` as a proxy]. Dogfood tests (so the card cannot lie): (1) slice from the first `graph {` line to the first column-0 `}`; assert `check` returns zero diagnostics and `fmt` is a fixed point except for the inline `//` alignment (or normalise card comments to single-space); (2) for each "Do NOT write" left-hand example, assert `check` yields `diagram/mermaid-syntax` (this doubles as the M-catalogue smoke test).

## Test Strategy (insta + proptest)

**Fixtures** (`tests/fixtures/**/*.vdg`): `valid/` (canonical example verbatim from CONTEXT; every DGM-01 construct; comments everywhere), `broken/NN-name.vdg` each starting with `// expect: diagram/a, diagram/b` (the harness compares the exact sorted code list, then runs `repair` and `insta::assert_snapshot!` of the repaired text), `mermaid/` (corpus-shaped snippets), `adversarial/` generated in code (no giant committed files).

**proptest families** (`ProptestConfig::with_cases(128)`, debug-build friendly; commit any `proptest-regressions/` files as the repo does for `crates/vox-db-types/proptest-regressions/kb_text.txt`):
- **S1 model-first generator** (independent oracle, not the resolver): `prop_recursive` tree of containers (depth <= 3) with globally unique ids from a pool (`a b c gw db x_1 api-gw ...`, via `prop::sample::subsequence`), labels from `"[ -~]{0,20}"` plus a weird-Unicode arm filtered of forbidden controls, shapes, edges between declared nodes written as fully-qualified dotted paths (valid because ids are globally unique, so no shadowing).
- **S2 noisy printer:** model + `Vec<NoiseChoice>` -> source with random indent (spaces/tabs), 0-3 blank lines, own-line and trailing comments, explicit `shape: rect`, multi-line attr blocks, trailing spaces, CRLF (`.replace`). Properties: P1 zero diagnostics and `ir_stripped == model_ir`; P2 `fmt(fmt(x)) == fmt(x)`; P3 `ir(fmt(x)) == ir(x)` and `ast_stripped(fmt(x)) == ast_stripped(x)`; P5 `serde_json::to_string(&ir)` equal across two independent parses.
- **S3 hostile input:** `\\PC{0,400}` and token-soup (`prop::collection::vec(prop::sample::select(FRAGMENTS), 0..120)` over `node`, `container`, `->`, `-->`, `{`, `}`, `"`, `\`, `//`, `%%`, `[`, `]`, `;`, `.`, newline, multibyte chars). Assert: no panic; `diagnostics.len() <= 100`; sorted; every span (and fix span) in range and on char boundaries; `apply_fixes` result is valid UTF-8 (type-guaranteed) and `check` on it does not panic; wherever `fmt` returns `Some`, it is idempotent.
- **S4 mutate-and-repair** (the strongest guard of "applying fixes is safe"): take S2's valid source, apply ONE corruption from a table whose inverse is exact (`->` to `-->`; unquote an edge label that ends its line; delete the final `}`; drop the closing `"` of a label that ends its line; rewrite a `node` line to `A[Label]` form), run `repair`, assert `check` valid and `ir_stripped` equals the original. Any counterexample is a real fix bug.

**Mutation proofs** (repo rule, keep even under lean verification; guard each mutation with a before/after presence check because a concurrent `cargo fmt`/agent can revert it): (a) remove the fuel decrement -> adversarial test must hang/fail; (b) remove the depth guard -> deep-brace test must fail; (c) remove control-char rejection -> `invalid_character` test fails; (d) change a fix span by one byte -> repair test fails; (e) insert a `HashMap` -> determinism guard fails.

## Crate Registration Checklist (this repo, verified)

Order matters only for what the gates read; do all in one PR.

| # | Artifact | Exact action | Gate that reads it |
|---|----------|--------------|--------------------|
| 1 | `crates/vox-diagram/Cargo.toml` | template above; `description` mandatory | `vox-arch-check` Rule 6 (`description = "error"` [VERIFIED: layers.toml:70]) |
| 2 | `crates/vox-diagram/src/lib.rs` | first non-empty line must start with `//!` | Rule 5: `if !first_nonempty.trim_start().starts_with("//!") { ... let is_strict = layer <= 2;` [VERIFIED: crates/vox-arch-check/src/main.rs:1390-1395] so strict at L2 |
| 3 | Root `Cargo.toml` | `members = ["crates/*","crates/workspace-hack"]` already covers it [VERIFIED: Cargo.toml:3]. Add a `[workspace.dependencies]` line in the style `vox-kb-facets             = { path = "crates/vox-kb-facets" }` [VERIFIED: Cargo.toml:101]: `vox-diagram               = { path = "crates/vox-diagram" }` (needed by Phases 87/88; harmless now). `Cargo.lock` gains the `vox-diagram` package automatically; commit it (CI builds `--locked`) | cargo |
| 4 | `docs/src/architecture/layers.toml` `[crates]` | add in the L2 block (after line 187): `vox-diagram          = { layer = 2, max_loc = 12_000, orphan_exempt = true }   # L2 pure VoxDiagram core (graph DSL parser, IR, diagnostics, fmt, fence); first consumer (vox-doc-pipeline) lands Phase 87 which removes orphan_exempt`. Format precedents: [VERIFIED: layers.toml:187] `vox-similarity          = { layer = 2, max_loc = 6_000 }          # L2 pure simhash/minhash/LSH; no async runtime deps` and [VERIFIED: layers.toml:227] `vox-oauth-pkce          = { layer = 3, max_loc = 2_000, orphan_exempt = true }   # RFC 8252 ...; no in-tree consumer yet -- wired in by ...`. `max_loc` is only Rule 13's growth baseline: `loc_budget = "off"` [VERIFIED: layers.toml:67]; 12_000 follows ARCHITECTURE.md [ASSUMED A5] | Rule 4 orphan: `if entry.kind != "library" \|\| entry.orphan_exempt { continue; }` [VERIFIED: main.rs:1361]; `MissingLayer` in `crate_edges.rs:105,169-171` |
| 5 | `docs/src/architecture/where-things-live.md` | (a) L2 table row in `### L2 — domain libraries` (line 92 block), format `\| [\`vox-similarity\`](../../../crates/vox-similarity/) \| Pure simhash/... \|` [VERIFIED: WTL:112]: `\| [\`vox-diagram\`](../../../crates/vox-diagram/) \| Pure VoxDiagram core ... \|`; (b) concept rows under "Common tasks → exact path" (line 205+): diagram DSL parser/IR/diagnostics/fmt -> `crates/vox-diagram/src/{lexer,parser,resolve,ir,print}.rs`; markdown fence scanner -> `crates/vox-diagram/src/fence.rs`; grammar card -> `crates/vox-diagram/grammar-card.txt`. **The check is a substring test:** `let needle = format!("crates/{}/", name); if !body.contains(&needle)` [VERIFIED: main.rs:2033-2034]; Rule 12 additionally warns on any `crates/<name>/` mention whose directory is missing [VERIFIED: main.rs:1898-1923], so create the directory in the same commit | `where_things_live = "error"`, `wtl_parity = "error"` [VERIFIED: layers.toml:71,73] |
| 6 | hakari | `workspace-hack` dep is in the Cargo.toml template. Run `cargo hakari generate --diff` (installed 0.9.39); expect no change. Nightly runs `cargo hakari generate --diff` [VERIFIED: .github/workflows/nightly.yml:669]. `vox-diagram` is NOT in `.config/hakari.toml` excludes, so it keeps the dep | nightly |
| 7 | `docs/src/api/vox-diagram.generated.md` + `index.generated.md` | run `vox ci crate-api-sync --write` and commit. The generator reads only `lib.rs`: top-level `pub mod/struct/enum/trait/fn/type/const` and `//!`/`///` docs (`parse_lib_rs`), plus the Cargo `description` [VERIFIED: crate_api_docs.rs:56-153, 200-215]. So: give each `pub mod` in `lib.rs` a `///` line; re-run after the last `lib.rs`/description edit. Keep DSL examples in `//!` out of ```` ```vox-diagram ```` fences (they are copied verbatim into the generated page and would later be linted/rendered) — use ```` ```text ```` or none | `ssot-drift` stage `crate_api_docs` (docs.rs:651-654) |
| 8 | `contracts/ci/crate-graph.v1.json` | `vox ci affected-crates --regen --out contracts/ci/crate-graph.v1.json` (cheap, cargo metadata only). Bot also does it | `ssot-drift` `affected_cmd::check_graph` (docs.rs:678-684) |
| 9 | `contracts/ci/crate-build-map.v1.json`, `contracts/capabilities/capability-index.v1.json`, `crates/vox-gui/ui/src/generated/capabilityIndex.generated.ts`, `contracts/reports/test-inventory.v1.json` | generated; leave to `ssot-autoregen` (it runs every generator in `scripts/ssot-regen.vox`); or run `vox run scripts/ssot-regen.vox` locally in the background. Do not hand-edit. Precedent for what a new L3 crate touched in these files: commit `efb263fd1` changed only `contracts/ci/crate-graph.v1.json` | `ssot-drift` |
| 10 | NOT needed | `contracts/ci/crate-edges.allow.v1.json` (no live edge besides exempt `workspace-hack`; never write `exceptions`); `fan-in-snapshot.v1.json` (only counts crates already in the snapshot: `if let Some(c) = counts.get_mut(to)` [VERIFIED: fan_in_budget.rs:44]); `crates/_public.toml` (not a publish crate); `vox-schema.json` (lists 59 of 140 crates, recent L2 crates absent); `docs/agents/doc-inventory.json` (verified only by nightly `vox ci doc-inventory verify`, last regenerated 2026-10-10 and already lacks `vox-kb-facets`); `deny.toml` (no new third-party crate) | — |

**Gates that fire on this zero-consumer crate:** `vox-arch-check` (layer row, orphan -> exempt, description, `//!`, WTL substring, Rule 12 parity), `vox ci crate-edges` (nothing for it; `MissingLayer` would fire only if an edge existed without a row), `vox ci ssot-drift` (api page, crate graph, capability index, others via bot), hosted `linux` job runs clippy on affected crates with `-D warnings`, `tests` shards run nextest on the FULL set (Critical Finding 3), nightly hakari/doc-inventory.

**Phase-87 follow-ups to leave as comments in `layers.toml`:** remove `orphan_exempt`; the consumer edges are already authorized in STATE.md (never write `exceptions` yourself).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown fence extraction | a line scanner with ad hoc indentation rules | `pulldown-cmark` offset iterator | CommonMark edge cases (4-backtick nesting, tildes, list/blockquote indentation, unclosed fences) must agree with micromark in Astro |
| JSON of IR/diagnostics | manual string building | `serde` derives | field order = struct order = deterministic bytes |
| Snapshot plumbing | custom golden-file diffing | `insta` `assert_json_snapshot!` / `assert_snapshot!` | review workflow; `*.snap.new` is git-ignored [VERIFIED: .gitignore:264-265] |
| Property generation/shrinking | hand-rolled random loops | `proptest` | shrinking to minimal failing source |
| Edit distance for did-you-mean | new Levenshtein | `strsim::levenshtein` | precedent in an L2 crate |
| Line/col lookup | per-diagnostic rescans | one `LineIndex` (sorted `Vec<u32>` + `partition_point`) | O(log n), tiny; this is the ~25 lines you DO write |
| Parser generator | chumsky/winnow/logos | hand-written RD | needs custom recovery and span control (locked by milestone research) |
| Hashing / ids | `blake3`/`sha2` | none needed | crypto policy; keys are source text (Phase 89) |

**Key insight:** the leverage is in the *design rules* (declare-then-diagnose, hoisted resolution, atomic fix groups, ordered-entry AST), not in libraries; each rule removes a whole class of cascade/idempotence/determinism bugs.

## Common Pitfalls

### Pitfall 1: tdd-guard is file-level and sees only `Visibility::Public`
**What goes wrong:** commit blocked: "File has N public fn(s) ... but no #[test] blocks". **Why:** the detector parses each file ≥30 non-blank lines and flags it when it contains a `pub fn` (free fn or impl method with `syn::Visibility::Public`) and no `#[test]`/`#[cfg(test)] mod` in the SAME file; `pub(crate)` is `Restricted` and ignored; `tests/` files and files <30 non-blank lines are exempt [VERIFIED: crates/vox-code-audit/src/detectors/untested_pub_api.rs:114,131-163,157,173]. The hook line is `toestub {staged_files} --rules skeleton --min-severity warning --mode enforce-strict ...` [VERIFIED: lefthook.yml:55]. **How to avoid:** make internals `pub(crate)`; put an inline `#[cfg(test)] mod tests` with real assertions in every module that has a genuinely public fn/method; integration/proptest suites in `tests/` do NOT satisfy the hook. Sibling detector `weak_test` (Warning) rejects tautological/shallow-only tests (`.is_ok()`, `.is_some()`-only), so assert values. **Warning signs:** commit refused at `tdd-guard`.

### Pitfall 2: God-object method-density heuristic
`GodObjectDetector` counts lines containing `pub fn `, `fn ` or `impl ` and raises an Error above 12 per file, and Error above 500 non-blank lines (Warning >400, Info >300) [VERIFIED: god_object.rs:16-19,72-75,145-161]. `vox-similarity/src/index.rs` already has 16 such lines, and `.github/workflows/ci.yml` contains no `toestub` invocation (grep found none; nightly has a scoped TOESTUB job at nightly.yml:530) [ASSUMED A15: absence by grep], so treat the fn-count as soft but keep files <400 non-blank lines and split test modules out when a file grows.

### Pitfall 3: drift-check duplicate-literal/numeric/body warnings
`drift-patterns.toml`: `duplicated_numeric` threshold 3 (warning), `duplicated_body` threshold 2 / min_lines 5 (warning), `duplicated_literal` threshold 3 / min_length 8 (info) [VERIFIED: drift-patterns.toml]. Put caps in `limits.rs`, code strings in `codes.rs`, and factor repeated recovery bodies into one helper; the fast pre-push tier runs `vox-drift-check`.

### Pitfall 4: Rustdoc examples compile as doctests
Bare or `rust` fences in `//!` / `///` are compiled; `rustdocflags = ["-D", "warnings"]` turns broken intra-doc links into errors [VERIFIED: .cargo/config.toml `rustdocflags = ["-D", "warnings"]`]. Use ```` ```text ````. nextest does not run doctests; if you add any real ones run `cargo test --doc -p vox-diagram`.

### Pitfall 5: Hard-coded path/shared-lib literals in non-test src
`forbidden_pattern` regexes flag string literals beginning `"/tmp|usr|etc|var|home|opt|bin|root` or `X:\` and `"...so|dll|dylib"` outside tests [VERIFIED: layers.toml:501-534, `exempt_tests = true`]. Don't write such literals in diagnostic messages; keep fixtures relative via `env!("CARGO_MANIFEST_DIR")`.

### Pitfall 6: String slicing panics
`&src[a..b]` on a non-boundary panics; adversarial multibyte input finds it instantly. Funnel all slicing through `Span::slice(src) -> Option<&str>` (`str::get`). Add `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]` [ASSUMED A16: hosted clippy enforces] and `#![forbid(unsafe_code)]`.

### Pitfall 7: Idempotence leaks
Typical sources: trailing-whitespace trim order vs comment text, blank-line flags at block edges, explicit-default shape, CRLF in comments, a label containing `\n` re-escaped differently, `//x` vs `// x` normalization, container with only comments. The ordered-entry AST + "print is a function of AST only" removes most; S2's noise generator targets the rest.

### Pitfall 8: Fix round-trip surprises
Declare-fix duplicates (two refs to the same missing id), fixes that create new errors (unterminated-string fix swallowing a trailing `{ shape: x }` into the label), overlapping edits (Mermaid `A[Lab --> B` on one line). Mitigations: first-diagnostic-owns-the-fix, defensive overlap skip, bounded `repair` rounds, S4 property.

### Pitfall 9: Process traps from the repo
Registration PR forces full CI (batch); `crate-api-sync` is manual; `*.snap.new` files must not be committed; CRLF fixtures break `line-endings`; a rustfmt/merge "move + reformat" can duplicate a fn silently (grep `fn name` count after merges).

## Code Examples

### Cargo/lib scaffolding (first lines of `lib.rs`)
```rust
//! VoxDiagram core: graph DSL lexer, recovering parser, resolved IR, diagnostics with
//! machine-applicable fixes, canonical formatter and markdown fence scanner. Pure and
//! synchronous; no I/O, no workspace dependencies.
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// Named limits shared by parser and resolver.
pub mod limits;
```

### `LineIndex` (the one helper worth writing)
```rust
pub(crate) struct LineIndex { starts: Vec<u32> }
impl LineIndex {
    pub(crate) fn new(src: &str) -> Self {
        let mut starts = vec![0u32];
        starts.extend(src.match_indices('\n').map(|(i, _)| (i + 1) as u32));
        Self { starts }
    }
    /// 1-based (line, col in scalar values) for a byte offset.
    pub(crate) fn locate(&self, src: &str, off: u32) -> (u32, u32) {
        let line = self.starts.partition_point(|&s| s <= off).max(1) - 1;
        let col = src.get(self.starts[line] as usize..off as usize).map_or(0, |s| s.chars().count());
        (line as u32 + 1, col as u32 + 1)
    }
}
```

### Source: statement loop skeleton with fuel and progress
```rust
fn block(&mut self, depth: usize) -> Vec<Entry> {
    let mut out = Vec::new();
    while let Some(tok) = self.peek() {
        if !self.spend_fuel() { break; }            // emits limit-diagnostics once
        match tok.kind {
            K::RBrace => { self.bump(); break; }
            K::Newline | K::Comment => self.trivia(&mut out),
            K::Ident if self.is_kw("node") => out.push(self.node_decl()),     // declare-then-diagnose inside
            K::Ident if self.is_kw("container") => out.push(self.container(depth + 1)),
            _ => { if let Some(item) = self.edge_or_mermaid() { out.push(item) } else { self.skip_to_stmt_end() } }
        }
    }
    out
}
```

### Insta + proptest wiring (house style)
```rust
// tests/graph_constructs.rs  — mirrors crates/vox-compiler/tests/diagnostic_snapshots.rs
#[test]
fn canonical_example_ir_snapshot() {
    let src = include_str!("fixtures/valid/canonical.vdg");
    let report = vox_diagram::check(src);
    assert!(report.valid);
    insta::assert_json_snapshot!(report.ir);
}
// proptest! { #[test] fn fmt_is_idempotent(src in noisy_graph()) { ... prop_assert_eq!(fmt(&once).unwrap(), once); } }
// House precedent for proptest! blocks: crates/vox-db-types/src/kb_text.rs:741-762
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Regex "validate and repair" for Mermaid (advisory) | tokenizer + recovering parser with spans and exact fixes | this milestone | all errors per pass; fixes without LLM round-trips |
| Single-error parsers | panic-mode recovery + declare-then-diagnose | standard practice | fewer repair turns |
| Hand-rolled fence regexes (3 copies in the unmerged WIP) | CommonMark parser offsets | this phase | agreement with the docs renderer |

**Deprecated/outdated here:** `miette` in the library (rejected), a `vox-diagram-core` split (rejected), `HashMap`-keyed scope tables (banned).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Identifiers may contain `-` (kebab) with the `->` lookahead rule | DSL Grammar | Phase 84 Mermaid emitter must sanitize ids; if user wants snake-only, drop the rule (grammar card changes) |
| A2 | Mermaid-isms are `Error` severity (not Warning) | Mermaid-isms | If Warning, DOCS-02 lint would accept non-canonical fences |
| A3 | Single blank lines preserved, runs collapsed, none at block edges | Formatter | If user wants all blank lines dropped, canonical example in CONTEXT is no longer a fixed point |
| A4 | `pulldown-cmark` for the fence module (vs hand-rolled) | Standard Stack / Fence | Adds a (locked, already-unified) dep to an "L2 pure" crate |
| A5 | `max_loc = 12_000` (ARCHITECTURE.md value) | Registration | Only Rule 13 growth baseline; low risk |
| A6 | Card token budget approximated as `len() <= 2_600` bytes (~4 bytes/token) | Grammar Card | Real tokenizer count may differ; recheck in Phase 88 |
| A7 | Fixture extension `.vdg` is tentative and internal | Structure | Phase 88 CLI may choose another; renaming fixtures is cheap |
| A8 | Reject forbidden control + bidi-override chars in strings | Lexical decisions | Not in CONTEXT; a user may want bidi text in labels (RTL labels use plain RTL letters, not overrides) |
| A9 | `;` handled as Mermaid-ism (not a legal separator) | Mermaid-isms | If wanted as legal, remove M8 and update card |
| A10 | Multi-line `node` attr blocks accepted leniently | DSL Grammar | Simplifies LLM D2-style input; costs a little recovery code |
| A11 | All `fixes` of one diagnostic form one atomic remedy | Diagnostics | Consumers (Phase 88 MCP) must apply groups atomically; if they cherry-pick single fixes, multi-edit rewrites break |
| A12 | Cascade-suppression is a testable requirement (exactly k diagnostics for k independent mistakes) | Recovery | Weakening it makes agents see noisy duplicate errors |
| A13 | `schema_version: 1` on `CheckReport` only | Diagnostics | Cheap now, rename-hard later |
| A14 | Declare-fix inserted at scope top via Rule R (not before the referencing statement) | Scope Resolution | Style only; dedup logic depends on a single insertion point per scope |
| A15 | `ci.yml` does not run toestub god-object rules (grep absence) | Pitfall 2 | If a TOESTUB gate is added, fn-density heuristic could block parser files |
| A16 | Hosted clippy (`-D warnings`) enforces the `deny(clippy::unwrap_used,...)` attrs | Pitfall 6 | Attrs are compile-time lints; no risk if clippy not run |
| A17 | Hakari needs no change (all deps already unified) | Registration #6 | If it drifts, nightly fails; run `cargo hakari generate --diff` once |
| A18 | `pulldown-cmark` yields `Text` events whose concatenation equals micromark's `node.value` for fenced bodies | Fence | Must be proven in Phase 89 spike; for 83 only tests against CommonMark rules |

## Open Questions

1. **Missing Mermaid validator and 8 extra fences (milestone-level, not blocking 83).**
   - What we know: `mermaid.rs`, `extract_mermaid_blocks`, `validate_and_repair_mermaid`, `vox_validate_mermaid`, `mermaid_repair.rs` are not in committed git; they exist only as uncommitted work in `/Users/brbrainerd/dev/vox`. HEAD corpus is 33/21 not 41/26.
   - What's unclear: whether that WIP will be committed before Phases 84/87/88, or those phases should re-scope (own the Mermaid oracle/validator themselves, re-baseline the ratchet seed).
   - Recommendation: orchestrator asks the user once now; Phase 83 proceeds independent of it (greenfield `fence`).
2. **Mermaid-ism severity and `;` policy (A2, A9).** Recommendation as written; user may veto.
3. **Kebab ids (A1).** Cheap now, awkward to add later. Recommendation: allow.
4. **IR title/description for accessibility (Phase 85).** No `title:` statement exists in the locked grammar; the header's `...` in `graph { direction: right ... }` suggests room. Recommendation: do nothing now (additive later); Phase 85 can synthesize `<title>` from node/edge counts.
5. **Does the executor have `vox ci crate-api-sync` fast enough locally?** Installed `vox 0.6.0+build.7197 (79d9cd454)` supports it; if the installed binary is stale versus the worktree, build `vox-cli` in the background or let the executor commit the page generated by the first CI failure output (the gate prints `run \`vox ci crate-api-sync --write\``).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ | pinned `1.98.1` (rust-toolchain.toml); shell `cargo 1.98.1` | — |
| cargo-nextest | per-crate test runs | ✓ | 0.9.143 | `cargo test -p vox-diagram` |
| cargo-hakari | registration check | ✓ | 0.9.39 | nightly catches drift |
| `vox` CLI (installed) | `ci crate-api-sync`, `ci affected-crates`, `ci crate-edges`, `ci ssot-drift` | ✓ | 0.6.0+build.7197 (79d9cd454) | `cargo run -p vox-cli -- ci ...` (background, slow) |
| `toestub` | tdd-guard hook | ✓ installed at `~/.cargo/bin/toestub` (could not be executed from this research shell: lean-ctx allowlist blocked it) | — | hook runs it on commit |
| `vox-arch-check` | local registration proof | ✗ not installed | — | `cargo run -p vox-arch-check` (background) or rely on hosted CI |
| `cargo-deny` | licence gate | ✓ | — | not needed (no new crates) |
| Node/pnpm, Playwright | — | not used this phase | — | — |

**Missing with no fallback:** none. **Missing with fallback:** `vox-arch-check` binary (build on demand).

## Validation Architecture

(`workflow.nyquist_validation` is absent from `.planning/config.json`, so it is enabled.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo-nextest 0.9.143` + `insta 1.48` (json) + `proptest 1.11` (std only) |
| Config file | `.config/nextest.toml` (default profile: `retries = 2`, `slow-timeout = { period = "60s", terminate-after = 3 }`); no per-crate config needed |
| Quick run command | `cargo nextest run -p vox-diagram <name-substring>` |
| Full suite command | `cargo nextest run -p vox-diagram` (+ `cargo test --doc -p vox-diagram` only if real doctests exist) |
| Compile check | `cargo check -p vox-diagram --all-targets` |

Note `retries = 2` can mask a flaky proptest: run property tests with the `ci` profile locally when diagnosing (`--profile ci` has `retries = 1`), and treat any retry-only pass as a failure to investigate.

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DGM-01 | Every construct (direction, escapes, nested containers, edges, labels, dotted refs, six shapes, self-loops, comments) parses; IR JSON identical across runs | integration + insta | `cargo nextest run -p vox-diagram graph_constructs` | ❌ Wave 0 |
| DGM-01 | IR byte-identical across repeated parses; no `HashMap`/`HashSet` in src | unit/guard | `cargo nextest run -p vox-diagram determinism_guard ir_json_is_stable` | ❌ Wave 0 |
| DGM-03 | k independent mistakes -> exactly k diagnostics, each with code/severity/span; stable codes | integration | `cargo nextest run -p vox-diagram diagnostics_all_errors` | ❌ Wave 0 |
| DGM-03 | Codes never renamed (golden subset), all emitted codes registered | unit | `cargo nextest run -p vox-diagram codes_` | ❌ Wave 0 |
| DGM-04 | Seeded broken set repairs to zero errors via `repair`; Mermaid-ism catalogue P1 rows each have a fix test; every grammar-card "Do NOT" line yields `diagram/mermaid-syntax` | integration + insta | `cargo nextest run -p vox-diagram fixes_repair mermaid_` | ❌ Wave 0 |
| DGM-04 | mutate-and-repair property restores original IR; hostile-input fix spans valid | proptest | `cargo nextest run -p vox-diagram mutate_and_repair hostile_` | ❌ Wave 0 |
| DGM-05 | `fmt(fmt(x)) == fmt(x)`, `parse(fmt(x)) == parse(x)` (AST+IR), canonical example is a fixed point, comments/blank lines kept | proptest + golden | `cargo nextest run -p vox-diagram fmt_props canonical_example_is_fixed_point` | ❌ Wave 0 |
| DGM-06 | Each cap at N (ok) and N+1 (one span-precise `diagram/limit-*`); container endpoint; adversarial corpus terminates, no panic, <=100 diagnostics | integration | `cargo nextest run -p vox-diagram limits_adversarial` | ❌ Wave 0 |
| (support) | Fence scanner CommonMark cases; sequence IR serde round-trip; grammar card dogfood + size | unit/integration | `cargo nextest run -p vox-diagram fence_scan sequence_ir grammar_card` | ❌ Wave 0 |
| Success crit. 5 | Registration: arch rules, crate-edges, ssot-drift, hakari | gates | `cargo run -p vox-arch-check`; `vox ci crate-edges`; `vox ci crate-api-sync`; `vox ci ssot-drift`; `cargo hakari generate --diff` (all background / hosted CI is authoritative) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo check -p vox-diagram` plus the single new test: `cargo nextest run -p vox-diagram <filter>` (backgrounded if the broker queue is busy).
- **Per wave merge:** `cargo nextest run -p vox-diagram`.
- **Phase gate:** hosted CI green (full suite due to sentinel paths) + `vox ci crate-api-sync` (verify mode) clean + mutation proofs (a)-(e) recorded; then `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `crates/vox-diagram/` scaffold (Cargo.toml, lib.rs, limits.rs, codes.rs, span.rs, diag.rs) — everything is new; no existing test infrastructure covers this crate.
- [ ] `tests/fixtures/{valid,broken,mermaid}/*.vdg` including the CONTEXT canonical example verbatim.
- [ ] proptest generators (S1-S4) shared module in `tests/support/mod.rs` (a directory-module so it is not compiled as a test target).
- [ ] Framework install: none (insta, proptest, nextest already available).

## Security Domain

(`security_enforcement` absent in config => enabled.)

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | **yes** | The DSL source is untrusted (LLM, PR, user). Up-front size cap, per-kind counts, depth cap, label cap, forbidden control/bidi chars rejected at the lexer, escapes whitelisted |
| V6 Cryptography | no | none used; do not add `blake3`/`sha2` |
| V12 Files/Resources (resource exhaustion) | **yes** | bounded time/memory: fuel, iterative depth skip, diagnostic cap |

### Known Threat Patterns for a parser exposed to LLM/PR input
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Pathological input (deep braces, 64 KiB of `[`/`"`, huge dotted ref) causing hang/stack overflow | Denial of Service | 64 KiB reject first; fuel; iterative skip beyond depth 4; amortized scan-ahead rule; adversarial tests |
| Panic on multibyte boundary / indexing | Denial of Service | `str::get` only, `deny(unwrap/expect/panic)`, proptest S3 |
| Label text later spliced into SVG/Mermaid (`</text><script>`, XML-illegal controls, bidi override spoofing) | Tampering / Spoofing | IR guarantees labels contain no forbidden controls/bidi; Phase 85's single escaping writer remains mandatory (this phase does not remove that duty) |
| Diagnostics echoing user text | Information disclosure / injection into logs | Keep user text only inside JSON string fields; truncate echoed snippets to a short length; never build markdown/HTML from it |
| Fix that corrupts a document | Tampering | atomic groups, overlap rejection, char-boundary validation, S4 property |
| Unbounded diagnostics output | DoS (agent token cost) | cap 100 with sentinel |

## Sources

### Primary (HIGH confidence)
- Repo files read this session (paths with line ranges inline above): `docs/src/architecture/layers.toml` (1-229, 301-534), `docs/src/architecture/where-things-live.md` (1-42, 90-125, 205-235), `crates/vox-arch-check/src/main.rs` (1340-1419, 1875-1928, 2010-2043), `crates/vox-cli-ci/src/crate_edges.rs` (100-220), `crates/vox-cli-ci/src/affected.rs` (1-175), `crates/vox-cli-ci/src/crate_api_docs.rs` (1-260), `crates/vox-cli-ci/src/fan_in_budget.rs`, `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` (569-690), `scripts/ssot-regen.vox`, `lefthook.yml`, `crates/vox-code-audit/src/detectors/{god_object,untested_pub_api}.rs`, `docs/agents/governance.md`, `drift-patterns.toml`, root `Cargo.toml`, `.config/{nextest,hakari}.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `crates/vox-similarity/Cargo.toml`, `crates/vox-kb-facets/Cargo.toml`, `crates/vox-doc-pipeline/src/{claims/extractor.rs,pipeline/{lint,doctest}.rs}`, `crates/vox-db-types/src/kb_text.rs` (proptest precedent), `crates/vox-compiler/tests/diagnostic_snapshots.rs` (insta precedent).
- Git evidence: `git ls-tree origin/main crates/vox-doc-pipeline/src/pipeline/`, `git log --all -- .../mermaid.rs`, `git show --stat efb263fd1 2ef9442ba 79d9cd454`, `git worktree list`, `git -C /Users/brbrainerd/dev/vox status --short` (read-only).
- `gsd-tools query package-legitimacy check --ecosystem crates pulldown-cmark strsim serde insta proptest` (all OK).
- Milestone research reused (not redone): `.planning/workstreams/voxdiagram/research/{SUMMARY,ARCHITECTURE,STACK,FEATURES,PITFALLS}.md`.

### Secondary (MEDIUM confidence)
- CommonMark 0.31.2 fenced code block rules (spec.commonmark.org, fetched this session; first 100k chars read).
- mermaid.js.org flowchart page (fetched; code samples were stripped, so only the prose facts were confirmed: `end` in lowercase breaks flowcharts, leading `o`/`x` need a space or capital, `%%` comments, `subgraph id [Title]`, TB/TD/BT/RL/LR, `-.->`, `==>`, circle/cross edges, classDef/class/linkStyle/click forms).

### Tertiary (LOW confidence / design judgment)
- Recovery strategy (panic-mode + synchronizing tokens), Mermaid shape-delimiter map, `-->|t|` / `-- t -->` label spellings, token-count heuristic: training knowledge and corpus census from FEATURES.md, marked [ASSUMED] above where decisive.

## Metadata

**Confidence breakdown:**
- Registration checklist / gates: HIGH — every rule read from source; one unrun item (`cargo hakari generate --diff` no-op expectation, A17).
- Standard stack: HIGH — all pins and locked versions read; no new packages.
- Parser/recovery/resolver/fmt design: MEDIUM — sound, standard techniques, but decisions A1-A14 await planner/user confirmation and nothing was executed.
- Mermaid-ism catalogue: MEDIUM — corpus-grounded for frequency, Mermaid syntax details partly from training knowledge.
- Pitfalls: HIGH for repo-specific traps (detector semantics read), MEDIUM for design pitfalls.

**Research date:** 2026-10-16
**Valid until:** 2026-11-15 (stable); re-check registration details if `layers.toml`/`ssot-regen.vox` or the arch-check rules change, and re-check Open Question 1 whenever the primary checkout's WIP is committed.
