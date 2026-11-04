---
phase: "316"
slug: "before-after-comparison-bench-01"
status: verified
verdict: SECURED
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-26"
---

# Phase 316 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `block_on` is `high`, as the plan's threat model states. The plan-time register has eleven rows, five of them high (T-316-01 to T-316-05). The security auditor checked the code at `6044b8a56` against every row on 2026-10-26. That is after the review fixes: CR-01 `b15479690`, WR-05 `a47ca1453`, WR-06 `3d515f62c`, WR-01 `0166ad161`. All eleven rows are closed. No implementation file was changed for this audit.

Tests the auditor ran: `cargo nextest run -p vox-ml-cli --features gpu --lib reasoning_compare` passed 33 of 33, and `cargo nextest run -p vox-eval --lib reasoning_bench` passed 23 of 23. The temp volume on this host is case-insensitive, so the macOS case-variant test really runs its alias check instead of returning early. The real record at `mens/runs/reasoning_compare/Qwen3-8B-b968826d9c46.json` has status `not_compared`, `gate_input: false`, and no `accuracy` or `think_initiation` text. The mutation evidence comes from the executor (316-01-SUMMARY, eight mutations) and the fixer (316-REVIEW-FIX, CR-01 and WR-05). The auditor did not re-run those mutations. The present branch is proven only with a fake scorer. A green test is not a model score.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI args → command | `--base`, `--adapter`, `--hub-root`, `--bench-root`, `--out` are untrusted text and paths | Pin text, paths |
| environment → hub root | `HF_HUB_CACHE`, `HUGGINGFACE_HUB_CACHE`, `HF_HOME`, `XDG_CACHE_HOME`, home dir pick the cache root | Layout paths, not secrets |
| local filesystem → detection | HF cache tree, `model.safetensors.index.json`, `adapter_manifest.json` may be partial, stale, or hostile | Index and manifest JSON, file existence |
| model backend → scorer | `load_model` / `run_inference` results enter `score_loaded` | Completions |
| command → files serve reads | The report must never be read by `vox mens serve` as a pass | Report JSON |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation (evidence) | Status |
|-----------|----------|-----------|----------|-------------|------------------------|--------|
| T-316-01 | Spoofing | report read by serve as a pass | high | mitigate | `check_out_path` is called once, from the single entry `run_eval_reasoning_compare_with` (`reasoning_compare.rs:342`), before detection or any write. The only callers are the dispatch arm (`dispatch.rs:751`) and tests. It refuses a symlink `--out` (`:103`) and the gate names `collateral_damage_report.json` and `reasoning_report.json` (`:28`, `:107-114`). It refuses any `collateral_eval` component (`:115-122`), checking both names case-insensitively on the given and the resolved path. It also refuses anything that resolves under the hub root, the bench root, or the adapter dir (`:124-135`). After CR-01, `resolved` (`:70-93`) canonicalizes every existing prefix, so symlinks, `..`, and on-disk letter case are seen. Neither report shape has a `benchmarks`, `reports` or `reasoning_slices` key (`reasoning_bench.rs:278-298`, `:330-341`). Serve code was not changed: the `dispatch.rs` diff over `4bf115518..HEAD` adds only the arm. Tests `out_named_like_a_gate_file_is_refused`, `out_inside_hub_root_or_adapter_is_refused`, `out_through_a_symlink_into_the_snapshot_is_refused`, `out_with_a_case_variant_of_the_hub_root_is_refused` and `no_gate_files_are_created` all go through the entry. SUMMARY mutation 1 deletes the call site. | closed |
| T-316-02 | Spoofing | another revision or size scored as the pinned base | high | mitigate | `BaseLabel::from_pin` keeps a revision only when it is exactly 40 lowercase hex characters (`reasoning_bench.rs:180-185`). `snapshot_dir` joins that exact revision and never lists `snapshots/` (`detect.rs:126-134`). No revision gives `PinUnverifiable` (`detect.rs:159`). At the entry, `resolve_base_pin` (`detect.rs:89-123`, called at `reasoning_compare.rs:333`) returns `Err` for unknown or ambiguous input before anything is written. That is stricter than the plan's `pin_unverifiable`. Tests: `other_revision_is_not_the_pinned_snapshot`, `hub_with_only_qwen3_0_6b_is_absent_for_the_8b_pin`, `pin_without_40_hex_revision_is_unverifiable`, `base_resolution_unknown_or_ambiguous_is_an_error`, `cli_entry_records_the_resolved_pin`, `unresolvable_base_is_an_error_and_writes_nothing`. SUMMARY mutations 2 and 3. | closed |
| T-316-03 | Spoofing | adapter trained on another base | high | mitigate | The manifest `base_model` is read with `get().and_then(as_str)`. It must be an existing dir that canonicalizes equal to the detected snapshot (`detect.rs:178-184`, `same_dir` `:199-201`). Otherwise the reason is `AdapterBaseNotPinnedSnapshot`. A malformed manifest gives `AdapterIncomplete` (`:175-177`). Test `adapter_manifest_base_must_be_the_pinned_snapshot` covers another dir, a Hub id, a non-string, a missing key, malformed JSON, and a canonically equal roundabout path. SUMMARY mutation 4. | closed |
| T-316-04 | Tampering | writing through HF cache symlinks or into adapter `collateral_eval/` | high | mitigate | A grep finds no `score_reasoning_file` or `write_outcome` in either `reasoning_compare` file. `score_slices_loaded` (`reasoning_compare.rs:249-261`) drops the rows. `score_loaded` (`reasoning_eval.rs:184-234`) does no filesystem I/O, and its diff is three visibility lines. The only write is `write_report` to the guarded `--out` (`:140-147`). A symlink `--out` is refused (`:103`). An existing file that is not a prior record is refused and left in place (`remove_stale_record` `:152-170`, from WR-05), so a write can never go through an existing link. Hub root and adapter dir containment is decided on resolved paths (CR-01). | closed |
| T-316-05 | Repudiation | invented, misattributed, or stale score | high | mitigate | `AbsenceRecord::to_json` has no `n` and no score key (`reasoning_bench.rs:271-299`). It is locked by the recursive walk in `absence_record_has_no_score_keys_anywhere` and SUMMARY mutation 6. `pick` matches outcomes to slices by name. A load error or a missing outcome becomes `not_scored` (`reasoning_compare.rs:208-220`). `compared` requires every slice once, with both sides `Scored` (`reasoning_bench.rs:316-320`). `incomplete` writes the report and then returns `Err` (`reasoning_compare.rs:228-243`). `remove_stale_record` runs before either branch writes (`:180`). `compared_report_json` records the base pin and `adapter.dir` (WR-04). Tests: `stale_compared_report_is_replaced`, `existing_file_that_is_not_a_prior_record_is_refused_and_kept`, `scorer_outcomes_are_matched_by_name`, `slice_load_failure_is_not_scored_on_both_sides`, `one_not_scored_side_returns_err_and_writes_incomplete`. | closed |
| T-316-06 | Spoofing | partial download counted as present | medium | mitigate | `snapshot_complete` (`detect.rs:207-235`) requires `config.json` and `tokenizer.json` to be files. It requires every distinct `weight_map` value to be a single plain segment that `is_file()` (which follows links, so a dangling link is false), or `model.safetensors` when there is no index. Tests: `snapshot_missing_tokenizer_is_incomplete`, `snapshot_with_dangling_shard_link_is_incomplete`, `snapshot_with_all_indexed_shards_is_present` (including a `../escape` entry), `snapshot_with_single_safetensors_is_present`. | closed |
| T-316-07 | Denial of service | huge or malformed manifest/index, hostile `base_model` | medium | mitigate | The index and manifest are read only through `read_json` → `vox_bounded_fs::read_utf8_path_capped` (`detect.rs:193-196`). Parsing uses `get().and_then()`. There is no `unwrap`, `expect` or `panic!` above `#[cfg(test)]` in either new file. `base_model` gets only `is_dir` and `canonicalize`. Every failure maps to an absence reason. See residual note 1. | closed |
| T-316-08 | Tampering | `--base` text in the default output path | medium | mitigate | `report_file_name` keeps the last `/` segment, replaces every character outside `[A-Za-z0-9._-]` and a leading `.` with `_`, and slices the already-validated 40-hex revision (`reasoning_bench.rs:198-217`). `report_file_name_is_per_base_and_path_safe` covers `../evil@main`, `..@main`, `a/..@main`, `x/.hidden`, `a\..\b`. The resulting path still goes through `check_out_path`. | closed |
| T-316-09 | Information disclosure | 32-item slice read as a full benchmark | low | mitigate | The compared report carries `scope_note` (`REASONING_SLICE_SCOPE_NOTE`) and `slice_size` (`reasoning_bench.rs:333-334`). Asserted in `compared_report_has_separate_keys_per_slice_and_side`. | closed |
| T-316-10 | Elevation of privilege | two 8B models resident at once | low | mitigate | `backend_scores` calls `load_model` once, then calls `unload_model` before it returns (`reasoning_compare.rs:305-319`). The base and adapter sides run one after the other (`:204-205`). If no slice loaded, no model is loaded (`:200-201`). See residual note 6. | closed |
| T-316-SC | Tampering | npm/pip/cargo installs | low | accept | `git diff --stat 4bf115518..HEAD -- Cargo.lock '**/Cargo.toml'` is empty, so no package and no crate edge was added. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Residual Notes (non-blocking)

These are outside the register or beyond L1 depth. None reopens a row. They are listed so later phases can pick them up.

1. **T-316-07, L2 gap.** `model.safetensors.index.json` is read without an `is_file()` check (`detect.rs:212-215`). `read_utf8_path_capped` checks `metadata().len()` and then calls `fs::read`. A FIFO, or a symlink to `/dev/zero`, in a hostile local snapshot reports length 0, so the read would hang or exhaust memory. The manifest is safe because the pair check needs `is_file()` first. Fix: require `index.is_file()`, or read through `Read::take`.
2. **Serve reads `gate_receipt.json`** (`dispatch.rs:905-938`). That name is not in `GATE_FILE_NAMES`, and the name check folds ASCII case only. Neither report shape can satisfy a serve pass condition: there is no `status: "pass"` (`dispatch.rs:851`), no `scored` plus `n` 32 at top level (`:883-893`), and no `overall_passed` (`:920`). A guard miss therefore fails closed.
3. An `--out` inside an HF cache other than the resolved hub root can create a new file there. It cannot replace an existing file or write through a link.
4. Early errors exit non-zero before any write: an unresolvable `--base`, no hub root, or an overlay load failure. In those cases a prior record at `--out` stays. That record still names the base and adapter it measured.
5. A local check-then-write race exists between `check_out_path` and `write_report`. This is a single-operator CLI.
6. **T-316-10.** An `unload_model` failure only prints a warning (`reasoning_compare.rs:317-319`), so the adapter side then loads with the base possibly still resident.
7. IN-04, Windows only: a `\` in the `--base` repo id acts as a path separator in `snapshot_dir`. It was deferred in 316-REVIEW-FIX. WR-03 (deferred) is a false negative and fails safe.

## Unregistered Flags

316-01-SUMMARY has no `## Threat Flags` section. One piece of attack surface has no threat mapping (WARNING, not a blocker). Three out-of-plan commits are part of this phase's branch: `4bf115518` (four vox-code-audit detector modules), `35dd5d679` (nine vox-orchestrator modules copied in) and `09a012bcc` (a `guest_share.rs` key and MAC change). They were left out of 316-REVIEW and are not in this register. They should get their own review and threat rows. They are not BENCH-01 work.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-316-SC | T-316-SC | This phase installs nothing and adds no crate dependency. `Cargo.lock` and every `Cargo.toml` are unchanged over the phase range. | phase 316 plan | 2026-10-26 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-26 | 11 | 11 | 0 | security auditor (ASVS L1) at `6044b8a56` |

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
| Threats found | 11 |
| Closed | 11 |
| Open | 0 |
| Unregistered flags | 1 |
