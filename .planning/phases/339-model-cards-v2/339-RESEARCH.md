# Phase 339: Model cards v2 - Research

**Researched:** 2026-11-06
**Domain:** Rust text generation (askama markdown templates) over MENS run records; Hugging Face model-card metadata; Starlight docs page; Hub push mechanics
**Confidence:** HIGH on code facts (all read this session), MEDIUM on askama/hakari build behaviour (not compiled here), LOW on eval-time estimates

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D1** All shared explanation lives on one voxlang.org page, `docs/src/reference/mens-models.md`. The Hub index has a one-paragraph summary that links to it. Model cards carry only size-specific content.
- **D2** Before/after examples are chosen by a fixed, stated rule: 2 wins plus 1 loss. Each card states the rule and the totals.
- **D3** "Hub sync text" means generated card prose, not commit messages or CLI output.
- **D4** Structure: typed `CardFacts` layer, askama markdown templates, data file with per-size wording.
- **D5** State is three independent facts (trained, spoke, gate); each card states each fact exactly once.
- State model (spec §3): Trained `trained` | `not trained`; Spoke `vox-lang` for all current ladder adapters (planned: `harness: tool-selection`, `harness: argument-generation`, `rust`); Gate `passed` | `needs re-gate` | `failed` | `not evaluated`. `needs re-gate` = report passed under a non-current rule (slice pins `slice_blake3`, paired rule, alpha, `grader` id must all match `eval-gates-ladder.yaml` and the running grader). The check is phase 338 wave 1's `vox_eval::collateral_evidence::check_collateral_report`; until 338 lands cards v2 calls a stand-in with the SAME signature and 338 replaces it with no other change. `failed` = a current-rule evaluation measured the adapter and it did not pass. `not evaluated` = trained but a required current-rule evaluation is missing. Expected today: 1.7B/4B/8B trained, vox-lang, needs re-gate; 14B trained, vox-lang, not evaluated; 32B not trained; 0.6B whatever its 128-item re-gate (running now) produces.
- Each fact appears once per card in a status line under the title, linking to the docs page's state legend. The gate part names what blocks it. A model card never states another size's state. The index grid is the only all-size state listing.
- Model card content, in order (spec §4): 1 Frontmatter (valid HF metadata; `model-index` holds current-rule metrics only); 2 Title + status line; 3 Why this size exists (2-3 sentences from `mens/config/hub-cards.yaml`); 4 What it does, shown (wins = 2 shortest held-out items by prompt length then item id that adapter passes and base fails; loss = shortest item the adapter still fails, or say none; each shows prompt, base output, adapter output, one-line compiler verdict each; one line states rule and totals); 5 Use it (pull, serve, one request, that request's real recorded output; Vox in that block must pass `vox check`); 6 Results (one table: held-out pass@1 base vs adapter; collateral per slice base vs adapter with McNemar p-value; column headers link to docs page; one interpreting sentence; removed: second reasoning table, invalid-results table, raw gate-message rows; receipt file stays linked); 7 Training (only fields that differ between sizes); 8 Files (one line); 9 Footer (one line linking index + docs page; the only sentence allowed on more than one card).
- Removed from model cards: "What this is / is not", other-sizes list, PEFT explanation, links block. The 32B repo keeps its hand-written card under the existing `ensure_ladder_repos` rule.
- Docs page and index (spec §5): `docs/src/reference/mens-models.md` with full repo frontmatter, topic/domain/authorship facets (`authorship: llm-assisted` + `authorship_model`), kb:summary; one anchored section per concept (What MENS is; Hub and spoke with a `vox-diagram` fence, never mermaid; The harness; Gating; Prompt protocol and training method; Links). Index (`vox-mens-index`): one paragraph linking into docs; size x spoke grid (the only all-size state listing; harness/rust columns "planned"); Which model should I use (static rule over memory and task + pull command; while re-gates pending names only gate-current sizes and says so; replaced by the 340 selector later); What's next (built from records; sizes needing re-gate, spokes with no adapter, estimated eval time each); one held-out pass@1 comparison across sizes (current-rule only); links.
- Architecture (spec §6): `hub_card/facts.rs`, `hub_card/examples.rs`, `hub_card/render.rs`, `crates/vox-populi/templates/hub/` (`model.md`, `index.md`, partials), `mens/config/hub-cards.yaml` (validated on load: unknown size or missing blurb is an error), `hub_sync.rs` interface unchanged (`render_model_card`, `render_index_readme` keep signatures). askama: already in Cargo.lock; new direct dep of vox-populi; must add no workspace crate edge. Data flow: run dir -> CardFacts -> render -> `mens/runs/hub_cards/<repo>/README.md` -> user reviews side by side -> `vox mens hub sync` pushes. Nothing pushed before review.
- Testing (spec §7): goldens one per state (passed, needs re-gate, failed, not evaluated, not trained), one with examples, one without, and the index, each from a fixture run dir. Lints as tests over every rendered card: no duplicate sentence of >=4 words across two cards (footer excepted); every `voxlang.org/reference/mens-models/#...` anchor exists as a heading in mens-models.md; frontmatter validates against HF model-card metadata rules via a local fixture (no network); every `vox` block passes `vox check`. Mutation proofs: stale pass mapped to `passed` fails a test; dropping the loss example fails the honesty test; stating gate state twice fails the duplication lint. Docs: `vox-doc-pipeline --lint-only --paths docs/src/reference/mens-models.md` clean.
- Push rules: final plan ends at a human checkpoint (side-by-side review of every rendered card before any push). After approval: `vox mens hub sync` pushes cards for all repos + the index; card-only for every size except one whose weights are gated-current; never `--force-ungated`; 32B card untouched.
- Process: test-first; mutation proofs per §7; no hardcoded counts (totals, item counts derive from data); per-crate cargo only; no new crate edge without user authorization. Coordinate with phase 335 and 338-01.

### Claude's Discretion
- Exact `CardFacts` field shapes, template partial breakdown, golden fixture layout.
- Where the `vox check` lint runs (vox-populi test vs a crate that already depends on both) - must not add a crate edge.
- The HF metadata fixture format and which rules it encodes.
- Static "which model" rule wording and the What's-next eval-time estimate source.
- Wave split and ordering vs 338-01.

### Deferred Ideas (OUT OF SCOPE)
- Phase-340 selector output replacing the index's static "which model" rule.
- Hub commit messages and CLI output wording (D3).
- Re-gating 1.7B/4B/8B/14B (phases 337/338).
</user_constraints>

<phase_requirements>
## Phase Requirements

No IDs were minted in ROADMAP (`Requirements: TBD`). Proposed IDs (CARD-01..CARD-12), mapped to spec sections. The planner should mint them in REQUIREMENTS.md next to SPOKE-01..08.

| ID | Description | Spec | Research support |
|----|-------------|------|------------------|
| CARD-01 | Three-fact state model (trained, spoke, gate) computed once per size; `needs re-gate` via the same-signature stand-in | §3 | Q7 table of real inputs; stand-in contract in "Q7" |
| CARD-02 | `facts.rs` is the only reader of run records and holds no wording; `RunRecords`/`RunRecords::load`/`render_*` signatures preserved | §6 | Q1 signature inventory and function mapping |
| CARD-03 | `examples.rs` applies the D2 rule (2 wins + 1 loss) over base/adapter checkpoint+report joined by item id; missing output renders "no recorded output" | §4.4, §6 | Q2: preview join, verdict fields, real counts |
| CARD-04 | askama markdown templates under `crates/vox-populi/templates/hub/`; all wording in templates/yaml; output deterministic and host-free | §6 | Q3 askama facts |
| CARD-05 | `mens/config/hub-cards.yaml` per-size blurbs + spoke descriptions + eval-time estimates, validated on load | §4.3, §5 | Q9 data sources |
| CARD-06 | Model card sections in spec order, each fact stated once, removed sections gone | §4 | Q1, Pitfalls 4-6 |
| CARD-07 | "Use it" block with real recorded output (honest labelled fallback) whose Vox passes the compiler | §4.5, §7 | Q2 fallback, Q4 lint |
| CARD-08 | `docs/src/reference/mens-models.md` with facets, summary, anchored sections, `vox-diagram` fence; doc lint clean | §5, §7 | Q6 |
| CARD-09 | Index: paragraph, size x spoke grid, which-model static rule, what's next, one pass@1 comparison, links | §5 | Q9 |
| CARD-10 | Lints as tests: sentence duplication, docs anchors, HF metadata fixture, Vox blocks compile | §7 | Q4, Q5, Q6 |
| CARD-11 | Goldens per state + examples/no-examples + index, and the three named mutation proofs (plus the extras listed under Validation Architecture) | §7 | Validation Architecture |
| CARD-12 | Card-only push path, human side-by-side review checkpoint, then push (never `--force-ungated`; 32B untouched) | §6, brief | Q10 (a card-only path does NOT exist today) |
</phase_requirements>

## Summary

Phase 336's `hub_card.rs` is a 1,102-line `format!` renderer whose only external surface is small: `hub_sync.rs` calls `render_model_card`, `render_index_readme`, `RunRecords` (struct-update syntax on `has_adapter`), `RunRecords::load`, `compare_json_path`, `repo_name`. Nothing outside `hub_card.rs`/`hub_card_tests.rs` uses `CardStatus`, `card_status`, `recommended_size`, `heldout_scores`, `Score` or `PROMPT_PROTOCOL` (grep over `crates/`), so the split into `hub_card/{mod,facts,examples,render}.rs` is a private refactor behind a stable `mod.rs` re-export. The eval data needed for real examples already exists for 0.6B, 1.7B, 4B and 8B (52 items, base and adapter), and a content-verified join of checkpoint text to bench item id works on all 8 base/adapter pairs without reconstructing prompts.

Five findings change the plan shape and are not in the spec or CONTEXT. (1) **`vox mens hub sync --push` has no card-only mode**: the README is inside the hashed bundle manifest, so a card change makes a published size `Diverged`, and `--push` then runs the full gated `publish_adapter_run_dir` (weights, new `vN` tag) or refuses. Pull verifies every file in `hf_sync_manifest.json`, so a card-only upload must also rewrite those manifest entries. CARD-12 therefore needs new code (own plan, own tests). (2) `mens/runs/` is **gitignored**, so goldens and lints cannot read real run dirs; fixtures must be committed under `crates/vox-populi/`. (3) **The `vox check` lint needs no new edge, not even a dev-dep**: `vox_corpus::corpus::corpus_readiness::verify_vox_admission` is a public, ungated function in a crate vox-populi already depends on, and it wraps `vox_compiler::pipeline::run_frontend_str`. (4) The per-item compiler detail the spec's example verdict shows ("`let mut` is not Vox") **is not stored** and the pipeline discards parse detail ("Parsing failed with N error(s)"); the honest verdict line comes from stored fields (`parse_ok`, `typecheck_ok`, `diag_errors`, `parse_error`). (5) The duplication lint as specified conflicts with spec-mandated shared sentences (rule statement, blocker text identical on 1.7B/4B/8B); the lint's "sentence" definition and per-card numbers must be chosen deliberately (Pitfall 4).

askama 0.12.1 is already in `Cargo.lock` (only via `vox-runtime-rn` -> `uniffi` -> `uniffi_bindgen`), handles `.md` templates with no escaping by default, and as a third-party crate creates no `vox ci crate-edges` edge. Use `askama = { version = "0.12", default-features = false, features = ["config"], optional = true }` (the exact feature set `uniffi_bindgen` already uses) behind `mens-hf-hub`, so no new crate version and no workspace-hack feature delta is expected. No user-authorized ledger entry is needed.

**Primary recommendation:** Order 339 BEFORE 338-01's Task 2 (339 owns the `hub_card` split, the `NeedsReGate` state and the `needs_re_gate` golden through a same-signature stand-in in `hub_card/collateral_standin.rs`); have 338-01/02/06 retarget their `hub_card` edits to `hub_card/facts.rs` and the templates as listed in "Q8"; put every lint in vox-populi's own `--lib` tests using `verify_vox_admission`; and plan card-only push as a real, separately tested feature before the human review checkpoint.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Read run records, compute state/scores/examples | Library (`vox-populi` `hub_card/facts.rs`, `examples.rs`) | - | Pure functions over local files; no network; only reader of records |
| Wording / layout | Templates (`crates/vox-populi/templates/hub/*.md`) + `mens/config/hub-cards.yaml` | - | D4: no wording in Rust |
| Shared explanation | Docs site (`docs/src/reference/mens-models.md`) | - | D1; cards link by anchor |
| Gate currency decision | `vox_eval::collateral_evidence` (338-01) / stand-in in populi until then | - | Single shared check (SPOKE-01) |
| Card-only upload, manifest rewrite | Library (`vox-populi` hub module, new) | CLI flag in `vox-ml-cli` `mens/hub.rs` | Network + manifest integrity live with `hub_sync`; CLI only parses a flag |
| Render to local review dir | `hub_sync::write_local_cards` (unchanged) | CLI `vox mens hub sync` | Existing seam, writes `mens/runs/hub_cards/` |
| Lints (duplication, anchors, HF metadata, Vox blocks) | `vox-populi` `--lib` tests | CI (hosted) runs them with the crate tests | Zero new crate edges |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| askama | 0.12.1 (lock) [VERIFIED: Cargo.lock:391-398 `name = "askama"` `version = "0.12.1"`] | compile-time markdown templates | Already in lock; spec mandates it; `.md` extension maps to the no-escape `Text` escaper [VERIFIED: ~/.cargo/registry/src/*/askama_derive-0.12.5/src/config.rs:303 `(&["md", "none", "txt", "yml", ""], "::askama::Text")`] |
| askama_derive | 0.12.5 (lock) | derive macro | Same lock entry [VERIFIED: Cargo.lock:401-403] |
| serde_yaml | workspace (already optional dep under `mens`) | load `hub-cards.yaml` and the HF fixture | `Cargo.toml:` `mens = [... "dep:serde_yaml" ...]`; already used by hub_card_tests |
| serde_json | workspace | records | already a dependency |
| vox-corpus (existing edge) | workspace | `verify_vox_admission` for the Vox-block lint | Existing allowed edge (`["vox-populi","vox-corpus"]` in crate-edges.allow.v1.json); function is `pub`, ungated |
| vox-eval (existing edge, optional) | workspace | `GRADER_ID`, `PAIRED_RULE`, `SLICE_NAMES` for the stand-in | Existing allowed edge; add `dep:vox-eval` to `mens-hf-hub` (identical to 338-01 Task 1) |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | workspace dev-dep (present) | build fixture run dirs in tests | goldens, join tests |
| vox-crypto | workspace (present) | BLAKE3 for card-only manifest rewrite (`vox_crypto::hash_fast_hex`, as `stage_publish_bundle` uses at hub_sync.rs:545) | CARD-12 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| askama 0.12.1 | askama 0.14/0.15 (rinja merge) | New crate versions in the lock, `multiple-versions = "warn"` noise, no benefit; spec says "already in Cargo.lock" |
| `verify_vox_admission` | direct `vox-compiler` dev-dep in vox-populi | `crate-edges` ignores dev-deps [VERIFIED: crates/vox-cli-ci/src/crate_edges.rs:185-187 "dev-deps excluded in v1"], but the user said no new edge; the corpus function needs nothing |
| yaml data file embedded with `include_str!` | read from workspace root at runtime | Embedded copy makes render independent of cwd (determinism) and mirrors `spoke_base_resolver.rs:181`; add a byte-equality test against the on-disk file if both are kept |

**Installation (vox-populi/Cargo.toml):**
```toml
# [features]
mens-hf-hub = ["mens", "dep:hf-hub", "tokio/rt-multi-thread", "dep:askama", "dep:vox-eval"]
# [dependencies]
askama = { version = "0.12", default-features = false, features = ["config"], optional = true }
```
`dep:vox-eval` on `mens-hf-hub` is the same line 338-01 Task 1 adds (see 338-01-PLAN.md:180); identical edit, trivial merge.

**Version verification:** `cargo tree -p vox-populi --features mens-hf-hub -e normal --offline -i askama` -> "package ID specification `askama` did not match any packages" (askama is not in vox-populi's graph yet; it reaches the lock only through `vox-runtime-rn`). `cargo tree -i vox-compiler` shows `vox-compiler` -> `vox-corpus` -> `vox-populi` (already in the graph, so the lint adds no compile cost).

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| askama | crates.io | since 2017-02-15 | ~1.32M/wk | github.com/askama-rs/askama | OK (`gsd_run query package-legitimacy check --ecosystem crates askama askama_derive`) | Approved; already in Cargo.lock |
| askama_derive | crates.io | since 2017-02-15 | ~1.32M/wk | github.com/askama-rs/askama | OK | Approved; already in Cargo.lock |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
No new crate version enters the lock (0.12.1 / 0.12.5 / askama_parser 0.2.1 / askama_escape 0.10.3 already present). Licence MIT OR Apache-2.0 [VERIFIED: ~/.cargo/registry/src/*/askama-0.12.1/Cargo.toml `license = "MIT OR Apache-2.0"`], inside deny.toml `[licenses] allow` (Apache-2.0, MIT). deny.toml has no ban list entry for askama and `[bans] multiple-versions = "warn"` only [VERIFIED: deny.toml:55-60].

## Architecture Patterns

### System Architecture Diagram

```text
 mens/runs/qwen3-<size>/ (gitignored)                 mens/config/hub-cards.yaml (embedded)
   train-record.json  run_summary.json                     per-size blurbs, spokes, eval ETA
   training_manifest.json adapter_manifest.json                      |
   adapter_card.json  gate_receipt.json                              v
   collateral_damage_report.json              +------------------ config loader (validated vs SIZES)
   evidence/heldout_{base,adapter}_nothink.json                     |
   evidence/heldout_{base,adapter}_nothink.ckpt.jsonl               |
            |                                                       |
            v                                                       |
   RunRecords::load  (unchanged signature)                          |
            |                                                       |
            v                                                       v
   facts.rs: CardFacts { trained, spoke, gate, scores, training, collateral }
        |        \__ gate: check_collateral_report (stand-in -> vox_eval in 338-01)
        |
        +--> examples.rs: join report results <-> ckpt text by completion_preview,
        |                 D2 pick (2 wins + 1 loss), verdict from stored checks
        v
   render.rs: askama Template structs  --> templates/hub/{model,index}.md (+ partials)
        |
        v
   render_model_card / render_index_readme  (String, unchanged signatures)
        |
        v
   hub_sync::write_local_cards -> mens/runs/hub_cards/<repo>/README.md
        |                                  |
        |                        HUMAN side-by-side review (non-autonomous)
        v                                  v
   lints (cargo test, no network)      push: card-only (NEW) | gated weights push (existing) | index push
```

### Recommended Project Structure
```
crates/vox-populi/
├── askama.toml                      # optional; only if whitespace control is wanted
├── templates/hub/
│   ├── model.md                     # top-level model card
│   ├── index.md                     # ladder index
│   └── partials/*.md                # {% include %} fragments (status line, example, results table)
├── src/mens/hub_card/
│   ├── mod.rs                       # pub use of the old public paths; #[cfg(test)] mod tests
│   ├── facts.rs                     # RunRecords, load, GateState, CardFacts (no wording)
│   ├── collateral_standin.rs        # DELETED by 338-01 (same-signature stand-in)
│   ├── config.rs                    # hub-cards.yaml loader + validation
│   ├── examples.rs                  # D2 rule, join, verdicts
│   ├── render.rs                    # askama structs + filters + render_* fns
│   └── tests/                       # moved from hub_card_tests.rs; goldens + lints
├── tests/golden/hub_cards/*.md      # goldens
└── tests/fixtures/hub_cards/        # committed run-dir fixtures (mens/runs is gitignored)
mens/config/hub-cards.yaml
docs/src/reference/mens-models.md
```

### Q1 - Current code inventory (hub_card.rs, 1,102 lines)

**Public surface that MUST stay (via `pub use` in `hub_card/mod.rs`)**
- `pub fn render_model_card(cx: &CardContext, rec: &RunRecords) -> String` (hub_card.rs:869)
- `pub fn render_index_readme(namespace: &str, entries: &[(SizeSyncRow, RunRecords)]) -> String` (:994)
- `pub struct CardContext<'a> { pub size: &'a str, pub base_pin: &'a str, pub namespace: &'a str }` (:336-340)
- `pub struct RunRecords` with all-`pub` fields, `#[derive(Debug, Clone, Default)]` (:34-49) and `RunRecords::load(run_dir: &Path, size: &str, base_pin: &str) -> Self` (:81). `hub_sync.rs:322-325` builds `RunRecords { has_adapter: false, ..RunRecords::load(..) }`, so the struct must stay constructible that way.
- `pub fn compare_json_path(run_dir: &Path, size: &str, base_pin: &str) -> Option<PathBuf>` (:66) and `pub fn repo_name(size: &str) -> String` (:331); `pub const PROMPT_PROTOCOL` (:18).

**Callers (all in `hub_sync.rs`)** [VERIFIED: grep over crates/]: `ensure_ladder_repos` renders a reserved card with `has_adapter: false` (:312-326); `stage_publish_bundle` uses `RunRecords::load`, `compare_json_path` and `render_model_card` (:509-537); `index_entries` (:1090-1100); `render_index_repo_files` (:1103-1116); `write_local_cards` (:1123-1148). No other crate references them.

**RunRecords fields** (all `Option<Value>` unless noted): `has_adapter: bool`, `train_record`, `run_summary`, `training_manifest`, `adapter_manifest`, `adapter_card`, `gate_receipt`, `collateral`, `heldout_base`, `heldout_adapter`, `compare`, `invalid_evidence: Vec<String>`. `load` reads (relative to run dir): `train-record.json`, `run_summary.json`, `training_manifest.json`, `adapter_manifest.json`, `adapter_card.json`, `gate_receipt.json`, `collateral_damage_report.json`, `evidence/heldout_base_nothink.json`, `evidence/heldout_adapter_nothink.json` (accepted only when `prompt_protocol == "qwen3_non_thinking_codegen_v1"`, otherwise pushed to `invalid_evidence`), the three `LEGACY_EVIDENCE` think128 files (listed invalid when present with another protocol), and the compare file via `compare_json_path`. `has_adapter` = `candle_qlora_adapter.safetensors` or `adapter.safetensors` exists. `read_json` treats `{}`, `[]`, `null` and unreadable as absent (:51-59). **Not read today (new in 339):** `evidence/heldout_*_nothink.ckpt.jsonl`.

**CardStatus** (:130-139): `Passed | Refused(Vec<String>) | NotEvaluated(Vec<String>) | NotTrained(String)`, `label()` returns `passed | refused | not evaluated | not trained`. `card_status` (:199-268) = no adapter -> NotTrained; collateral `status == "pass"` else Refused; receipt gates; `overall_passed`; `substantive < 2` -> missing. 338-01 adds `NeedsReGate` and `RunRecords.collateral_req`; 339 supersedes this (see Q8).

**Goldens**: `crates/vox-populi/tests/golden/hub_cards/{index,invalid_protocol,not_evaluated,not_trained,passed,refused}.md`; `golden_cases()` (hub_card_tests.rs:188-200) renders them from in-test `json!` fixtures; update with `VOX_UPDATE_GOLDEN=1` (read at hub_card_tests.rs:209; any value; rewrites every file then `continue`s). Other tests: frontmatter parse (:235), no-score-without-record (:297), status quotes refusal (:334), determinism + host-free `!contains("/Users/")` (:371), index rule (:387), `load` reads records/rejects retired protocol (:406).

**Proposed mapping into `hub_card/`**

| Existing item (hub_card.rs) | Goes to | Notes |
|---|---|---|
| `PROMPT_PROTOCOL`, `LEGACY_EVIDENCE`, `RunRecords`, `read_json`, `protocol_ok`, `compare_json_path`, `RunRecords::load` | `facts.rs` | add ckpt path resolution; keep `compare` (hub_sync :509-511 still uses `records.compare`) |
| `CardStatus`, `card_status`, `collateral_failure`, `num`, `pct2` | `facts.rs` as `GateState` + `gate_of(rec)`; wording strings leave Rust | reasons become data (`Vec<Blocker>`), prose in templates |
| `Score`, `score`, `heldout_scores`, `collateral_rows`/`CollateralRow`, `category_table` data | `facts.rs` | collateral rows read `reasoning_slices.benches.<slice>.accuracy.{n,pre,post,p_value}` (Q7) |
| `training_section` rows, `hours`, `field`, `s`, `or_nr`, `cell` | `facts.rs` (`TrainingFacts`, only differing fields) + askama filters in `render.rs` | shared rows (method, rank/alpha, preset, seq_len, seed, digests) move to the docs page |
| `role()` | deleted; replaced by `hub-cards.yaml` blurbs | "no hardcoded sizes in Rust" |
| `repo_name`, `CardContext` | `mod.rs` or `facts.rs`, re-exported | |
| `frontmatter`, `results_section`, `usage_section`, `links_section`, `render_model_card`, `hub_cell`, `render_index_readme`, `recommended_size` | `render.rs` (structs + render fns) and `templates/hub/*.md`; `recommended_size` logic into `facts.rs` as `IndexFacts` | `render_*` keep signatures |
| (new) D2 selection, join, verdicts | `examples.rs` | |
| (new) yaml loader/validator | `config.rs` | validate keys == `SIZES` (hub_repo.rs:14 `["0.6B", "1.7B", "4B", "8B", "14B", "32B"]`) |
| `#[path = "hub_card_tests.rs"] mod tests;` (:1100-1102) | `hub_card/tests/` | With `hub_card/mod.rs`, a `#[path]` is relative to the mod.rs directory; simplest is a normal `#[cfg(test)] mod tests;` pointing at `hub_card/tests/mod.rs` |

**32B and `ensure_ladder_repos`**: it creates every repo with `exist_ok(true)`, skips a repo that has an `hf_sync_manifest.json`, and skips one that has a `README.md` sibling (hub_sync.rs:292-311). It only renders+uploads a reserved card when neither exists. So a hand-written 32B README is never overwritten by `--push`. 32B is `SizeSyncStatus::NotTrained`, which `sync_all_sizes` never pushes (:1216-1219 only `LocalOnly|Diverged`). The v2 32B card is still rendered into `mens/runs/hub_cards/vox-mens-qwen3-32b/` (write_local_cards loops all rows), so the review set shows a 32B card that must not be uploaded; any new card-only push must skip `NotTrained` rows explicitly.

**Module size budgets**: no per-crate LoC gate (`[guards] loc_budget = "off"`, layers.toml `vox-populi max_loc = 20_000` is documentation) but `arch/god_object` is Info 300 / Warning 400 / Error 500 lines per file [VERIFIED: docs/agents/governance.md:70 "Soft Info `300 lines`, Warning `400 lines`, Error `500 lines` or `12 methods`"]. `hub_card.rs` (1,102) has no entry in `contracts/toestub/suppressions.v1.json` (grep empty) so it is already over; target each new file under 400 lines, split `tests/` into several files, and do not add suppressions. Layers: vox-populi layer 3, vox-compiler layer 3, vox-eval layer 2, vox-ml-cli layer 3 [VERIFIED: docs/src/architecture/layers.toml:186,201,210,217].

### Q2 - Example data (real files)

**Files today** (item counts derived from the files, not to be hardcoded):

| size | `evidence/heldout_{base,adapter}_nothink.json` | `*.ckpt.jsonl` lines (header + items) | report `total` |
|---|---|---|---|
| 0.6B | both | 53 + 53 | 52 |
| 1.7B | both | 53 + 53 | 52 |
| 4B | both | 53 + 53 | 52 |
| 8B | both | 53 + 53 | 52 |
| 14B | none (no `evidence/heldout_*`) | none | - |
| 32B | none (only `train-record.json`) | none | - |

All eight reports carry `"prompt_protocol":"qwen3_non_thinking_codegen_v1"`, `k=1`, `n=1`, `temperature 0.0`, `max_tokens 1024`, `seed_base 1337`, `bench "mens/data/heldout_bench"`.

**Report schema (`results[]`)**: keys `manifest_index, id, file, context_files, category, description, n, c, k, pass_at_1, pass_at_k, semantic_pass_at_k, samples[]`. `samples[0]` keys: `sample_index, seed, pass, pass_compile, pass_ast, pass_exec, semantic_pass, anti_stub_pass, distinct_4_ratio, constraint_adherence, symbol_binding_accuracy, tool_call_json_valid, tool_name_exists, tool_call_salvaged, opened_think, checks, completion_preview`. `checks` keys include `non_empty, parse_ok, typecheck_ok, diag_errors, parse_error, placeholder_marker_hits, trivial_placeholder_output, anti_stub_pass, pass_compile, pass_ast, pass_exec`. `pass` is the sample's overall verdict (eval_local.rs:402-428 builds it from `verify_completion`). `completion_preview` is `completion.chars().take(240)` of `answer_region(raw)` (eval_local.rs:401,428). In the 1.7B base report `parse_error` is the string `"Parsing failed with N error(s)"` when the parse fails, else null; typecheck failures give only `diag_errors` (a count).

**Is a per-item verdict/diagnostic text stored?** No text. Stored: `parse_ok`, `typecheck_ok`, `diag_errors` (count), `parse_error` (count string), anti-stub flags. The compiler pipeline itself returns only `anyhow!("Parsing failed with {} error(s)")` for parse failure (crates/vox-compiler/src/pipeline.rs:~159-160), so a recompute through `run_frontend_str` would not give "let mut is not Vox" either; that needs `vox_compiler::lexer::lex` + `parser::parse` (a direct vox-compiler dep). **Recommendation:** verdict line from stored fields, deterministic and honest: `does not parse (N parse errors)` | `parses, does not type-check (N errors)` | `parses and type-checks, rejected as a stub` | `passes`. Treat message-level detail as an Open Question (needs a helper in vox-corpus or a user-authorised edge).

**Checkpoint -> item id mapping (ckpt `item` key).** Header line 1: `{"schema":"vox_eval_item_checkpoint.v1","key":<blake3>,"inputs":{scorer, prompt_protocol, model, base_only, bench_manifest_blake3, max_tokens, temperature, samples, seed_base}}` (writer: crates/vox-ml-cli/src/commands/mens/item_checkpoint.rs:23 `pub(crate) const ITEM_CHECKPOINT_SCHEMA: &str = "vox_eval_item_checkpoint.v1";`, header append at :259). Each later line is `{"item": <hash>, "response": <string>}` where `response` is itself a JSON string `{"generated_text": ..., "prompt_tokens": N}`. `item` = `item_hash(prompt_json)` = BLAKE3 hex of the exact request text (item_checkpoint.rs:317-320); `prompt_json` = `json!({"prompt": &prompt, "max_tokens", "temperature", "seed"}).to_string()` with `seed = sample_seed(seed_base, &id, i)` (eval_local.rs:382-388, sample_seed :45-55) and `prompt` the ChatML string assembled in eval_local_prompt.rs:121-139 (system prompt from `vox_corpus::training::generate_training_system_prompt()`, user turn from `user_for_lane`). **There is no item id in the checkpoint**, and ckpt line order is the sorted-prompt order while the report is in manifest order (measured: positional match 0/52 on every pair).

**Chosen join (verified on all 8 pairs: 52/52 unique, 0 ambiguous, 0 unmatched):** for each report result take `samples[0].completion_preview`; the ckpt line whose `answer_region(generated_text)[..240 chars]` equals it is that item's full output. `answer_region` (vox_eval::qwen_reasoning :165-174) is: text after the last `</think>`, else strip a leading `<think>`, else `.trim()`. Rules for examples.rs: unique match -> use; zero or >1 -> `no recorded output` (never guess, never silently skip). It needs no prompt reconstruction, no dependency on the 335/337 files, and is content-verified by construction. Exact-hash reconstruction is possible later (vox-populi already depends on vox-corpus; vox-eval Lane helpers would be needed) but duplicates >50 lines of `prepare_bench_item`; not recommended.

**Prompt text**: `mens/data/heldout_bench/manifest.json` has `{schema: "vox_mens_bench_manifest_v1", description, benchmarks[52]}`; each benchmark has keys `answer, category, description, id, semantic_expected_contains`, no `file`, no context files. The prompt is inline: `Write a Vox program that: {description}` (eval_local_prompt.rs:98-110), then wrapped in the ChatML protocol. The report's `results[].description` already carries the description, so examples need no manifest read (the manifest also has a reference `answer`; do not show it as model output).

**What the real data gives under the D2 rule** (computed this session; shown only to prove the rule yields content; do not hardcode): fixes/breaks by size: 0.6B 16/3, 1.7B 14/0, 4B 10/1, 8B 9/1; wins (2 shortest by description length then id): 0.6B `page_index`, `humaneval_vox_043_triangle_valid`; 1.7B same; 4B `humaneval_vox_069_binary_search`, `mutation_update`; 8B `humaneval_vox_041_nth_prime`, `humaneval_vox_069_binary_search`. Loss under the literal spec rule ("shortest item the adapter still fails") is `humaneval_vox_042_reverse_list` for all four.

**Pitfall found by probe:** running `vox check` on the raw `generated_text` of the 1.7B pairs, 17 of 20 report-passing samples passed; the 3 failures were base outputs wrapped in a markdown fence (`extract_code` strips fences before scoring, metrics.rs:142-167). Examples must render the *extracted code*, not raw text, and only adapter/base outputs that the report marked `pass` may sit in a ` ```vox ` fence (failing outputs go in ` ```text `), otherwise the Vox-block lint contradicts the example design.

**Serve transcript for "Use it":** none exists. Searched `mens/runs/qwen3-1.7b|4b|0.6b` for `chat/completions` (only the old rendered cards match), `external_serving_handoff_v1.json` has keys `adapter_path, artifact_dir, backend_family, base_model, notes, prompt_format, schema, schema_version, tokenizer_source` (no transcript), and docs mention no memory/transcript. **Honest fallback to specify:** the request block uses the first win's prompt; the output shown is that item's recorded held-out adapter generation, labelled "recorded from the held-out run (`evidence/heldout_adapter_nothink.ckpt.jsonl`, same prompt protocol and greedy decode as the `codegen` lane); not a transcript of `vox mens serve`". Phase 340 will produce real `vox mens run` transcripts (ROADMAP); this block then switches data source. For non-`passed` gate states `vox mens serve` refuses (current text at hub_card.rs:809-813), so the block shows pull with `--allow-ungated` and the recorded output, without a serve command.

### Q3 - askama

- Version: `askama 0.12.1`, `askama_derive 0.12.5`, `askama_parser 0.2.1`, `askama_escape 0.10.3` [VERIFIED: Cargo.lock:391-429]. Only dependent: `uniffi_bindgen 0.28.3` (Cargo.lock:16004-16010) via `uniffi` <- `vox-runtime-rn` (crates/vox-runtime-rn/Cargo.toml `uniffi = { version = "0.28", features = ["build", "tokio"] }`); uniffi_bindgen depends on askama with `version = "0.12"`, `features = ["config"]`, `default-features = false` [VERIFIED: uniffi_bindgen-0.28.3/Cargo.toml:31-33]. No `[workspace.dependencies]` entry (`grep '^askama' Cargo.toml` empty).
- Edge verdict: **no workspace crate edge.** `crates/vox-cli-ci/src/crate_edges.rs:185-187`: "Live in-tree edge set from `cargo metadata`: workspace members only, normal + build dependency kinds (dev-deps excluded in v1...)". A third-party dep is never an edge. No third-party allowlist ledger exists (`contracts/ci/` holds crate-edges, dep-backedges, fan-in, etc.; `deny.toml` bans nothing relevant). **No user-authorized ledger entry is needed.** The cryptography transport ledger (AGENTS.md) concerns TLS/QUIC providers only.
- hakari: `crates/workspace-hack` is hakari-managed and `cargo hakari generate --diff` runs only in nightly.yml (:671). Using exactly uniffi_bindgen's feature set (`default-features = false, features = ["config"]`) means askama's feature set is identical in both graphs, so no unification delta is expected; the plan should still run `cargo hakari generate --diff` once (cargo-hakari 0.9.39 is installed) after adding the dep and commit any `workspace-hack/Cargo.toml` change. [ASSUMED: not run - requires editing Cargo.toml]
- Markdown, no escaping: `.md` template extension selects `::askama::Text` (config.rs:303 above). The derive also accepts `escape = "none"` (input.rs:291 reads `escape`; input.rs:376 `"none" => None`). Use both (`#[template(path = "hub/model.md", escape = "none")]`) and add a test that `<`, `&&` and backticks in Vox output pass through unchanged (mutation: switch to `escape = "html"`).
- Template dir: `<CARGO_MANIFEST_DIR>/templates` by default (config.rs:27 `default_dirs = vec![root.join("templates")]`), so `crates/vox-populi/templates/hub/model.md` is `path = "hub/model.md"`. `askama.toml` is only needed for non-default dirs/whitespace; with `config` enabled it is read from the crate root (config.rs:298 `CONFIG_FILE_NAME = "askama.toml"`). Includes/macros/imports are supported by the derive; custom filters live in a `mod filters` next to the template struct.
- Risk: askama 0.12.1 (edition 2021) has not been compiled in the vox-populi graph on this toolchain (rust-version 1.96). First task must be a tracer: one template + one test, `cargo check -p vox-populi --features mens-hf-hub` in the background.
- `render()` returns `Result<String, askama::Error>`; `render_model_card` must stay `-> String`. Template data is owned and infallible, so convert with a single helper that maps an `Err` to a loud placeholder and is covered by a test; avoid scattering `expect`.

### Q4 - `vox check` lint placement

- vox-populi has **no direct** `vox-compiler` dependency (normal or dev); it reaches it through `vox-corpus` (cargo tree above). `contracts/ci/crate-edges.allow.v1.json` has no `["vox-populi","vox-compiler"]` edge; dev-deps are not ratcheted, but the user rule is zero new edges.
- API that `vox check` ends in: `vox_compiler::pipeline::run_frontend_str(source: &str, file_path: &str) -> Result<FrontendResult>` (pipeline.rs:96); error iff `Err` or `FrontendResult::has_errors()` (:53). `vox check` is crates/vox-cli/src/commands/check.rs -> `run_frontend_with_options` -> the same compiler function.
- **Recommended placement: vox-populi `--lib` test calling `vox_corpus::corpus::corpus_readiness::verify_vox_admission(source: &str) -> OracleReceipt`** (corpus_readiness.rs:~66-148; `pub mod corpus;` lib.rs:8, `pub mod corpus_readiness;` corpus/mod.rs:8; no cfg gating; `mens` feature already pulls vox-corpus: `mens = [... "dep:vox-corpus" ...]`). It runs `run_frontend_str` (lex, parse, HIR, typeck), rejects retired decorators (`@table`, `@query fn`, `@mutation fn`, `@component fn`, `@mcp.tool`, `@server fn`, `@endpoint`) and runs an interpreter smoke check. It is stricter than `vox check`. `OracleReceipt { passed, stage, diagnostics, ... }` gives a failure message for the test output. Zero new edges of any kind, no new compile cost.
- Alternatives and why not: `vox-ml-cli` already depends on `vox-compiler` and optionally `vox-populi` (`mens-hf-hub` feature), so a test there also has zero new edges, but vox-ml-cli is where 335/337 are editing and its tests need heavy features. A vox-populi dev-dep on vox-compiler would not trip `crate-edges` and stays inside the existing SCC in `dep-backedges.allow.json` (20-crate cycle containing both), but it is a manifest edge the user excluded.
- Risk to check in the first lint task: `verify_vox_admission` also runs an interpreter smoke; run it over every adapter win in the committed fixtures first. If a legitimate winning sample is rejected only by the smoke step, fall back to asserting on the `stage` of the receipt (parse/typeck) and record why.
- Doc snippets: ` ```vox ` fences in `mens-models.md` are compiled by vox-doc-pipeline's doctest runner; use no ` ```vox ` fences there unless they compile, or mark `// vox:skip` with a reason (AGENTS.md).

### Q5 - HF metadata lint

Rules, from the Hub documentation fetched this session [CITED: https://huggingface.co/docs/hub/model-cards, https://raw.githubusercontent.com/huggingface/hub-docs/main/modelcard.md]:
- `license`: a Hub licence id; `other` requires `license_name` and `license_link`. `library_name` optional string. `language` list of codes. `tags` list. `datasets` list of Hub dataset ids. `metrics` list of metric ids.
- `base_model`: "either a single model ID, or a list ... specified by their Hub identifiers"; `base_model_relation` one of `"adapter", "merge", "quantized", "finetune"`. Properties "will be validated by the Hub when git pushing changes to your README.md file".
- `pipeline_tag`: task id (the Hub says the metadata UI validates it; the allowed set is not in the fetched pages -> keep `text-generation`).
- `model-index`: list of `{name, results[]}`; each result `{task:{type (required), name}, dataset:{type (required), name (required), config, split, revision, args}, metrics:[{type (required), value (required), name, config, args, verifyToken}], source:{name, url (url required if source present)}}`. The template does not list `verified`; the existing cards emit `verified: false`. [ASSUMED: `verified` is accepted; phase 336 shipped it and the Hub accepted the pushes. Drop it unless needed.]
- Existing project evidence: hub_card_tests.rs:250-256 asserts `base_model` has no `@` ("Hub rejects a revision in base_model"), `base_model_relation == "adapter"`, `base_model_revision` 40 chars (a non-standard key we add), and model-index shape.

Proposed fixture `crates/vox-populi/tests/fixtures/hub_cards/hf-metadata-rules.v1.yaml` (committed, no network), parsed with `serde_yaml`:
```yaml
schema: hf_model_card_metadata_rules.v1
source: https://huggingface.co/docs/hub/model-cards   # retrieved 2026-11-06
required: [license, library_name, pipeline_tag, language, tags, base_model, base_model_relation]
base_model: { forbid_char: "@", shape: repo_id }          # owner/name, no revision
base_model_relation: [adapter, merge, quantized, finetune]
extra_keys_allowed: [base_model_revision]                 # project key, ignored by Hub
tags: { type: list_of_string, require_prefix_tags: ["gate-", "spoke-"] }
model-index:
  entry: { required: [name, results] }
  result: { required: [task, dataset, metrics] }
  task: { required: [type] }
  dataset: { required: [type, name] }
  metric: { required: [type, value], value_type: number }
```
Reuse: only the test-local `frontmatter_of` and `hub_card_frontmatter_parses_with_required_keys` (hub_card_tests.rs:226-294); there is no HF validation code anywhere else (`grep model-index|base_model_relation crates/` hits only hub_card.rs and its tests). The tag prefixes make the spoke and gate-state tags (spec §4.1) enforceable.

### Q6 - Docs page

- `docs/src/reference/mens-models.md` **does not exist**. Existing neighbours: `mens-serving-ssot.md`, `mens-training.md`, `mens-training-data-contract.md` (the cards' existing link targets `https://voxlang.org/reference/mens-training/`, `.../mens-training-data-contract/`, `.../mens-serving-ssot/`).
- Required frontmatter (documentation-governance.md:85-107 starter and :143-185 facets): `title`, `description`, `category`, `status`, `topic`, `domain`, `authorship` (+ `authorship_model`), then H1 and a 40-120 word `<!-- kb:summary -->` block directly under it. Do not hand-add `last_updated`, `last_verified`, or `human_attested_by`; do not write `training_eligible: true` for LLM authorship. `category` must be one of the enforced labels; for `reference/` pages the repo uses `"Language Reference"` (mens-serving-ssot.md:4, vox-diagram.md:4).
- Taxonomy [VERIFIED: contracts/documentation/kb-taxonomy.v1.yaml:216-217,20,34]: topic prefLabel `mens-training` (`broader: ml`), domain prefLabel `ml`. Quote exactly: `topic: "mens-training"`, `domain: "ml"`. (`model-evaluation` is the alternative topic, also `broader: ml`; `mens-training` fits the page best.)
- `authorship: "llm-assisted"`, `authorship_model: "claude-sonnet-5-5"` (same value vox-diagram.md:8-9 uses; the model id of this session is claude-sonnet-5-5).
- Draft header:
```md
---
title: "Reference: MENS models, spokes and gating"
description: "What the MENS adapter ladder is, how hub and spoke work, what the gate states mean, and how the model cards on Hugging Face are produced."
category: "Language Reference"
status: "current"
topic: "mens-training"
domain: "ml"
authorship: "llm-assisted"
authorship_model: "claude-sonnet-5-5"
---
```
- VoxDiagram: read docs/src/reference/vox-diagram.md (grammar card :22-72). Draft hub-and-spoke fence, **not validated**: the installed `vox` (0.6.0+build.7529, `~/.cargo/bin/vox`) has no `diagram` subcommand ("unrecognized subcommand 'diagram'"), and the repo binaries (`target/{debug,release}/vox`, Oct 28 / Nov 4) predate it. Checked by hand against the card's rules (all labels quoted, ids letters/digits, no reserved word, dotted paths into a container, one arrow per line):
```vox-diagram
graph {
    direction: right

    node base: "Pinned Qwen3 base model, one repo per size"
    node harness: "Harness: gates, eval and prompt protocol"

    container spokes: "Spoke adapters" {
        node vox: "vox-lang (trained)" { shape: rounded }
        node tool: "harness: tool-selection (planned)" { shape: rounded }
        node args: "harness: argument-generation (planned)" { shape: rounded }
        node rust: "rust (planned)" { shape: rounded }
    }

    base -> spokes.vox: "QLoRA adapter"
    base -> spokes.tool: "adapter"
    base -> spokes.args: "adapter"
    base -> spokes.rust: "adapter"
    harness -> spokes.vox: "gate"
}
```
  The plan must validate it (`vox_validate_diagram` MCP tool or `vox --json diagram check -` once a current `vox` exists; apply every `fixes[]`; re-check) and must not cite it as validated.
- Anchors: Astro markdown headings get ids from `github-slugger` (docs-astro/package.json `"github-slugger": "2.0.0"`; page slugs from `docSlug`, docs-astro/src/utils/doc-slug.mjs). Rules the link lint must mirror: lowercase; drop punctuation except `-` and `_`; spaces to `-`; duplicate headings get `-1`, `-2`. Page URL is `https://voxlang.org/reference/mens-models/` and the H1 is stripped (`remarkStripTitleH1`, astro.config.mjs:25-30), so anchors come only from `##`/`###`. Keep headings ASCII with no punctuation ("Hub and spoke" -> `hub-and-spoke`, "State legend" -> `state-legend`, "Prompt protocol and training method" -> `prompt-protocol-and-training-method`). Implement a ~10-line slugger in the test (no Rust crate for it is in `Cargo.lock`; only `heck` exists) and unit-test it against these examples. The lint must also fail on anchors with a trailing `?`/`.` etc.
- Doc lint: `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/mens-models.md`. **`--paths` is resolved relative to `docs/src/`** (pipeline/mod.rs:152-177: relative path is `docs_src.join(p)`, and a missing path is an error: ":156 `--paths docs/src/archive/foo.md` joined onto `docs/src/`"). The CONTEXT/spec command `--paths docs/src/reference/mens-models.md` therefore errors "missing path"; use the relative form (absolute also works). Flags confirmed in main.rs:6-8 (`--lint-only`, `--fix`, `--paths`). Not run here (it builds the crate).
- Do not hand-regenerate SSOT artefacts after adding the page; `ssot-autoregen` handles drift (AGENTS.md). The cards' `voxlang.org/...#anchor` links 404 until the page is merged and deployed: publish the docs page before the Hub push.

### Q7 - Gate state inputs (real data)

| size | adapter weights | train-record | collateral_damage_report.json | gate_receipt.json | state today (stand-in) |
|---|---|---|---|---|---|
| 0.6B | yes | `status: trained`, `peak_mb 15319` | `status: fail`, `failed_on: gsm8k_think`, `threshold 0.05`, NO `slice_blake3`, NO `grader`, `reasoning_slices` has only `benches, scope_note, slice_size` (no `rule`/`alpha`); report 2026-11-05 05:15 (old); 128-item re-gate running | `overall_passed: false`; collateral gate row "collateral_damage_report.json miss..." (stale) | `failed` (status != pass is checked first) until the re-gate rewrites the report |
| 1.7B | yes | trained, `27007` | `status: pass`, no `slice_blake3`, no `grader`, no `rule`/`alpha` | `overall_passed: true`, gates leakage/anti_stub/pass_at_k/collateral pass | `needs re-gate` |
| 4B | yes | trained, `34460` | `status: pass`, same stale shape | `overall_passed: true` | `needs re-gate` |
| 8B | yes | trained, `49374` | `status: pass`, same stale shape | `overall_passed: true` | `needs re-gate` |
| 14B | yes (`candle_qlora_adapter.safetensors`) | trained, `63076` | none | `overall_passed: false`; anti_stub/pass_at_k/collateral rows "missing" | `not evaluated` |
| 32B | none | `status: not_trained`, `reason: "above non-fitting 4B (D-06)"` | none | none | `not trained` |

(Fields printed by `jq` this session from mens/runs/qwen3-*/; `slice_blake3`/`grader` are written by the current writer at eval_collateral.rs:183-184 and :344-345, so the 0.6B re-gate output will carry them.) Spoke: no run record carries lanes today (`adapter_card.json` keys: `adapter_sha256, base_hf_id, base_revision, base_rung, corpus_hash, cost_usd, created, git_sha, lora_alpha, lora_rank, metrics, preset_version, provider, quantization, seed, syntax_fingerprint_kind, train_data_sha256, training_manifest_digest, training_syntax_fingerprint`; no `train_lanes`/`spokes`; `training_manifest.json`'s `train_file` is a host path outside the repo). 338-06 adds `AdapterCard.train_lanes`/`spokes` (338-06-PLAN.md:105-107).

**Decision-relevant fields in the new writer's report** for the Results table: `reasoning_slices.benches.<slice>.accuracy` = `{n, pre, post, losses, gains, p_value, regressed}` (vox-eval reasoning_slice.rs:257-266 `to_json`), `reasoning_slices.pooled` same keys, `rule` = `"exact one-sided McNemar on paired items, fail when p < alpha"` (:22 `PAIRED_RULE`), `alpha`, `slice_size` 128 (reasoning_bench.rs:15), `slice_blake3`, `grader` = `"grade_final_answer v2"` (qwen_reasoning.rs:153).

**Stand-in contract (must equal 338-01's, see 338-01-PLAN.md:119-122):**
```rust
// crates/vox-populi/src/mens/hub_card/collateral_standin.rs  (338-01 deletes this file)
#[derive(Debug, Clone, PartialEq)]
pub struct CollateralRequirement { pub alpha: f64, pub slice_pins: BTreeMap<String, String>, pub grader: String }
pub enum CollateralRefusal { Failed(String), Stale(String) }   // + Display
pub fn check_collateral_report(report: &serde_json::Value, req: &CollateralRequirement) -> Result<(), CollateralRefusal>;
impl CollateralRequirement { pub fn from_policy_json(policy: &serde_json::Value) -> Result<Self, String>; }
```
Order (copy 338-01): status `pass` else `Failed`; `reasoning_slices.rule == PAIRED_RULE` and `alpha == req.alpha` else `Stale`; every `SLICE_NAMES` entry's `slice_blake3` equals `req.slice_pins[name]` else `Stale`; `grader == req.grader` else `Stale`; finally `vox_eval::reasoning_slice::verify_reasoning_slices(report, req.alpha)` (exists, public, reasoning_slice.rs:~473) with `Err` mapped to `Failed`. It may use `vox_eval::{reasoning_slice::PAIRED_RULE, reasoning_bench::SLICE_NAMES, qwen_reasoning::GRADER_ID}` (which needs `dep:vox-eval` under `mens-hf-hub`, same edit as 338-01) so it does not fork the constants. Pins come from `mens/config/eval-gates-ladder.yaml` `collateral_damage.alpha: 0.05` and `slices.<name>.blake3` (:95-111). The requirement loader lives in `facts.rs` (private `load_requirement`), NOT in `gate_policy.rs`, because 338-01 creates `gate_policy.rs` (its Task 1) and 338-06 edits it. 338-01's retarget is then one `use` line plus assigning `collateral_req` from `gate_policy::ladder_requirement`.

**Where the stand-in must not live:** not in `crates/vox-eval` (338-01 creates `collateral_evidence.rs` there; 335/337 edit the vox-eval/vox-ml-cli eval files) and not in `gate_policy.rs`. **Edge check:** `["vox-populi","vox-eval"]` is in crate-edges.allow.v1.json; the dep is optional and currently only in `mens-train = ["mens", "mens-candle-qlora", "mens-hf-hub", "dep:vox-eval", "dep:vox-plugin-host"]` (Cargo.toml:78); `mens-hf-hub = ["mens", "dep:hf-hub", "tokio/rt-multi-thread"]` (:79) does not include it yet.

### Q8 - Ordering vs 338-01 and retarget instructions

State of the files: `git log --oneline -15 -- hub_card.rs hub_sync.rs` shows only `4f466df1b feat(mens): render Hub model cards and ladder index from run records`, `218d8cdea`, `10aa40d02`; `git status` shows neither file modified; no 335/337/338 SUMMARY exists; the only other worktree on 335 (`vox-mens-compare-p335-clean`, detached f51c569c0) is clean and 335-01 lists no hub files. Nobody is editing them now.

**Conflict surface:** 338-01 Task 2, 338-02 (its hub_card task, `RunRecords.spoke_gates`, "Gate state by spoke"), and 338-06 Task 3 (`spoke-<name>` tag and `Spoke:` line) all edit `hub_card.rs`, `hub_card_tests.rs` and the `passed/refused/needs_re_gate/not_evaluated` goldens. 338-01 Task 1/3 (vox-eval, gate_policy.rs, hub_sync verify functions, dispatch/check_run) do NOT touch the card code.

**Recommendation: land 339's code waves first (stand-in), and retarget 338.** Reasons: (a) 338-01 is blocked on 335 and 337 landing (its precondition at 338-01-PLAN.md:140); 339 is explicitly brought forward and needs neither; (b) splitting a file under three queued plans is far cheaper before they run; (c) 339 produces `needs_re_gate` states and goldens that 338-01 would otherwise produce. Concrete retargets for the 338 plans (put in 339's SUMMARY and edit the 338 PLAN frontmatter before it runs):
- 338-01 Task 1 and 3: unchanged, except Task 1's `crates/vox-populi/Cargo.toml` edit is already done by 339 (skip if `dep:vox-eval` is present).
- 338-01 Task 2: `hub_card.rs`/`hub_card_tests.rs` -> `hub_card/facts.rs`, `hub_card/mod.rs`, `hub_card/tests/`; delete `hub_card/collateral_standin.rs`; in `facts.rs` change the one import to `vox_eval::collateral_evidence::{CollateralRefusal, CollateralRequirement, check_collateral_report}`; set `collateral_req` via `gate_policy::ladder_requirement`; DO NOT re-add `CardStatus::NeedsReGate` (339 has `GateState::NeedsReGate`); `needs_re_gate.md` golden already exists - only its fixture builder changes to `gate_policy::test_support::passing_collateral_report`; the `verify_pulled_adapter_dir`/`stale_pull_*` part stays as written.
- 338-02: the card half -> `facts.rs` (`spoke_gates` field) and a partial under `templates/hub/partials/`; its "Gate state by spoke" table conflicts with D5 (gate stated once). Decide with the user: keep one status line plus a collapsed per-spoke detail, or defer the table.
- 338-06 Task 3: the `spoke-<name>` tag -> the frontmatter block in `templates/hub/model.md`; the `Spoke:` line -> the status-line partial; spoke value -> `facts.rs` `SpokeFact` (339 reads `adapter_card.json` `spokes` when present, otherwise a documented fallback from `hub-cards.yaml`; 338-06 swaps the source for `gate_policy::spokes_of_run`).
- `hub_sync.rs`: 339 adds nothing to the existing functions that 338 edits. Put card-only push in a **new** file (`crates/vox-populi/src/mens/hub_cards_push.rs`) and a flag in `vox-ml-cli` `mens/hub.rs` (338-09 edits hub.rs in wave 5, later) to avoid text conflicts with 338-01/02/06.

### Q9 - Index inputs

- Measured memory per size: `train-record.json` `peak_mb` + `peak_source` (0.6B 15319, 1.7B 27007, 4B 34460, 8B 49374, 14B 63076 MiB on Apple Silicon unified memory; all `peak_source: "run_summary"`). These are **training peaks, not serving requirements**; no serving memory measurement exists anywhere (`mens-serving-ssot.md` has no memory figure; the existing index text already says so, hub_card.rs:1042-1046). The yaml `gpu-specs.yaml` `train_bases.qwen3_code` rungs carry `floor_mb` and `floor_measured` (0.6B/1.7B/4B measured, 8B/14B/32B `floor_measured: false` estimates that disagree with the 8B train-record: 122876 estimated vs 49374 measured). Prefer `train-record.json` for card numbers; use `gpu-specs.yaml` (via `size_select::load_ladder_overlay`, already imported by hub_sync.rs) only to order/describe rungs. Probe and constants: `crates/vox-populi/src/mens/tensor/size_select.rs:1-9` (Metal working set / `nvidia-smi memory.free` x0.88) and `memory_model::SAFETY_FRACTION`; presets in `SIZE_PRESETS` (:21-28) and `contracts/mens/training-presets.v1.yaml`.
- "Which model" static rule: generic logic, data in yaml, no sizes in Rust: among sizes whose `GateState` is `Passed` (current rule), pick the highest held-out adapter pass@1, ties to the smaller size in `SIZES` order (this is today's `recommended_size`, hub_card.rs:978-991, minus the size-specific text); show the training-peak column; if none qualifies print "no size currently has a current-rule pass" and name the sizes that need a re-gate (today that is every trained size). The rule sentence and wording live in `hub-cards.yaml`/the template; Rust only filters and sorts. The 340 selector replaces it later.
- "What's next" eval time: **no measured wall time exists in any run record** (`grep` of the held-out report found no timing field; 337-RESEARCH.md:294 "There is no timing field in any report JSON"). The only figures are phase 337's projections (337-RESEARCH.md:310-318, LOW confidence: 4B 128/slice collateral gate ~3.8 h expected, ~5.8 h conservative; per-size AFTER-fix s/item 0.6B 6, 1.7B 10, 4B 24, 8B 27, 14B 42) which 337 itself says will be replaced by THRU-12 measurements. Recommendation: `hub-cards.yaml` per-size `regate_eta: { hours_expected, hours_conservative, basis: "<doc ref>", confidence: low }`, rendered as "about N h (estimate, <basis>)", omitted when absent (validation: a size that `needs re-gate` or is `not evaluated` must have an estimate or the render says "no estimate"). Updating it after 337 is a one-file edit. Do not derive from checkpoint mtimes (non-deterministic).
- Spokes with no adapter: `hub-cards.yaml` `spokes:` list with `status: trained | planned` and a `plan` link (docs anchor or 338 reference); "planned" cells in the grid come from there, not from Rust.

### Q10 - Hub sync push mechanics

`vox mens hub sync` flags [VERIFIED: crates/vox-ml-cli/src/commands/mens/hub.rs:63-85]: `--namespace`, `--push`, `--pull`, `--force-ungated`, `--private`, `--json`, `--size <S>`.
- Without `--push`: nothing uploaded; always renders all cards and the index into `mens/runs/hub_cards/` (`write_local_cards`, hub_sync.rs:1123-1148, called at :1267 on every run). This is the dry run for the human review.
- With `--push`: (1) `ensure_ladder_repos` (create repos; reserved card only where no README, :269-344); (2) per selected size whose status is `LocalOnly|Diverged` -> `publish_adapter_run_dir` (:1216-1238): `verify_publish_gates` first (refuses unless gated, or `--force-ungated`), stages the whole bundle (weights, evidence JSON, **README.md and MODEL_CARD.md**, `hf_sync_manifest.json` with a BLAKE3 for every staged file incl. the README, :536-551), uploads the folder, creates a new `v<N>` tag (:729-745); a refusal is recorded as `push_refused: ...` and nothing is uploaded for that size; (3) `upload_ladder_index` (:1268-1270, :1276-1296) uploads `README.md` + `ladder_index.json` for `<ns>/vox-mens-index` regardless of per-size outcomes. `--size` limits step 2 only (:1215 `selected`).
- **There is no card-only mode.** Because the README is part of `files` in `HfSyncManifest`, any card change makes `manifests_match` false (:229-234 compares `files` and `gate_status`), so a published size reads `Diverged` and `--push` either re-uploads the full bundle with a new tag (only if its gate passes the current check) or refuses. After 338-01 a stale size refuses, so `--push` cannot update the 1.7B/4B/8B/14B cards at all. `--force-ungated` would upload but is forbidden by the brief.
- Pull integrity: `verify_pulled_adapter_dir` re-hashes every manifest file (per the card text and hub_sync.rs:789-870), so a card-only upload that changes `README.md`/`MODEL_CARD.md` without rewriting those two manifest entries breaks `vox mens hub pull` on `main`. The new path must: fetch the remote `hf_sync_manifest.json` (`fetch_remote_sync_manifest`, :650), replace the `README.md` and `MODEL_CARD.md` entries (BLAKE3 via `vox_crypto::hash_fast_hex`, size_bytes) and set `gate_status` to the label the local gate check now gives (otherwise the manifest keeps saying `passed` for 1.7B v5 while the card says `needs re-gate`), stage exactly `README.md`, `MODEL_CARD.md`, `hf_sync_manifest.json` in a temp dir, and call `upload_model_folder` (hub.rs:407-456, a folder upload that adds/overwrites only the staged paths) with a "Update model card (no weights)" message and **no new `v<N>` tag** (tags pin published weights). Split the pure staging (`stage_card_only(remote_manifest, card, gate_label) -> (files, manifest)`) from the network call so it is unit-testable; skip `NotTrained` rows (32B) and `HubOnly` rows.
- Which size is "gated-current" today: **none.** 1.7B/4B/8B are stale passes, 14B has no collateral report, 0.6B's report is the old failing one. After the running 0.6B 128-item re-gate, 0.6B may become the only gated-current size; its weights then go through the existing gated `--push --size 0.6B` (new `vN` tag, card v2 included) while the other trained sizes use card-only.
- `ensure_ladder_repos` never touches an existing README, so 32B's hand-written card is safe from `--push`; the new card-only path must be explicit about excluding it.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown generation | more `format!`/`writeln!` prose | askama templates + yaml data | D4; the repetition came from prose in Rust |
| Compiler check of a snippet | custom lexer/parser calls or a shelled-out `vox check` | `vox_corpus::corpus::corpus_readiness::verify_vox_admission` | existing public API, zero edges |
| Collateral currency | a new comparison of report fields in vox-populi | the stand-in with 338-01's exact signature, later `vox_eval::collateral_evidence` | one decision point (SPOKE-01) |
| McNemar / p-values | recomputing statistics for the card | read `reasoning_slices.benches.*.accuracy.p_value` from the report | the report is the record; the card must not invent numbers |
| BLAKE3 | a new hash | `vox_crypto::hash_fast_hex` (as hub_sync.rs:545) | AGENTS.md crypto policy |
| Fenced-code extraction for examples | a different extractor | port `extract_code` (metrics.rs:142-167) exactly, with `// vox:defactored-from vox-ml-cli 2026-11-06` (<50 lines, defactor policy) | shown code must equal scored code |
| YAML parse | hand parsing | `serde_yaml` (already a dep under `mens`) | |
| HF YAML validation | a network call or a vendored JSON schema | the local rules fixture (Q5) | spec §7: no network |
| Docs anchors | ad hoc regex | a small tested github-slugger port | must match Astro |

**Key insight:** every number and every sentence on a card is either a record field or template/yaml text; the moment Rust formats prose again the duplication returns.

## Common Pitfalls

### Pitfall 1: `mens/runs/` is gitignored
**What goes wrong:** a golden or lint that reads `mens/runs/qwen3-1.7b/...` passes locally and fails on hosted CI (the directory is absent). `git check-ignore -v` reports `.gitignore:212:mens/runs/` for every record and ckpt.
**How to avoid:** commit minimal fixture run dirs under `crates/vox-populi/tests/fixtures/hub_cards/<case>/` (real-shaped JSON + a handful of ckpt lines); build the report previews and ckpt text from one source of truth in the fixture builder. Keep one fixture copied from the real 1.7B schema to prove the preview join against the real shape.

### Pitfall 2: published manifest and README are coupled
**What goes wrong:** treating the card push as "just upload README.md" breaks `vox mens hub pull` (hash mismatch) and leaves a stale `gate_status` label. See Q10.

### Pitfall 3: raw model text is not what was scored
**What goes wrong:** showing `generated_text` verbatim puts fenced/prose text in a ` ```vox ` block; 3 of 20 passing 1.7B samples failed `vox check` raw. **How to avoid:** render `extract_code(answer_region(text))`; passing outputs only in ` ```vox `; failing outputs and "no recorded output" in ` ```text `.

### Pitfall 4: the duplication lint contradicts the spec content
**What goes wrong:** spec §4 requires the D2 rule statement and a gate blocker on every card, and 1.7B/4B/8B share the same blocker ("passed the retired 32-item collateral rule..."); a literal "no sentence of >=4 words on two cards" fails goldens immediately. Mutation 3 (state twice fails the lint) also implies the lint checks within one card.
**How to avoid:** define "sentence" precisely in the lint and the plan: split prose on `. `/`? `/`! ` after removing frontmatter, fenced code, table rows, headings, bare link lines and the footer; a card's own numbers/size name make rule and blocker sentences unique ("Of 52 items the 1.7B adapter fixes 14 and breaks 0; wins are the two shortest..." with the rule explained once on the docs page and linked). Add a separate `gate_state_stated_once` test (count of the gate label in the body) so mutation 3 fails for the right reason. Present the exact definition to the user in the plan (Open Question 3).

### Pitfall 5: stale collateral numbers leaking
**What goes wrong:** a `needs re-gate` card showing the old 32-item table, or a retired-rule fail shown as "failed". **How to avoid:** collateral table only when the gate state is `Passed`; for `Failed` show only the refusal's one-line text; `NeedsReGate`/`NotEvaluated` show no collateral numbers; `model-index` gets the held-out pass@1 (protocol-checked) and collateral metrics only for `Passed`.

### Pitfall 6: wording drift in yaml vs template
Blurbs containing numbers (memory, items) rot. Keep digits out of yaml prose; memory comes from `train-record.json`; add a test that yaml strings contain no digits other than inside size names.

### Pitfall 7: askama specifics
Compile-time templates need a rebuild when a template changes (the derive tracks the file); `{{ }}` of Vox code containing `{{`/`{%` inside fenced blocks would be parsed as template syntax only if it is in the template source, not in data, so keep model outputs as data. Watch trailing-newline/whitespace: goldens are byte-compared, and `render_*` must be byte-stable (`manifests_match` hashes the card).

### Pitfall 8: docs ahead of Hub
Cards link `voxlang.org/reference/mens-models/#...`; push only after the page is deployed. Include a deploy check in the human checkpoint.

### Pitfall 9: tool/feature gating
`hub_card` is `#[cfg(feature = "mens-hf-hub")]` (mens/mod.rs:30-31). All test commands need `--features mens-hf-hub`. `unexpected_cfgs = deny` is set workspace-wide (Cargo.toml `[workspace.lints.rust]`), so no stray `cfg(...)`.

## Code Examples

### askama template struct (pattern; compile unverified here)
```rust
// Source: askama 0.12 derive attributes - input.rs:291 `escape`, config.rs:27 default dir
use askama::Template;

#[derive(Template)]
#[template(path = "hub/model.md", escape = "none")]
struct ModelCard<'a> {
    facts: &'a CardFacts,
    cfg: &'a SizeBlurb,
}

pub fn render_model_card(cx: &CardContext, rec: &RunRecords) -> String {
    let facts = facts::card_facts(cx, rec);
    render_or_marker(ModelCard { facts: &facts, cfg: config::size(cx.size) })
}
```

### Preview join (examples.rs core)
```rust
// Join one report result to its checkpoint text. Verified on 8/8 real pairs (52/52 unique).
fn full_text<'a>(preview: &str, ckpt: &'a [(String /* answer_region(text) */, String /* text */)]) -> Option<&'a str> {
    let mut hit = ckpt.iter().filter(|(ans, _)| ans.chars().take(240).eq(preview.chars()));
    match (hit.next(), hit.next()) {
        (Some((_, full)), None) => Some(full),
        _ => None, // zero or ambiguous: "no recorded output"
    }
}
```

### D2 pick (sort keys)
```rust
// wins: adapter pass && !base pass; key = (description.chars().count(), id)
// loss: adapter !pass; same key; None => card says there is no failing item
```

### Frontmatter mutation hook
Keep `tags:` assembly in the template from `facts.gate.tag()` (`gate-passed`, `gate-needs-re-gate`, `gate-failed`, `gate-not-evaluated`, `gate-not-trained`) and `facts.spoke.tag()` (`spoke-vox-lang`).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Prose in `format!` (phase 336) | askama + yaml | this phase | wording reviewable, lintable |
| 32-item, 5%-relative collateral | 128-item paired McNemar, pinned slices, grader id | phase 335/338 | all pre-335 passes are stale -> `needs re-gate` |
| Two reasoning tables, invalid-results table | one results table | this phase | removes conflicting numbers |

**Deprecated/outdated:** `CardStatus::Refused` label `refused` (spec label is `failed`); the `invalid_protocol` golden (the invalid table is removed from cards; the loader may still classify retired-protocol evidence as absent).

## Runtime State Inventory

Not a rename/migration phase. One runtime fact: published Hub repos for 1.7B v5, 4B v5, 8B v4 hold `hf_sync_manifest.json` with the old `gate_status` label and the old README hash; the card-only push path rewrites both (Q10). Nothing else is stored under a renamed string.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / Rust toolchain | per-crate tests | yes | repo pin 1.96 (`rust-version`) | - |
| `vox` binary | `vox check` sanity probes, diagram check | yes but old | 0.6.0+build.7529 (no `diagram` subcommand) | build a current `vox` once (outside this research) for diagram validation |
| cargo-hakari | workspace-hack diff | yes | 0.9.39 | nightly job |
| `jq`, `python3` | probes | yes | - | - |
| Network (Hugging Face) | push only | not needed for tests | - | tests never touch the network |
| GPU / model processes | none | must stay idle (0.6B eval running) | - | all work is file-level |

**Missing dependencies with no fallback:** none for planning. A current `vox` build is needed to validate the diagram fence and run `vox check`-style probes; the corpus function covers the Vox-block lint.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness via `cargo test` (nextest is the CI runner; same filters apply) |
| Config file | none for these tests; goldens use `VOX_UPDATE_GOLDEN=1` (any value) |
| Quick run command | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card` |
| Full suite command | `cargo test -p vox-populi --features mens-hf-hub --lib` (per-crate only; never workspace-wide) |
| Docs lint | `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/mens-models.md` |
| Edge check | `vox ci crate-edges` (installed binary) |

Run cargo in the background (project memory: never block foreground on cargo); one crate at a time.

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CARD-01 | gate state per fixture: passed / needs re-gate (stale pass, no pins) / failed / not evaluated / not trained | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::facts` | Wave 0 |
| CARD-01 | stand-in order and refusals (rule, alpha, pins, grader) | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::collateral_standin` | Wave 0 |
| CARD-02 | `RunRecords::load` unchanged behaviour (existing test ported), public paths still resolve for hub_sync | unit + compile | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::tests::hub_card_load` and `cargo check -p vox-populi --features mens-hf-hub` | port |
| CARD-03 | D2 pick, join, ambiguity -> "no recorded output", verdict strings, no-loss case | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::examples` | Wave 0 |
| CARD-04 | templates render, `<`/`&&` unescaped, byte-deterministic, host-free | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::render` | Wave 0 |
| CARD-05 | yaml: unknown size / missing blurb / extra size is an error; no digits in prose | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::config` | Wave 0 |
| CARD-06/07 | goldens (7 cards + index) | golden | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::tests::goldens` | Wave 0 |
| CARD-07 | every ` ```vox ` block passes `verify_vox_admission` | lint | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::lints::vox_blocks_compile` | Wave 0 |
| CARD-08 | doc lint clean; diagram valid | lint | docs command above; `vox --json diagram check -` (current build) | Wave 0 |
| CARD-09 | index grid, which-model rule, what's-next from records | unit/golden | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::index` | Wave 0 |
| CARD-10 | duplication, anchors, HF metadata | lint | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card::lints` | Wave 0 |
| CARD-12 | card-only staging rewrites README/MODEL_CARD manifest entries and gate label, skips NotTrained; push is human-gated | unit + checkpoint | `cargo test -p vox-populi --features mens-hf-hub --lib hub_cards_push` | Wave 0 |

### Mutation-proof list (each: record `grep -c` presence of the guarded code before, break it, run the named test and see FAILED, restore, repeat the grep and the test green; per AGENTS.md "verify by mutation")
1. Map stale pass to `passed` (stand-in returns `Ok` for a report without pins) -> `stale_pass_is_needs_re_gate` fails. (spec)
2. Drop the loss example (`loss = None` always) -> `loss_example_present_or_stated_none` fails. (spec)
3. Render the gate label a second time in a template -> `gate_state_stated_once` and the duplication lint fail. (spec)
4. Reverse the D2 sort (longest first) -> `wins_are_the_two_shortest_by_length_then_id` fails.
5. Accept an ambiguous preview match -> `ambiguous_join_renders_no_recorded_output` fails.
6. Switch a template to `escape = "html"` -> `vox_symbols_are_not_escaped` fails.
7. Remove the unknown-size check in `config.rs` -> `unknown_size_is_an_error` fails.
8. Rename a docs heading in the lint fixture -> `every_card_anchor_exists_in_mens_models` fails.
9. Put an unparseable body in a ` ```vox ` fence in a fixture -> `vox_blocks_compile` fails; and a failing output in a ` ```vox ` fence is caught.
10. Card-only staging forgets to rewrite the README hash -> `card_only_manifest_matches_staged_files` fails.
11. Show collateral numbers on a `needs re-gate` card -> `no_stale_numbers_without_current_rule` fails.

### Sampling Rate
- **Per task commit:** the single new module's filter (e.g. `... --lib hub_card::examples`), background.
- **Per wave merge:** `cargo test -p vox-populi --features mens-hf-hub --lib hub_card` plus `cargo check -p vox-ml-cli --features mens-hf-hub` once when `hub_sync`/CLI flag changes.
- **Phase gate:** full vox-populi `--lib` for the touched feature, `vox ci crate-edges`, doc lint, hosted CI green; then the human review checkpoint. Do not run `cargo clippy --workspace` locally (AGENTS.md).

### Wave 0 Gaps
- [ ] `crates/vox-populi/tests/fixtures/hub_cards/` fixture run dirs (state matrix + one real-schema ckpt sample + docs fixture) and a builder in `hub_card/tests/support.rs`
- [ ] `crates/vox-populi/tests/fixtures/hub_cards/hf-metadata-rules.v1.yaml`
- [ ] askama tracer (one template, one test) to confirm the dependency compiles on rust 1.96
- [ ] `verify_vox_admission` probe over the fixtures' adapter wins
- [ ] first-written failing tests for each CARD-ID (test-first policy)

## Security Domain

`security_enforcement` is absent in `.planning/config.json` (treated as enabled). This phase renders text from local records and uploads to the Hub.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (push) | Hub token only through `require_clavis_hf_token()` / `vox_secrets::resolve_secret` (existing; AGENTS.md Clavis rule); never read env directly |
| V3 Session Management | no | - |
| V4 Access Control | yes | gate checks stay fail-closed; card-only push must not bypass `NotTrained`/32B exclusion; never `--force-ungated` |
| V5 Input Validation | yes | records and ckpt are untrusted JSON (a pulled bundle): `serde_json` parse with typed fallbacks, yaml validated on load |
| V6 Cryptography | yes | BLAKE3 via `vox-crypto` only; no direct `blake3` |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Model output injected into markdown (links, HTML, front-matter terminators) | Tampering | model text only inside fenced blocks; fence length longer than any run of backticks in the text; no model text in frontmatter; `---` lines inside a fence cannot end frontmatter since frontmatter is rendered first |
| Host path / user name leak (`/Users/...`) in rendered text | Information disclosure | existing host-free test kept; fixtures contain no host paths; model text scanned by the same test |
| Stale gate state presented as passed | Spoofing/Repudiation | stand-in/shared check; mutation 1 |
| Card-only upload desynchronising manifest (integrity break) | Tampering | manifest rewrite + test; pull re-hash |
| Pushing the 32B card over the hand-written one | Tampering | explicit `NotTrained` skip + `ensure_ladder_repos` rule |
| Dependency supply chain (askama) | Tampering | already in lock; legitimacy check OK; no new version |

## Sources

### Primary (HIGH confidence)
- Local source files read this session: `crates/vox-populi/src/mens/hub_card.rs` (1-1102), `hub_card_tests.rs` (1-453), `hub_sync.rs` (120-360, 361-610, 676-757, 1030-1349), `crates/vox-populi/Cargo.toml`, `crates/vox-ml-cli/src/commands/mens/{hub.rs,item_checkpoint.rs,eval_local.rs,eval_local_prompt.rs,metrics.rs}`, `crates/vox-eval/src/{reasoning_slice.rs,qwen_reasoning.rs}`, `crates/vox-compiler/src/pipeline.rs`, `crates/vox-corpus/src/corpus/corpus_readiness.rs`, `crates/vox-cli-ci/src/crate_edges.rs`, `contracts/ci/crate-edges.allow.v1.json`, `docs/src/architecture/layers.toml`, `docs/agents/governance.md`, `docs/src/contributors/documentation-governance.md`, `docs/src/reference/vox-diagram.md`, `contracts/documentation/kb-taxonomy.v1.yaml`, `mens/config/{eval-gates-ladder.yaml,gpu-specs.yaml}`, `mens/data/heldout_bench/manifest.json`, `mens/runs/qwen3-*/` records, `.planning/phases/338-*/338-01,02,06-PLAN.md`, `.planning/phases/337-eval-throughput/337-RESEARCH.md`, spec `docs/superpowers/specs/2026-11-06-mens-hub-cards-v2-design.md`
- `~/.cargo/registry/src/*/askama_derive-0.12.5/src/{config.rs,input.rs}`, `askama-0.12.1/Cargo.toml`, `uniffi_bindgen-0.28.3/Cargo.toml`
- `gsd_run query package-legitimacy check --ecosystem crates askama askama_derive` -> OK, OK

### Secondary (MEDIUM confidence)
- https://huggingface.co/docs/hub/model-cards and https://raw.githubusercontent.com/huggingface/hub-docs/main/modelcard.md (fetched 2026-11-06) for metadata rules

### Tertiary (LOW confidence)
- Eval-time estimates (337 projections), hakari no-delta reasoning, askama compile on rust 1.96 (not built)

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | askama 0.12.1 compiles in the vox-populi graph on rust 1.96 / edition 2024 workspace with `default-features = false, features = ["config"]` | Q3 | tracer task fails; fall back to a newer askama (new lock entries) or user decision |
| A2 | Matching uniffi_bindgen's askama feature set yields no workspace-hack delta | Q3 | nightly `cargo hakari generate --diff` fails; fix by regenerating |
| A3 | Hub accepts the non-template `verified: false` key in `model-index` metrics (phase 336 pushed it) | Q5 | Hub warns on push; drop the key |
| A4 | `verify_vox_admission` (interp smoke) accepts every adapter win | Q4 | lint false-fails; fall back to receipt `stage` check |
| A5 | A folder upload of only README.md/MODEL_CARD.md/hf_sync_manifest.json leaves other repo files untouched (hf-hub `upload_folder` adds/overwrites, does not delete) | Q10 | card-only push could delete weights; verify in the card-only plan's first test against the hf-hub 1.0 API docs before any real push |
| A6 | `pipeline_tag: text-generation` and `library_name: vox` remain valid | Q5 | same as phase 336 behaviour |
| A7 | Starlight heading ids equal github-slugger output for ASCII headings | Q6 | link lint false-passes; verify once against the built site |
| A8 | 337's projected eval times are the best available estimate | Q9 | wrong ETA on the index; labelled low-confidence |

## Open Questions / Risks

1. **Message-level compiler verdicts** ("base: does not parse - `let mut` is not Vox") are not obtainable without recomputing parse detail, and the pipeline returns only an error count. Recommended: stored-field verdicts. If the user wants messages: add a small `first_diagnostic(source)` helper in vox-corpus (no new edge) or approve a vox-compiler edge. Needs a decision before CARD-03.
2. **"Loss" semantics.** Literal spec ("shortest item the adapter still fails") gives `humaneval_vox_042_reverse_list` on all four sizes even where the adapter did not break anything; a regression ("breaks") exists for 0.6B/4B/8B but not 1.7B. Recommend literal rule for D2 (as written and locked), and add the regression count to the totals line; ask whether a regression should take priority.
3. **Duplication-lint definition** (Pitfall 4) must be shown to the user: sentence splitter, exclusions, treatment of the status line and the rule sentence.
4. **Failed under a retired rule.** 338-01's check order returns `Failed` for any `status != pass` report, so 0.6B's current old-rule fail renders `failed`, which the spec defines as "current-rule". Keep 338-01's order (stand-in must match); the state is transient (re-gate running). Flag the wording on the card ("failed" text quotes the refusal).
5. **Card-only push is new code** and touches Hub integrity; it needs its own plan, tests, and a real-API check of A5 before the human push step. If the user prefers, ship cards v2 locally and push only the gated 0.6B through the existing path.
6. **Manifest `gate_status` lag.** Rewriting the label in the remote manifest during card-only push changes `classify_size_sync` outcomes; confirm the desired post-push `sync` output (`Diverged` vs `InSync`) with a test.
7. **Docs-before-Hub ordering** and the cards' links: the docs page must be merged and deployed before pushing.
8. **338-02 "Gate state by spoke" table** vs D5 (gate once): needs a decision.
9. **Spoke fact source** until 338-06: `adapter_card.json` has no lanes; fallback is a yaml-documented default (`vox-lang`) with its provenance printed, not a derivation. If the user rejects a default, the spoke reads "not recorded" for all sizes until 338-06.
10. **Eval-time estimates** come from 337 projections (LOW); replace after THRU-12.
11. Risk: three plans (338-01/02/06) must be edited before they run; if they have already started, 339 must wait.
12. Risk: workspace `unexpected_cfgs = deny` and `#[path]` handling when moving the test module; do the move in the first task.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (lock, registry source, legitimacy check) except A1/A2 (not compiled)
- Architecture and mapping: HIGH (read all callers)
- Examples / join: HIGH (measured on 8 real pairs)
- Push mechanics: HIGH on current behaviour, MEDIUM on the proposed card-only design (A5)
- Pitfalls: HIGH

**Research date:** 2026-11-06
**Valid until:** 2026-11-20 (338/335/337 are in flight and may change `hub_card`, the collateral report shape and `vox-eval`)
