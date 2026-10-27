---
phase: "315"
slug: "serve-gate-gate-01-gate-02"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-26"
---

# Phase 315 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `workflow.security_block_on` is `high`. The plan-time register has seven high threats (T-315-01, T-315-02, T-315-03, T-315-07, T-315-08, T-315-09, T-315-10). The security auditor checked the committed writer and serve gate on 2026-10-26 and closed all 15 rows. No implementation files were changed for this audit. A green test is not a model score.

The gate still runs only when `training_manifest.json` or `candle_qlora_adapter.safetensors` is present. That limit is written in plan 315-02. Serve checks slice-report `status` and `n`, not numeric equality with the collateral report.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| baseline file → writer | Operator JSON supplies pre-scores | Bench names and numbers |
| scorer → writer | Per-slice accuracy, think-initiation, and `n` | Scores |
| report file → serve | `collateral_damage_report.json` and per-slice `reasoning_report.json` are read back later | Status and stored numbers |
| slice name → path | Serve joins only names from `SLICE_NAMES` | Fixed filenames |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-315-01 | Tampering | `eval_reasoning_slice` | high | mitigate | `guard` refuses non-finite values, `pre <= 0`, and values outside [0, 1]. `n` other than 32 and unknown slices refuse before arithmetic. Reasoning keys do not use the generic zero-pre path. | closed |
| T-315-02 | Repudiation | stale collateral report | high | mitigate | `remove_file` runs before scoring. The report is written only after every check succeeds. | closed |
| T-315-03 | Tampering | missing reasoning keys | high | mitigate | `require_reasoning_keys` demands all six keys before any bench is scored. | closed |
| T-315-04 | Spoofing | reasoning key classified by filesystem | medium | mitigate | Classification is by name against `SLICE_NAMES`. A reasoning key with `n: None` writes no report. | closed |
| T-315-05 | Repudiation | unlabeled benches | medium | mitigate | `reasoning_slices` carries `scope_note`, `slice_size`, and per-slice `n` on pass and fail. | closed |
| T-315-06 | Denial of service | baseline parse | low | accept | Parse errors return `Err`. No panic on the baseline path. | closed |
| T-315-07 | Tampering | stored pass | high | mitigate | `verify_reasoning_slices` recomputes each drop and ignores stored `degradation_rate`, `exceeds_threshold`, and `status`. | closed |
| T-315-08 | Repudiation | old-format or missing report | high | mitigate | A missing `reasoning_slices` object and a missing report file both refuse serve. | closed |
| T-315-09 | Tampering | unscored slice file | high | mitigate | Every slice must have `reasoning_report.json` with `status == "scored"` and `n == 32`. | closed |
| T-315-10 | Tampering | widened threshold | high | mitigate | The stored `threshold` must equal the default 0.05. Serve recomputes with that default. | closed |
| T-315-11 | Tampering | path from report content | medium | mitigate | The slice path is built from `SLICE_NAMES` and a fixed filename. | closed |
| T-315-12 | Denial of service | malformed report JSON | medium | mitigate | Missing or null numbers refuse. `1e999` is rejected. The server does not start. | closed |
| T-315-13 | Repudiation | slice label dropped at serve | medium | mitigate | Serve requires the exact `scope_note`, `slice_size` 32, and per-slice `n`. The how-to names the six keys. | closed |
| T-315-SC | Tampering | npm/pip/cargo installs | low | accept | No package installed. No new crate edge. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-315-06 | T-315-06 | A bad baseline returns an error and writes no report. The plan accepts that as the control. | phase 315 plan | 2026-10-26 |
| AR-315-SC | T-315-SC | This phase installs nothing and adds no crate dependency. | phase 315 plan | 2026-10-26 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-26 | 14 | 14 | 0 | security auditor (ASVS L1); orchestrator wrote this file |

The two plan rows named T-315-SC are one accepted supply-chain row here.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-26

## Security Audit 2026-10-27

| Metric | Count |
|---|---|
| Threats found | 14 |
| Closed | 14 |
| Open | 0 |
