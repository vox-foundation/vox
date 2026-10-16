# Mens Track Code Review (Phases 77–78, 85–88)

**Reviewed:** 2026-10-16 · inline, standard depth (no subagents) · scope: files listed in the phase SUMMARY.md files.

## Findings & fixes (`--fix` applied)

| # | Severity | Finding | Status |
|---|---|---|---|
| 1 | Warning | `contracts/reports/mens-flywheel-burnin.v1.json` was hand-written with fabricated URIs (`trusty://blake3/...`, per-cycle snapshot URIs, `batch_count: 1`) that did not match the real burn-in output | **Fixed** — replaced with the values observed from `run_two_cycle_autonomous_burnin` (`trusty://xxh3/...`, shared snapshot URI, `batch_count: 6`) |
| 2 | Warning | SSoT doc misstated `OracleReceipt`/`RecordProvenance`/checkpoint fields and claimed 13-gram/8-gram/`0.85` dedup constants and `blake3` snapshot URIs | **Fixed** — corrected against source; doc lint passes |
| 3 | Warning | `persist_burnin_report_to_db` / `load_burnin_report_from_db` (`LOOP-02`) have no DB round-trip test (`vox-corpus` has no `tokio` dev-dep; the pure-JSON round-trip test does not exercise CAS) | **Open** — needs a dev-dependency change (outside the stop-conditions allowed here) |
| 4 | Info | `VerifiedRepairDpoRecord` / `pair_verified_repair_with_failure` (`AUDIT-02`) duplicates `MintedDpoPairV2` / `pair_verified_repair_to_dpo_and_sft` in the same file | **Open** — candidate for removal (YAGNI) |
| 5 | Info | `SpokeRegressionRatchetV1` (`AUDIT-04`) uses `String` spoke keys and is not yet wired into the `regression-budget` gate run | **Open** |
| 6 | Info | Burn-in report JSON is still hand-maintained; it should be emitted by a generator/test to avoid drift | **Open** |

## Verification
- Doc lint after fixes: see command output in the conversation (`no hard errors`).
- Earlier green runs: `vox-corpus` 231, `vox-eval` 109, `vox-audit` 102 passed.
