---
phase: 323-high-value-starlight-mdx-conversions-and-ingest-parity
depth: standard
files_reviewed: 14
status: clean
findings_summary:
  critical: 0
  warning: 4
  info: 1
  fixed: 4
---

# Phase 323 Code Review Report (`--fix`)

## Scope

Reviewed 14 files modified/created in Phase 323:

- `crates/vox-kb-facets/src/summary.rs`
- `crates/vox-kb-facets/src/lib.rs`
- `crates/vox-search/src/kb_ids.rs`
- `crates/vox-search/src/ingest.rs`
- `crates/vox-search/src/library/chunk.rs`
- `crates/vox-doc-pipeline/src/pipeline/facets.rs`
- `crates/vox-doc-pipeline/src/pipeline/facets_tests.rs`
- `crates/vox-corpus/src/corpus/extract_docs.rs`
- `docs-astro/src/plugins/remark-doc-links.mjs`
- `docs-astro/astro.config.mjs`
- `docs/src/index.mdx`
- `docs/src/tutorials/tut-fullstack-walkthrough.mdx`
- `docs/src/tutorials/tut-getting-started.md`
- `docs/src/reference/ref-syntax.md`

## Findings & Fixes Applied (`--fix`)

### Warning Findings (All Fixed)

1. **`[WR-01]` Mismatched HTML/MDX summary open and close markers accepted (`crates/vox-kb-facets/src/summary.rs`)**
   - **Issue:** `inspect_summary` and `strip_summary_blocks` allowed opening with `<!-- kb:summary -->` and closing with `{/* /kb:summary */}` (or vice versa). Because MDX v3 rejects HTML comments outright, a mixed pair is always a syntax error.
   - **Fix:** Paired `SUMMARY_OPEN` strictly with `SUMMARY_CLOSE` and `SUMMARY_OPEN_MDX` strictly with `SUMMARY_CLOSE_MDX` in both `inspect_summary` and `strip_summary_blocks`, and added unit test coverage verifying `SummaryStatus::Unterminated` and fail-closed stripping on mismatched markers.

2. **`[WR-02]` `FenceTracker::step` called on trimmed line instead of raw line (`crates/vox-kb-facets/src/summary.rs`)**
   - **Issue:** `strip_mdx_boilerplate` passed `line.trim()` to `fences.step(...)`, stripping leading spaces (`> 3`) and causing 4-space indented ``` lines to be misclassified as CommonMark fence openers.
   - **Fix:** Passed `raw_line` (preserving leading indentation) to `fences.step(raw_line)` and added unit test coverage with a 4-space indented fence line.

3. **`[WR-03]` CR-only (`\r`) line endings not split in `strip_mdx_boilerplate` (`crates/vox-kb-facets/src/summary.rs`)**
   - **Issue:** `strip_mdx_boilerplate` iterated over `text.split_inclusive('\n')`, whereas the rest of `vox-kb-facets` uses `line_spans(text, 0)` to support `\n`, `\r\n`, and lone `\r` line endings.
   - **Fix:** Switched `strip_mdx_boilerplate` to iterate via `line_spans(text, 0)` and preserve `&text[start..end]`, with unit test coverage for `\r`-only input.

4. **`[WR-04]` Repository boundary check ordered after `existsSync` probing (`docs-astro/src/plugins/remark-doc-links.mjs`)**
   - **Issue:** `resolveDocLink` probed `existsSync(absTarget)` and `.md`/`.mdx` alternates before verifying that `absTarget` did not escape `ctx.repoRoot`.
   - **Fix:** Moved the `relToRepo.startsWith('..') || isAbsolute(relToRepo)` guard above `existsSync(absTarget)` and the `.md` ↔ `.mdx` fallback check.

### Info Findings

1. **`[IN-01]` `FacetIssue::MissingSummary` diagnostic message (`crates/vox-doc-pipeline/src/pipeline/facets.rs`)**
   - **Note:** Kept existing message string `"no complete <!-- kb:summary --> block"` for backward compatibility with existing facet lint test assertions while `inspect_summary` transparently accepts `{/* kb:summary */}` in `.mdx` files.
