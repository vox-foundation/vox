---
phase: 3
phase_name: "extract-misplaced-crates-to-plugin-architecture"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 28
  patterns: 13
  surprises: 10
missing_artifacts:
  - "STATE.md"
---

# Phase 3 Learnings: extract-misplaced-crates-to-plugin-architecture

## Decisions

### vox-grammar-export without the zero-consumer automaton module, with in-f...
vox-grammar-export without the zero-consumer automaton module, with in-file tests

**Rationale:** Capability delivered per summary metadata.
**Source:** 03-01-SUMMARY.md

---

### vox-populi without its unused vox-grammar-export dependency
vox-populi without its unused vox-grammar-export dependency

**Rationale:** Capability delivered per summary metadata.
**Source:** 03-01-SUMMARY.md

---

### ROADMAP SC#1/SC#3 and REQUIREMENTS acceptance amended per D-11 / D-05
ROADMAP SC#1/SC#3 and REQUIREMENTS acceptance amended per D-11 / D-05

**Rationale:** Capability delivered per summary metadata.
**Source:** 03-01-SUMMARY.md

## Lessons

### Step 2 crate_audit.json left as found (not overwritten)
**Step 2 `crate_audit.json` left as found (not overwritten).** The compile_s comparison differed only in that HEAD's map lists `vox-mesh-transport` and `vox-plugin-mens-candle-core` at `compile_s: 0.0`. Those are exactly the map's `crates_without_compile_times: 2`, and the file has no entries for them. The existing file is the input that produced HEAD's map: the baseline run reported `missing=2`, and its rows equal HEAD's rows exactly. Overwriting it would have given those two crates explicit 0.0 times without changing the delta. The before and after runs used the same input, so the delta is unaffected.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-01-SUMMARY.md

---

### Shared-file state had changed since planning
**Shared-file state had changed since planning.** At start, `contracts/ci/crate-build-map.v1.json` and `docs/src/architecture/where-things-live.md` were clean, because the other sessions' edits had already been committed. Only `Cargo.lock` was dirty. The HEAD-blob protocol was still followed. For where-things-live, the first awk-built blob added a trailing newline that HEAD lacks (numstat 2/2). I rebuilt the blob as HEAD plus the one row: the working-tree file, clean before my edit. That gave numstat 1/1.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-01-SUMMARY.md

---

### Commit 1b message wording
**Commit 1b message wording.** The plan's template body says HEAD's map "carries unrelated parity drift". At execution HEAD's rows matched the generator exactly (per the parent's note), so the message says the delta was one field and HEAD already matched.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-01-SUMMARY.md

---

### Pre-commit fmt-fix side effect (hook behaviour, not my edit)
**Pre-commit `fmt-fix` side effect (hook behaviour, not my edit).** On commit 1a the hook printed `fmt: formatted 48 file(s).` It runs `scripts/fmt.vox`, which formats every dirty `.rs` file in the shared working tree, including other sessions' uncommitted files. Only this commit's two staged paths were re-staged and committed, and I did not touch or revert the others. They may now carry rustfmt-only changes from the hook.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-01-SUMMARY.md

---

### [Pre-existing, out of scope] vox-ml-cli --features quantize check red on...
**[Pre-existing, out of scope] vox-ml-cli `--features quantize` check red on HEAD.**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-02-SUMMARY.md

---

### Cause: crates/vox-populi/src/mens/hub.rs:201, which is untouched and cle...
Cause: `crates/vox-populi/src/mens/hub.rs:201`, which is untouched and clean in the working tree, omits two struct fields added on 2026-09-17 (E0063). vox-ml-cli's default `mens-base` reaches this code through `vox-populi/mens-train` -> `mens-hf-hub`, whether or not `quantize` is set.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-02-SUMMARY.md

---

### Proof it is independent of this plan
Proof it is independent of this plan: `cargo check -p vox-populi --features mens-hf-hub` fails with the identical E0063, and `cargo tree -p vox-populi --features mens-hf-hub -i vox-quantize` reports vox-quantize is not in that graph.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-02-SUMMARY.md

---

### The plan's fails_when does not cover this failure
The plan's fails_when does not cover this failure: it expects an unresolved `vox_quantize::` path, and none occurred. The scope boundary puts it outside this plan, so I did not fix it and recorded it in `deferred-items.md`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-02-SUMMARY.md

---

### [Rule 3 - Blocking] crates/vox-plugin-api/tests/smoke.rs pins the ABI li...
**[Rule 3 - Blocking] `crates/vox-plugin-api/tests/smoke.rs` pins the ABI literal.** Its `crate_compiles` test asserts `VOX_PLUGIN_ABI_VERSION == 12`, so `cargo test -p vox-plugin-api` failed after the bump. The precedent lockstep commit `39ed9bf60` bumped this same line. I changed it to 13 (numstat `1 1`) and added it to Task 1's commit and to `target/phase03-03-t1-expected.txt`, so Task 1 has 20 paths instead of the planned 19. Other `abi-version = 12` literals are test fixtures whose value does not matter (`crates/vox-cli-ci/src/plugin_surface.rs` macOS-rule fixtures, `crates/vox-plugin-sdk/src/lib.rs` manifest-JSON test) or a doc sample (`crates/vox-plugin-sdk/README.md:38`). The precedent left those alone, and so did I; see Follow-ups.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-03-SUMMARY.md

---

### [Plan defect] git grep -x does not exist ("error
**[Plan defect] `git grep -x` does not exist** ("error: unknown switch `x'"). The precondition, Task 1 verify 2 and the acceptance criteria use it. As written, verify 2 would get `g=129` and fail for the wrong reason. I used the equivalent anchored pattern: `git grep -l '^abi-version = 12$' -- 'crates/**/Plugin.toml'`. Plain `grep -x` on single files works and was left as written.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-03-SUMMARY.md

---

### [Test hygiene, Task 1] The existing semcov_wave3_tests took the shared l...
**[Test hygiene, Task 1] The existing `semcov_wave3_tests` took the shared lock and stopped their listeners** once `start_listening` began filling the process-wide slot. Without that, a leftover slot could have made `poll_events_before_start_is_an_error` flaky. Task 2 rewrote the module as planned (now `listener_lifecycle_tests`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-03-SUMMARY.md

---

### [Rule 2] shutdown() aborts the listener
**[Rule 2] `shutdown()` aborts the listener.** The plan only specified `stop_listening`. The plugin-owned runtime is a process-lifetime static, so the old comment ("dropped when the runtime shuts down") no longer held, and a listener would outlive plugin shutdown. Both paths share a private `stop_listener()`. Two other small cleanups: the crate-level `#![cfg_attr(test, allow(unsafe_code))]` was removed because no test mutates the environment any more, and the `anyhow::Result`/`async_trait` imports went with the deleted `LoggingWebhookSink`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-03-SUMMARY.md

---

### [Superseded] Environment fact 2b (hollow-fn comments) was skipped
**[Superseded] Environment fact 2b (hollow-fn comments) was skipped.** My first run stopped at the 2b checkpoint. The five always-`None` accessors had once been real pilot-gated clients, lost in `a9d9641de`, and the docs still advertise them. The user then committed `e564ee714`, which restores the status-tool pilot and deletes the dead accessors. After that, server_state.rs passes toestub, so Task 1 added no comment lines, and its server_state.rs diff is only the module declaration.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-04-SUMMARY.md

---

### [Plan defect] where-things-live path
**[Plan defect] where-things-live path.** The plan's row text said `src/webhook_intake.rs`. The module lives at `src/server_state/webhook_intake.rs` (environment fact 2), so the row uses that path.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-04-SUMMARY.md

---

### HEAD-blob staging was not needed
**HEAD-blob staging was not needed.** where-things-live.md, ROADMAP.md and REQUIREMENTS.md were all clean at commit time. Editing the working tree and running `git add --` gave the identical blob. I checked numstat `1 1` for each before committing.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-04-SUMMARY.md

---

### Test scope additions
**Test scope additions.** `configured_section_with_token_plans_listener_start` also checks that `addr` is present when `bind_addr` is set and that the upper clamp caps 5000 at 1024. `untrusted_fields_are_bounded_and_control_free` also checks the hints, not just the intent.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-04-SUMMARY.md

---

### [Rule 3 - Blocking] serve.rs docs and chunk decoding
**[Rule 3 - Blocking] serve.rs docs and chunk decoding.** `clippy --features serve` failed on two things already in HEAD's code: `missing_docs` for `pub mod serve` and for `run_serve_worker`. It also flagged `chunks_exact` with a constant size in the new code. I added a module doc and a function doc, and used `as_chunks::<4>()`. All of this is in T1.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-05-SUMMARY.md

---

### [Rule 1 - Plan's lint rule] Follow-up commit bd60ce91f
**[Rule 1 - Plan's lint rule] Follow-up commit `bd60ce91f`.** `clippy -p vox-ml-cli --features oratio -D warnings` found nine diagnostics in `oratio_cmd.rs`'s existing code: seven collapsible_if, one explicit_counter_loop and one let_unit_value. The file only compiles under the non-default `oratio` feature. As the plan directs, I fixed them in a separate `fix(03-05)` commit limited to that file. The changes are mechanical and behaviour-preserving; the counter stays 1-based.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-05-SUMMARY.md

---

### Sherpa-fallback test renamed to create_backend_auto_falls_back_to_whispe...
**Sherpa-fallback test renamed** to `create_backend_auto_falls_back_to_whisper_when_sherpa_init_fails`. It is gated `stt-sherpa` and asserts the fallback backend's name is `"whisper (oratio plugin)"`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-05-SUMMARY.md

---

### Behaviour change in serve
**Behaviour change in serve.** The posted `sample_rate` is now passed to the backend; before, it was ignored. The in-process Candle fallback requires 16 kHz, so a different rate now returns HTTP 500 instead of a silently mis-rated transcript. The oratio plugin ignores `sample_rate`, as it did before.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-05-SUMMARY.md

---

### Doc lint ran per file
**Doc lint ran per file.** `vox-doc-pipeline --lint-only --paths a b` only linted the first path: the log shows a single "linting docs/src/architecture/where-things-live.md" line. I ran `--paths reference/oratio-speech.md` separately (`doclint-oratio=0`, no hard errors). Verify block 1 still ran as written and passed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-06-SUMMARY.md

---

### where-things-live staging
**where-things-live staging.** The file was clean at execution time; the plan expected it to be dirty. The HEAD-blob method (`hash-object` + `update-index`) therefore staged HEAD plus the one row. Numstat is `1 1`, and `git diff HEAD` showed only that row before staging.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-06-SUMMARY.md

---

### srt.rs stub gate
**srt.rs stub gate.** As the plan says, the stub is now gated `not(feature = "audio-decode")`. Because `lib.rs` gates `pub mod subtitle` on `audio-decode`, the stub is never compiled. It was dead under the old gates too. I kept it as the plan specified.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-06-SUMMARY.md

---

### Clippy of the four vox-speech feature sets before commit A.
Clippy of the four vox-speech feature sets before commit A.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-06-SUMMARY.md

---

### #### 1. Webhook end to end through the real dylib over HTTP
#### 1. Webhook end to end through the real dylib over HTTP

**Context:** Verified in phase acceptance checks.
**Source:** 03-VERIFICATION.md

---

### Test: Install vox-plugin-webhook and set VOX_WEBHOOK_INGRESS_TOKEN. Add ...
**Test:** Install vox-plugin-webhook and set `VOX_WEBHOOK_INGRESS_TOKEN`. Add `[orchestrator.webhook]` with `bind_addr = "127.0.0.1:9080"` to Vox.toml and start the MCP server. `POST /webhooks/github` with `Authorization: Bearer <token>` and `X-GitHub-Event: push`, then repeat without the header. **Expected:** A Webhook-sourced hopper item appears. The request without the header gets 401 and creates no item. **Why human:** No test covers the dlopen, start_listening, poll loop or bearer middleware.

**Context:** Verified in phase acceptance checks.
**Source:** 03-VERIFICATION.md

---

### total: 3 passed: 3 issues: 0 pending: 0 skipped: 0
total: 3 passed: 3 issues: 0 pending: 0 skipped: 0

**Context:** Observed during user acceptance testing.
**Source:** 03-UAT.md

---

### [none]
[none]

**Context:** Observed during user acceptance testing.
**Source:** 03-UAT.md

## Patterns

### derived build-map applied as generator before/after delta
derived build-map applied as generator before/after delta

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-01-SUMMARY.md

---

### HEAD-blob staging for shared dirty files
HEAD-blob staging for shared dirty files

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-01-SUMMARY.md

---

### optional heavy dependency behind a named feature, consumers opt in expli...
optional heavy dependency behind a named feature, consumers opt in explicitly

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-02-SUMMARY.md

---

### poll-style extension point with a JSON boundary
poll-style extension point with a JSON boundary

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-03-SUMMARY.md

---

### plugin-owned lazily-built tokio runtime (copied from vox-plugin-browser)
plugin-owned lazily-built tokio runtime (copied from vox-plugin-browser)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-03-SUMMARY.md

---

### synchronous bind, then hand the bound listener to the async server
synchronous bind, then hand the bound listener to the async server

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-03-SUMMARY.md

---

### presence-gated Option config section
presence-gated Option config section

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-04-SUMMARY.md

---

### JSON boundary across the plugin ABI (no crate edge)
JSON boundary across the plugin ABI (no crate edge)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-04-SUMMARY.md

---

### spawn_blocking for dlopen, then tokio interval poll with MissedTickBehav...
spawn_blocking for dlopen, then tokio interval poll with MissedTickBehavior::Delay

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-04-SUMMARY.md

---

### host-registered fn-pointer seam instead of a new crate edge
host-registered fn-pointer seam instead of a new crate edge

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-05-SUMMARY.md

---

### defactored ~15-line adapter
defactored ~15-line adapter

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-05-SUMMARY.md

---

### lockfile-only commit staged from the HEAD blob with one package block re...
lockfile-only commit staged from the HEAD blob with one package block replaced

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-06-SUMMARY.md

---

### manifest check proven load-bearing by mutation
manifest check proven load-bearing by mutation

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-06-SUMMARY.md

## Surprises

### Estimate variance on 03-01
Estimate variance on 03-01: planned 70000 tokens vs 2800 actual (0.04x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-01-PLAN.md, 03-01-SUMMARY.md

---

### Estimate variance on 03-02
Estimate variance on 03-02: planned 50000 tokens vs 2250 actual (0.04x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-02-PLAN.md, 03-02-SUMMARY.md

---

### Estimate variance on 03-03
Estimate variance on 03-03: planned 90000 tokens vs 10700 actual (0.12x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-03-PLAN.md, 03-03-SUMMARY.md

---

### [Rule 3 - Blocking] crates/vox-plugin-api/tests/smoke.rs pins the ABI li...
**[Rule 3 - Blocking] `crates/vox-plugin-api/tests/smoke.rs` pins the ABI literal.** Its `crate_compiles` test asserts `VOX_PLUGIN_ABI_VERSION == 12`, so `cargo test -p vox-plugin-api` failed after the bump. The precedent lockstep commit `39ed9bf60` bumped this same line. I changed it to 13 (numstat `1 1`) and added it to Task 1's commit and to `target/phase03-03-t1-expected.txt`, so Task 1 has 20 paths instead of the planned 19. Other `abi-version = 12` literals are test fixtures whose value does not matter (`crates/vox-cli-ci/src/plugin_surface.rs` macOS-rule fixtures, `crates/vox-plugin-sdk/src/lib.rs` manifest-JSON test) or a doc sample (`crates/vox-plugin-sdk/README.md:38`). The precedent left those alone, and so did I; see Follow-ups.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-03-SUMMARY.md

---

### [Rule 2] shutdown() aborts the listener
**[Rule 2] `shutdown()` aborts the listener.** The plan only specified `stop_listening`. The plugin-owned runtime is a process-lifetime static, so the old comment ("dropped when the runtime shuts down") no longer held, and a listener would outlive plugin shutdown. Both paths share a private `stop_listener()`. Two other small cleanups: the crate-level `#![cfg_attr(test, allow(unsafe_code))]` was removed because no test mutates the environment any more, and the `anyhow::Result`/`async_trait` imports went with the deleted `LoggingWebhookSink`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-03-SUMMARY.md

---

### Estimate variance on 03-04
Estimate variance on 03-04: planned 85000 tokens vs 6900 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-04-PLAN.md, 03-04-SUMMARY.md

---

### Estimate variance on 03-05
Estimate variance on 03-05: planned 95000 tokens vs 9439 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-05-PLAN.md, 03-05-SUMMARY.md

---

### [Rule 3 - Blocking] serve.rs docs and chunk decoding
**[Rule 3 - Blocking] serve.rs docs and chunk decoding.** `clippy --features serve` failed on two things already in HEAD's code: `missing_docs` for `pub mod serve` and for `run_serve_worker`. It also flagged `chunks_exact` with a constant size in the new code. I added a module doc and a function doc, and used `as_chunks::<4>()`. All of this is in T1.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-05-SUMMARY.md

---

### [Rule 1 - Plan's lint rule] Follow-up commit bd60ce91f
**[Rule 1 - Plan's lint rule] Follow-up commit `bd60ce91f`.** `clippy -p vox-ml-cli --features oratio -D warnings` found nine diagnostics in `oratio_cmd.rs`'s existing code: seven collapsible_if, one explicit_counter_loop and one let_unit_value. The file only compiles under the non-default `oratio` feature. As the plan directs, I fixed them in a separate `fix(03-05)` commit limited to that file. The changes are mechanical and behaviour-preserving; the counter stays 1-based.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-05-SUMMARY.md

---

### Estimate variance on 03-06
Estimate variance on 03-06: planned 90000 tokens vs 26947 actual (0.30x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-06-PLAN.md, 03-06-SUMMARY.md

