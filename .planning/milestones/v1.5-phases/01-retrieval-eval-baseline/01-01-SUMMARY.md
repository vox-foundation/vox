---
phase: 01-retrieval-eval-baseline
plan: 01
subsystem: retrieval-eval
tags: [vox-search, eval, bm25, beir, ir-metrics, kb]
requires: []
provides:
  - "vox_search::kb_ids: KB identity SSOT (corpus roots, heading_path encoding, chunk ids, ancestor matching)"
  - "vox_search::kb_eval: run_eval + deterministic Report (doc + chunk Recall@5/10, MRR@10, nDCG@10, answer_in_budget)"
  - "kb_eval::golden: BEIR loader with split isolation, strict validation, canary reader, corpus snapshot builder"
  - "kb_eval::bm25: pinned-chunker in-memory BM25 baseline"
affects: [01-05, 01-06, 01-07, 01-10, 01-11, phase-02-chunker]
tech-stack:
  added: []
  patterns: ["retriever as a closure Fn(&str)->Vec<RankedChunk>", "ordered collections only", "quantized BM25 ranking (score*1e9 as i64)"]
key-files:
  created:
    - crates/vox-search/src/kb_ids.rs
    - crates/vox-search/src/kb_eval/mod.rs
    - crates/vox-search/src/kb_eval/metrics.rs
    - crates/vox-search/src/kb_eval/golden.rs
    - crates/vox-search/src/kb_eval/golden/validation_tests.rs
    - crates/vox-search/src/kb_eval/bm25.rs
    - crates/vox-search/tests/kb_eval_determinism.rs
    - crates/vox-search/tests/fixtures/kb_eval/
  modified:
    - crates/vox-search/src/lib.rs
    - crates/vox-search/src/ingest.rs
    - crates/vox-search/src/memory_hybrid.rs
key-decisions:
  - "heading_path = title > H2 > H3 joined by ' › ' (U+203A with spaces); H1 never a segment (AP-06)"
  - "run_eval takes the retriever's doc->chunk-id map as an explicit argument so it stays retriever-agnostic"
  - "golden validation tests live in golden/validation_tests.rs to keep golden.rs under the 500-line limit"
requirements-completed: [EVAL-01]
status: complete
duration: ~1h
completed: 2026-10-04
commits: 3
plan_head_before: 6cd05af6c103128745615d5440af692d5b43acfe
actuals:
  tokens: 17000
  tasks: 3
  commits: 3
---

# Phase 1 Plan 01: Retrieval-eval engine (tracer) Summary

Deterministic `vox-search` retrieval-eval engine: hand-written IR metrics, a strict BEIR golden-set loader with stored held-out split, a pinned-chunker BM25 baseline, and a byte-identical doc+chunk report including the answer-in-budget rate.

## What was built

- **Task 1 (tracer, `33a7aab0e`)**: fixture golden set through chunking, BM25 and metrics to a deterministic report. The hand-computed 3-query fixture yields exactly R@5=0.333333, R@10=0.666667, MRR@10=0.380952, nDCG@10=0.364507.
- **Task 2 (`436afcfe2`)**: answer-in-budget (B=1500, `ceil(bytes/4)`, prefix semantics), ancestor-or-equal chunk-qrel resolution, run dedupe, `dev/all` + `dev/<origin>` keys, `to_json_string`.
- **Task 3 (`400b13833`)**: strict validation (deny_unknown_fields, closed origin/intent, scores 0..=2, duplicate/unknown-id rejection, canary match, heading_path existence), `validate_splits`, `snapshot_corpus_jsonl`, `load_corpus`, `load_query_texts`.

## Public API for 01-05 / 01-06 / Phase 2

`kb_ids` (all `pub`):
```rust
pub const KB_CORPUS_ROOTS: &[&str] = &["docs/src"];
pub const KB_EXCLUDED_DIR_NAMES: &[&str] = &["archive"];
pub const HEADING_PATH_SEP: &str = " › ";
pub fn normalize_heading_text(raw: &str) -> String;
pub fn is_fence_line(line: &str) -> bool;
pub fn parse_heading(line: &str) -> Option<(usize, String)>;
pub fn doc_title(doc_path: &str, text: &str) -> String;
pub fn chunk_id(doc_path: &str, heading_path: &str) -> String;           // "doc#heading_path"
pub fn split_chunk_id(id: &str) -> Option<(&str, &str)>;                 // first '#'
pub fn heading_path_is_ancestor_or_equal(ancestor: &str, path: &str) -> bool;
pub fn is_kb_corpus_path(rel: &str) -> bool;
pub fn doc_heading_paths(doc_path: &str, text: &str) -> BTreeSet<String>;
```
`kb_eval`:
```rust
pub const RETRIEVER: &str = "bm25-inmem-v1"; pub const CHUNKER: &str = "md-h1h2-4096-v1";
pub struct RankedChunk { pub id: String, pub bytes: usize }
pub struct LevelMetrics { recall_at_5, recall_at_10, mrr_at_10, ndcg_at_10 }  // serde: recall@5 ...
pub struct Block { pub queries: usize, pub doc: LevelMetrics, pub chunk: LevelMetrics, pub answer_in_budget: f64 }
pub struct Report { split, retriever, chunker, budget_tokens, token_estimator, queries, keys: BTreeMap<String, Block> }
impl Report { pub fn render_text(&self) -> String; pub fn to_json_string(&self) -> anyhow::Result<String>; }
pub fn run_eval<F: Fn(&str) -> Vec<RankedChunk>>(golden: &GoldenSet,
        doc_chunks: &BTreeMap<String, BTreeSet<String>>, retrieve: F) -> anyhow::Result<Report>;
```
`kb_eval::metrics`: `recall_at_k`, `mrr_at_k`, `ndcg_at_k`, `answer_in_budget`, `round6`, `BUDGET_TOKENS=1500`, `TOKEN_ESTIMATOR="bytes/4"`, `type Qrels = BTreeMap<String,u8>`.
`kb_eval::bm25`: `chunk_doc(doc_path, text) -> Vec<Chunk>`, `Bm25Index::{build(&[CorpusDoc]), rank(&str) -> Vec<RankedChunk>, doc_chunk_ids() -> BTreeMap<String, BTreeSet<String>>}`.
`kb_eval::golden`: `Split{Dev,Test}`, `Origin{Human,LlmExpansion}`, `Intent`, `CorpusDoc`, `QueryMeta`, `QueryLine`, `GoldenSet{split,corpus,queries,doc_qrels,chunk_qrels,canary}` with `GoldenSet::load(dir, split)`, `SplitSummary{dev,test}`, `read_canary_guid(eval_dir)`, `validate_splits(dir)`, `snapshot_corpus_jsonl(root, &[String])`, `load_corpus(dir)`, `load_query_texts(dir)`.

Typical wiring: `let idx = Bm25Index::build(&golden.corpus); run_eval(&golden, &idx.doc_chunk_ids(), |q| idx.rank(q))`.

## heading_path encoding (for the Phase 2 planner)

`heading_path` = normalized heading display texts joined by `" › "`; the root is always the doc title (frontmatter `title`, else first H1, else file stem); H1 is never a segment. Normalization strips ATX markers and backticks and collapses whitespace. Chunk id = `doc_path#heading_path`. Frontmatter and fenced code are skipped when enumerating headings. A chunk qrel resolves to the deepest indexed chunk whose heading_path is an ancestor-or-equal, so finer Phase 2 chunks are an exact-match no-op. Phase 2 MUST call `kb_ids` rather than re-derive the encoding. The Phase 1 chunker splits only at `# `/`## ` lines; a `## ` inside a fence splits the chunk but the new chunk inherits the previous heading_path.

## Verification

- `cargo nextest run -p vox-search --lib kb_`: 25 passed. `--test kb_eval_determinism`: 2 passed.
- `cargo clippy -p vox-search --all-targets -- -D warnings`: clean.
- Mutation proofs (with before/after grep presence checks): nDCG discount `log2(rank+1)` changed to `log2(rank+2)` makes `hand_fixture_three_queries` fail, restored and green; making `GoldenSet::load(Dev)` also open `qrels/test.tsv` makes `dev_load_never_opens_test_qrels` fail, restored and green.
- Acceptance greps: `pub mod kb_ids;`/`pub mod kb_eval;` present; `DEFAULT_MARKDOWN_CHUNK_BYTES` used in bm25.rs; no `HashMap`/`HashSet` in kb_eval or kb_ids; `mod common;` is line 1 of the integration test; `HEADING_PATH_SEP` and `BUDGET_TOKENS: usize = 1500` defined; `deny_unknown_fields` count 3; all files under 500 non-blank lines (largest: mod.rs 365, golden.rs 415).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] tdd-guard pre-commit hook blocked the Task 1 commit**
- **Found during:** Task 1 commit
- **Issue:** `skeleton/untested-pub-api` warned on `Split::as_str` (golden.rs) and `Report::render_text` (mod.rs), which had no tests in their files.
- **Fix:** added same-file tests (`split_names_are_stable`, `loads_checked_in_fixture`, `render_text_uses_fixed_four_decimals`) before committing.
- **Commit:** 33a7aab0e

**2. [Rule 3 - Blocking] golden.rs exceeded the 500 non-blank-line limit after rustfmt (655)**
- **Fix:** moved the validation tests to `crates/vox-search/src/kb_eval/golden/validation_tests.rs` (new file, `#[cfg(test)] mod validation_tests;`), keeping a small inline test module so tdd-guard still sees same-file tests.
- **Commit:** 400b13833

### Notes

- TDD ordering: Task 2 had an observed RED (compile failure on the new `run_eval` signature and `answer_in_budget` field) before implementation. Tasks 1 and 3 had tests and implementation written together and were proven meaningful afterwards by the two mutation checks above; no separate `test(...)` RED commits were made, since a red commit would break the crate build and the pre-commit hook. The plan is `type: execute` (not `type: tdd`), so no plan-level gate compliance section applies.
- `answer_in_budget` and its constants landed in `metrics.rs` in the Task 1 commit (formula module written once); the report wiring, resolution and dedupe are Task 2.
- `GoldenSet` gained a `canary: String` field (the plan says the loader returns the GUID it read).
- Fixture covers 3 dev queries (2 human, 1 llm-expansion) over 3 docs; the held-out/test split is tested on tempdirs, as planned.

## Known Stubs

None.

## Threat Flags

None. No new network, auth or file-write surface; mitigations T-01-01..T-01-05 implemented (strict loader, GUID only via `read_canary_guid`, Dev never opens test qrels, `is_kb_corpus_path`, ordered/quantized determinism).

## Self-Check: PASSED

- FOUND: crates/vox-search/src/kb_ids.rs, kb_eval/{mod,metrics,golden,bm25}.rs, kb_eval/golden/validation_tests.rs, tests/kb_eval_determinism.rs, tests/fixtures/kb_eval/
- FOUND commits: 33a7aab0e, 436afcfe2, 400b13833
