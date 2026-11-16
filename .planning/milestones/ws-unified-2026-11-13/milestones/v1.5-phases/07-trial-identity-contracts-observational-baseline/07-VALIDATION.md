---
phase: "7"
slug: "trial-identity-contracts-observational-baseline"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-12"
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `07-RESEARCH.md` §Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]`/`#[tokio::test]` under cargo-nextest; `wiremock`/`serial_test` for the few HTTP and environment tests |
| **Config file** | `.config/nextest.toml` |
| **Quick run command** | `cargo nextest run -p <crate> -E 'test(<filter>)'`, backgrounded, with output to the scratchpad |
| **Full suite command** | hosted CI `ci-gate` (`vox ci pre-push --complete` locally; executors do not run it) |
| **Estimated runtime** | ~60–180 s per crate filter on a warm cache. A fresh binary can add ~13 min while syspolicyd checks it on macOS |

---

## Sampling Rate

- **After every task commit:** `cargo check -p <touched crates>` plus that task's new tests (backgrounded)
- **After every plan wave:** all new tests from the wave, one nextest filter per crate
- **Before `/gsd-verify-work`:** hosted CI green, with all nine mutation proofs recorded in SUMMARY files
- **Max feedback latency:** 180 seconds (excluding the syspolicyd first-link stall)

---

## Per-Task Verification Map

Task IDs are bound below as `<plan>-T<task>`. The commands were corrected so their filters match the planned test names, because nextest `test(...)` matches substrings of names like `trial::runner::tests::runner_refuses_...`.

| Task ID | Requirement | Behavior | Test Type | Automated Command | File Exists | Status |
|---------|-------------|----------|-----------|-------------------|-------------|--------|
| 07-05-T1, 07-05-T2 | TRIAL-02 | signing freezes; flipping any field or `campaign_id` fails; NaN fails; supersession links | unit | `cargo nextest run -p vox-research-trial-core -E 'test(gate::) \| test(campaign::)'` | ❌ W0 (created by the task) | ⬜ pending |
| 07-05-T3, 07-07-T2 | TRIAL-03 | illegal campaign and run transitions are rejected (pure table + DB compare-and-set) | unit | `cargo nextest run -p vox-research-trial-core -E 'test(lifecycle::)'`; `cargo nextest run -p vox-db --lib -E 'test(trials::tests::)'` | ❌ W0 | ⬜ pending |
| 07-05-T2, 07-09-T2 | TRIAL-05 | gate and runner entry refuse foreign, unsigned and tampered campaigns | unit | `cargo nextest run -p vox-research-trial-core -E 'test(gate::)'`; `cargo nextest run -p vox-cli-research --lib -E 'test(runner_refuses_)'` | ❌ W0 | ⬜ pending |
| 07-01-T1 | TRIAL-05 | canonical bytes ignore key order at every depth, reject NaN, match golden bytes | unit | `cargo nextest run -p vox-crypto -E 'test(canonical::)'` | ❌ W0 | ⬜ pending |
| 07-01-T1 | TRIAL-05 | orchestrator Trusty URI is unchanged after the rewire (golden captured first) | unit | `cargo nextest run -p vox-orchestrator --lib -E 'test(preregistration::)'` | ✅ (+2 golden tests) | ⬜ pending |
| 07-02-T1, 07-02-T2 | TRIAL-05 (D-11) | two SecretIds registered, signing key vault-only in strict mode, parity green | unit + CI gate | `cargo nextest run -p vox-secrets --lib -E 'test(research_trial_)'`; `vox ci secrets-parity`; `vox ci secret-env-guard` | ❌ W0 | ⬜ pending |
| 07-10-T1 | TRIAL-01 | identity is consistent across the pipeline label, session key, `trial_*` rows and `research_trial.*` rows | integration | `cargo nextest run -p vox-cli-research --lib -E 'test(identity_consistent_across_surfaces)'` | ❌ W0 | ⬜ pending |
| 07-10-T3 | TRIAL-04 | an interrupted run is terminalized; the retry is linked with `attempt_index+1` | integration | `cargo nextest run -p vox-cli-research --lib -E 'test(recover_) \| test(retry_)'` | ❌ W0 | ⬜ pending |
| 07-04-T1, 07-04-T3 | STORE-01 | contract DDL executes; human decision recorded | doc + gate | `awk '/^```sql$/{f=1;next} /^```$/{f=0} f' docs/src/architecture/research-trial-tier-a-schema-2026.md \| sqlite3 -bail :memory:` | ❌ W0 | ⬜ pending |
| 07-06-T1, 07-06-T2 | STORE-01 | baseline 96 digest matches policy; the version-stamped-without-tables guard is loud | unit | `cargo nextest run -p vox-db --lib -E 'test(baseline_policy_matches_compiled_schema) \| test(trials_tables_exist_after_open) \| test(stamped_version_)'` | ✅ / ❌ | ⬜ pending |
| 07-06-T1 | STORE-01 | `LEGACY_EXPORT_TABLES` covers the new tables | unit | `cargo nextest run -p vox-db --lib -E 'test(legacy_export_covers_all_baseline_tables)'` | ✅ | ⬜ pending |
| 07-07-T1, 07-07-T2 | STORE-02 | a mid-write failpoint leaves no partial unit, including under concurrent writers | integration | `cargo nextest run -p vox-db --lib -E 'test(forced_mid_write_failure_leaves_no_partial_run) \| test(concurrent_batches_do_not_interleave)'` | ❌ W0 | ⬜ pending |
| 07-10-T2 | TELEM-01 | the trial runner emits only allowlisted rows; canary text never appears | integration | `cargo nextest run -p vox-telemetry --lib -E 'test(research_trial)'`; `cargo nextest run -p vox-cli-research --lib -E 'test(canary_) \| test(content_bearing_trial_row_is_rejected_at_the_choke_point)'` | ✅ / ❌ | ⬜ pending |
| 07-03-T2, 07-09-T2 | TELEM-01 (D-12) | the legacy content bridge is skipped in trial runs and kept in non-trial runs | integration | `cargo nextest run -p vox-research-shim --lib -E 'test(trial_run_skips_content_metrics_bridge) \| test(non_trial_run_keeps_metrics_bridge)'`; `cargo nextest run -p vox-cli-research --lib -E 'test(trial_config_sets_label_and_bypass_only)'` | ❌ W0 | ⬜ pending |
| 07-03-T1, 07-10-T1 | TELEM-02 | a run shares one trace and label; attempts get distinct session keys | unit / integration | `cargo nextest run -p vox-actor-runtime --lib -E 'test(run_trace)'`; `cargo nextest run -p vox-research-shim --lib -E 'test(trial_label_reaches_llm_telemetry_session)'`; `cargo nextest run -p vox-cli-research --lib -E 'test(two_attempts_same_item_get_distinct_sessions)'` | ✅ + ❌ | ⬜ pending |
| 07-10-T1 | METER-01 | usage and duration reach run scalars; unknown cost is Missing; TTFT is NotApplicable | unit | `cargo nextest run -p vox-cli-research --lib -E 'test(meter_)'` | ❌ W0 | ⬜ pending |
| 07-03-T2, 07-08-T1, 07-08-T2 | SCORE-04 | residual R5/R6/R8; a failed query gets the worst score; v1 schema validates for real | unit | `cargo nextest run -p vox-cli-research --lib -E 'test(eval::tests::)'`; `cargo nextest run -p vox-research-shim --lib -E 'test(judge_error_is_missing)'` | ✅ partial | ⬜ pending |
| 07-08-T3 | TEST-01 | the default lane makes no external network call (egress blocked) | unit | `env HTTPS_PROXY=http://127.0.0.1:9 HTTP_PROXY=http://127.0.0.1:9 ALL_PROXY=http://127.0.0.1:9 NO_PROXY=127.0.0.1,localhost,::1 cargo nextest run -p vox-cli-research -p vox-research-shim -p vox-research-trial-core` | ❌ W0 | ⬜ pending |
| 07-11-T1, 07-11-T2 | BASE-01 | the fixture-run baseline emits every listed metric with explicit Missing; v2 schema validates | integration | `cargo nextest run -p vox-cli-research --lib -E 'test(baseline_)'`; `cargo nextest run -p vox-eval --lib -E 'test(quantile_nearest_rank_edges) \| test(jaccard_index_edges)'` | ❌ W0 | ⬜ pending |
| 07-12-T1 | BASE-01 | draft, sign plus verify, run and report end to end through the CLI functions; no policy change | integration | `cargo nextest run -p vox-cli-research -E 'test(baseline_draft_) \| test(sign_refuses_) \| test(test_eval_)'` | ❌ W0 | ⬜ pending |
| 07-01-T2, 07-05-T2 | Crate edges | only user-approved edges exist | CI gate | `cargo run -q -p vox-cli -- ci crate-edges`; `cargo run -q -p vox-arch-check` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `vox-research-trial-core` crate and its test modules; `vox-crypto/src/canonical.rs` tests
- [ ] `vox-db` trials tests (guard, failpoint, lifecycle CAS) on `DbConfig::Memory`
- [ ] `vox-cli-research` `trial::` test modules, a fake `TrialExecutor`, and an egress guard
- [ ] Fixtures: canned `ResearchResult` archetypes (good, hallucinated-no-evidence, uncited, negation flip, fabricated `[9]`, abstention, failure)

### Mutation proofs

Each proof records a presence count before and after the mutation.

1. Runner calls `verify_campaign` — **07-09-T2**. The bypass mutation passes an allowlist built from the embedded key, which makes `runner_refuses_foreign_signer` fail. Deleting the call outright does not compile, which proves the typestate.
2. Allowlist membership — **07-05-T2** (`foreign_signer_refused`, `empty_allowlist_trusts_nobody`)
3. Trusty-id recompute — **07-05-T2** (`tampered_campaign_id_fails`)
4. Canonical recursion / NaN rejection — **07-01-T1** (`nested_key_order_independent`, `nan_rejected`)
5. Failpoint rollback — **07-07-T1** (`forced_mid_write_failure_leaves_no_partial_run`)
6. Baseline guard arm — **07-06-T2** (`stamped_version_without_tables_errors`)
7. Terminal immutability — **07-07-T2** (`terminal_run_is_immutable`)
8. Telemetry choke point — **07-10-T2** (`content_bearing_trial_row_is_rejected_at_the_choke_point`)
9. Judge-failure Missing — **07-03-T2** (`judge_error_is_missing`)

The manual-only rows map to **07-04-T2** (STORE-01 `checkpoint:decision`) and **07-12-T2** (live baseline `checkpoint:human-verify`).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live observational baseline campaign | BASE-01 | Needs provider keys, network and spend | The user runs `vox research eval --campaign <signed> --baseline` and confirms the results.v2 report records every BASE-01 metric |
| STORE-01 schema approval | STORE-01 | Human gate (D-04) | Review the contract doc from plan 07-04 and approve the run grain and the `model_benchmark_results` option |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 180s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
