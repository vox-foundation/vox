---
phase: "316"
slug: "before-after-comparison-bench-01"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-28"
---

# Phase 316 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `workflow.security_block_on` is `high`. The plan-time register has eleven rows (T-316-01 through T-316-10, plus T-316-SC). The security auditor checked the compare entry, detection, and report JSON on 2026-10-28 and closed all eleven. No implementation files were changed for this audit. `316-01-SUMMARY.md` has no Threat Flags section.

The CLI entry is `run_eval_reasoning_compare_with`. `populi/dispatch.rs` calls `run_eval_reasoning_compare`, which calls that entry. The report is written only after `check_out_path`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI args → command | `--base`, `--adapter`, `--hub-root`, `--out` are untrusted text and paths | Pin strings and filesystem paths |
| local filesystem → detection | HF cache tree, `model.safetensors.index.json`, and `adapter_manifest.json` may be partial, stale, or hostile | Snapshot layout and `base_model` |
| command → files serve reads | The report must never be read by `vox mens serve` as a collateral pass | Absence or comparison JSON |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-316-01 | Spoofing | absence or comparison file read by serve as a pass | high | mitigate | `check_out_path` (`reasoning_compare.rs:104`) refuses `collateral_damage_report.json`, `reasoning_report.json`, any `collateral_eval` component, the hub root, and the adapter dir. The single entry calls it at `reasoning_compare.rs:330` before detection or write. Absence and compared JSON set `gate_input` false and omit `benchmarks`, `reports`, and `reasoning_slices` (`reasoning_bench.rs:286`, `reasoning_bench.rs:339`). | closed |
| T-316-02 | Spoofing | another revision or size scored as the pinned base | high | mitigate | `snapshot_dir` (`detect.rs:44`) joins `snapshots/<sha>` from the resolved pin. `BaseLabel::from_pin` (`reasoning_bench.rs:173`) keeps a revision only for 40 lowercase hex chars. No revision becomes `PinUnverifiable` (`detect.rs:61`). Unknown or ambiguous size names stay unpinned (`detect.rs:254`). The entry calls `resolve_base_pin` at `reasoning_compare.rs:320`. | closed |
| T-316-03 | Spoofing | adapter trained on another base compared to the pinned base | high | mitigate | `manifest_matches` (`detect.rs:191`) requires `base_model` to canonicalize to the detected snapshot. A missing key, a non-directory (Hub id), or a different path is `AdapterBaseNotPinnedSnapshot` (`detect.rs:139`). | closed |
| T-316-04 | Tampering | writing through HF cache symlinks or into adapter `collateral_eval/` | high | mitigate | `reasoning_compare` does not call `score_reasoning_file`. `score_slices_loaded` (`reasoning_compare.rs:140`) calls `score_loaded` and drops the rows. `score_loaded` (`reasoning_eval.rs:184`) returns outcomes in memory. `write_outcome` stays on the slice-eval path. `check_out_path` refuses the hub root and the adapter dir. `containment_path` canonicalizes the nearest existing ancestor before the prefix check. | closed |
| T-316-05 | Repudiation | invented, misattributed, or stale score | high | mitigate | `AbsenceRecord::to_json` (`reasoning_bench.rs:275`) has no accuracy, think-initiation, or `n` keys. `named_outcome` (`reasoning_compare.rs:276`) turns a missing slice outcome into `NotScored`. Any unscored side sets `incomplete` and returns `Err` (`reasoning_compare.rs:265`). `remove_file` runs before every write (`reasoning_compare.rs:217`). | closed |
| T-316-06 | Spoofing | partial download counted as present | medium | mitigate | `snapshot_complete` (`detect.rs:149`) requires `config.json`, `tokenizer.json`, and every indexed shard to be `is_file()`. `shard_is_file` (`detect.rs:177`) rejects absolute names and `..`. A dangling symlink fails `is_file()`. | closed |
| T-316-07 | Denial of service | huge or malformed manifest/index, hostile `base_model` path | medium | mitigate | Index and manifest reads use `read_utf8_path_capped` (`detect.rs:155`, `detect.rs:192`). Parse uses `get().and_then()` and maps failure to an absence reason. `base_model` is only `is_dir` then `canonicalize` (`detect.rs:202`). | closed |
| T-316-08 | Tampering | `--base` text used to build the default output path (traversal) | medium | mitigate | `report_file_name` (`reasoning_bench.rs:195`) keeps the last `/` segment, replaces characters outside `[A-Za-z0-9._-]`, and rewrites a leading `.`. The default out path joins that name under `reasoning_compare/` (`reasoning_compare.rs:325`). | closed |
| T-316-09 | Information disclosure | a 32-item slice read as a full benchmark result | low | mitigate | `compared_report_json` sets `scope_note` and `slice_size` (`reasoning_bench.rs:342`). | closed |
| T-316-10 | Elevation of privilege | two 8B models resident at once | low | mitigate | `score_side_with_backend` loads one directory, scores, then `unload_model` (`reasoning_compare.rs:203`) before return. The ready path scores the base, then the adapter (`reasoning_compare.rs:247`). | closed |
| T-316-SC | Tampering | npm/pip/cargo installs | low | accept | No package installed. Phase file list and `tech-stack.added` add no dependency and no crate edge. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-316-SC | T-316-SC | This phase installs nothing and adds no crate dependency. The plan accepts that supply-chain row. | phase 316 plan | 2026-10-28 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-28 | 11 | 11 | 0 | security auditor (ASVS L1) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-28
