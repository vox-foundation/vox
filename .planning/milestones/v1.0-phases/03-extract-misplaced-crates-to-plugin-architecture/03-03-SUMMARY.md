---
phase: 03-extract-misplaced-crates-to-plugin-architecture
plan: 03
status: complete
subsystem: plugin ABI / vox-plugin-webhook
tags: [plugin-abi, webhook, D-10, D-14, D-04, fail-closed, SC-2]
requires: [03-02]
provides:
  - "WebhookInbox extension point (poll_events(max) -> RResult<RVec<RString>, RBoxError>, one JSON event per item) and VoxPlugin::as_webhook_inbox (default RNone)"
  - "VOX_PLUGIN_ABI_VERSION = 13 (MIN_SUPPORTED 12), with every code manifest, the extension-points contract and the catalog in lockstep"
  - "vox-plugin-webhook: inert on load; start_listening({ingress_token, addr?}) is fail closed on a plugin-owned runtime; events drained via as_webhook_inbox()"
affects: [vox-plugin-api, vox-plugin-webhook, vox-plugin-catalog, contracts/plugin, "all code Plugin.toml manifests", "03-04 (host poller)"]
tech-stack:
  added: []
  patterns: ["poll-style extension point with a JSON boundary", "plugin-owned lazily-built tokio runtime (copied from vox-plugin-browser)", "synchronous bind, then hand the bound listener to the async server"]
key-files:
  created:
    - crates/vox-plugin-api/src/extensions/webhook_inbox.rs
  modified:
    - crates/vox-plugin-api/src/extensions/mod.rs
    - crates/vox-plugin-api/src/abi.rs
    - crates/vox-plugin-api/src/lib.rs
    - crates/vox-plugin-api/tests/smoke.rs
    - contracts/plugin/extension-points.v1.yaml
    - crates/vox-plugin-webhook/src/lib.rs
    - crates/vox-plugin-webhook/src/webhook/router.rs
    - crates/vox-plugin-webhook/Plugin.toml
    - crates/vox-plugin-catalog/catalog.toml
    - docs/src/reference/plugin-catalog.generated.md
    - "10 other Plugin.toml files (abi-version 12 -> 13)"
decisions:
  - "ABI 12 -> 13 is additive; MIN_SUPPORTED stays 12 (approved with D-10). as_webhook_inbox is appended as the last VoxPlugin method."
  - "shutdown() also aborts the listener, because the plugin-owned runtime outlives the plugin object."
  - "No crate-edge, exception, edges-array or fan-in change. No Cargo.toml dependency changed."
metrics:
  duration: "~27 min wall clock (start 2026-09-26T08:40Z, end 09:07Z); most of it was build-broker queueing behind other sessions"
  completed: 2026-09-26
plan_head_before: 90f408a459e9aa280cf0bc34c2a79f5108cb6dc5
actuals:
  tokens: 10700
  tasks: 3
  commits: 2
---

# Phase 3 Plan 03: WebhookInbox poll extension (ABI 13) and a fail-closed webhook listener Summary

vox-plugin-api now has a `WebhookInbox` extension point. `poll_events(max)` returns up to `max` pending webhook events as JSON strings, in arrival order, and `RErr` if the listener was never started. `VoxPlugin::as_webhook_inbox` defaults to `RNone`. The ABI moved 12 -> 13 in one commit together with every code manifest, the generated contract, the smoke test and the catalog. vox-plugin-webhook serves the inbox from its router's own broadcast channel.

The plugin's network surface is now fail closed:
- Loading the plugin binds nothing and reads nothing from the environment.
- A listener starts only when the host calls `start_listening` with a non-blank `ingress_token`.
- The listener runs on a tokio runtime the plugin owns, binds synchronously, and reports bind errors to the caller.
- `stop_listening` and `shutdown` actually abort it.
- The router has no unauthenticated path left.

This fixes the contract 03-04 needs: load the plugin, call `start_listening({ingress_token, addr?})`, then poll `as_webhook_inbox().poll_events(max)`.

## Commits

| Task | Commit | Message | Paths |
|---|---|---|---|
| 1 | `299f7ec6b` | feat(03-03): add WebhookInbox poll extension (ABI 12 -> 13), served by vox-plugin-webhook | 20 paths (+227/-37): the plan's 19 plus `crates/vox-plugin-api/tests/smoke.rs` (see Deviation 1) |
| 2 | `b115193a3` | fix(03-03): webhook listener starts only on request and refuses to run without a token | `crates/vox-plugin-webhook/src/lib.rs` (+182/-119), `crates/vox-plugin-webhook/src/webhook/router.rs` (+44/-53) |

Task 3 needed no follow-up commit: clippy was clean.

## Verification verdicts

- **Task 1 precondition:** the index was empty, the plan's paths were clean, and the webhook manifest had exactly one `abi-version = 12` line. Met.
- **Task 1 RED** (`target/phase03-03-red.txt`, rc=101). First error: `error[E0425]: cannot find value \`LISTENER\` in this scope`. It was followed by three `E0599: no method named \`as_webhook_inbox\``.
- **Task 1 verify 1:** the first run gave `webhook=0 api=101`. The failure was `tests/smoke.rs` asserting `VOX_PLUGIN_ABI_VERSION == 12` (Deviation 1). After that fix the result was `webhook=0 api=0`, `verify1=0`, and all four named tests were `... ok`.
- **Task 1 verify 2:** `rc=0 grep=1`, `verify2=0`, with `plugin-surface-sync OK (13 extension point(s); ...)`. The YAML has `abi_version: 13` and `as_webhook_inbox`. The bad-abi fixture still says `abi-version = 1`. The stale-manifest grep used an anchored pattern (Deviation 2).
- **Task 1 verify 3:** the catalog `--check` gave `rc=0`. The generated row reads `` `HttpListener`, `WebhookInbox` ``. toestub reported `No issues found`, so `verify3=0`. The generator also rewrote `distribution-bundles.generated.md`, but its bytes are unchanged (`git diff --quiet` passed), so no STOP was needed.
- **Task 1 staged set:** `diff` of sorted expected vs staged was empty (20 paths). `crates/vox-plugin-browser/Plugin.toml` numstat was `1 1`. The index held no Cargo.lock. Hooks: fmt-fix (`formatted 53 file(s)`, other sessions' dirty files, none of them staged), plugin-catalog-docs, and tdd-guard (`No issues found`). The commit contains exactly the 20 paths and no deletions.
- **Tracer gate (Task 1):** reran `cargo test -p vox-plugin-webhook` on the committed tree: `webhook=0`, 3 `poll_events_*` ok. Verified end to end before expanding.
- **Task 2 precondition:** the Task 1 commit exists and the index was empty. Met.
- **Task 2 RED** (`target/phase03-03-t2-red.txt`, rc=101). First error: `error[E0061]: this function takes 1 argument but 2 arguments were supplied` (`WebhookState::new` arity). It was followed by `E0614` (`Option<Arc<str>>` deref) and `E0425` (`new_plugin`).
- **Task 2 verify 1:** `rc=0`, `verify1=0` (52 passed). All 7 behaviour tests and the drain test were `... ok`, and `poll_events_before_start_is_an_error` and `poll_events_respects_max` also passed.
- **Task 2 verify 2:** `envgrep=1`, `verify2=0`. `init.txt` is the 4-line side-effect-free `init` (no spawn, bind or env read). There are 0 `degraded` hits in both files, 0 `Option<Arc<str>>` and 0 `with_ingress_token` in router.rs. toestub reported `No issues found`.
- **Task 2 staged set:** exactly the two paths. Hooks passed: fmt-fix and tdd-guard (`No issues found`).
- **Task 3 verify 1 (commit audit):** `t3verify1=0`. A = `299f7ec6b…` (20 paths, equal to the expected list) and B = `b115193a3…` (exactly the 2 paths).
- **Task 3 mutation proof:**
  - (a) Before the mutation, `git diff --quiet HEAD -- crates/vox-plugin-webhook/src/lib.rs` passed.
  - (b) The mutant replaced `.ok_or("…refusing to start the listener without an ingress token")?;` with `.unwrap_or("MUTANT-PLACEHOLDER-TOKEN");`, so a missing or blank token became a fixed placeholder (fail open).
  - (c) The presence check `grep -c` returned `presence=1`.
  - (d) The mutant compiled: no `error[` lines, and the test actually ran. The result was `rc=101`, `listener_lifecycle_tests::start_listening_refuses_without_ingress_token ... FAILED`, panicking at `lib.rs:381` (`config … must be refused`).
  - (e) Restored with an edit; `git diff --quiet HEAD` passed (`restored-identical`).
  - (f) Post-mutant run: `rc=0`, `... ok`.
- **Task 3 verify 2:** `t3verify2=0`. The mutant log has FAILED, the file matches HEAD, and the post-mutant run passed. On the final tree, `plugin-surface-sync OK` and `plugin catalog docs are up to date`.
- **Task 3 verify 3 (clippy):** `rc=0`, `t3verify3=0`, and 0 location lines in either crate. The log contains 0 ESC bytes, so the grep was meaningful (Deviation 5). vox-plugin-webhook was re-checked fresh (`Checking vox-plugin-webhook`).
- **Plan verification 2:** `git status --porcelain -- crates/vox-plugin-webhook crates/vox-plugin-api crates/vox-plugin-catalog contracts/plugin` is empty. No mutant was left behind.
- **crate-edges:** not run. No `Cargo.toml` dependency changed in either commit, so no edge could have changed. No `--tighten`, no exceptions, and no edits to `edges` or fan-in-snapshot.

## Deviations from Plan

1. **[Rule 3 - Blocking] `crates/vox-plugin-api/tests/smoke.rs` pins the ABI literal.** Its `crate_compiles` test asserts `VOX_PLUGIN_ABI_VERSION == 12`, so `cargo test -p vox-plugin-api` failed after the bump. The precedent lockstep commit `39ed9bf60` bumped this same line. I changed it to 13 (numstat `1 1`) and added it to Task 1's commit and to `target/phase03-03-t1-expected.txt`, so Task 1 has 20 paths instead of the planned 19. Other `abi-version = 12` literals are test fixtures whose value does not matter (`crates/vox-cli-ci/src/plugin_surface.rs` macOS-rule fixtures, `crates/vox-plugin-sdk/src/lib.rs` manifest-JSON test) or a doc sample (`crates/vox-plugin-sdk/README.md:38`). The precedent left those alone, and so did I; see Follow-ups.
2. **[Plan defect] `git grep -x` does not exist** ("error: unknown switch `x'"). The precondition, Task 1 verify 2 and the acceptance criteria use it. As written, verify 2 would get `g=129` and fail for the wrong reason. I used the equivalent anchored pattern: `git grep -l '^abi-version = 12$' -- 'crates/**/Plugin.toml'`. Plain `grep -x` on single files works and was left as written.
3. **[Test hygiene, Task 1] The existing `semcov_wave3_tests` took the shared lock and stopped their listeners** once `start_listening` began filling the process-wide slot. Without that, a leftover slot could have made `poll_events_before_start_is_an_error` flaky. Task 2 rewrote the module as planned (now `listener_lifecycle_tests`).
4. **[Rule 2] `shutdown()` aborts the listener.** The plan only specified `stop_listening`. The plugin-owned runtime is a process-lifetime static, so the old comment ("dropped when the runtime shuts down") no longer held, and a listener would outlive plugin shutdown. Both paths share a private `stop_listener()`. Two other small cleanups: the crate-level `#![cfg_attr(test, allow(unsafe_code))]` was removed because no test mutates the environment any more, and the `anyhow::Result`/`async_trait` imports went with the deleted `LoggingWebhookSink`.
5. **[Env] `CARGO_TERM_COLOR=never` did not stop cargo from emitting ANSI codes here.** The RED log is colored even with that variable set. The clippy block was therefore run with an explicit `cargo clippy --color never …`, and I checked the log has zero ESC bytes before trusting the location grep (the lesson from 03-02). I also ran clippy once before Task 2's commit, so any finding would land inside it; there were none. rustfmt wrapped two long `assert!` lines in lib.rs before that commit.

No `--no-verify`, `git add -A`, stash, checkout, restore or reset. No `VOX_SKIP_FRESHNESS_CHECK`. Cargo.lock was not staged. STATE.md and ROADMAP progress rows were not updated. No hook stalled.

## Follow-ups

- `crates/vox-plugin-sdk/README.md:38`: the plugin-author sample manifest still says `abi-version = 12`. A plugin copied from it would fail `plugin-surface-sync`. It is a one-line doc bump for whoever next touches the SDK docs.
- `webhook::config::bind_addr_from_env` has no callers now; the crate-level `dead_code` allow hides it. Delete it when the crate's blanket allow is removed.
- `plugin-abi-parity` (which loads built dylibs) is left to fleet CI, per the plan's exclusions.
- 03-04 must resolve `SecretId::WebhookIngressToken` through `vox_secrets` and pass it as `ingress_token` in the `start_listening` JSON. The plugin reads no secret from the environment.

## Known Stubs

None. `LoggingWebhookSink` (the log-only placeholder) was deleted.

## Threat Flags

None beyond the plan's register: T-03-07, T-03-08, T-03-09, T-03-11 and T-03-12 are mitigated as specified, and T-03-10 is bounded by `max` and the existing channel capacity.

## Self-Check: PASSED

- `crates/vox-plugin-api/src/extensions/webhook_inbox.rs` exists and has `pub const WEBHOOK_INBOX_REVISION: u32 = 1;` and `pub trait WebhookInbox`. `as_webhook_inbox` is the last `VoxPlugin` method. `lib.rs` has `VOX_PLUGIN_ABI_VERSION: u32 = 13` and `VOX_PLUGIN_ABI_MIN_SUPPORTED: u32 = 12`.
- Commits `299f7ec6b` and `b115193a3` are on `main`. `git rev-list --count 90f408a45..HEAD` = 2 before this SUMMARY commit.
