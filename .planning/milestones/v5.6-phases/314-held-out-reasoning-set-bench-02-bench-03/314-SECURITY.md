---
phase: "314"
slug: "held-out-reasoning-set-bench-02-bench-03"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-26"
---

# Phase 314 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `workflow.security_block_on` is `high`. The plan-time register has three high threats (T-314-06, T-314-07, T-314-08). The security auditor checked the committed scorer and slice loader against those rows on 2026-10-26 and closed all of them. No implementation files were changed for this audit. A green test is not a model score.

Two non-blocking notes stay outside the register. `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` still mentions `VOX_REASONING_EVAL_LIMIT` in an uncommitted edit and was left unstaged. If `fail_closed` cannot write the NotScored file, it still returns `Err`, so the collateral gate does not treat the run as a score; a reader of a leftover file outside that gate is not covered.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| repo data file → loader | `reasoning.jsonl` is parsed by `load_slice` | Slice rows, low sensitivity, size-capped |
| repo data file → interpreter | The oracle executes `program` strings from the committed `vox_think` file | Program source, step-budgeted |
| held-out set → training corpus | Overlap with training text or the plan template would end the hold-out | Question text |
| model backend → scorer | `load_model` and `run_inference` results enter the scoring decision | Completions |
| scorer → report file | `reasoning_report.json` is read later by the collateral gate and, in phase 315, by serve | Score or not-scored status |
| environment → scorer | `VOX_REASONING_EVAL_MAX_TOKENS` sets the generation cap | Integer cap |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-314-01 | Tampering | `mens/data/*_think/reasoning.jsonl` | medium | mitigate | Count 32 and first/last ids pinned in `checked_in_gsm8k_and_math500_slices_have_32_pinned_rows` and `vox_think_shape`. The three slices are tracked. | closed |
| T-314-02 | Elevation of privilege | `vox_think_oracle_test.rs` | medium | mitigate | `programs_are_pure_and_nontrivial` denies I/O identifiers. `Interpreter::new(1_000_000)` is the step budget. Only the committed file is run. | closed |
| T-314-03 | Denial of service | `load_slice` | low | mitigate | `Read::take(MAX_SLICE_BYTES + 1)` rejects a file one byte over 1 MiB before parsing. Locked by `size_cap_boundary`. | closed |
| T-314-04 | Repudiation | `vox_think` hold-out | medium | mitigate | Template-marker and golden-copy guards. Gold is the interpreter's `main` return. | closed |
| T-314-05 | Spoofing | MATH-500 `source` provenance | low | accept | Licence unverified. No licence text added. Existing `source` string left as is. | closed |
| T-314-06 | Repudiation | `score_reasoning_file` load failure | high | mitigate | `fail_closed` writes `NotScored` (`n` 0, no `accuracy` or `think_initiation_rate` key) and returns `Err`. `ReasoningReport` is built only from `Scored`. Tests assert key absence. | closed |
| T-314-07 | Tampering | stale `reasoning_report.json` | high | mitigate | `write_outcome` uses `std::fs::write`, which truncates. `a_not_scored_write_replaces_a_stale_scored_report` asserts the score keys are gone. | closed |
| T-314-08 | Tampering | per-item inference errors | high | mitigate | `score_loaded` stops at the first error, unparseable body, or empty text and does not grade that item. `from_score` is `Scored` only when `n_requested >= 1` and `score.n == n_requested`. | closed |
| T-314-09 | Spoofing | phantom adapter directory | medium | mitigate | `adapter.is_dir()` is checked before any write. `a_missing_adapter_path_is_err_and_not_created` asserts the path still does not exist. | closed |
| T-314-10 | Tampering | truncated run passed off as a slice score | medium | mitigate | `VOX_REASONING_EVAL_LIMIT` is deleted. `from_score` rejects a partial count. `VOX_REASONING_EVAL_MAX_TOKENS` is registered once, owner `vox-ml-cli`. | closed |
| T-314-11 | Information disclosure | `completion_preview` | low | accept | Preview is truncated to 240 characters and written only on a Scored report, under the operator's adapter directory. | closed |
| T-314-SC | Tampering | npm/pip/cargo installs | low | accept | No package installed. No new crate edge. `vox-ml-cli` and `vox-compiler` already depend on `vox-eval`. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-314-05 | T-314-05 | MATH-500 licence was not verified. The plan forbids adding a licence claim and leaves the existing source string unchanged. | phase 314 plan | 2026-10-26 |
| AR-314-11 | T-314-11 | A short completion preview of a public benchmark question is stored only in the operator's adapter directory, and only for a scored run. | phase 314 plan | 2026-10-26 |
| AR-314-SC | T-314-SC | This phase installs nothing and adds no crate dependency. | phase 314 plan | 2026-10-26 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-26 | 12 | 12 | 0 | security auditor (ASVS L1); orchestrator wrote this file |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-26

## Security Audit 2026-10-26

| Metric | Count |
|---|---|
| Threats found | 12 |
| Closed | 12 |
| Open | 0 |
