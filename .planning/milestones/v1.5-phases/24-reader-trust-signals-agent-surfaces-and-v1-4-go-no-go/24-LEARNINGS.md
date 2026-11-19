---
phase: 24
phase_name: "reader-trust-signals-agent-surfaces-and-v1-4-go-no-go"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 15
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 24 Learnings: reader-trust-signals-agent-surfaces-and-v1-4-go-no-go

## Decisions

### Reads contracts/reports/docs-freshness/ledger.v1.json dynamically using ...
Reads `contracts/reports/docs-freshness/ledger.v1.json` dynamically using `DEFAULT_REPO_ROOT`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-01-SUMMARY.md

---

### Exports getFreshnessForPath(sourcePath, filePath) and loadFreshnessLedge...
Exports `getFreshnessForPath(sourcePath, filePath)` and `loadFreshnessLedger()`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-01-SUMMARY.md

---

### Returns { state, shortCommit, commitUrl, title, anchorsCount, brokenMent...
Returns `{ state, shortCommit, commitUrl, title, anchorsCount, brokenMentions }`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-01-SUMMARY.md

---

### Replaced default banner in docs-astro/astro.config.mjs via components
Replaced default banner in `docs-astro/astro.config.mjs` via `components: { Banner: './src/components/Banner.astro' }`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-01-SUMMARY.md

---

### Preserves default status banners (research, roadmap, deprecated, legacy)...
Preserves default status banners (research, roadmap, deprecated, legacy).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-01-SUMMARY.md

---

### Deleted docs/src/.well-known/llms-full.txt and docs-astro/public/.well-k...
Deleted `docs/src/.well-known/llms-full.txt` and `docs-astro/public/.well-known/llms-full.txt`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-02-SUMMARY.md

---

### Added permanent 301 redirect in docs-astro/public/_redirects
Added permanent 301 redirect in `docs-astro/public/_redirects`: `/.well-known/llms-full.txt   /llms-full.txt   301`

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-02-SUMMARY.md

---

### Preserves single-source truth
Preserves single-source truth: agents and downstream scrapers receive the generated, Internals-filtered `/llms-full.txt` produced by `starlight-llms-txt`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-02-SUMMARY.md

---

### Audited docs/src/.well-known/llms.txt against contracts/reports/docs-fre...
Audited `docs/src/.well-known/llms.txt` against `contracts/reports/docs-freshness/ledger.v1.json`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-02-SUMMARY.md

---

### Pruned and updated all links to point strictly to state === 'verified' p...
Pruned and updated all links to point strictly to `state === 'verified'` public documentation.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-02-SUMMARY.md

---

### Committed 10 real historical documentation drift cases from Vox git hist...
Committed 10 real historical documentation drift cases from Vox git history (`e2bb133a1`, `49a08f34e`, `f48dbc810`, `59e30f6b6`, `94822d35d`, `0bae3b6a2`, `7fa4d0ead`, `2875eacba`, `58539cd17`, `fb9fab9fd`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-03-SUMMARY.md

---

### Covered key drift failure modes
Covered key drift failure modes: factual claim contradictions, removed grammar syntax (`@endpoint`), escape byte corruption, dead agent index targets, policy linter discrepancies, and stale skip reasons.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-03-SUMMARY.md

---

### Tagged each case with code_anchor, snippet_stale, snippet_fixed, and det...
Tagged each case with `code_anchor`, `snippet_stale`, `snippet_fixed`, and `detection_strategy`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-03-SUMMARY.md

---

### Evaluated commercial hosted bots (Promptless, Dosu, Mintlify Agent, Deep...
Evaluated commercial hosted bots (Promptless, Dosu, Mintlify Agent, DeepDocs) across code grounding, PR acceptance rates, false positives, pricing, and `.vox` suitability.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-03-SUMMARY.md

---

### Analyzed academic baselines (DocPrism's LCEF architecture showing uncons...
Analyzed academic baselines (DocPrism's LCEF architecture showing unconstrained LLM verification has 90–98% false-positives; CASCADE's dual-condition executable test synthesis).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 24-03-SUMMARY.md

## Lessons

### Files modified
**Files modified:** `crates/vox-doc-pipeline/src/claims/findings.rs` **Commit:** working-tree (staged for Claude Code verification per Rule 6) **Applied fix:** Populated `RealityAuditFinding.claim_ids` with `m.extracted.raw_text`, the synthetic `claim-mention-XXXX` ID, and `m.extracted.normalized_target` (when distinct and non-empty), ensuring `evaluate_drift_gate` suppressions and `audit.rs` `--generate-baseline` resolve target mentions accurately.

**Context:** Surfaced in structured phase code review.
**Source:** 24-REVIEW-FIX.md

---

### let mut claim_ids = vec![m.extracted.raw_text.clone(), claim_id]; if !m....
let mut claim_ids = vec![m.extracted.raw_text.clone(), claim_id]; if !m.extracted.normalized_target.is_empty() && m.extracted.normalized_target != m.extracted.raw_text { claim_ids.push(m.extracted.normalized_target.clone()); }

**Context:** Surfaced in structured phase code review.
**Source:** 24-REVIEW.md

## Patterns

## Surprises

