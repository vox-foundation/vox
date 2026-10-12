# Phase 4: Taxonomy Contract and Facet Lint - Pattern Map

**Mapped:** 2026-10-10
**Files analyzed:** 22 new/modified files or registration points
**Analogs found:** 21 / 22 (one partial: GUI TS type has a render analog but no schema-to-TS analog)

All analog paths below were checked with `git ls-files`; every one is tracked source (no `.gsd/capabilities` mirrors). Line numbers are from the worktree at the time of mapping.

## Blockers the planner must surface first

1. **`crate-edges` will reject the new dependency edges.** `contracts/ci/crate-edges.allow.v1.json` is an EXACT edge-set ratchet (`crates/vox-cli-ci/src/crate_edges.rs` `check()`, lines 116-160): every live `[from, to]` pair not in `edges` or `exceptions` is `Violation::NewEdge`. `vox-doc-pipeline -> vox-kb-facets`, `vox-search -> vox-kb-facets` and `vox-corpus -> vox-kb-facets` are all new pairs. D-01 says "no `crate-edges` exceptions entry", but that is only true for the layer rule (L3 -> L0 is downward, so no `UpwardEdge`). The `NewEdge` verdict still fires. `exceptions` entries are USER-AUTHORIZED-ONLY (AGENTS.md Dependency Discipline) and `--tighten` refuses to add edges (`crate_edges.rs` lines 255-260). The plan needs an explicit user-authorization checkpoint for 3 exception entries (`from`, `to`, `reason`, `date`, `authorized_by`), or the planner must pick one of the narrower options below.
   - Option A (matches D-01): keep the crate, ask the user to authorize 3 ledger entries in a `checkpoint:human-action` task. Write the PR-description proposal text; never write the entry.
   - Option B: avoid `vox-search -> vox-kb-facets` by passing altLabel data through `IndexDeps` (the indexer already takes injected closures, see `library/mod.rs` lines 19-30), and have the caller in `vox-orchestrator-mcp`/`vox-cli` load the taxonomy. That still leaves doc-pipeline and corpus edges.
   - Do not plan a baseline regeneration to admit the edges; that is forbidden.
2. **`workspace-hack` / hakari.** 118 of 129 crates carry `workspace-hack = { version = "0.6", path = "../workspace-hack" }`. Copy that line (the `vox-jsonschema-util` / `vox-populi-types` shape), then let `cargo hakari generate` update `crates/workspace-hack/Cargo.toml`. `vox-db-types` instead lists it under `[package.metadata.cargo-shear] ignored`; do not mix the two.
3. **The `schemars` crate name is ambiguous.** Root `Cargo.toml` line 305 has `schemars = { version = "1", features = ["derive"] }` and line 419 has `schemars08 = { package = "schemars", version = "0.8" }`. Use `schemars = { workspace = true }` (v1). Also `serde_yaml` is an alias for `serde_yaml_ng` (`Cargo.toml` line 210).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/vox-kb-facets/Cargo.toml` | config | n/a | `crates/vox-db-types/Cargo.toml` + `crates/vox-populi-types/Cargo.toml` | exact |
| `crates/vox-kb-facets/src/lib.rs` (Facets struct, enums, `effective_training_eligible`) | model (L0 types) | transform | `crates/vox-db-types/src/kb_text.rs` (+ `crates/vox-gamify/src/companion.rs` for `schemars::JsonSchema` derive) | role-match |
| `crates/vox-kb-facets/src/taxonomy.rs` (YAML loader, altLabel/alias resolution) | utility | file-I/O + transform | `crates/vox-search/src/library/chunk.rs` `frontmatter_value` (serde_yaml) + `crates/vox-cli-ci/src/crate_edges.rs` `AllowFile` load | role-match |
| `crates/vox-kb-facets/src/frontmatter.rs` (shared `split_frontmatter`, summary-block extract/strip) | utility | transform | `crates/vox-search/src/kb_ids.rs` `frontmatter_range` + `crates/vox-doc-pipeline/src/pipeline/anchors.rs` `extract_marked_block` | exact |
| `docs/src/architecture/layers.toml` (+1 `[crates]` row) | config | n/a | `vox-db-types = { layer = 0 }` (line 126) | exact |
| `Cargo.toml` (workspace dep row) | config | n/a | `vox-db-types = { path = ... }` (line 100) | exact |
| `docs/src/architecture/where-things-live.md` (+ crate row + concept rows) | docs | n/a | rows at lines 51, 69, 140, 216-225 | exact |
| `contracts/documentation/kb-taxonomy.v1.yaml` | contract (YAML) | static data | `contracts/documentation/canonical-map.v1.yaml` / `retired-symbols.v1.yaml` | exact |
| `contracts/documentation/kb-facets.v1.schema.json` (generated) | contract (JSON Schema) | generated | `contracts/documentation/canonical-map.v1.schema.json` (shape); schemars generator at `crates/vox-orchestrator-mcp/src/input_schemas.rs` lines 10-33 | role-match |
| schema golden/drift test (in `vox-kb-facets`) | test | file-I/O compare | `lint.rs` `valid_categories_match_sidebar_sections` (lines 1083-1117) + `UPDATE_GOLDEN` in `crates/vox-publisher/tests/route_simulation_golden.rs` (lines 45-52) | exact |
| `contracts/index.yaml` (+3 entries) | config | n/a | `documentation-retired-symbols` (lines 115-121), `kb-retrieval-baseline` (lines 1614-1620) | exact |
| facet lint rules in `crates/vox-doc-pipeline/src/pipeline/{lint,types,mod}.rs` | service (lint) | request-response (per file) | the same files: `lint_frontmatter`, `LintKind`, `kind_label`, `workflow_for_kind` | exact |
| `contracts/documentation/kb-facet-backlog.v1.yaml` + seed/tighten logic | model + utility | batch / file-I/O | `crates/vox-cli-ci/src/crate_edges.rs` (`AllowFile`, `tighten`, `write_allow_file`) and `crates/vox-search/src/kb_eval/baseline.rs` (`tightened`, pure fn) | exact |
| seed/tighten entry point (CLI flag or `vox ci` subcommand) | controller (CLI) | batch | `--fix` flag handling in `vox-doc-pipeline/src/pipeline/mod.rs` lines 217-225; or `CrateEdges { tighten }` in `cmd_enums.rs` 855-862 | role-match |
| `crates/vox-corpus/src/corpus/extract_docs.rs` (typed parse, summary strip) | service | transform | itself: `parse_frontmatter` (lines 115-168) + `SKIP_FENCE_MD` fixture test (lines 603-700) | exact |
| `crates/vox-search/src/library/lexical.rs` (+ `chunk.rs` `card_body`) altLabel integration | service | request-response | `LibraryLexicalIndex::search` (lines 117-140), `chunk.rs` `card_body` (lines 148-165) | exact |
| `crates/vox-gui/ui/src/generated/kbFacets.generated.ts` | generated model | generated | `crates/vox-cli-ci/src/config_gui_codegen.rs` `render_generated_index_ts` + `run(check)` (lines 206-235); `gui_surface_registry.rs` `run(root, write)` (199-251) | role-match |
| TS drift check (vitest) | test | file-I/O compare | `crates/vox-gui/ui/src/config/budget.test.ts` style; `nightly.yml` lines 297-304 for the CLI `--check` wiring | partial |
| `vox ci` registration (only if a `vox ci` subcommand is chosen) | config | n/a | `crate-edges` rows in `cmd_enums.rs`, `run_body.rs`, `contracts/cli/command-registry.yaml`, `contracts/operations/catalog.v1.yaml`, `contracts/policy/policy-registry.v1.yaml`, `contracts/capability/capability-registry.yaml`, `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt` | exact |
| `vox ci ssot-drift` wiring of the TS/schema drift | config | n/a | `ds!("gui_surface_registry", ...)` in `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` lines 623-626 | exact |
| new `docs/src/**` pages authored this phase | docs | n/a | frontmatter rules in AGENTS.md | n/a |

## Pattern Assignments

### `crates/vox-kb-facets/Cargo.toml` (config)

**Analog:** `crates/vox-db-types/Cargo.toml` (L0 leaf) and `crates/vox-jsonschema-util/Cargo.toml` (for the `workspace-hack` line).

```toml
[package]
name = "vox-db-types"
description = "Pure-data L0 leaf for vox-db: row types, IDs, schema descriptors."
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }

[lints]
workspace = true
```

Plan: `serde`, `serde_yaml`, `schemars` (all `{ workspace = true }`), `workspace-hack = { version = "0.6", path = "../workspace-hack" }`, `[lints] workspace = true`. Keep it free of workspace-crate deps (L0 may depend only on L0, and every extra workspace dep is another `crate-edges` entry). Do NOT depend on `vox-db-types` for `ContentHash`; its validator is ~10 lines (`kb_text.rs` lines 346-367), so use the defactor rule: copy with `// vox:defactored-from vox-db-types 2026-10-10`.

**Registration rows (copy shapes verbatim):**
- `Cargo.toml` line 100 shape: `vox-kb-facets             = { path = "crates/vox-kb-facets" }`. Workspace `members = ["crates/*", ...]` already globs it.
- `docs/src/architecture/layers.toml` line 126 shape, in the "L0 — pure types" block: `vox-kb-facets          = { layer = 0 }`. Missing row => `Violation::MissingLayer` from `crate-edges` AND an arch-check failure.
- `docs/src/architecture/where-things-live.md`: crate-table row (line 51 shape: ``| [`vox-db-types`](../../../crates/vox-db-types/) | Pure-data L0 leaf ... |``) and concept rows near lines 216-225 ("KB taxonomy vocabulary and facet schema", "Facet lint", "Facet backlog ratchet"). That doc has frontmatter `training_eligible: true` already; do not change it.

---

### `crates/vox-kb-facets/src/lib.rs` (model, transform)

**Analog:** `crates/vox-db-types/src/kb_text.rs` for the "L0 type guarding a trust boundary" idiom, `crates/vox-gamify/src/companion.rs` for the schemars derive.

**Derive idiom** (`vox-gamify/src/companion.rs` lines 14-16):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Mood {
```

**Validated-newtype idiom to copy for `sources[].hash`** (`kb_text.rs` lines 346-367):
```rust
/// A blake3 content hash stored as `blake3:<64 lowercase hex>`. Hashing itself stays
/// in `vox-crypto` consumers; this type only validates the encoded form.
pub struct ContentHash(String);
impl ContentHash {
    pub fn from_blake3_hex(hex: &str) -> Result<Self, String> {
        if hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            Ok(Self(format!("blake3:{hex}")))
        } else { Err(format!("expected 64 lowercase hex characters, got {} bytes", hex.len())) }
    }
}
```
Discretion answer (sources[]): the Phase 3 inbox stores `content_hash` as `blake3:<64 lowercase hex>` and `fetched_at_ms` as epoch milliseconds (`crates/vox-db/src/schema/domains/knowledge.rs` lines 115-132). So `sources[].hash` is `blake3:<hex>` and `sources[].url` is `source_url`. For a hand-edited YAML frontmatter, store `date` as an ISO `YYYY-MM-DD` string (the existing `last_updated`/`verified_at` style in `extract_docs.rs` is `%Y-%m-%d`); document the ms-to-date conversion in the promotion step. Keep the hash prefix check in this crate and hash NOTHING here (Cryptography Policy: hashing stays in `vox-crypto`).

**`origin` and trust enums:** Phase 3 already has `FindingOrigin` (`kb_text.rs` line 370) with serde `snake_case`. D-05 names a different `origin` value (`web-promoted`) on a doc, so define a separate doc-level `Origin` enum here. Do not import `FindingOrigin`.

**Eligibility function (D-06).** No analog exists; the semantics are fixed by D-05:
```rust
// pure, no I/O. Inputs: parsed Facets (+ raw training_eligible Option<bool>).
pub fn effective_training_eligible(f: &Facets) -> bool {
    let forced_false = matches!(f.origin, Some(Origin::WebPromoted))
        || matches!(f.authorship, Some(Authorship::LlmAssisted | Authorship::LlmGenerated));
    if forced_false && f.human_attested_by.is_none() { return false; }
    f.training_eligible.unwrap_or(true) // grandfathered default, matches extract_docs today
}
```
Test-first (repo policy): write the fixture tests before the body. Required cases: web-promoted without attestation => false; llm-assisted without attestation => false; llm-assisted WITH `human_attested_by` => honors the flag; no `authorship` + flag absent => true (D-05 grandfathering); no `authorship` + `training_eligible: false` => false.

**Test-first / lint constraint:** every new `pub fn` in `crates/*/src/**` needs a `#[test]` in the same file (`skeleton/untested-pub-api`), unless the file is under 30 non-blank lines.

---

### `crates/vox-kb-facets/src/frontmatter.rs` (utility, transform)

**Analog 1:** `crates/vox-search/src/kb_ids.rs` `frontmatter_range` (line 79-88), the line-exact `---` rule used by the indexer:
```rust
pub fn frontmatter_range(text: &str) -> Option<Range<usize>> {
    let mut lines = line_spans(text, 0);
    let (_, _, first) = lines.next()?;
    if first.trim_end() != "---" { return None; }
    lines.find(|(_, _, l)| l.trim_end() == "---").map(|(_, end, _)| 0..end)
}
```
**Analog 2:** `crates/vox-doc-pipeline/src/pipeline/anchors.rs` `extract_marked_block(content, start_needle, end_needle)` (lines 10-15) is exactly the primitive for `<!-- kb:summary -->` / `<!-- /kb:summary -->` (D-08). Reuse its logic for `extract_summary` and a `strip_summary_block` (the MENS strip). Do not make `vox-kb-facets` depend on `vox-doc-pipeline` (upward); copy the 6-line function with a defactor comment, or move it down into `vox-kb-facets` and have `anchors.rs` re-export (that touches a third file; prefer the copy).

**Why one shared splitter matters (three divergent parsers today):**
- `lint.rs` `lint_frontmatter` line 363-369: `content.strip_prefix("---")` then `after_dash.find("---")` (first `---` anywhere, even inside a value).
- `extract_docs.rs` line 146: `content.splitn(3, "---")`.
- `chunk.rs` lines 73-86: `kb_ids::frontmatter_range` + `serde_yaml::from_str::<serde_json::Value>`.
Phase 4's typed parse must replace the first two with one function, so a doc cannot be eligible in the lint and ineligible in the builder. The new `Facets` parse should use `serde_yaml::from_str::<Facets>` over the line-exact block, with `#[serde(default)]` on optional fields and NO `deny_unknown_fields` (existing docs carry many extra keys such as `schema_type`, `training_rationale`, `sort_order`).

**Summary word count (D-08):** `block.split_whitespace().count()` in `40..=120` (inclusive), link text counted (so do not strip markdown). Place the block check relative to the H1: summary must be "directly under the H1", i.e. the first non-blank content after the first `# ` heading outside a fence (`kb_ids::parse_heading` and `is_fence_line`, lines 50-62, are the helpers).

**Hazard:** `vox_db_types::kb_text::sanitize` strips HTML comments (`kb_text.rs` lines 52-64 docs: "(2) strip HTML comments"). The markers are empty comments so the summary text between them survives, but any consumer that sanitizes doc text will lose the markers; do strip-by-marker BEFORE sanitizing, never after.

---

### `crates/vox-kb-facets/src/taxonomy.rs` + `contracts/documentation/kb-taxonomy.v1.yaml` (utility + contract)

**Contract file analog:** `contracts/documentation/retired-symbols.v1.yaml` (flat `symbols:` list with `id`, `replacement`, `rationale`) and `canonical-map.v1.yaml` (`x-vox-version`, `schema_version: 1`, list of domains with `id`, `title`, `aliases: []`). Header comment style:
```yaml
x-vox-version: 1
# Vox Documentation: Retired Symbols and Canonical Replacements
# Used by `vox ci retired-symbol-check` ...
symbols:
  - id: vox-dei-old-crate
    pattern: "..."
    replacement: "vox-orchestrator"
    rationale: "..."
```
Planned shape (SKOS-lite, D-07): `x-vox-version: 1`, `schema_version: 1`, `domains: [{id, prefLabel, altLabels: []}]`, `topics: [{id, prefLabel, broader: <domain id>, altLabels: [], deprecated_aliases: [{label, replaced_by}]}]`. Seed `altLabels` from the crate/concept columns of `docs/src/architecture/where-things-live.md` (crate names such as `vox-orchestrator` become altLabels of the orchestration topic).

**Loader analog:** `crates/vox-cli-ci/src/crate_edges.rs` lines 36-42 and 318-330 (typed serde struct, `serde_json`/`serde_yaml` read, `with_context` on parse). For YAML in this repo use `serde_yaml::from_str` (e.g. `chunk.rs` line 80). The loader takes `&str`, not a path: `vox-kb-facets` stays pure (no `vox-bounded-fs`); callers read with `vox_bounded_fs::read_utf8_path_capped` (as in `lint_one_file`, `lint.rs` line 185).

**Registration:** add three entries to `contracts/index.yaml` using the `retired-symbols` shape (lines 115-121): `id: documentation-kb-taxonomy`, `kind: yaml`, `owner: platform-ci`, `enforced_by: [vox-doc-pipeline lint]`; plus the schema (`kind: json-schema`) and the backlog. `vox ci contracts-index` (`crates/vox-cli-ci/src/contracts_index.rs`) checks every listed path exists and that a YAML contract with a companion `{yaml_id}-schema` entry validates against it (lines 12-25 show the override table for ids that do not follow that naming). If the taxonomy yaml should be validated by a schema, name its index id `documentation-kb-taxonomy` and the schema `documentation-kb-taxonomy-schema`; otherwise skip a taxonomy schema.

---

### Generated facet JSON Schema + golden drift test (D-02)

**Generator analog:** `crates/vox-orchestrator-mcp/src/input_schemas.rs` lines 10-33 (schemars 1 API):
```rust
let settings = schemars::generate::SchemaSettings::draft07().with(|s| { s.inline_subschemas = true; });
let schema_generator = settings.into_generator();
let root = schema_generator.into_root_schema_for::<$t>();
let serde_json::Value::Object(mut map) = serde_json::to_value(&root).expect(...) else { panic!(...) };
```
Use the same `SchemaSettings` call but keep `$schema`. The facet schema's other consumers use `contracts/documentation/canonical-map.v1.schema.json` conventions (`x-vox-version`, `$id` under `https://voxlang.org/schemas/...`).

**Drift-test analog (exact):** `crates/vox-doc-pipeline/src/pipeline/lint.rs` lines 1083-1117 reads a contract by `CARGO_MANIFEST_DIR` ancestors and asserts equality:
```rust
let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .ancestors().nth(2).expect("repo root")
    .join("contracts/documentation/docs-sidebar-section-order.v1.json");
let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
```
**Bless convention analog:** `crates/vox-publisher/tests/route_simulation_golden.rs` lines 45-52:
```rust
if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
    std::fs::write(path, serde_json::to_string(&actual).unwrap()).unwrap();
} else {
    assert_eq!(actual, expected);
}
```
Plan: the golden test lives in `vox-kb-facets` (unit test, `serde_json` as dev-dep), compares `serde_json::to_string_pretty(&schema) + "\n"` with the committed file, with `UPDATE_GOLDEN=1` to regenerate. Assert the failure message names the command. Mutation-prove it (AGENTS.md: break a field, see the test fail, restore; guard with a presence check).

**Compile-validation (optional):** `vox_jsonschema_util::compile_validator(&schema, ctx)` (`vox-jsonschema-util/src/lib.rs` lines 14-17) can confirm the generated schema compiles and validates a sample facet instance. This adds `vox-kb-facets -> vox-jsonschema-util` only if done inside the L0 crate; DO NOT (L0 -> L1 is upward). Do the compile check in a `vox-doc-pipeline` test or skip it.

---

### Facet lint rules: `crates/vox-doc-pipeline/src/pipeline/{lint.rs,types.rs,mod.rs}`

**Analog:** the same files. Four registration points per new rule, all in existing style:

1. `types.rs` `LintKind` enum (lines 14-73): add variants with payload structs, e.g.
```rust
UnknownCategory { value: String },
MissingTrainingRationale,
HandAuthoredLastUpdated,
```
New variants (names are the planner's choice; keep kebab labels stable because they are the "rule ids" stored in the backlog): `MissingFacet{field}`, `UnknownTopic{value}`, `UnknownDomain{value}`, `DeprecatedTopicAlias{value, replaced_by}`, `SummaryBlockMissing`, `SummaryWordCount{words}`, `MissingAuthorship`, `MissingHumanAttestation`, `TrainingEligibleContradiction`, `SourceMissingField{field}`, `VerbatimQuoteOverCap{...}`.
2. `mod.rs` `kind_label` (lines 75-100): `LintKind::UnknownCategory { .. } => "unknown-category"`. The returned string IS the rule id the backlog keys on.
3. `mod.rs` `workflow_for_kind` (lines 28-73): one-line remediation per kind.
4. `mod.rs` message printing `match &e.kind` (lines 325-466) and the `hard_errors` `matches!` list (lines 470-493). A kind missing from the `hard_errors` list is only a warning: add every new blocking kind there.

**Core lint pattern to copy** (`lint.rs` lines 378-389) for a vocabulary check with did-you-mean:
```rust
if let Some(value) = line.strip_prefix("category:") {
    let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
    if !VALID_CATEGORIES.contains(&value) {
        errors.push(LintError { file: path.to_owned(), line: line_no,
            kind: LintKind::UnknownCategory { value: value.to_string() } });
    }
}
```
and in `mod.rs` lines 364-372 the message with `suggestion_hint(value, candidates)` -> `Did you mean "X"?` using `lint::suggest` (lint.rs lines 56-76). Feed `suggest` the taxonomy prefLabels, and also resolve altLabels/deprecated aliases before suggesting.

**Facet rules should NOT be added to the line-by-line `lint_frontmatter` loop.** That loop is substring based (`line.strip_prefix("category:")`) and cannot see lists (`sources[]`, `human_attested_by`). Add a new function `lint_facets(path, content, ctx, errors)` called from `lint_file` right after `lint_frontmatter(path, content, errors)` (line 204), parsing via `vox_kb_facets` once. Keep the existing `training_eligible: true` + `MissingTrainingRationale` check (lines 443-453) untouched.

**Context threading.** `lint_file(path, content, repo_root, errors)` (line 192) is called per file inside a rayon `par_iter` (lines 141-144). Load the taxonomy and the backlog ONCE in `collect_lint_errors_target_with_root` (line 128) into a `FacetCtx` and pass `&FacetCtx` into `lint_one_file`/`lint_file`. Note the existing tests call `lint_file(md_path, content, repo, &mut errs)` directly (lines 703-716); keep that signature working by adding a second function or an `Option<&FacetCtx>` default, to avoid rewriting ~25 tests.

**Backlog semantics (D-03):** for a doc whose repo-relative path is in the backlog, drop only the errors whose rule id is listed for that doc; every other doc gets all facet errors. A rule id listed but no longer violated is a stale entry: report it as a warning that points to `--tighten` (mirror `crate_edges.rs` stale handling, lines 172-184, printed at line ~313).

**Archive exclusion:** `gather_md_files` (lint.rs 149-181) does NOT skip `archive/`. Skip facet rules for archive paths exactly like the existing check at lint.rs lines 421-424:
```rust
let is_archive = path.to_string_lossy().replace('\\', "/").contains("/archive/");
```
`index.mdx` and other `.mdx` are gathered too; decide explicitly whether facet rules apply to `.mdx` (recommend: `.md` only, matching `kb_ids::is_kb_corpus_path` which requires `.md`).

**Scoped mode:** `--paths` is handled by `parse_paths_arg` (mod.rs 125-156) and `collect_lint_errors_target_with_root` per target (mod.rs 303-309). The facet context must be built in that same function so `vox-doc-pipeline --lint-only --paths docs/src/x.md` works and a "new doc not in backlog" is checked correctly.

**Tests:** add inline `#[test]`s in `lint.rs` `mod tests` using the string-fixture style (lines 703-756): a `&str` doc, `lint_file(...)`, `assert!(errs.iter().any(|e| matches!(e.kind, LintKind::X { .. })))`. Required fixtures per ROADMAP success criterion 1: missing summary, 39/121-word summary, unknown topic, missing `authorship`, `llm-*` without `human_attested_by`, promoted page with a source missing `hash`, over-cap verbatim quote.

**Verbatim-quote cap (discretion):** no analog. Suggested detection: markdown blockquote lines (`>`) plus double-quoted spans over N words; cap per quote and per page, applied only when `origin: web-promoted`. Keep numbers in `kb-taxonomy.v1.yaml`-adjacent constants in `vox-kb-facets` so the lint and a future promotion step share them.

---

### `contracts/documentation/kb-facet-backlog.v1.yaml` + seed/tighten (D-04)

**Analog (exact):** `crates/vox-cli-ci/src/crate_edges.rs`.

Typed file (lines 36-50):
```rust
pub struct AllowFile {
    pub schema_version: u32,
    /// Frozen baseline, sorted [from, to] pairs. Machine-tightened only.
    pub edges: Vec<[String; 2]>,
    /// Human-gated ledger. Append requires explicit user authorization.
    pub exceptions: Vec<ExceptionEntry>,
}
```
Tighten core (lines 220-290): read prior, compute `additions`, `bail!` with a heal message when any addition exists, else rewrite sorted and `println!` a count. Writer (lines 212-216): `serde_json::to_string_pretty(file)? + "\n"`. Heal text constant `HEAL` (lines 22-33) is the model for the backlog's "adding a row needs user authorization" message.

**Second analog (pure function split):** `crates/vox-search/src/kb_eval/baseline.rs` `tightened(baseline, report) -> Baseline` (lines 179-189), a pure function, with the CLI (`crates/vox-cli/src/commands/kb/gate.rs` `run_tighten`, line 355) doing only I/O and printing. Follow this: pure `tighten(prior: &Backlog, live: &Backlog) -> Result<Backlog>` (removal only) and `seed(violations) -> Backlog` in `vox-doc-pipeline` (or `vox-kb-facets`), with a thin file wrapper.

Planned shape (per-file with rule ids, D-04): `schema_version: 1`, `docs: [{path: docs/src/..., rules: [missing-summary, missing-authorship, ...]}]`, sorted by path then rule. `--tighten`: removes a row when the doc is clean, removes a rule id when no longer violated, refuses to add either (`bail!` with HEAL). Seeding command: run only when the file is absent (or with an explicit `--force-seed`), prints `seeded N docs, M rule ids` (CONTEXT `<specifics>`), and excludes `docs/src/archive/`.

**Home for the command (discretion), two options:**
- **Recommended (lowest registration cost):** flags on the existing `vox-doc-pipeline` binary, parsed with `args.contains(...)` exactly like `--fix`/`--lint-only` in `mod.rs` lines 218-225, e.g. `--seed-facet-backlog` and `--tighten-facet-backlog`. No `vox ci` registration needed. The binary is already reachable as `cargo run -p vox-doc-pipeline -- ...` (the form AGENTS.md cites for `--lint-only --paths`).
- `vox ci kb-facet-backlog [--seed|--tighten]`: matches the `crate-edges` precedent but needs ALL of: `cmd_enums.rs` variant (shape at lines 855-862), `run_body.rs` dispatch (line 599: `CiCmd::CrateEdges { tighten } => vox_cli_ci::crate_edges::run(&root, tighten)`), `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt`, `contracts/cli/command-registry.yaml`, `contracts/operations/catalog.v1.yaml`, `contracts/policy/policy-registry.v1.yaml`, `contracts/capability/capability-registry.yaml`, and generated `docs/src/reference/cli-command-surface.generated.md` via `vox ci command-sync`. It would also add a `vox-cli-ci -> vox-doc-pipeline` edge (another `crate-edges` exception) unless the logic stays in `vox-cli-ci` and reads files only.

Also add the backlog file and its heal text to `contracts/index.yaml` (`enforced_by`).

---

### `crates/vox-corpus/src/corpus/extract_docs.rs` (service, transform) - D-06

**Analog:** itself. Current code to replace (lines 115-168): `parse_frontmatter(content, path) -> (Frontmatter, bool, u8)` with three substring paths:
```rust
if content.contains("status: deprecated") || content.contains("status: \"deprecated\"") || ... {
    return (Frontmatter::default(), false, 0);
}
if !content.starts_with("---") {
    let eligible = !(content.contains("training_eligible: false") || content.contains("training_eligible:false"));
...
let parts: Vec<&str> = content.splitn(3, "---").collect();
...
let fm: Frontmatter = serde_yaml::from_str(yaml_str).unwrap_or_default();
if !fm.training_eligible { return (fm, false, 0); }
```
Notes for the planner:
- The substring `status: deprecated` checks the WHOLE file (a body mention disqualifies a doc). Fixing that is a behavior change; keep it out of scope unless the corpus regression test says otherwise, or fold it into the typed parse deliberately and call it out.
- A malformed YAML block currently falls to `unwrap_or_default()` = eligible `true`. Decide fail-closed vs. preserve; D-05 says "keeps today's behavior" for grandfathered docs, so preserve for no-`authorship` docs but a block that parses as having `origin: web-promoted` must be excluded.
- Replace `Frontmatter.training_eligible: bool` (default `true`, lines 18-35) with the shared `vox_kb_facets::Facets` plus `effective_training_eligible(&facets)`. Keep `last_updated`, `title`, `status` reads (staleness penalty at lines 150-166 depends on `last_updated`).
- **Summary strip:** both lanes read the raw `source`. `extract_code_blocks` (lines 207-290) accumulates every non-empty, non-fence, non-`>` line into `preceding_context`, and `extract_qa_sections` (lines 295-440) accumulates all section bodies; so the summary paragraph under the H1 flows into prompts and answers. Strip the block once in `extract_from_md_file` (line 171-180) right after `parse_frontmatter`: `let source = vox_kb_facets::strip_summary_block(&source);` so both lanes see stripped text. Strip the HTML-comment markers AND the enclosed text.

**Test pattern to copy** (lines 603-666, the `SKIP_FENCE_MD` fixture): a `const ..._MD: &str = r#"..."#;` document at the top of `mod tests`, then `#[test]` calling `extract_qa_sections(...)` / `extract_code_blocks(...)` directly with `&Frontmatter::default()` and asserting on `out`. Phase 4 needs: (1) web-promoted page without `human_attested_by` => `extract_from_md_file` returns empty (note: `extract_from_md_file` takes a `&Path`, so write a temp file with `tempfile` or test `parse_frontmatter` directly; the existing tests deliberately avoid the file API, see the "Can't test with real file" comment at line ~771); (2) the same page WITH attestation honors the flag; (3) a doc with a summary block whose text contains a sentinel word emits NO pair whose `prompt`/`response` contains that sentinel (success criterion 4). Existing tests build `Frontmatter::default()` directly; if the struct changes shape, update those call sites (lines ~775-803).

**Dependency note:** `vox-corpus/Cargo.toml` already has `serde_yaml` and `vox-bounded-fs`; add `vox-kb-facets = { workspace = true }`. That is the new `vox-corpus -> vox-kb-facets` edge (see Blockers).

---

### `crates/vox-search/src/library/lexical.rs` + `chunk.rs` (altLabel synonyms, facets) - success criterion 3

**Analog:** `LibraryLexicalIndex::search` (lexical.rs lines 117-140) and `build_schema`/`rebuild_from_rows` (lines 46-105).

Two viable integrations (discretion); recommendation first.

**Recommended: query-time synonym expansion.** The index is rebuilt wholesale from DB rows and a rebuild is gated by the chunker version and row counts (`library/mod.rs` lines 86-91, 194-210); index-time terms would force a `CHUNKER_VERSION` bump and a full reindex whenever the YAML changes. Query-time keeps the taxonomy edit PR-reviewable and index-stable. Pattern: expand inside `search()` before `parser.parse_query_lenient(query)` (line 125):
```rust
let mut parser = QueryParser::for_index(&self.index, vec![f("title"), f("heading"), f("body")]);
parser.set_field_boost(f("title"), TITLE_BOOST);
parser.set_field_boost(f("heading"), HEADING_BOOST);
let (q, _ignored) = parser.parse_query_lenient(query);
```
Expansion function in `vox-kb-facets` (pure): `expand_query(&Taxonomy, &str) -> String` mapping an altLabel (e.g. "orchestrator") to `(orchestrator OR "vox-orchestrator")` plus the prefLabel; resolves deprecated aliases to the replacement. Keep the tie-break/stable-order logic (lines 134-139) unchanged. Because queries are parsed leniently, build the expansion as a text query, not as raw tantivy `Query` objects, so malformed input still degrades.
The `search` signature is `search(&self, query: &str, limit: usize)`; the public entry is `library::search(db, lexical_dir, query, limit)` (mod.rs lines 273-300). Thread the taxonomy as an optional parameter (e.g. `Option<&Taxonomy>`) or a field on `LibraryLexicalIndex`, never a global. Loading the taxonomy requires the repo root, which `search` does not have; the MCP tool (`crates/vox-orchestrator-mcp/src/kb_tools/library.rs`) is the caller that knows it. This is where Option B in the Blockers (inject, avoid the `vox-search -> vox-kb-facets` edge) fits.

**Facets into the index (card chunk):** `chunk.rs` `card_body` (lines 148-165) already appends facets as `category: X` / `status: Y` lines to the doc's card chunk. Mirror it for `topic: ...`/`domain: ...` so the card chunk carries the PREF label as indexable text. Frontmatter is already parsed there into `serde_json::Value` (`frontmatter_value`, lines 73-86; stored as `frontmatter_json`), so reading `topic`/`domain` needs no new parse; only changes to `card_body` mean a `CHUNKER_VERSION` bump (`chunk::CHUNKER_VERSION`, used at mod.rs line 91).
Add the summary block text to the card too (D-08 says it is a lede paragraph; `first_paragraph` (lines 119-139) currently takes the first non-blank non-heading paragraph of the intro, which will now often BE the comment marker line `<!-- kb:summary -->`; handle it by stripping marker lines before `first_paragraph`).

**Test pattern:** `lexical.rs` tests (lines 160-215): build `LibraryChunkRow`s with a helper `row(hash, heading, body)`, `rebuild_from_rows` in a `tempfile` dir, `idx.search("...", 5)`, assert the hash order. Add: a row whose body only contains the prefLabel is found by an altLabel query; deprecated alias resolves; a query with no synonym is unchanged. A `library/mod.rs` round trip (lines 321-352, `index_then_search_round_trip_and_missing_index_errors`) is the model for an end-to-end test with `doc.md` frontmatter containing `topic:`.

---

### GUI generated TypeScript facet type (D-02)

**Analog:** `crates/vox-cli-ci/src/config_gui_codegen.rs`: a pure `render_*_ts(...) -> String` plus `run(check: bool)`:
```rust
const OUT_REL_PATH: &str = "crates/vox-gui/ui/src/config/generatedSettingsIndex.ts";
...
format!("// @generated by `vox ci config-gui-codegen` from CONFIG_KEYS — DO NOT EDIT.\n ...")
...
if check {
    let on_disk = std::fs::read_to_string(path).map_err(|e| anyhow::anyhow!("... cannot read {OUT_REL_PATH}: {e}. Run `vox ci config-gui-codegen` to generate it."))?;
    if on_disk != rendered { anyhow::bail!("config-gui-codegen drift: {OUT_REL_PATH} is stale. Regenerate with `vox ci config-gui-codegen` and commit."); }
```
and `crates/vox-cli/src/commands/ci/gui_surface_registry.rs` `run(repo_root, write)` (lines 199-251) which writes YAML + generated TS + a report and verifies drift with `existing_ts.trim() != ts.trim()`. Generated TS header convention in the existing file: `// AUTO-GENERATED by `vox ci gui-surface-registry --write`. DO NOT EDIT.` (`crates/vox-gui/ui/src/generated/surfaceRegistry.generated.ts` line 1). Output dir: `crates/vox-gui/ui/src/generated/` (kbFacets.generated.ts). The `@generated-hash` header rule in AGENTS.md applies to generated Vox files, not to this one.

**D-02 says the TS is generated from the committed JSON Schema file, not from the Rust types.** So the renderer takes `serde_json::Value` read from `contracts/documentation/kb-facets.v1.schema.json` and emits `export type Authorship = 'human' | 'llm-assisted' | ...; export interface KbFacets {...}`. That keeps `vox-cli-ci` free of a `vox-kb-facets` edge (it reads a data file). No analog exists for schema-to-TS (no `json-schema-to-typescript` in `crates/vox-gui/ui/package.json`; its scripts are only `typecheck`, `test`, `build`, `bundle`, `tokens:build`). Hand-write the small renderer for the closed shape (string enums, string arrays, optional fields, nested `sources` objects) rather than adding an npm dependency (Ponytail ladder: no new dep for ~60 lines).

**Where to hang it:** extend an existing command to avoid new `vox ci` registration, e.g. add a `--kb-facets` flag beside `--fields` in `ConfigGuiCodegen` (`cmd_enums.rs` lines 193-202; dispatch `run_body.rs` lines 106-112), or a `write: bool` in a new fn called from `vox ci ssot-drift`. Hooking it into `run_ssot_drift` follows the `ds!` macro lines in `run_body_helpers/docs.rs` (lines 568-626, e.g. `ds!("gui_surface_registry", crate::commands::ci::gui_surface_registry::run(root, false))`), which is how the drift becomes a PR-time failure (success criterion 2).

**Divergence check on the TS side:** a vitest that imports the generated module and a `tsc --noEmit` consumer. Style analog: small unit tests next to the module, e.g. `crates/vox-gui/ui/src/config/budget.test.ts`. A `satisfies`-based fixture (one object per enum value) makes `pnpm typecheck` fail if the generated enum changes shape. No new GUI surface ships this phase, so the Playwright visual-verification invariant does not apply; say so in the plan.

---

## Shared Patterns

### Tighten-only ratchet (backlog, and its proof)
**Source:** `crates/vox-cli-ci/src/crate_edges.rs` lines 22-33 (HEAL), 220-290 (`tighten`).
**Apply to:** backlog seed/tighten code and its tests.
Tests to copy from `crate_edges.rs` lines 605-640: `tighten_is_removal_only` (adding an entry errors), `tighten_bootstraps_when_missing_...`, `run_tighten_mode_writes_baseline...`. Mutation-prove the removal-only guard (AGENTS.md "Verify security-relevant guards by mutation").

### Contract YAML + schema + index registration
**Source:** `contracts/documentation/*.v1.yaml`, `contracts/index.yaml` lines 100-121.
**Apply to:** `kb-taxonomy.v1.yaml`, `kb-facets.v1.schema.json`, `kb-facet-backlog.v1.yaml`. `.v1` suffix, `x-vox-version`, `schema_version: 1`, an index entry each (owner `knowledge-base` as `kb-retrieval-baseline` uses, lines 1614-1620).

### Drift gate wiring
**Source:** `run_ssot_drift` `ds!(...)` list in `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` lines 568-648; plus the guard test at line 886 (`run_ssot_drift_dispatches_cache_key_lint` asserts a call exists via `include_str!("docs.rs")`; add the same kind of assertion for any new step).
**Apply to:** the generated-schema and generated-TS checks. Nightly also runs `config-gui-codegen --check` (`.github/workflows/nightly.yml` lines 297-304); adding a check to `ssot-drift` already runs it in PR CI.

### Doc-path scoping
**Source:** `lint.rs` lines 421-424 (`/archive/` test), `kb_ids.rs` `KB_EXCLUDED_DIR_NAMES = ["archive"]` and `is_kb_corpus_path` (lines 18-21, 155-176).
**Apply to:** facet lint, backlog seeding, MENS typed parse. Seeding must walk the same file set as the indexer: reuse the `.md`-only, no-symlink, `archive`-pruned rule rather than `gather_md_files`.

### Frontmatter authoring rules for any doc this phase writes
**Source:** AGENTS.md "Authored Markdown Frontmatter". Any new `docs/src/**` page (e.g. an authoring guide for facets, an ADR for the summary-block markers since D-08 is one-way) must start with `title`, `description`, `category` frontmatter; do not add `last_updated`; verify with `cargo run -p vox-doc-pipeline -- --lint-only --paths <file>.md`. Research-style pages must not be `training_eligible: true` without `training_rationale` (lint.rs lines 443-453).

### Rust conventions applied to every new file
- Test-first policy: failing test before each new `pub fn` (`skeleton/untested-pub-api`).
- No heavy local verification: `cargo check -p <crate>` and `cargo nextest run -p <crate>` only (AGENTS.md Local CI Gate Tiers); route cargo through the build broker.
- Secrets/crypto: no hashing in the new crate (Cryptography Policy); validate `blake3:` strings only.
- No direct vendor LLM calls (not touched).
- New automation must be `.vox` or a Rust CLI flag, never `.py/.sh/.ps1`.

## No Analog Found

| File / concern | Role | Data Flow | Reason |
|---|---|---|---|
| `effective_training_eligible` semantics | utility | transform | No existing authorship/origin model; semantics come from D-05/D-06. Closest shape is the substring rule in `extract_docs.rs` `parse_frontmatter`. |
| Verbatim-quote detection and cap | utility | transform | No Markdown quote scanner exists in the lint; `extract_docs.rs` `extract_qa_sections` only tracks fences. Define in `vox-kb-facets` and unit-test with fixtures. |
| JSON-Schema-to-TypeScript renderer | utility | generated | Only data-to-TS renderers exist (`config_gui_codegen.rs`, `gui_surface_registry.rs` `generate_ts`). Hand-write a small closed-shape renderer. |
| Query-time synonym expansion | service | request-response | `search()` has no expansion hook; `llm_query_expansion.rs` in `vox-search/src` is LLM based and not suitable (model-agnostic facade, non-deterministic). |

## Metadata

**Analog search scope:** `crates/vox-db-types`, `vox-doc-pipeline`, `vox-cli-ci`, `vox-cli` (ci + kb commands), `vox-search` (library, kb_ids, kb_eval), `vox-corpus`, `vox-jsonschema-util`, `vox-orchestrator-mcp` (input_schemas), `vox-gamify`, `vox-publisher` (golden test), `vox-gui/ui`, `contracts/documentation`, `contracts/index.yaml`, `contracts/ci`, `docs/src/architecture/{layers.toml,where-things-live.md}`.
**Files scanned:** ~60 (read in full or in targeted ranges).
**Pattern extraction date:** 2026-10-10
