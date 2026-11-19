---
phase: 2
phase_name: "library-index-and-kb-tool-surface"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 48
  lessons: 31
  patterns: 25
  surprises: 20
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 2 Learnings: library-index-and-kb-tool-surface

## Decisions

### schema:approve-as-proposed (trust tier = writer parameter + kb_index_sta...
schema:approve-as-proposed (trust tier = writer parameter + kb_index_state.trust_tier, no per-row trust column)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-01-SUMMARY.md

---

### feature:vox-search-default (tantivy-lexical added to vox-search default ...
feature:vox-search-default (tantivy-lexical added to vox-search default features)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-01-SUMMARY.md

---

### Schema 95 (Task 2)
**Schema 95 (Task 2).** `BASELINE_VERSION` is now 95, and `baseline-version-policy.yaml` carries the new digest `0xaaa2601b...a9130`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-01-SUMMARY.md

---

### search_documents gained corpus, rel_path, frontmatter_json, indexed_comm...
`search_documents` gained `corpus`, `rel_path`, `frontmatter_json`, `indexed_commit` and `index_error`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-01-SUMMARY.md

---

### search_document_chunks gained chunk_hash, chunk_kind, heading_path, anch...
`search_document_chunks` gained `chunk_hash`, `chunk_kind`, `heading_path`, `anchor` and `token_estimate`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-01-SUMMARY.md

---

### kb_index_state is new.
`kb_index_state` is new.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-01-SUMMARY.md

---

### open.rs adds the columns to a pre-95 database through a PRAGMA-guarded l...
`open.rs` adds the columns to a pre-95 database through a PRAGMA-guarded loop, then creates `idx_search_documents_corpus`, the UNIQUE `idx_search_chunks_doc_hash` and `idx_search_chunks_hash`. These are not in the baseline text, because a v94 database lacks the columns when that text runs.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-01-SUMMARY.md

---

### Card heading_path is the root path (doc title), AP-23, so rel_path#Title...
Card heading_path is the root path (doc title), AP-23, so rel_path#Title always resolves and the parity set always contains the root

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-03-SUMMARY.md

---

### Card first paragraph = first prose paragraph before any H2+ (preamble an...
Card first paragraph = first prose paragraph before any H2+ (preamble and H1 intro), capped at 1024 bytes

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-03-SUMMARY.md

---

### Anchors are derived from the section's own heading line via kb_ids::pars...
Anchors are derived from the section's own heading line via kb_ids::parse_heading, not the last heading_path segment, so H1 sections get their real slug

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-03-SUMMARY.md

---

### Card (IDX-03 card half)
**Card (IDX-03 card half).** `chunk_document` emits exactly one `ChunkKind::Card` at index 0 with `heading_path` = `kb_ids::doc_title`. Body is title, frontmatter `description`, the first intro paragraph, then `category: …` / `status: …` lines when present. No raw YAML ever enters a body. The card goes through the same hash and `#n` ordinal map as sections.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-03-SUMMARY.md

---

### Anchors. github_slug(heading_text, &mut seen) follows github-slugger: lo...
**Anchors.** `github_slug(heading_text, &mut seen)` follows github-slugger: lowercase, drop everything but letters/digits/`-`/`_`/space, each space to `-`, repeats get `-1`, `-2`. Every heading consumes a slug in document order, including headings whose section body is otherwise skipped. The card has no anchor. Three real headings from `search-retrieval-ssot-2026.md` are pinned in a unit test (`3-two-distinct-voxdb-surfaces-do-not-conflate`, `31-rust-vox-db-turso--libsql`, `language-surface-decision`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-03-SUMMARY.md

---

### Oversize split (IDX-02)
**Oversize split (IDX-02).** Sections over `SECTION_TOKEN_BUDGET` (512) go through `MarkdownSplitter::new(ChunkConfig::new(256..512).with_sizer(EstimatedTokens))`, overlap 0. `EstimatedTokens` calls `estimate_tokens`, the same estimator as Phase 1's answer-in-budget; no model tokenizer. `CHUNKER_VERSION` is now `kb_ids-sections-v2+card+est4b+md512`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-03-SUMMARY.md

---

### Walker (IDX-04)
**Walker (IDX-04).** `corpus.rs` already carried `follow_links(false)`, the symlink skip and `kb_ids::is_kb_corpus_path` as the sole authority (02-01). This plan adds the tests and the mutation proof; no code change was needed there.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-03-SUMMARY.md

---

### Hash skip requires a stored kb_index_state whose chunker_version equals ...
Hash skip requires a stored kb_index_state whose chunker_version equals CHUNKER_VERSION; a missing manifest forces a full re-chunk (a pre-02-04 DB has rows of unknown provenance)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-04-SUMMARY.md

---

### Trust tier is validated at the top of index_library, because skipped fil...
Trust tier is validated at the top of index_library, because skipped files never reach the store's own validation

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-04-SUMMARY.md

---

### indexed_at is assigned by the store (datetime('now')); the value on KbIn...
indexed_at is assigned by the store (datetime('now')); the value on KbIndexState passed to put is ignored

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-04-SUMMARY.md

---

### library_document_by_path takes no corpus argument and is hard-filtered t...
library_document_by_path takes no corpus argument and is hard-filtered to the library corpus; library_chunk_by_hash likewise

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-04-SUMMARY.md

---

### File-hash skip (IDX-01)
**File-hash skip (IDX-01).** `index_library` hashes the normalized text before chunking and, when it equals `VoxDb::library_document_file_hash`, counts the file in `IndexReport.docs_unchanged` and does no DB writes for it. A changed `CHUNKER_VERSION` in the stored manifest (or no manifest) disables the skip.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-04-SUMMARY.md

---

### Identity-level proof
**Identity-level proof.** `kb_reindex_unchanged` (3 docs unchanged, snapshot of every `(rowid, chunk_hash)` identical), `kb_reindex_one_section` (exactly 1 delete + 1 insert, every other pair survives, docs A and C untouched), `kb_reindex_insert_middle_section` (1 insert, 0 deletes, `chunk_index` contiguous and in document order) and `kb_kept_hash_keeps_vector`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-04-SUMMARY.md

---

### Prune. prune_library_documents_not_in deletes chunk rows explicitly and ...
**Prune.** `prune_library_documents_not_in` deletes chunk rows explicitly and then the doc row, in one transaction, only inside the requested corpus. `prune_orphan_kb_chunk_embeddings` deletes `kb_chunk` vectors whose `source_id` is no longer any `chunk_hash`; other `source_type`s are untouched. The indexer passes every walked path as the keep list.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-04-SUMMARY.md

---

### Manifest (IDX-03, IDX-07)
**Manifest (IDX-03, IDX-07).** `put_kb_index_state` (upsert) / `get_kb_index_state`. Each run records corpus root and commit, doc/chunk/card counts, chunker version, `lexical_backend = "tantivy-0.22"`, trust tier, NULL embed fields (AP-07), `errors_json` and duration. Doc count is distinct docs with chunks; cards are `chunk_kind = 'card'` rows.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-04-SUMMARY.md

---

### Status. library::status(db, lexical_dir) returns LibraryStatus { state, ...
**Status.** `library::status(db, lexical_dir)` returns `LibraryStatus { state, db_chunk_count, lexical_docs, consistent }`. `consistent` needs a manifest at the current chunker version and Tantivy `num_docs == DB chunk count`. Deleting the Tantivy directory flips it to false; the next run rebuilds it with no doc changes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-04-SUMMARY.md

---

### Kept SecretId::VoxSearchTantivyRoot (AP-14)
Kept SecretId::VoxSearchTantivyRoot (AP-14): library/lexical.rs still resolves it for the Library index root

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-05-SUMMARY.md

---

### Left SEARCH_CORPUS_BUCKET_COUNT untouched
Left SEARCH_CORPUS_BUCKET_COUNT untouched: changing it would shift citation_coverage scores, out of scope for a pure dead-path removal

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-05-SUMMARY.md

---

### validate_doc_path adds only a 512-byte cap on top of kb_ids::is_kb_corpu...
validate_doc_path adds only a 512-byte cap on top of kb_ids::is_kb_corpus_path, which already rejects backslash, colon, empty and dot components, traversal and archive; duplicating those checks would be dead code

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-06-SUMMARY.md

---

### vox_kb_reindex tolerates the dispatch meta keys (agent_id, session_id, t...
vox_kb_reindex tolerates the dispatch meta keys (agent_id, session_id, trace_id, correlation_id, task_id) and rejects every other key, instead of a bare deny_unknown_fields

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-06-SUMMARY.md

---

### Handlers pull the repo root from state.repository.root, never from tool ...
Handlers pull the repo root from state.repository.root, never from tool input

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-06-SUMMARY.md

---

### Catalog and registry
**Catalog and registry.** Five rows (`kb.search`, `kb.get`, `kb.cite`, `kb.status`, `kb.reindex`) copy the `kb.query` shape and use new ids, so the 12 legacy `vox_kb_*` tools are untouched (AP-08). `kb.reindex` is `side_effect_class: writes_files`, `http_read_role_eligible: false`. Registries (`tool-registry.canonical.yaml`, `capability-registry.yaml`, `command-registry.yaml`) and `operations-catalog-inventory.v1.json` came from `vox ci operations-sync` / `operations-verify`, not hand edits. The read-role list gained `vox_kb_cite`, `vox_kb_get`, `vox_kb_search`, `vox_kb_status` and not `vox_kb_reindex`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-06-SUMMARY.md

---

### kb_tools/library.rs
**`kb_tools/library.rs`.** Self-contained (no import from `handlers.rs` / `params.rs`, which Phase 3 deletes).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-06-SUMMARY.md

---

### kb_search: trims the query and rejects empty input or more than 1024 byt...
`kb_search`: trims the query and rejects empty input or more than 1024 bytes (no truncation). `limit` defaults to 10 and clamps to 1..=50. A missing DB or index returns an error envelope with a remediation, never "Unknown tool".

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-06-SUMMARY.md

---

### kb_get: exactly one of path or chunk_hash. By path it returns doc metada...
`kb_get`: exactly one of `path` or `chunk_hash`. By path it returns doc metadata, the chunk list and the markdown rebuilt from section bodies in `chunk_index` order (card excluded). By hash it returns the chunk with its body. An unindexed but valid key returns `not indexed`; an invalid key returns `invalid path` / `invalid chunk_hash`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-06-SUMMARY.md

---

### kb_cite: returns link (doc_path#anchor, or just the path when there is n...
`kb_cite`: returns `link` (`doc_path#anchor`, or just the path when there is no anchor), `citation` (`[heading_path](link)`), `kb://chunk/<hash>` and `indexed_commit`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-06-SUMMARY.md

---

### Resource names for concrete kb://doc entries are the rel_path (no per-fi...
Resource names for concrete kb://doc entries are the rel_path (no per-file title read); the stdio process stays DB-free and file-read-free

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-07-SUMMARY.md

---

### Parse errors map to invalid_params and daemon/tool errors to internal_er...
Parse errors map to invalid_params and daemon/tool errors to internal_error by parsing in read_resource before the daemon call, so read_kb_resource keeps the plan's Result<(String,String),String> shape

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-07-SUMMARY.md

---

### kb_resources.rs
`kb_resources.rs`: `parse_kb_uri` reuses `validate_doc_path` / `validate_chunk_hash` from 02-06. `read_kb_resource` parses first, then calls `call_tool_via_daemon(daemon, "vox_kb_get", {path | chunk_hash})`, and unwraps the envelope (`data.markdown` for docs, `data.chunk.body` for chunks; an error envelope becomes `Err(message)`, so an unindexed path reports `not indexed` from the daemon). `kb_resource_templates` returns the two templates, `list_kb_doc_resources` maps `walk_corpus` to `kb://doc/<rel_path>`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-07-SUMMARY.md

---

### server.rs: read_resource gets a kb:// branch before the workspace branch...
`server.rs`: `read_resource` gets a `kb://` branch before the workspace branch (parse error -> `invalid_params`, daemon/tool error -> `internal_error`); new `list_resource_templates`; `list_resources` appends the concrete kb://doc entries (AP-17).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-07-SUMMARY.md

---

### kb_resources_and_tools_via_daemon (in stdio_daemon_routing_tests.rs, #[s...
`kb_resources_and_tools_via_daemon` (in `stdio_daemon_routing_tests.rs`, `#[serial]`, reusing the file's `IsolatedHomeEnv` / `wait_ready` and the token-file daemon setup): indexes a temp corpus into the daemon's DB with the vox-crypto hasher into `default_index_dir()` under `VOX_SEARCH_TANTIVY_ROOT`, then asserts `vox_kb_status` is `indexed: true`, `vox_kb_search("turbofrobnicate")` returns the `Guide > Install` hit, `kb://chunk/<hash>` returns `text/markdown` with the body, `kb://doc/docs/src/guide.md` returns markdown containing `## Install`, and an unindexed valid path returns an error mentioning `not indexed`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-07-SUMMARY.md

---

### kb::run became async (Cli::Kb dispatch awaits it); the eval arm stays sy...
kb::run became async (Cli::Kb dispatch awaits it); the eval arm stays synchronous inside it

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-08-SUMMARY.md

---

### FileError entries are rendered from path (the real field) with rel_path ...
FileError entries are rendered from `path` (the real field) with `rel_path` accepted as a fallback, since the plan text said rel_path but the serialized struct uses path

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-08-SUMMARY.md

---

### handler_rust is null on the two new catalog rows as the plan specifies (...
handler_rust is null on the two new catalog rows as the plan specifies (kb.eval keeps commands::kb)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-08-SUMMARY.md

---

### daemon_tool.rs
**`daemon_tool.rs`.** `daemon_client()` and `tool_call()` moved out of `dei.rs` (which is `cfg(feature = "dei")`) into a non-gated `pub(crate) mod daemon_tool`, with the envelope match factored into `unwrap_tool_envelope` (success returns `data`, failure bails with the error text, a value without `success` passes through). `dei.rs` imports them; its rationale comments moved with the helper.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-08-SUMMARY.md

---

### kb.rs. KbCmd::Status { json } and KbCmd::Reindex { json } beside Eval. B...
**`kb.rs`.** `KbCmd::Status { json }` and `KbCmd::Reindex { json }` beside `Eval`. Both call one `daemon_report(tool, json, render)`: `daemon_client` then `tool_call("vox_kb_status" | "vox_kb_reindex", {})`, then pretty JSON or the rendered report. An "already running" or any other tool error propagates through `anyhow` as a non-zero exit with the message as-is. `render_kb_status` prints last-indexed commit, corpus root, doc/chunk/card counts, chunker version, lexical backend, embed model, trust tier, index dir, error count with one line per file, and consistency (a failing check names both counts and points at `vox kb reindex`); an unindexed library prints "not indexed" plus `vox kb reindex`. `render_index_report` prints docs seen/unchanged/changed/pruned, chunks inserted/deleted/kept, cards, corpus commit, lexical backend, duration and the per-file errors. `reindex` takes no path arguments, so `vox kb reindex docs/src` is a clap error.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-08-SUMMARY.md

---

### Catalog. kb.status and kb.reindex gained cli blocks ([kb, status], [kb, ...
**Catalog.** `kb.status` and `kb.reindex` gained cli blocks (`[kb, status]`, `[kb, reindex]`, active, `ref_cli_required: false`). `vox ci operations-sync --target all --write` regenerated `command-registry.yaml`, `capability-registry.yaml` and the tool registry; `operations-verify` reports 1305 rows OK.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-08-SUMMARY.md

---

### Docs. where-things-live.md: new rows for the Library index, the vox_kb_*...
**Docs.** `where-things-live.md`: new rows for the Library index, the `vox_kb_*` tools and `kb://` resources, and `docs/src/library/`; the existing `vox kb eval` row now reads `vox kb eval|status|reindex` (no duplicate). `search-retrieval-ssot-2026.md`: the "Docs mirror" row is replaced by the KB Library corpus row, the RAG chunk row now says FTS5 is unavailable on turso 0.6.1 and LIKE is what runs, the five tools join the section 6 table, and a line notes the `kb://` resources and the daemon restart after upgrade. `CHANGELOG.md` `[Unreleased]`: Added (tools, resources, CLI, Library category, schema baseline 95), Removed (the Tantivy docs mirror, `SearchExecution::tantivy_doc_lines`, `SearchPolicy::tantivy_index_root`), Notes (restart the daemon).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 02-08-SUMMARY.md

---

### Retuned the Library lexical title/heading boosts from 2.0/1.5 to 0.25/0....
Retuned the Library lexical title/heading boosts from 2.0/1.5 to 0.25/0.25 (Rule 1) instead of stopping on the first-run regression; swept on the dev split only, held-out test split never scored

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-09-SUMMARY.md

---

### Deepened dev chunk qrels q046 and x041 to › 2.1 Full Stack Layers once, ...
Deepened dev chunk qrels q046 and x041 to `› 2.1 Full Stack Layers` once, under the Phase 1 re-epoch (epoch 2); 19 candidates reviewed, q001/x001 flagged and deliberately left as-is

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-09-SUMMARY.md

---

### Re-epoched rather than tightened
Re-epoched rather than tightened: the retriever and chunker fields now name the Library index and a qrel diff requires an epoch bump

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-09-SUMMARY.md

## Lessons

### 1. [Rule 1 - Bug] Clippy lints on new test code
**1. [Rule 1 - Bug] Clippy lints on new test code**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-01-SUMMARY.md

---

### Found during
**Found during:** Task 3 verification (`cargo clippy --all-targets -D warnings`)

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-01-SUMMARY.md

---

### Issue: cloned_ref_to_slice_refs in the Task 2 library tests, and asserti...
**Issue:** `cloned_ref_to_slice_refs` in the Task 2 library tests, and `assertions_on_constants` on the `cfg!` guard test.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-01-SUMMARY.md

---

### Fix: Used std::slice::from_ref, and added a scoped #[allow(clippy::asser...
**Fix:** Used `std::slice::from_ref`, and added a scoped `#[allow(clippy::assertions_on_constants)]` on the guard test, because the constant is the assertion.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-01-SUMMARY.md

---

### None in substance
None in substance. The plan's `lint.rs` edits spanned both tasks, so the parity test was temporarily held back from the Task 1 commit to keep the two commits atomic.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-02-SUMMARY.md

---

### 1. [Rule 3 - Blocking] cargo update -p text-splitter is impossible befor...
**1. [Rule 3 - Blocking] `cargo update -p text-splitter` is impossible before the crate is in the lockfile**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-03-SUMMARY.md

---

### Issue: error: package ID specification 'text-splitter' did not match any...
**Issue:** `error: package ID specification 'text-splitter' did not match any packages`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-03-SUMMARY.md

---

### Fix: cargo fetch resolved and locked it. This also bumped the ICU family...
**Fix:** `cargo fetch` resolved and locked it. This also bumped the ICU family (`icu_provider` 2.2 to 2.3 etc., `icu_segmenter` 2.3, `itertools` 0.15, `auto_enums`, `derive_utils`, `core_maths`) exactly as 02-RESEARCH predicted. Licenses are within `deny.toml`; `cargo deny` is not installed locally, hosted CI `linux` runs it on dependency changes.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-03-SUMMARY.md

---

### 1. [Rule 1 - Bug] Bad trust tier no longer rejected once unchanged files...
**1. [Rule 1 - Bug] Bad trust tier no longer rejected once unchanged files skip the store**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-04-SUMMARY.md

---

### Issue: the store was the only trust-tier validator; a skipped file never...
**Issue:** the store was the only trust-tier validator; a skipped file never reaches it.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-04-SUMMARY.md

---

### Fix: vox_db::validate_trust_tier_wire(deps.trust_tier)? at the top of in...
**Fix:** `vox_db::validate_trust_tier_wire(deps.trust_tier)?` at the top of `index_library`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-04-SUMMARY.md

---

### 1. Plan shape adjustment, validate_doc_path is not a separate layered ch...
**1. Plan shape adjustment, `validate_doc_path` is not a separate layered check**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-06-SUMMARY.md

---

### The plan asks to layer length, backslash, colon, and empty or
The plan asks to layer length, backslash, colon, and empty or `.` component checks on top of `is_kb_corpus_path`. That function already rejects all of them (and `#`, archive, and absolute paths), so only the 512-byte cap was added. The mutation therefore targets the `..` rule in `kb_ids.rs`, which was restored immediately.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-06-SUMMARY.md

---

### 2. Plan shape adjustment, vox_kb_reindex argument rejection
**2. Plan shape adjustment, `vox_kb_reindex` argument rejection**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-06-SUMMARY.md

---

### A plain deny_unknown_fields struct would also reject agent_id / session_...
A plain `deny_unknown_fields` struct would also reject `agent_id` / `session_id` / `trace_id` / `correlation_id` / `task_id`, which the dispatch layer reads from `args` for any tool. `reject_reindex_args` allows exactly those meta keys and rejects everything else. No root or path parameter exists. A rejected call returns `Err` from dispatch, like a failed `serde_json::from_value` on the other arms.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-06-SUMMARY.md

---

### 1. [Rule 3 - Blocking] sql-surface-guard failure — assigned by the orche...
**1. [Rule 3 - Blocking] sql-surface-guard failure** — assigned by the orchestrator, described above. Commit `f691aa05b`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-07-SUMMARY.md

---

### Otherwise the plan ran as written
Otherwise the plan ran as written. In Task 1 the plan's "red first" step was folded into one pass because the test and module only compile together; the test was written before the implementation and passed on first run. The in-file test count for the plan's `fewer than 3 tests` gate was met (4 + 3).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-07-SUMMARY.md

---

### 1. [Rule 3 - Blocking] cli-command-surface.generated.md regenerated and ...
**1. [Rule 3 - Blocking] `cli-command-surface.generated.md` regenerated and committed**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-08-SUMMARY.md

---

### Fix: ran the generator (vox ci command-sync --write, not a hand edit) an...
**Fix:** ran the generator (`vox ci command-sync --write`, not a hand edit) and committed its output. The verify gate is satisfied; ssot-autoregen will find no drift.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-08-SUMMARY.md

---

### Commit: ed33ab6d5
**Commit:** ed33ab6d5

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-08-SUMMARY.md

---

### 1. [Rule 1 - Bug] Lexical title/heading boosts retuned after a first-run...
**1. [Rule 1 - Bug] Lexical title/heading boosts retuned after a first-run regression**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-09-SUMMARY.md

---

### Issue: with boosts 2.0/1.5 every doc-level metric fell below the epoch-1...
**Issue:** with boosts 2.0/1.5 every doc-level metric fell below the epoch-1 baseline (doc recall@5 0.3421 vs 0.4697), which would fail the plan's non-regression truth.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-09-SUMMARY.md

---

### Fix: swept the title/heading boosts on the dev split only and settled on...
**Fix:** swept the title/heading boosts on the dev split only and settled on 0.25/0.25. The held-out test split was never scored.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-09-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Gateway test moved out of http_gateway_tests.rs
**1. [Rule 3 - Blocking] Gateway test moved out of `http_gateway_tests.rs`**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-11-SUMMARY.md

---

### Issue: crates/vox-orchestrator-mcp/src/http_gateway_tests.rs has no mod ...
**Issue:** `crates/vox-orchestrator-mcp/src/http_gateway_tests.rs` has no `mod` declaration anywhere, so it is never compiled; `metadata_read_role_eligible_contains_expected_tools` in it never runs. A test added there would be a green-by-absence.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-11-SUMMARY.md

---

### Fix: Added metadata_read_role_excludes_vox_kb_status to the live mod tes...
**Fix:** Added `metadata_read_role_excludes_vox_kb_status` to the live `mod tests` in `src/http_gateway/mod.rs` (where `metadata_read_role_eligible_tools` is in scope). The dead file is left unchanged. The plan's artifact `http_gateway_tests.rs` therefore does not contain the test.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-11-SUMMARY.md

---

### 1. [Plan inconsistency, no code impact] Acceptance grep grep -c 'fn real...
**1. [Plan inconsistency, no code impact] Acceptance grep `grep -c 'fn real_\|fn degraded_' kb_eval_golden_contract.rs` prints 1, not 0.** The match is the retained `fn real_dir()` helper, which the five golden-set contract tests use and which the plan itself says stays. All three ratchet tests, `real_baseline`, `real_report` and the BM25 import are gone from that file (`kb_eval::bm25` count 0).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-12-SUMMARY.md

---

### 2. [Formatting] rustfmt expanded the build_blocking(...) call in the lib...
**2. [Formatting] rustfmt expanded the `build_blocking(...)` call in the library.rs unit test onto multiple lines** after the `from_ref` swap (10-line diff instead of one line); still test-only code.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-12-SUMMARY.md

---

### Otherwise: plan executed as written. No auth gates, no stubs, no new thr...
Otherwise: plan executed as written. No auth gates, no stubs, no new threat surface.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-12-SUMMARY.md

---

### Both gaps from the previous report are closed, verified by re-running th...
Both gaps from the previous report are closed, verified by re-running the checks myself, not from the 02-12 SUMMARY. The Phase-1 real-data ratchet now scores the production `LibraryRetriever` against the epoch-2 baseline and is mutation-proven (per 02-12; the test bodies directly exercise the guarded state). `cargo clippy -p vox-search --all-targets -- -D warnings` exits 0. The baseline, ratchet logic and lexical boosts are byte-identical to `eb47f2888`, so the ratchet was retargeted, not loosened. All five ROADMAP success criteria still hold; no regressions in the prune, chunking, MCP, or eval suites.

**Context:** Verified in phase acceptance checks.
**Source:** 02-VERIFICATION.md

---

### None outstanding (02-01 and 02-09 checkpoints recorded approved).
None outstanding (02-01 and 02-09 checkpoints recorded approved).

**Context:** Verified in phase acceptance checks.
**Source:** 02-VERIFICATION.md

## Patterns

### PRAGMA-guarded ALTER loop, with column-dependent indexes created after t...
PRAGMA-guarded ALTER loop, with column-dependent indexes created after the ALTERs in open.rs

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-01-SUMMARY.md

---

### Hash diff with two-pass renumbering through negative temporaries (UNIQUE...
Hash diff with two-pass renumbering through negative temporaries (UNIQUE(document_id, chunk_index))

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-01-SUMMARY.md

---

### Content hasher injected via IndexDeps (vox-crypto is a dev-dependency on...
Content hasher injected via IndexDeps (vox-crypto is a dev-dependency only, so no crate edge)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-01-SUMMARY.md

---

### Tantivy rebuilt wholesale from DB rows into a sibling temp dir, then swa...
Tantivy rebuilt wholesale from DB rows into a sibling temp dir, then swapped in

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-01-SUMMARY.md

---

### vocabulary parity test reading the contract JSON via CARGO_MANIFEST_DIR
vocabulary parity test reading the contract JSON via CARGO_MANIFEST_DIR

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-02-SUMMARY.md

---

### Chunk identity only from kb_ids; chunk.rs never parses headings or joins...
Chunk identity only from kb_ids; chunk.rs never parses headings or joins paths itself

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-03-SUMMARY.md

---

### Parts of an oversize section share heading_path, anchor and so the kb_id...
Parts of an oversize section share heading_path, anchor and so the kb_ids chunk id

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-03-SUMMARY.md

---

### Hash skip is sound only with a manifest whose chunker_version matches; n...
Hash skip is sound only with a manifest whose chunker_version matches; no manifest means re-chunk everything

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-04-SUMMARY.md

---

### Prune keeps every walked path, errored files included, so a transient re...
Prune keeps every walked path, errored files included, so a transient read error never deletes a doc

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-04-SUMMARY.md

---

### Tantivy is rebuilt from DB rows on any change, prune, or count mismatch ...
Tantivy is rebuilt from DB rows on any change, prune, or count mismatch (AP-16)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-04-SUMMARY.md

---

### Runtime source-scan guard test (CARGO_MANIFEST_DIR + read_to_string) lis...
Runtime source-scan guard test (CARGO_MANIFEST_DIR + read_to_string) listing banned symbols per file

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-05-SUMMARY.md

---

### Catalog row, then vox ci operations-sync --target all --write, then disp...
Catalog row, then vox ci operations-sync --target all --write, then dispatch/input_schemas arms; generated registries never hand-edited

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-06-SUMMARY.md

---

### Source-scan guard (include_str! of the handler file, production half onl...
Source-scan guard (include_str! of the handler file, production half only) to prove no file-system API is reachable from tool handlers

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-06-SUMMARY.md

---

### No-parameter write tool
No-parameter write tool: reject every key except dispatch meta keys before running

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-06-SUMMARY.md

---

### Resource reads reuse the tool path
Resource reads reuse the tool path: validate URI, call_tool_via_daemon(vox_kb_get), unwrap the ToolResult envelope

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-07-SUMMARY.md

---

### Resource listings derive only from walk_corpus, never from request input
Resource listings derive only from walk_corpus, never from request input

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-07-SUMMARY.md

---

### Test DB inspection goes through VoxDb methods so tests stay outside the ...
Test DB inspection goes through VoxDb methods so tests stay outside the sql-surface-guard allowlist

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-07-SUMMARY.md

---

### CLI never opens a second index writer
CLI never opens a second index writer: status and reindex go through orch.tool_call to the daemon's single-flight handler

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-08-SUMMARY.md

---

### Pure render_* functions over the tool's JSON payload, unit-tested withou...
Pure render_* functions over the tool's JSON payload, unit-tested without a daemon

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-08-SUMMARY.md

---

### A first-run quality regression on a new retriever is diagnosed and fixed...
A first-run quality regression on a new retriever is diagnosed and fixed on the dev split, then re-measured; never loosened by editing the baseline

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-09-SUMMARY.md

---

### Eval builds a throwaway Library index from the frozen corpus snapshot; n...
Eval builds a throwaway Library index from the frozen corpus snapshot; no dependency on the live repo tree or on a running daemon

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-09-SUMMARY.md

---

### Qrel keys and Library chunk ids come from the one kb_ids walk, so resolu...
Qrel keys and Library chunk ids come from the one kb_ids walk, so resolution is exact by construction and mutation-proven

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-09-SUMMARY.md

---

### Retrieval-quality tuning is swept on the dev split only; the held-out te...
Retrieval-quality tuning is swept on the dev split only; the held-out test split is never scored during tuning

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-09-SUMMARY.md

---

### config-aware baseline shape check
config-aware baseline shape check

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-12-SUMMARY.md

---

### mutation-proved ratchet
mutation-proved ratchet

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-12-SUMMARY.md

## Surprises

### Estimate variance on 02-01
Estimate variance on 02-01: planned 125000 tokens vs 14930 actual (0.12x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-01-PLAN.md, 02-01-SUMMARY.md

---

### 1. [Rule 1 - Bug] Clippy lints on new test code
**1. [Rule 1 - Bug] Clippy lints on new test code**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-01-SUMMARY.md

---

### Estimate variance on 02-02
Estimate variance on 02-02: planned 40000 tokens vs 2500 actual (0.06x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-02-PLAN.md, 02-02-SUMMARY.md

---

### Estimate variance on 02-03
Estimate variance on 02-03: planned 95000 tokens vs 6300 actual (0.07x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-03-PLAN.md, 02-03-SUMMARY.md

---

### 1. [Rule 3 - Blocking] cargo update -p text-splitter is impossible befor...
**1. [Rule 3 - Blocking] `cargo update -p text-splitter` is impossible before the crate is in the lockfile**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-03-SUMMARY.md

---

### Estimate variance on 02-04
Estimate variance on 02-04: planned 110000 tokens vs 14000 actual (0.13x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-04-PLAN.md, 02-04-SUMMARY.md

---

### 1. [Rule 1 - Bug] Bad trust tier no longer rejected once unchanged files...
**1. [Rule 1 - Bug] Bad trust tier no longer rejected once unchanged files skip the store**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-04-SUMMARY.md

---

### Estimate variance on 02-05
Estimate variance on 02-05: planned 75000 tokens vs 6000 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-05-PLAN.md, 02-05-SUMMARY.md

---

### Estimate variance on 02-06
Estimate variance on 02-06: planned 125000 tokens vs 11000 actual (0.09x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-06-PLAN.md, 02-06-SUMMARY.md

---

### The plan asks to layer length, backslash, colon, and empty or
The plan asks to layer length, backslash, colon, and empty or `.` component checks on top of `is_kb_corpus_path`. That function already rejects all of them (and `#`, archive, and absolute paths), so only the 512-byte cap was added. The mutation therefore targets the `..` rule in `kb_ids.rs`, which was restored immediately.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-06-SUMMARY.md

---

### Estimate variance on 02-07
Estimate variance on 02-07: planned 80000 tokens vs 8000 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-07-PLAN.md, 02-07-SUMMARY.md

---

### 1. [Rule 3 - Blocking] sql-surface-guard failure — assigned by the orche...
**1. [Rule 3 - Blocking] sql-surface-guard failure** — assigned by the orchestrator, described above. Commit `f691aa05b`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-07-SUMMARY.md

---

### Estimate variance on 02-08
Estimate variance on 02-08: planned 80000 tokens vs 8500 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-08-PLAN.md, 02-08-SUMMARY.md

---

### 1. [Rule 3 - Blocking] cli-command-surface.generated.md regenerated and ...
**1. [Rule 3 - Blocking] `cli-command-surface.generated.md` regenerated and committed**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-08-SUMMARY.md

---

### Estimate variance on 02-09
Estimate variance on 02-09: planned 85000 tokens vs 6560 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-09-PLAN.md, 02-09-SUMMARY.md

---

### 1. [Rule 1 - Bug] Lexical title/heading boosts retuned after a first-run...
**1. [Rule 1 - Bug] Lexical title/heading boosts retuned after a first-run regression**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-09-SUMMARY.md

---

### Estimate variance on 02-10
Estimate variance on 02-10: planned 80000 tokens vs 9000 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-10-PLAN.md, 02-10-SUMMARY.md

---

### Estimate variance on 02-11
Estimate variance on 02-11: planned 55000 tokens vs 4000 actual (0.07x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-11-PLAN.md, 02-11-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Gateway test moved out of http_gateway_tests.rs
**1. [Rule 3 - Blocking] Gateway test moved out of `http_gateway_tests.rs`**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-11-SUMMARY.md

---

### Estimate variance on 02-12
Estimate variance on 02-12: planned 45000 tokens vs 2500 actual (0.06x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-12-PLAN.md, 02-12-SUMMARY.md

