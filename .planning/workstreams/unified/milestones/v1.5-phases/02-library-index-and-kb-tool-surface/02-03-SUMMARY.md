---
phase: 02-library-index-and-kb-tool-surface
plan: 03
subsystem: search
tags: [chunking, kb-ids, text-splitter, github-slugger, library-index, walker]
requires:
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-01 library tracer (kb_ids sections, chunk_document, walk_corpus, index_library)"
  - phase: 01-retrieval-eval-baseline
    provides: "kb_ids identity SSOT (01-01)"
provides:
  - "library::chunk::{github_slug, EstimatedTokens, SECTION_TOKEN_BUDGET}; one root-keyed card chunk per doc; site-matching anchors"
  - "Oversize sections split by text-splitter 0.33 under the frozen est4b sizer (CHUNKER_VERSION kb_ids-sections-v2+card+est4b+md512)"
  - "chunk_paths_match_kb_ids: real-corpus parity between chunk heading_paths and kb_ids::doc_heading_paths"
  - "Mutation-proven archive and symlink exclusion in the walker"
affects: [02-04, 02-09, phase-05]
actuals:
  tokens: 6300
  tasks: 3
  commits: 3
plan_head_before: 46ce87b29e71db6aff734964cdbe9fcd1511b6a9
tech-stack:
  added: ["text-splitter 0.33 (markdown feature, default-features off)"]
  patterns:
    - "Chunk identity only from kb_ids; chunk.rs never parses headings or joins paths itself"
    - "Parts of an oversize section share heading_path, anchor and so the kb_ids chunk id"
key-files:
  created:
    - crates/vox-search/tests/kb_library_chunking.rs
  modified:
    - crates/vox-search/src/library/chunk.rs
    - crates/vox-search/Cargo.toml
    - Cargo.toml
    - Cargo.lock
    - crates/workspace-hack/Cargo.toml
key-decisions:
  - "Card heading_path is the root path (doc title), AP-23, so rel_path#Title always resolves and the parity set always contains the root"
  - "Card first paragraph = first prose paragraph before any H2+ (preamble and H1 intro), capped at 1024 bytes"
  - "Anchors are derived from the section's own heading line via kb_ids::parse_heading, not the last heading_path segment, so H1 sections get their real slug"
requirements-completed: [IDX-02, IDX-03, IDX-04]
status: complete
---

# Phase 2 Plan 03: Chunker completion and walker proof Summary

Every doc now yields one root-keyed summary card plus heading sections with github-slugger anchors, oversize sections are split by `text-splitter` under a frozen `ceil(bytes/4)` sizer, and `chunk_paths_match_kb_ids` proves on all of `docs/src` that chunk heading_paths equal Phase 1's `kb_ids` identity.

## Commits

- `5782d79ac` feat(02-03): card chunk, github-slugger anchors and kb_ids parity tests (Task 1, tracer)
- `dd8a7b85d` feat(02-03): split oversize sections with text-splitter and a frozen est4b sizer (Task 2)
- `977747dfa` test(02-03): prove archive, symlink and non-md exclusion against the shared kb_ids filter (Task 3)

## What was built

- **Card (IDX-03 card half).** `chunk_document` emits exactly one `ChunkKind::Card` at index 0 with `heading_path` = `kb_ids::doc_title`. Body is title, frontmatter `description`, the first intro paragraph, then `category: …` / `status: …` lines when present. No raw YAML ever enters a body. The card goes through the same hash and `#n` ordinal map as sections.
- **Anchors.** `github_slug(heading_text, &mut seen)` follows github-slugger: lowercase, drop everything but letters/digits/`-`/`_`/space, each space to `-`, repeats get `-1`, `-2`. Every heading consumes a slug in document order, including headings whose section body is otherwise skipped. The card has no anchor. Three real headings from `search-retrieval-ssot-2026.md` are pinned in a unit test (`3-two-distinct-voxdb-surfaces-do-not-conflate`, `31-rust-vox-db-turso--libsql`, `language-surface-decision`).
- **Oversize split (IDX-02).** Sections over `SECTION_TOKEN_BUDGET` (512) go through `MarkdownSplitter::new(ChunkConfig::new(256..512).with_sizer(EstimatedTokens))`, overlap 0. `EstimatedTokens` calls `estimate_tokens`, the same estimator as Phase 1's answer-in-budget; no model tokenizer. `CHUNKER_VERSION` is now `kb_ids-sections-v2+card+est4b+md512`.
- **Walker (IDX-04).** `corpus.rs` already carried `follow_links(false)`, the symlink skip and `kb_ids::is_kb_corpus_path` as the sole authority (02-01). This plan adds the tests and the mutation proof; no code change was needed there.

## Verification

| Check | Result |
|-------|--------|
| `cargo nextest run -p vox-search -E 'test(/chunk_\|kb_chunking\|kb_archive\|walker_matches_kb_filter\|kb_ids\|kb_eval\|kb_tracer\|library::\|github_slug\|card_body/)'` (default features, which include tantivy-lexical) | 92/92 pass |
| Task 1 red run before implementation | 4 failed (`chunk_paths_match_kb_ids`, `chunk_card_first`, `chunk_anchor_github_slugger`, `kb_chunking_end_to_end`) |
| `cargo clippy -p vox-search --all-targets -- -D warnings` | clean |
| `cargo hakari generate --diff` | exit 0 after `cargo hakari generate` |
| `cargo tree -p vox-search -e normal -i text-splitter` | `text-splitter v0.33.0 └── vox-search` |
| `VOX_SKIP_FRESHNESS_CHECK=1 vox ci crate-edges` | OK, 689 edges, no new edge (installed `vox` is stale, built at 6382, tree at 6565) |
| Acceptance greps | `pub fn github_slug` present; `kb_ids::`/`HEADING_PATH_SEP` count 6; no `" › "` literal in chunk.rs; `with_sizer(EstimatedTokens)` present; no `with_overlap`; `follow_links(false)` and `is_kb_corpus_path` present in corpus.rs; `library/mod.rs` untouched by every task commit |

## Mutation table (T-02-03, Task 3)

Test: `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(kb_archive_excluded)'` unless noted. Each row records the presence count of the mutated line before and after the edit and again after restore (restored from a backup copy, then re-grepped).

| # | Guard | Mutation | Mutated run | Restored run |
|---|-------|----------|-------------|--------------|
| 1 | `KB_EXCLUDED_DIR_NAMES` in `kb_ids.rs` (shared by the prune and by `is_kb_corpus_path`) | `&["archive"]` to `&[]` (presence 1, 0, 1) | FAIL: left `["docs/src/archive/old.md", "docs/src/library/page.md", "docs/src/ok.md", "docs/src/research/archive/older.md"]`, right `["docs/src/library/page.md", "docs/src/ok.md"]` | PASS, 1 passed |
| 2 | symlink skip in `corpus.rs` (`follow_links(false)` plus `entry.path_is_symlink() \|\|`) | both removed together: `follow_links(true)` and `if !entry.file_type().is_file()` | FAIL: left contains `docs/src/link.md` | PASS, 1 passed |

Findings from the single-line variants (recorded because they changed what "load-bearing" means here):

- **The two symlink guards back each other up.** Removing only `path_is_symlink()` (A) or flipping only `follow_links(true)` (B) both left `kb_archive_excluded` green. Only removing both fails. Neither line alone is proven by this test.
- **The archive rule is two layers**: the `filter_entry` prune in `corpus.rs` and the component check in `is_kb_corpus_path`. Disabling only the `is_kb_corpus_path` component check (`if false && …`) left `kb_archive_excluded` green (the prune still excludes), but it fails `walker_matches_kb_filter` (it asserts `!is_kb_corpus_path("docs/src/library/archive/x.md")`). Restored, both pass. So each layer is covered, by a different test.

## Deviations from Plan

**1. [Rule 3 - Blocking] `cargo update -p text-splitter` is impossible before the crate is in the lockfile**
- **Found during:** Task 2
- **Issue:** `error: package ID specification 'text-splitter' did not match any packages`.
- **Fix:** `cargo fetch` resolved and locked it. This also bumped the ICU family (`icu_provider` 2.2 to 2.3 etc., `icu_segmenter` 2.3, `itertools` 0.15, `auto_enums`, `derive_utils`, `core_maths`) exactly as 02-RESEARCH predicted. Licenses are within `deny.toml`; `cargo deny` is not installed locally, hosted CI `linux` runs it on dependency changes.
- **Commit:** dd8a7b85d

**2. [Rule 3 - Blocking] `cargo hakari generate` also absorbed pre-existing drift**
- **Found during:** Task 2
- **Issue:** `--diff` was never clean because of the pre-existing `reqwest` 0.12 `default-tls` feature drift that 02-01 reported. `generate` writes the whole file.
- **Fix:** Accepted the regenerated `crates/workspace-hack/Cargo.toml` as one change: the text-splitter/ICU/pulldown-cmark lines plus the reqwest line. `--diff` now exits 0.
- **Commit:** dd8a7b85d

**3. Card facet format versus the frontmatter test**
- The plan says no chunk body may contain `category:`, yet the card must carry a `category: …` line. `chunk_frontmatter_never_in_body` therefore asserts `description:`/`category:` absence on Section chunks, and `title:` and the raw YAML comment absence on all chunks (cards included). The truth that matters, raw YAML never leaking, holds for every chunk.

**4. Minor API choices**
- Used `MarkdownSplitter::chunks` rather than `chunk_indices` (offsets were not needed).
- Anchors come from the section's own heading line through `kb_ids::parse_heading`, not the last heading_path segment, because an H1 section's path is the root title, not its heading text.
- Task 3 required no `corpus.rs` change: 02-01 already contained the hardening, so its commit is test-only.

## Known Simplifications

- `github_slug` drops Unicode combining marks and does not strip inline link/emphasis syntax from heading text first (`ponytail:` comment in code). `docs/src` has neither in headings today; the real-corpus anchor check is the three pinned headings, not a full site-id comparison (docs-astro is not built in this worktree).
- A single fenced block larger than the 512 budget would still be split by the splitter; only fences that fit are never cut. None exists in `docs/src` today.
- `cargo check -p vox-search --no-default-features` fails in `policy.rs` (`tavily_research` gated out). Pre-existing, unrelated to this plan, not touched.

## Known Stubs

None.

## Threat Flags

None. New surface is the `text-splitter` dependency (T-02-SC, covered by the plan's threat model) and no new file or network access.

## Self-Check: PASSED

- `crates/vox-search/tests/kb_library_chunking.rs`, `crates/vox-search/src/library/chunk.rs`: present.
- Commits 5782d79ac, dd8a7b85d, 977747dfa: present on branch.
