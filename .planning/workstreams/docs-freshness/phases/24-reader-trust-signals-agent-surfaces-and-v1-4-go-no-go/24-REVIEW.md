---
phase: 24-reader-trust-signals-agent-surfaces-and-v1-4-go-no-go
reviewed: 2026-10-13T14:52:00Z
depth: deep
files_reviewed: 18
files_reviewed_list:
  - contracts/reports/docs-freshness/golden-drift-eval-set.v1.json
  - crates/vox-cli-ci/src/crate_api_docs.rs
  - crates/vox-cli/src/commands/audit.rs
  - crates/vox-doc-pipeline/src/claims/findings.rs
  - crates/vox-doc-pipeline/src/claims/gate.rs
  - crates/vox-doc-pipeline/src/claims/ledger.rs
  - crates/vox-doc-pipeline/src/claims/suppressions.rs
  - docs-astro/astro.config.mjs
  - docs-astro/public/_redirects
  - docs-astro/src/components/Banner.astro
  - docs-astro/src/components/PageSidebar.astro
  - docs-astro/src/components/Sidebar.astro
  - docs-astro/src/components/TableOfContents.astro
  - docs-astro/src/routeData.ts
  - docs-astro/src/styles/reader-layout.css
  - docs-astro/src/utils/docs-freshness.mjs
  - docs-astro/src/utils/sidebar.mjs
  - docs-astro/tests/build-output/llms.spec.ts
  - docs-astro/tests/smoke.spec.ts
  - docs-astro/tests/trust-signals.spec.ts
  - docs/src/.well-known/llms.txt
  - docs/src/architecture/docs-bot-evaluation-and-pivot-rule-2026.md
findings:
  critical: 1
  warning: 8
  info: 2
  total: 11
status: issues_found
---

# Phase 24: Code Review Report

**Reviewed:** 2026-10-13T14:52:00Z
**Depth:** deep
**Files Reviewed:** 22
**Status:** issues_found

## Summary

Conducted a deep cross-module adversarial review covering Phase 24 (`TRUST-01`..`TRUST-04`, `EVAL-01`..`EVAL-02`) and its supporting `docs-freshness` substrate across `docs-astro/`, `vox-doc-pipeline`, and `vox-cli`.

Found **1 Critical** cross-module bug where `generate_findings` omitted extracted mention text from `RealityAuditFinding.claim_ids`, breaking suppression matching in `evaluate_drift_gate` and `raw_text` recording in `--generate-baseline`; **8 Warnings** spanning suppression validation edge cases, `--attest` ledger metric staleness, CRLF frontmatter parsing in `ledger.rs`, commit SHA validation and unanchored page fallback in `docs-freshness.mjs`/`sidebar.mjs`, duplicate rules in `_redirects`, `docSlug` resolution in `llms.spec.ts`, ARIA state synchronization on TOC controls, and `localStorage` sanitization; and **2 Info** cleanup items.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `generate_findings` omits extracted mention text from `claim_ids`, breaking suppression matching and baseline `raw_text`

**File:** `crates/vox-doc-pipeline/src/claims/findings.rs:144-165`
**Issue:** `generate_findings` sets `claim_ids: vec![format!("claim-mention-{:04}", id_counter)]` and leaves `code_paths` and `contract_paths` empty. However, `evaluate_drift_gate` (`crates/vox-doc-pipeline/src/claims/gate.rs:112-123`) checks `suppressions.is_suppressed(doc_path, cid, today)` against `f.claim_ids`, `f.code_paths`, `f.contract_paths`, and `f.claim_type`, and `crates/vox-cli/src/commands/audit.rs:623` reads `f.claim_ids.first()` to populate `GrandfatheredDeficit.raw_text`. Because neither `m.extracted.raw_text` nor `m.extracted.normalized_target` is stored in `claim_ids`, any suppression entry in `suppressions.v1.json` targeting a specific mention string fails to match real findings from `scan_corpus`.
**Fix:**
```rust
        let mut claim_ids = vec![m.extracted.raw_text.clone(), claim_id];
        if !m.extracted.normalized_target.is_empty()
            && m.extracted.normalized_target != m.extracted.raw_text
        {
            claim_ids.push(m.extracted.normalized_target.clone());
        }
```

## Warnings

### WR-01: `SuppressionsFile` allows empty `page_path`/`target` and unanchored substring matches

**File:** `crates/vox-doc-pipeline/src/claims/suppressions.rs:50-104`
**Issue:** `SuppressionsFile::validate` does not verify that `page_path` and `target` are non-empty, and `is_suppressed` uses `norm_page.ends_with(&s.page_path)` and `target.contains(&s.target)`. An empty `target` (`""`) matches every finding on the page (`target.contains("") == true`), and `page_path: "cli.md"` matches `my-other-cli.md`.
**Fix:** Validate that `page_path` and `target` are non-empty in `validate()`, guard against empty strings in `is_suppressed()`, and require path-segment boundary matching for `page_path`.

### WR-02: `vox audit docs --attest` updates page state without recomputing `ledger.metrics`

**File:** `crates/vox-cli/src/commands/audit.rs:686-698`
**Issue:** When `vox audit docs --attest <page>` updates `page_rec.state` to `DocState::Verified`, it saves `ledger.v1.json` without recomputing `ledger.metrics` (`verified_count`, `drifting_count`, `broken_count`, `unanchored_count`, `historical_count`), leaving the ledger summary metrics out of sync with `ledger.pages`.
**Fix:** Add `pub fn recompute_metrics(&mut self)` to `FreshnessLedger` in `crates/vox-doc-pipeline/src/claims/ledger.rs` and call `ledger.recompute_metrics()` before `ledger.save_to_file(&ledger_path)` in `crates/vox-cli/src/commands/audit.rs`.

### WR-03: `parse_frontmatter_anchors` in `ledger.rs` fails on CRLF (`\r\n`) line endings

**File:** `crates/vox-doc-pipeline/src/claims/ledger.rs:121-122`
**Issue:** `content.strip_prefix("---\n")` and `stripped.split_once("\n---\n")` fail when files have CRLF line endings on Windows checkouts, silently dropping frontmatter `title`, `category`, `status`, `owner`, `review_cadence`, and `code_anchors`.
**Fix:** Normalize CRLF to LF (`let normalized = content.replace("\r\n", "\n");`) before stripping frontmatter delimiters.

### WR-04: `docs-freshness.mjs` does not validate `last_verified_commit` SHA or fall back to `unanchored` for pages absent from `ledger.v1.json`

**File:** `docs-astro/src/utils/docs-freshness.mjs:26-65`
**Issue:**
1. `entry.last_verified_commit` is interpolated into `commitUrl` without checking `/^[0-9a-f]{7,40}$/i`, which produces broken GitHub commit URLs if `last_verified_commit` is a placeholder string.
2. When a public doc page is not found in `ledger.v1.json` (e.g., newly generated crate API pages), `getFreshnessForPath` returns `null`, causing `Banner.astro` to render no trust banner at all instead of `Unverified`, and `sidebar.mjs:66` sets `freshnessState: null`, omitting the `Unverified` badge on non-verified main-sidebar pages (`TRUST-01`).
**Fix:** Validate `last_verified_commit` against `/^[0-9a-f]{7,40}$/i`, return a default `{ state: 'unanchored', shortCommit: 'unknown', commitUrl: null, title: null, anchorsCount: 0, brokenMentions: [] }` from `getFreshnessForPath` when a valid page path has no ledger entry, and default `freshnessState` in `sidebar.mjs` to `'unanchored'`.

### WR-05: Duplicate and conflicting redirect rules for `/architecture/architecture-index*` in `_redirects`

**File:** `docs-astro/public/_redirects:27-35`
**Issue:** Lines 27–29 redirect `/architecture/architecture-index{.html,,/}` to `/contributors/contributor-hub/`, while lines 33–35 duplicate those exact source paths and redirect them to `/architecture/research-index/` (which itself redirects to `/contributors/contributor-hub/` on lines 24–26).
**Fix:** Remove the duplicate lines 31–35 in `docs-astro/public/_redirects`.

### WR-06: `resolveUrlToLedger` in `llms.spec.ts` uses hardcoded `'agents'` check instead of `docSlug`

**File:** `docs-astro/tests/build-output/llms.spec.ts:113-121`
**Issue:** `resolveUrlToLedger` strips `.mdx?` manually instead of using `docSlug()`, requiring a hardcoded `(cleanPath === 'agents' && k === 'docs/src/AGENTS.md')` special case and failing on `index.md`, `README.md`, or mixed-case filenames.
**Fix:** Import `docSlug` from `../../src/utils/doc-slug.mjs` and compare `docSlug(k.replace(/^docs\/src\//, '')) === cleanPath`.

### WR-07: Right TOC toggle buttons in `PageSidebar.astro` and `TableOfContents.astro` lack `aria-expanded` state management

**File:** `docs-astro/src/components/PageSidebar.astro:6-39`, `docs-astro/src/components/TableOfContents.astro:8-39`
**Issue:** `#vox-toc-restore-btn` and `#vox-toc-hide-btn` lack `aria-expanded` attributes and do not synchronize ARIA state when toggling `document.documentElement.dataset.tocState` (`SITE-04`).
**Fix:** Add `aria-expanded="false"` on `#vox-toc-restore-btn` and `aria-expanded="true"` on `#vox-toc-hide-btn`, and synchronize both attributes on initialization and click.

### WR-08: Inline FOUC script in `astro.config.mjs` does not validate `localStorage` state values

**File:** `docs-astro/astro.config.mjs:75`
**Issue:** Raw values from `localStorage.getItem('vox-sidebar-state')` and `localStorage.getItem('vox-toc-state')` are assigned directly to `dataset.sidebarState` and `dataset.tocState` without allowlist validation.
**Fix:** Constrain `s` to `s === 'collapsed' ? 'collapsed' : 'pinned'` and `t` to `t === 'hidden' ? 'hidden' : 'visible'`.

## Info

### IN-01: `resolveContentPath` in `routeData.ts` is declared inside `onRequest` and called redundantly

**File:** `docs-astro/src/routeData.ts:93-138`
**Issue:** `resolveContentPath` is declared inside `onRequest` and invoked twice per request; fallback `statSync` also does not fall back to `resolve(DEFAULT_REPO_ROOT, sourcePath)` for mounted repo docs.
**Fix:** Hoist `resolveContentPath` to module scope, compute `diskPath` once, and fall back to `resolve(DEFAULT_REPO_ROOT, sourcePath)` if `diskPath` does not exist.

### IN-02: Unused imports in `docs-freshness.mjs` and duplicated repo-root traversal in `sidebar.mjs`

**File:** `docs-astro/src/utils/docs-freshness.mjs:2-3`, `docs-astro/src/utils/sidebar.mjs:10-15,93-95`
**Issue:** `resolve` and `fileURLToPath` are unused in `docs-freshness.mjs`. `sidebar.mjs` computes `repoRoot` twice via relative `import.meta.url` traversal instead of importing `DEFAULT_REPO_ROOT` from `./repo-mounts.mjs`.
**Fix:** Remove unused imports in `docs-freshness.mjs` and use `DEFAULT_REPO_ROOT` in `sidebar.mjs`.

---

_Reviewed: 2026-10-13T14:52:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
