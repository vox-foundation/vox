---
last_mapped_commit: 9a79f708de1ce8f8b3944dc99afb51a2877fb960
last_mapped_at: 2026-09-22
---
# Codebase Concerns

**Analysis Date:** 2026-09-22

## Tech Debt

**God-object / oversized files (systemic, ratcheted but large in volume):**

- Issue: `vox-arch-check`'s God Object detector (hard max ~500 non-blank lines / ~12 entities per file) currently reports 549 `severity: error` findings in the last captured scan (`contracts/reports/scaling-audit/findings-latest.json`, stale snapshot dated 2026-06-06 but the crate it profiles — then `vox-clavis`, now renamed `vox-secrets` — still has matching file sizes, e.g. `crates/vox-secrets/src/spec/registry/missing.rs` is 2,780 lines, `crates/vox-secrets/src/backend/vox_vault.rs` is 2,059 lines).
- Files: worst offenders by raw line count include `crates/vox-research-events/src/schema_types.generated.rs` (10,016 lines, generated), `crates/vox-compiler/src/eval/builtins.rs` (3,311), `crates/vox-secrets/src/spec/registry/missing.rs` (2,780), `crates/vox-arch-check/src/main.rs` (2,300), `crates/vox-orchestrator/src/runtime.rs` (2,247), `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` (2,247), `crates/vox-codegen-ts/src/hir_emit/mod.rs` (2,225), `crates/vox-orchestrator-mcp/src/dispatch.rs` (2,199).
- Impact: large files are hard to review safely (see AGENTS.md's own "move + reformat = silent duplicate definition" merge-hazard note) and slow onboarding; 17 files/globs are permanently exempted in `contracts/toestub/suppressions.v1.json` ("Legacy: pending decomposition", `owner: "robot"`), meaning the debt is acknowledged but not scheduled.
- Fix approach: split by domain per file (the suppression reasons already point at the natural seams, e.g. `vox-secrets/src/spec.rs` → domain-specific registry modules); re-run `cargo run -p vox-arch-check` and regenerate `contracts/reports/scaling-audit/findings-latest.json` to get a current (non-stale) baseline before prioritizing.

**Duplicated dispatch logic across mesh crates:**

- Issue: `crates/vox-populi/src/transport/handlers/dispatch.rs` (497 lines) and `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs` (415 lines) are near-identical forks — same function bodies, same `// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="mesh-phase6" canonical="vox_mesh_transport::InterpExecutor"` marker in both, differing mainly by an extra `mesh_store.put_dispatch_result` async fire-and-forget block and a `#[cfg(feature = "transport")]` gate in the `vox-populi` copy.
- Files: `crates/vox-populi/src/transport/handlers/dispatch.rs`, `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs`.
- Impact: a fix or behavior change applied to one copy (e.g. the dispatch-result mesh-store persistence) can silently not apply to the other; this is exactly the "fork we've been through 100+ line chunks" case AGENTS.md's Dependency Discipline section calls out.
- Fix approach: per AGENTS.md §Dependency Discipline, either consolidate behind a shared `-core` crate or, if truly meant to diverge post mesh-phase6, drop the identical deprecation marker from one side once retired.

**Deprecation markers due before next minor bump:**

- Issue: three `vox-deprecated-since="0.6.0" retire-by="0.7.0"` markers exist while the workspace is pinned at `version = "0.6.0"` (`Cargo.toml [workspace.package]`), so `vox ci retirement-audit` will start failing the moment the workspace version is bumped to 0.7.0 unless these are cleaned up first.
- Files: `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs:237`, `crates/vox-populi/src/transport/handlers/dispatch.rs:261` (mesh-phase6 → `vox_mesh_transport::InterpExecutor`), `crates/vox-cli/src/commands/chat.rs:155` and `crates/vox-hf-layout/src/lib.rs:100` (Qwen 2.5 → Qwen 3 retirement).
- Impact: a routine version bump becomes a blocked CI gate (`vox-cli-ci/src/retirement_audit.rs`) unless the vestigial code is removed in the same cycle.
- Fix approach: complete the mesh-phase6 migration to `InterpExecutor` and the Qwen 2.5→3 call-site removal before or alongside the 0.7.0 bump; see `crates/vox-cli-ci/src/retirement_audit.rs`.

**Error-swallowing hotspots:**

- Issue: `let _ = ...;` / `.ok();` / `Err(_) => ...` discard patterns cluster in a handful of files, meaning failures there are silently dropped rather than logged or propagated.
- Files (highest counts): `crates/vox-journal/src/file.rs` (48), `crates/vox-compiler/src/typeck/checker/expr.rs` (25), `crates/vox-compiler/src/parser/descent/decl/head_fn.rs` (22), `crates/vox-workflow-runtime/src/file_journal.rs` (20), `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` (19), `crates/vox-cli/src/commands/ci/runner_scale.rs` (17), `crates/vox-orchestrator-mcp/src/tool_images.rs` (16), `crates/vox-config/src/inference.rs` (16).
- Impact: journal/workflow-runtime files back durable execution (ADR-019/021/041) — a swallowed write failure there can silently desync the durability journal from actual state.
- Fix approach: audit each discard site in `vox-journal` and `vox-workflow-runtime` specifically (durability-critical paths) and convert unconditional discards to at least a `tracing::warn!` with context; the rest are lower priority (parser/typeck error-recovery paths are more often intentional).

## Known Bugs

**No open bug reports found in source comments.** No `FIXME`/`HACK`/`XXX` markers describe an active, unresolved defect distinct from the `TODO` planning notes counted under Tech Debt (171 total `TODO|FIXME|HACK|XXX` hits across `crates/**/*.rs`, concentrated in `crates/vox-scientia/src/manuscript/scaffold/render.rs` (24), `crates/vox-scientia/src/manuscript/latex/render.rs` (18), `crates/vox-gamify/src/quest_engine.rs` (9), `crates/vox-cli/src/commands/plugin/scaffold.rs` (8)). Treat these as scaffold/feature-completion notes, not bug reports — verify per-file before relying on this.

## Security Considerations

**Direct `std::env::var` reads for API keys/tokens outside `vox-secrets`:**

- Risk: AGENTS.md §Secret Management mandates all secret reads go through `vox_secrets::resolve_secret(...)`; direct `std::env::var("...KEY"/"...TOKEN"/"...SECRET")` calls bypass Clavis's resolution precedence, doctor diagnostics, and audit trail (`vox ci secret-env-guard`, `vox ci secrets-parity`).
- Files: `crates/vox-actor-runtime/src/llm/types.rs`, `crates/vox-actor-runtime/src/builtins/mod.rs`, `crates/vox-cli-ci/src/mens_scorecard.rs`, `crates/vox-cli/src/commands/ci/run_body_helpers/guards.rs`, `crates/vox-cli/src/commands/gui/drive.rs`, `crates/vox-db/src/config.rs`, `crates/vox-gui/src/drive/bridge.rs`, `crates/vox-gui/src/drive/flags.rs`, `crates/vox-ml-cli/src/commands/ai/generate.rs`, `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`, `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`, `crates/vox-orchestrator-mcp/src/llm_bridge/infer.rs`, `crates/vox-orchestrator-mcp/src/agent_tools.rs`, `crates/vox-orchestrator/src/models/policy.rs`, `crates/vox-orchestrator/src/models/select.rs`. Some of these are the detectors themselves (`crates/vox-code-audit/src/detectors/{unregistered_llm_env,retired_env_var,env_secret_shape,secrets}.rs`) — expected to reference env-var patterns as scan targets, not violations.
- Current mitigation: `vox-code-audit` ships detectors (`unregistered_llm_env`, `env_secret_shape`, `secrets`) that should catch new violations in CI; `vox ci secret-env-guard` / `vox ci secrets-parity` exist per AGENTS.md.
- Recommendations: run `cargo run -p vox-cli -- ci secret-env-guard` against this file list to confirm which hits are guard-suppressed vs. genuinely unmigrated call sites, and migrate the non-detector files to `vox_secrets::resolve_secret(...)`.

**Transport crypto provider duplication (documented, unresolved):**

- Risk: the resolved dependency graph carries two TLS/QUIC crypto providers simultaneously — `ring` 0.17.14 (workspace `rustls`/`reqwest 0.12` pin) and `aws-lc-rs` 1.17.0 (dragged in by `chromiumoxide` and `gix`→`jj-lib`→`vox-vcs`, both on `reqwest 0.13`) — per `contracts/crypto/transport-providers.v1.json`. Two independent crypto implementations in the trusted supply chain widen the audit surface versus a single vetted provider.
- Files: `contracts/crypto/transport-providers.v1.json` (ledger); root `Cargo.toml:227` (the `reqwest 0.12` pin that anchors the split, consumed by 28+ first-party crates).
- Current mitigation: the ledger requires reviewer sign-off (`brbrainerd`, reviewed 2026-09-04) for any provider add/remove/bump; `vox ci crypto-provider-check` is planned to verify *built* (not just resolved) providers per target/feature set.
- Recommendations: per the ledger's own analysis, collapsing to one provider requires a workspace-wide `reqwest 0.12` → `0.13` pin bump (which also forces `reqwest-middleware`/`reqwest-retry` major bumps) — tracked as its own task, not part of any completed migration. Do not attempt a partial fix (e.g. bumping one crate) without redoing the full `cargo tree -i <crate> -e features --workspace` verification the ledger documents.

**`unsafe` block usage — mostly Rust-2024-mandated `env::set_var` wrapping, not FFI risk:**

- Risk: 528 `unsafe fn|{|impl` occurrences across `crates/*/src/`; spot-checking the top files (`crates/vox-secrets/src/tests.rs` 26 hits, `crates/vox-plugin-speech/src/oratio_internals/acoustic_preprocess.rs` 12 hits) shows these are Rust 2024 edition's now-mandatory `unsafe { std::env::set_var(...) }` wrapper in test code guarded by a `TEST_LOCK`/`ENV_LOCK` mutex, not FFI or memory-unsafe code. Not independently verified for the full 528-count list.
- Files: highest-count files are `crates/vox-secrets/src/tests.rs`, `crates/vox-orchestrator/src/models/select.rs`, `crates/vox-config/src/inference.rs`, `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/tests.rs`, `crates/vox-orchestrator/src/config/tests.rs`, `crates/vox-telemetry/src/config.rs`, `crates/vox-secrets/src/backend/vox_vault.rs`.
- Recommendation: if auditing `unsafe` usage as a security task, filter out `env::set_var`/`env::remove_var` wrapper occurrences first (grep for `unsafe {\s*$` followed by `std::env::` on the next line) to isolate genuine FFI/pointer `unsafe` blocks (e.g. `vox_vault.rs`, `sherpa_model_config.rs`, `acoustic_preprocess.rs` DSP code) from edition-mandated noise.

## Performance Bottlenecks

**No profiled/measured bottleneck found in this pass.** No benchmark output, flamegraph, or perf-regression report was located in the repo root or `contracts/`. If performance work is planned, first check `contracts/budgets/test-tier-budgets.v1.yaml` (CI-tier time budgets — a proxy for build/test slowness, not runtime perf) and `vox ci tier-budget-check` output, since these are the only performance-adjacent artifacts this scan surfaced.

## Fragile Areas

**Durability journal / workflow-runtime file I/O:**

- Files: `crates/vox-journal/src/file.rs` (71 `.unwrap()/.expect()/panic!` + 48 discarded-error patterns), `crates/vox-workflow-runtime/src/file_journal.rs` (20 discarded-error patterns).
- Why fragile: this is the durable-functions journal contract backing ADR-019/021/041; a panic or silently-swallowed I/O error here can corrupt or desync the on-disk journal state that `actor`/`workflow`/`activity` durability guarantees depend on.
- Safe modification: any change to `crates/vox-journal/src/file.rs` should preserve or improve error propagation (convert `.unwrap()`/`let _ =` to explicit `Result` handling with `tracing::error!`) rather than adding new discard sites; changes should be checked against the durability-parity tests referenced by ADR-021.
- Test coverage: not independently verified in this pass — confirm `vox-journal` and `vox-workflow-runtime` have journal-corruption / partial-write test cases before modifying.

**God-object files under active suppression:**

- Files: the 17 paths listed in `contracts/toestub/suppressions.v1.json` under `rule_id_prefix: "arch/god_object"`, notably `crates/vox-secrets/src/spec.rs`, `crates/vox-cli/src/commands/harness/eval.rs` (617+ non-blank lines), `crates/vox-gui/src/commands/daemon.rs` (525+ non-blank lines), `crates/vox-plugin-mens-candle-core/src/**` (whole-directory suppression).
- Why fragile: these files are large enough that the arch-check detector would otherwise block them; large files increase the odds of the "move + reformat = silent duplicate definition" merge hazard AGENTS.md's Perennial Bug Patterns section documents, since reviewers are less likely to read every hunk of a 500+ line diff.
- Safe modification: prefer additive changes; if touching one of these files substantially, consider splitting it as part of the same change rather than growing the suppression further.
- Test coverage: not assessed per-file in this pass.

## Scaling Limits

**Not assessed in this pass.** No resource-capacity documentation (connection pool limits, queue depth limits, per-tenant quotas) was located in `contracts/` during this scan; a dedicated pass over `crates/vox-orchestrator-queue/`, `crates/vox-mesh-transport/`, and `contracts/budgets/` would be needed to characterize this.

## Dependencies at Risk

**`reqwest` split across two incompatible majors (0.12 and 0.13):**

- Risk: the workspace resolves both `reqwest 0.12.28` (root pin, 28+ first-party crates + `nanopub`, `tavily`, `reqwest-middleware 0.4`, `reqwest-retry 0.7`) and `reqwest 0.13.4` (via `chromiumoxide` and `gix-transport`←`jj-lib`←`vox-vcs`) simultaneously — see `contracts/crypto/transport-providers.v1.json` `duplicates` section, `reason` field, for the full dependency chain.
- Impact: doubled compiled TLS-stack surface (see Security Considerations above), larger binary/build footprint, and blocks collapsing to a single crypto provider.
- Migration plan: per the ledger, a workspace-wide `reqwest 0.12` → `0.13` pin bump is required (cascades to `reqwest-middleware`/`reqwest-retry` majors); explicitly **not** achieved by the completed hf-hub 1.0 port (2026-09-04, commit `ca7ce2023`), which removed `ureq` but left the reqwest split untouched since the root manifest pin — not hf-hub — anchors the 0.12 side.

## Missing Critical Features

**Not assessed — outside the scope of a structural/durable-issues scan.** Feature gaps are better sourced from `ROADMAP.md` / open GitHub issues (`gh issue list`) than from static code inspection.

## Test Coverage Gaps

**Suppressed test-first policy exceptions:**

- What's not tested: `contracts/toestub/suppressions.v1.json` records one `skeleton/untested-pub-api` suppression — `crates/vox-codegen/src/codegen_rust/emit/mod.rs`'s `generate()` and `emit_cargo_toml()` — justified as exercised by sibling `#[cfg(test)]` modules and out-of-file integration tests (`tests/generated_project_builds_outside_repo.rs`, `tests/ai_fixture_bundle_compiles.rs`) rather than an inline `#[test]` in the same file, which is what the `tdd-guard` pre-commit hook checks for.
- Files: `crates/vox-codegen/src/codegen_rust/emit/mod.rs`.
- Risk: low — the suppression reason documents an established split-test convention for this crate, not an actual coverage gap; flagged here only because AGENTS.md's Test-First Policy is otherwise a hard commit-blocking gate and any suppression is worth periodic re-justification.
- Priority: Low.

**Ignored tests:**

- What's not tested: 85 files carry at least one `#[ignore]` test attribute (215 total `#[ignore]` occurrences across `crates/`).
- Files: not enumerated individually in this pass — run `grep -rn '#\[ignore' --include="*.rs" crates/` to get the full list before relying on a specific count.
- Risk: an ignored test represents a known gap in the CI-verified surface (e.g. slow/flaky/environment-dependent tests); without per-test reasons captured here, it's not possible to distinguish "intentionally slow, covered by `--include-slow`" from "known-broken, silently skipped."
- Priority: Medium — worth a follow-up pass to confirm every `#[ignore]` carries a reason comment and is covered by the `--include-slow` local-CI tier per `docs/src/contributors/local-ci-pre-push.md`.

---

*Concerns audit: 2026-09-22*
