---
phase: 24-reader-trust-signals-agent-surfaces-and-v1-4-go-no-go
fixed_at: 2026-10-13T14:56:30Z
review_path: .planning/workstreams/docs-freshness/phases/24-reader-trust-signals-agent-surfaces-and-v1-4-go-no-go/24-REVIEW.md
iteration: 1
findings_in_scope: 11
fixed: 11
skipped: 0
status: all_fixed
---

# Phase 24: Code Review Fix Report

**Fixed at:** 2026-10-13T14:56:30Z
**Source review:** `.planning/workstreams/docs-freshness/phases/24-reader-trust-signals-agent-surfaces-and-v1-4-go-no-go/24-REVIEW.md`
**Iteration:** 1
**Verification environment:** Main checkout (`/Users/brbrainerd/dev/vox`; git commits deferred to Claude Code per `.agents/rules/00-core.md` Rule 6)

**Summary:**
- Findings in scope: 11 (1 Critical, 8 Warnings, 2 Info)
- Fixed: 11
- Skipped: 0

## Fixed Issues

### CR-01: `generate_findings` omits extracted mention text from `claim_ids`, breaking suppression matching and baseline `raw_text`

**Files modified:** `crates/vox-doc-pipeline/src/claims/findings.rs`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Populated `RealityAuditFinding.claim_ids` with `m.extracted.raw_text`, the synthetic `claim-mention-XXXX` ID, and `m.extracted.normalized_target` (when distinct and non-empty), ensuring `evaluate_drift_gate` suppressions and `audit.rs` `--generate-baseline` resolve target mentions accurately.

### WR-01: `SuppressionsFile` allows empty `page_path`/`target` and unanchored substring matches

**Files modified:** `crates/vox-doc-pipeline/src/claims/suppressions.rs`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Added non-empty `page_path` and `target` validation to `SuppressionsFile::validate()`, guarded `is_suppressed()` against empty strings, enforced path-segment boundary matching on `page_path`, and required exact target matching.

### WR-02: `vox audit docs --attest` updates page state without recomputing `ledger.metrics`

**Files modified:** `crates/vox-doc-pipeline/src/claims/ledger.rs`, `crates/vox-cli/src/commands/audit.rs`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Added `FreshnessLedger::recompute_metrics(&mut self)` in `ledger.rs` and invoked `ledger.recompute_metrics()` before saving in the `--attest` handler in `audit.rs`.

### WR-03: `parse_frontmatter_anchors` in `ledger.rs` fails on CRLF (`\r\n`) line endings

**Files modified:** `crates/vox-doc-pipeline/src/claims/ledger.rs`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Normalized `\r\n` to `\n` before stripping YAML frontmatter fences in `parse_frontmatter_anchors`.

### WR-04: `docs-freshness.mjs` does not validate `last_verified_commit` SHA or fall back to `unanchored` for pages absent from `ledger.v1.json`

**Files modified:** `docs-astro/src/utils/docs-freshness.mjs`, `docs-astro/src/utils/sidebar.mjs`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Validated `last_verified_commit` against `/^[0-9a-f]{7,40}$/i` before constructing `shortCommit` and `commitUrl`, returned a default `unanchored` record when a public doc page is absent from `ledger.v1.json`, and defaulted `freshnessState` to `'unanchored'` in `sidebar.mjs`.

### WR-05: Duplicate and conflicting redirect rules for `/architecture/architecture-index*` in `_redirects`

**Files modified:** `docs-astro/public/_redirects`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Removed the duplicate `/architecture/architecture-index*` rules that pointed to the retired `/architecture/research-index/` path.

### WR-06: `resolveUrlToLedger` in `llms.spec.ts` uses hardcoded `'agents'` check instead of `docSlug`

**Files modified:** `docs-astro/tests/build-output/llms.spec.ts`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Imported `docSlug` from `../../src/utils/doc-slug.mjs` and matched `docSlug(relUnderDocs) === cleanPath` in `resolveUrlToLedger`.

### WR-07: Right TOC toggle buttons in `PageSidebar.astro` and `TableOfContents.astro` lack `aria-expanded` state management

**Files modified:** `docs-astro/src/components/PageSidebar.astro`, `docs-astro/src/components/TableOfContents.astro`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Added `aria-expanded` attributes to `#vox-toc-restore-btn` and `#vox-toc-hide-btn` and synchronized ARIA states on initialization and click.

### WR-08: Inline FOUC script in `astro.config.mjs` does not validate `localStorage` state values

**Files modified:** `docs-astro/astro.config.mjs`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Constrained `dataset.sidebarState` to `'collapsed' | 'pinned'` and `dataset.tocState` to `'hidden' | 'visible'` in the inline head script.

### IN-01: `resolveContentPath` in `routeData.ts` is declared inside `onRequest` and called redundantly

**Files modified:** `docs-astro/src/routeData.ts`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Hoisted `resolveContentPath` to module scope, resolved `diskPath` once per route, and added a fallback to `resolve(DEFAULT_REPO_ROOT, sourcePath)` for `statSync`.

### IN-02: Unused imports in `docs-freshness.mjs` and duplicated repo-root traversal in `sidebar.mjs`

**Files modified:** `docs-astro/src/utils/docs-freshness.mjs`, `docs-astro/src/utils/sidebar.mjs`
**Commit:** working-tree (staged for Claude Code verification per Rule 6)
**Applied fix:** Removed unused `resolve` and `fileURLToPath` imports from `docs-freshness.mjs` and replaced relative `import.meta.url` traversal in `sidebar.mjs` with `DEFAULT_REPO_ROOT`.

---

_Fixed: 2026-10-13T14:56:30Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
