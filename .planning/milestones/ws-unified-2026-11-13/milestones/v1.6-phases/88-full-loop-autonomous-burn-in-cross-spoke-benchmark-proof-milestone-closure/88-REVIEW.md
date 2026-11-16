# Mens Track Code Review (Phases 77–78, 85–88)

**Reviewed:** 2026-10-16 · inline, standard depth (no subagents) · scope: files listed in the phase SUMMARY.md files.

## Findings & fixes (`--fix` applied)

| # | Severity | Finding | Status |
|---|---|---|---|
| 1 | Warning | `contracts/reports/mens-flywheel-burnin.v1.json` was hand-written with fabricated URIs (`trusty://blake3/...`, per-cycle snapshot URIs, `batch_count: 1`) that did not match the real burn-in output | **Fixed** — replaced with the values observed from `run_two_cycle_autonomous_burnin` (`trusty://xxh3/...`, shared snapshot URI, `batch_count: 6`) |
| 2 | Warning | SSoT doc misstated `OracleReceipt`/`RecordProvenance`/checkpoint fields and claimed 13-gram/8-gram/`0.85` dedup constants and `blake3` snapshot URIs | **Fixed** — corrected against source; doc lint passes |
| 3 | Warning | `persist_burnin_report_to_db` / `load_burnin_report_from_db` (`LOOP-02`) have no DB round-trip test | **Fixed** — added `burnin_report_persists_and_rehydrates_from_voxdb` in `crates/vox-corpus/src/flywheel.rs` using safe `std::pin::pin!` + `Waker::noop()` against `VoxDb::in_memory()` |
| 4 | Info | `VerifiedRepairDpoRecord` / `pair_verified_repair_with_failure` (`AUDIT-02`) duplicates `MintedDpoPairV2` / `pair_verified_repair_to_dpo_and_sft` in the same file | **Fixed** — delegated `pair_verified_repair_with_failure` through `pair_verified_repair_to_dpo_and_sft` and `AuditSpokeKind::parse_spoke` |
| 5 | Info | `SpokeRegressionRatchetV1` (`AUDIT-04`) uses `String` spoke keys and is not yet wired into the `regression-budget` gate run | **Fixed** — added `SpokeRegressionRatchetV1::canonical_five_spoke_floors` and wired `contracts/reports/mens-spoke-scorecard.v1.json` verification into `scan_workspace` in `crates/vox-audit/src/subcommands/regression_budget.rs` |
| 6 | Info | Burn-in report JSON is still hand-maintained; it should be emitted by a generator/test to avoid drift | **Fixed** — added `MensFlywheelBurninReportV1::to_contract_summary_json` and `burnin_report_matches_committed_contract_json` drift test in `crates/vox-corpus/src/flywheel.rs` |

## Verification
- Doc lint after fixes: see command output in the conversation (`no hard errors`).
- Test suites green: `vox-corpus` (including `--features database` CAS round-trip and contract JSON parity), `vox-eval`, and `vox-audit`.
