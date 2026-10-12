---
phase: 03-extract-misplaced-crates-to-plugin-architecture
plan: 04
status: complete
subsystem: vox-orchestrator-mcp / vox-orchestrator config / webhook intake
tags: [webhook, hopper, D-03, D-04, D-10, D-13, D-14, fail-closed, SC-2]
requires: [03-03]
provides:
  - "`[orchestrator.webhook]` section (`WebhookIntakeConfig { bind_addr, poll_interval_ms, max_events_per_poll }`), default None"
  - "vox-orchestrator-mcp opt-in poller: vox_secrets WebhookIngressToken -> cached_code_plugin(\"webhook\") -> start_listening -> WebhookInbox::poll_events -> HopperIntake::submit(IntakeSource::Webhook)"
  - "Bounded, payload-free webhook intents with plugin-bridge routing re-derived under the defactor rule"
affects: [vox-orchestrator, vox-orchestrator-mcp, docs/src/architecture/where-things-live.md, .planning/ROADMAP.md, .planning/REQUIREMENTS.md]
tech-stack:
  added: []
  patterns: ["presence-gated Option config section", "JSON boundary across the plugin ABI (no crate edge)", "spawn_blocking for dlopen, then tokio interval poll with MissedTickBehavior::Delay"]
key-files:
  created:
    - crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs
    - crates/vox-orchestrator/src/config/webhook_intake.rs
  modified:
    - crates/vox-orchestrator-mcp/src/server_state.rs
    - crates/vox-orchestrator/src/config/mod.rs
    - crates/vox-orchestrator/src/config/orchestrator_fields.rs
    - crates/vox-orchestrator/src/config/impl_default.rs
    - docs/src/architecture/where-things-live.md
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md
decisions:
  - "Environment fact 2b (hollow-fn comments in the daemon-RPC accessors) is superseded by e564ee714. The user restored the status-tool pilot and deleted the dead accessors instead of annotating them."
  - "The ingress token is resolved only via vox_secrets::resolve_secret(SecretId::WebhookIngressToken). A missing or blank token logs an error and fails closed before the plugin loads."
  - "IntakePlan has no Debug impl, so the start config JSON (which carries the token) cannot be formatted by accident."
metrics:
  duration: "~35 min of active work (first attempt 09:09Z stopped at the 2b checkpoint; resumed 10:22Z, finished 10:43Z)"
  completed: 2026-09-26
plan_head_before: e564ee7140ad9924396b604d7e9a67d56ced145b
actuals:
  tokens: 6900
  tasks: 3
  commits: 3
---

# Phase 3 Plan 04: Opt-in webhook -> hopper intake poller Summary

Webhook events accepted by vox-plugin-webhook now reach the orchestrator's hopper. The path runs through the plugin host, is opt-in, is gated by the ingress token, and adds no crate edge to the plugin.
- An `[orchestrator.webhook]` section makes both `ServerState` constructors start a poller. The poller:
  1. resolves `WebhookIngressToken` through vox_secrets;
  2. loads the plugin via `cached_code_plugin("webhook")` inside `spawn_blocking`;
  3. requires the ABI 13 `WebhookInbox` extension;
  4. starts the listener with `{ingress_token, addr?}`;
  5. polls `poll_events(max)` on a clamped interval.
- Each event's JSON becomes a `HopperIntake` item with `IntakeSource::Webhook`. The intent is built only from `id`/`source`/`event_type`, each bounded to 64 chars with control characters removed. The payload is never copied into the intent.

## Commits

| Task | Commit | Message | Paths |
|---|---|---|---|
| 1 | `f766adf78` | feat(03-04): route polled webhook events into the hopper as Webhook intake | `server_state.rs` (+3: doc line, `mod webhook_intake;`, blank), `server_state/webhook_intake.rs` (new) |
| 2 | `03d58042b` | feat(03-04): opt-in [orchestrator.webhook] section starts the webhook intake poller | the 6 planned paths (+262/-0) |
| 3 | `fd2dfa492` | docs(03-04): describe the webhook poll path in SC#2, REQUIREMENTS and where-things-live | 3 paths, numstat `1 1` each |

## Verification verdicts

- **Task 1 precondition** (re-run after the resume at HEAD `e564ee714`): the index was empty, the new file was absent, `server_state.rs` was clean, and the 03-03 commit exists. Met. `target/phase03-04-start-sha.txt` and the plan-head ledger were re-recorded from `e564ee714`.
- **Task 1 RED** (`target/phase03-04-red.txt`, rc=101, no ESC bytes). First error: `error[E0425]: cannot find function \`submit_webhook_events\` in this scope`.
- **Task 1 verify 1:** `rc=0`, `verify1=0`, 4 passed. All four behaviour tests were `... ok`.
- **Task 1 verify 2:** `verify2=0`. The server_state.rs diff contains only the doc line and `mod webhook_intake;`. Cargo.toml is unchanged. toestub reported `No issues found`.
- **Tracer gate:** I re-ran the tests on the committed tree (`target/phase03-04-tracer.txt`): `rc=0`, 4 passed. Verified end to end before expanding.
- **Task 2 RED** (`target/phase03-04-t2-red.txt`, rc=101). First error: `error[E0609]: no field \`webhook\` on type \`orchestrator_fields::OrchestratorConfig\``.
- **Task 2 verify 1:** `orch=0 mcp=0`, `verify1=0`. Results were orch 3 passed and mcp 8 passed, and all 8 named tests were `... ok`. All `vox-orchestrator --lib config` tests also passed (56).
- **Task 2 verify 2:**
  - `spawn_webhook_intake_poller(` appears at 2 call sites.
  - `envgrep=1`, so there is no direct env read of the token.
  - `tracing-token-hits=0`. I also inspected the multi-line macros by eye: none formats `config_json`, the token, or the raw event string.
  - Both manifests and crate-graph.v1.json are unchanged.
  - toestub on all 6 files reported `No issues found`.
- **Clippy before the Task 2 commit** (`--color never`, 0 ESC bytes): `clippy=0`, with no location lines.
- **Mutant A (config gate):**
  - The mutant replaced `let cfg = cfg?;` with `cfg.unwrap_or(&WebhookIntakeConfig::default())`.
  - Presence check: `presence=1` before and after the run.
  - `compile-errors=0`, so the mutant compiled and the test really ran.
  - Result: `rc=101`, `no_config_section_means_no_plan_and_no_token_read ... FAILED` (the panicking resolver was called).
  - Restored with an edit.
- **Mutant B (token gate):**
  - The mutant replaced `.filter(|t| !t.trim().is_empty())` with `.filter(|_| true)`.
  - Presence: `presence=1`. Before the run, the file's diff against HEAD was the single B line (`1 1`), so A had been restored.
  - `compile-errors=0`.
  - Result: `rc=101`, `missing_or_blank_token_fails_closed ... FAILED` at the blank-token assert.
  - Restored with an edit.
- **Post-mutant:** `restored-identical` (`git diff --quiet HEAD`). Both tests `... ok`. `t3verify1=0`.
- **Task 3 verify 2:** `clippy=0` (`--color never`, 0 ESC bytes, 0 location lines in the plan's files). crate-edges exited 1 with the 3 pre-existing violations (`vox-gui -> vox-db-types`, `vox-gui -> vox-research-shim`, `vox-research-shim -> vox-compiler`). None names vox-orchestrator or vox-orchestrator-mcp, so `t3verify2=0`. Nothing was edited: no `--tighten`, no exceptions, no fan-in snapshot change.
- **secret-env-guard:** rc=1, and the only finding is the other session's uncommitted `crates/vox-gui/src/commands/research.rs`. This plan's delta adds nothing.
- **Doc lint:** `doclint=0` ("no hard errors").
- **Task 3 verify 3 (commit audit):** `t3verify3=0`. The commits have exactly 2, 6 and 3 paths as planned. SC#2 and REQUIREMENTS cite D-10/D-13.
- **Hooks:** fmt-fix and tdd-guard passed on both feature commits (`No issues found`). No hook stalled.

## Deviations from Plan

1. **[Superseded] Environment fact 2b (hollow-fn comments) was skipped.** My first run stopped at the 2b checkpoint. The five always-`None` accessors had once been real pilot-gated clients, lost in `a9d9641de`, and the docs still advertise them. The user then committed `e564ee714`, which restores the status-tool pilot and deletes the dead accessors. After that, server_state.rs passes toestub, so Task 1 added no comment lines, and its server_state.rs diff is only the module declaration.
2. **[Plan defect] where-things-live path.** The plan's row text said `src/webhook_intake.rs`. The module lives at `src/server_state/webhook_intake.rs` (environment fact 2), so the row uses that path.
3. **HEAD-blob staging was not needed.** where-things-live.md, ROADMAP.md and REQUIREMENTS.md were all clean at commit time. Editing the working tree and running `git add --` gave the identical blob. I checked numstat `1 1` for each before committing.
4. **Test scope additions.** `configured_section_with_token_plans_listener_start` also checks that `addr` is present when `bind_addr` is set and that the upper clamp caps 5000 at 1024. `untrusted_fields_are_bounded_and_control_free` also checks the hints, not just the intent.
5. **Poll loop simplification.** An empty `Ok` batch goes through `submit_webhook_events`, which returns 0; there is no separate branch for it. The inbox accessor is re-fetched on every tick, so no FFI object is held across an await.

No `--no-verify`, `git add -A`, stash, checkout, restore or reset. No `VOX_SKIP_FRESHNESS_CHECK`. Cargo.lock and lib.rs were not staged. STATE.md and ROADMAP progress rows were not updated. The only ROADMAP change is the SC#2 text, as the plan specifies.

## Follow-ups

- serde_json error text for a malformed event can quote a short fragment of the rejected value in the `skipping malformed webhook event` warn. It never echoes the whole string or the payload, and the fields are string-typed. Acceptable for now; if logs are later treated as untrusted display, map the error to its category only.
- Manual end-to-end check (dlopen the real plugin and send a real signed HTTP delivery) remains in 03-VALIDATION.md, per the plan's exclusions.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register:
- T-03-13 is mitigated and tested.
- T-03-14 and T-03-15 are mutation-proven.
- T-03-16: IntakePlan has no Debug, and no macro formats the token.
- T-03-17: the interval and batch size are clamped.
- T-03-18: load and ABI errors end only the poller task, and a missing runtime returns None.

## Self-Check: PASSED

- The new files exist: `crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs` and `crates/vox-orchestrator/src/config/webhook_intake.rs`.
- The defactor marker is on the line directly above `fn route_kind` (line 25/26).
- Commits `f766adf78`, `03d58042b` and `fd2dfa492` are on `main`. `git rev-list --count e564ee714..HEAD` = 3 before this SUMMARY commit.
