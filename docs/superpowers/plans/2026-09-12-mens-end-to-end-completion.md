# MENS End-to-End Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Take MENS from "a locally trained model can emit text into a chat bubble" to "a locally trained, per-spoke fine-tune writes code that compiles, is reachable from the GUI and the harness, is validated by a gate that can actually fail, runs on both Metal and CUDA, and is publishable so end users download instead of train."

**Architecture:** Six phases ordered by dependency, not by discovery. Phase 0 removes two pieces of safety theater that would make every later result untrustworthy. Phase 1 fixes the four silent contract mismatches on the chat path. Phase 2 opens the agent loop to VoxLocal, which is the single change that converts "chats" into "writes code." Phase 3 validates the CUDA lane on the one reachable NVIDIA box. Phase 4 builds per-spoke corpora and a multi-spoke runner. Phase 5 publishes over Xet, which is already linked in.

**Tech Stack:** Rust (candle, qlora-rs, peft-rs, axum, tauri), `hf-hub 1.0` + `hf-xet 1.6` (already in the lockfile), VoxScript for glue, iroh QUIC for the mesh, Tailscale for host reachability.

**Spec:** [`docs/src/architecture/mens-program-status-and-requirements-audit-2026-09-12.md`](../../src/architecture/mens-program-status-and-requirements-audit-2026-09-12.md) — the requirements inventory and rating this plan executes against. Read it first; it names every original requirement by ID (R1-R6, D1-D9, M1-M4) and this plan cites those IDs.

## Global Constraints

- Training is **Candle on CUDA and Metal only**. No second ML framework. SafeTensors on disk. (`mesh-mens-distributed-training-and-execution-plan-2026.md` charter.)
- **No new `.ps1` / `.sh` / `.py` glue.** VoxScript via `vox run`. The one existing exception — `huggingface-cli upload` inside `infra/containers/entrypoints/populi-entrypoint.vox:149` — is replaced in Phase 5, not extended.
- **Crypto only via `vox-crypto`.** No direct `sha2`/`blake3`/`ed25519-dalek` imports outside that crate.
- **Crate-edge additions require a USER-AUTHORIZED ledger entry** in `contracts/ci/crate-edges.allow.v1.json`. Propose in the PR description; never self-author.
- **Every new `pub fn` in `crates/*/src/**` needs a same-file test** before the commit lands (`skeleton/untested-pub-api`, enforced by the `tdd-guard` pre-commit hook).
- **New docs under `docs/src/` need frontmatter** (`title`, `description`, `category` from the governance vocabulary, optional `status`). Verify with `cargo run -p vox-doc-pipeline -- --lint-only --paths <path-relative-to-docs/src>`.
- **Never run `cargo fmt --all`** (Windows `CreateProcess` limit). Use `vox run scripts/fmt.vox`.
- `vox mens serve` has **no authentication**. It may only ever be bound beyond loopback on a tailnet or through an SSH tunnel. Do not add a `0.0.0.0` default anywhere.
- Verify security-relevant guards **by mutation**: break the guard, confirm the test fails, restore.

---

## Discoveries that reshaped this plan

Four facts, each verified this session, that invalidate assumptions carried in earlier planning. Read these before executing any task — several tasks exist *only* because of them.

**D-1. `blaptop04` was never blocked.** Three sessions recorded it as "blocked on an operator fixing a Windows OpenSSH ACL." The real cause was the client invocation: bare `ssh blaptop04` defaults to the local username `brbrainerd`, but the authorized key lives under `iacch`. `ssh iacch@blaptop04.tail4f69a0.ts.net` connects instantly and returns `BLAPTOP04` / `blaptop04\iacch`. The `administrators_authorized_keys` fix documented in `docs/superpowers/2026-09-04-cross-machine-mesh-handoff.md` was already applied on 2026-09-04. **No operator action is required.**

**D-2. There is reachable CUDA hardware, and it is small.** `blaptop04` carries an **NVIDIA Quadro T1000, 4096 MiB, driver 610.88** (plus an Intel UHD 630 and a Parsec virtual adapter). `bdesktop`/`bdesktop2` (the 4080 Supers) remain offline — connection timeout, unchanged. 4 GB VRAM fits exactly one rung in `mens/config/gpu-specs.yaml`: `qwen3_code`'s Qwen3-0.6B at `floor_mb: 2000`. Every other `train_bases` rung floors at ≥6000. That is enough to validate the CUDA lane, which has had **zero live hardware validation** across this entire program — but not enough to train a production spoke.

**D-3. The serve gate cannot fail.** `vox mens serve` refuses any adapter whose `collateral_damage_report.json` lacks `"status": "pass"` (`crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:352-370`). The only producer is `vox mens eval-collateral-damage`, and `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` contains, verbatim, `// TRACKED: run inference, currently assuming pre == post for structural demo` followed by `let post = pre;`. It never loads the adapter and never runs inference. Every degradation is 0.0; every report passes. The underlying `vox_eval::eval_collateral_damage_suite` (`crates/vox-eval/src/lib.rs:315`) is genuinely implemented and unit-tested — it is the caller that feeds it fabricated inputs.

**D-4. The agent loop is closed to VoxLocal by construction.** `model_spec_to_llm_config` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:70-118`) returns `Some` only for OpenRouter and Ollama; `ProviderType::VoxLocal` falls into the `None` arm at lines 106-117. `try_run_agent_turn` `?`-propagates that `None` and returns, so `run_agent_turn` — the only code path that calls `select_tools_for_turn`, sends tool schemas, parses `resp.tool_calls`, and dispatches them (agent_loop.rs:531-600) — is never entered for a MENS model. Chat falls back to `call_llm_with_pref` → `VoxLocalAdapter::infer`, a single completion round-trip with no tool channel. **A locally served MENS model cannot write a file or run a command today.**

---

## Phase 0 — Stop lying to ourselves

Two tasks. Both remove safety theater. Nothing downstream is trustworthy until these land, because every "the gate passed" result currently means nothing.

### Task 0.1: Make the collateral-damage eval actually run inference

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (the whole `run_collateral_damage` body)
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:359` (the wrong remediation string)
- Test: `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (same-file `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `vox_eval::eval_collateral_damage_suite(&[BenchRef], &CollateralDamageConfig) -> CollateralDamageReport` (`crates/vox-eval/src/lib.rs:315`), already implemented and unit-tested — do not modify it.
- Consumes: the same `MlBackend` plugin dispatch `vox mens eval-local` uses (`crates/vox-ml-cli/src/commands/mens/eval_local.rs`) to run inference against an adapter.
- Produces: `collateral_damage_report.json` whose `status` is `"pass"` only when measured post-adapter scores are within `max_degradation_rate` (0.05) of the pre-scores.

- [ ] **Step 1: Write the failing test**

In `eval_collateral.rs`'s test module. The point is to prove a *degraded* adapter fails — the exact case the stub cannot express:

```rust
#[test]
fn a_degraded_adapter_fails_the_gate() {
    // pre-scores from a baseline run
    let pre = serde_json::json!({ "general_bench": 0.85, "code_bench": 0.90 });
    // post-scores measured from the adapter — general_bench collapsed
    let post = serde_json::json!({ "general_bench": 0.40, "code_bench": 0.89 });
    let report = build_report_from_scores(&pre, &post, 0.05);
    assert_eq!(report.status, "fail", "a 53% collapse must not pass: {report:?}");
    assert!(
        report.degradations.iter().any(|d| d.name == "general_bench"),
        "the failing bench must be named so an operator knows what broke"
    );
}

#[test]
fn an_intact_adapter_passes_the_gate() {
    let pre = serde_json::json!({ "general_bench": 0.85 });
    let post = serde_json::json!({ "general_bench": 0.84 });
    assert_eq!(build_report_from_scores(&pre, &post, 0.05).status, "pass");
}
```

- [ ] **Step 2: Run it and confirm it fails**

Run: `cargo test -p vox-ml-cli --features gpu -- eval_collateral`
Expected: FAIL — `build_report_from_scores` does not exist.

- [ ] **Step 3: Extract a pure scoring function**

Split the measurable arithmetic out of the I/O so it is testable without a GPU. Add to `eval_collateral.rs`:

```rust
/// Pure: given pre/post score maps, produce the report the gate reads.
/// Separated from inference so the pass/fail arithmetic is testable on any host.
pub(crate) fn build_report_from_scores(
    pre: &serde_json::Value,
    post: &serde_json::Value,
    max_degradation_rate: f64,
) -> vox_eval::CollateralDamageReport {
    let refs: Vec<vox_eval::BenchRef> = pre
        .as_object()
        .map(|m| {
            m.iter()
                .filter_map(|(name, pre_v)| {
                    let pre_score = pre_v.as_f64()?;
                    let post_score = post.get(name)?.as_f64()?;
                    Some(vox_eval::BenchRef { name: name.clone(), pre: pre_score, post: post_score })
                })
                .collect()
        })
        .unwrap_or_default();
    vox_eval::eval_collateral_damage_suite(
        &refs,
        &vox_eval::CollateralDamageConfig { max_degradation_rate },
    )
}
```

Adjust the `BenchRef`/`CollateralDamageReport` field names to whatever `crates/vox-eval/src/lib.rs:315` actually declares — read it first; do not assume these spellings.

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test -p vox-ml-cli --features gpu -- eval_collateral`
Expected: PASS, both tests.

- [ ] **Step 5: Replace `let post = pre;` with real inference**

In `run_collateral_damage`, delete the `// TRACKED` comment and the `let post = pre;` line. In their place, run the bench prompts through the adapter using the same plugin dispatch `eval_local.rs` uses, collect per-bench scores into a `post` map with the same keys as `pre`, then call `build_report_from_scores`. If the adapter cannot be loaded, **fail loudly** — do not fall back to `pre == post`, which is what made this a stub in the first place.

- [ ] **Step 6: Fix the wrong remediation string**

`dispatch.rs:359` currently prints `vox mens eval collateral-damage --pre-score <baseline.json> --post <adapter>`. Both halves are wrong: the subcommand is hyphenated and the flag is `--post-adapter`. Correct it to match the real clap surface (`#[arg(long, id = "post")]` on field `post_adapter`, subcommand `eval-collateral-damage`). `scripts/mens-macos-metal-e2e.vox:126` and `docs/src/how-to/how-to-train-mens-macos-metal.md:63` already have it right — match them.

- [ ] **Step 7: Mutation-verify the gate**

Temporarily make `build_report_from_scores` always return `status: "pass"`. Run `cargo test -p vox-ml-cli --features gpu -- eval_collateral` and confirm `a_degraded_adapter_fails_the_gate` FAILS. Restore, confirm it passes. Confirm `git diff` is clean afterward.

- [ ] **Step 8: Commit**

```bash
git add crates/vox-ml-cli/src/commands/mens/eval_collateral.rs \
        crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
git commit -m "fix(mens): make the collateral-damage gate able to fail

The serve gate refused any adapter whose report was not status:pass, but
the only producer assigned post = pre without running inference, so every
report passed. The gate could not fail. Run the bench against the adapter
for real; fail loudly if it cannot be loaded."
```

### Task 0.2: Retire the stub reports that passed under the old gate

**Files:**
- Modify: `scripts/mens-macos-metal-e2e.vox` (regenerate its report through the real eval)
- Modify: `docs/src/architecture/mens-m4-live-demo-2026-09-12.md` (the M4 demo used a stub report — say so where the gate is described)

**Interfaces:**
- Consumes: Task 0.1's real `vox mens eval-collateral-damage`.

- [ ] **Step 1: Find every stub report on disk**

```bash
find . -name collateral_damage_report.json -not -path './target/*' \
  -exec sh -c 'echo "--- $1"; cat "$1"' _ {} \;
```

Any report not produced by the Task 0.1 code is untrustworthy — it passed a gate that could not fail.

- [ ] **Step 2: Delete them and regenerate for any run you still care about**

For each run directory you intend to keep serving, produce a real baseline and a real report:

```bash
vox mens eval-local --model <run>/candle_qlora_adapter.safetensors \
  --bench mens/data/heldout_bench --output <run>/baseline_scores.json
vox mens eval-collateral-damage --pre-score <run>/baseline_scores.json \
  --post-adapter <run>
```

- [ ] **Step 3: Commit**

```bash
git add scripts/mens-macos-metal-e2e.vox docs/src/architecture/mens-m4-live-demo-2026-09-12.md
git commit -m "chore(mens): regenerate collateral-damage reports through the real eval"
```

---

## Phase 1 — Make the chat contract honest

Four silent mismatches on the `VoxLocalAdapter` ↔ `vox mens serve` wire. Each is small; together they are why a MENS chat reply is truncated, unsteered, and unvalidated.

### Task 1.1: Send `max_tokens`, the system prompt, and sampling params

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/llm_bridge/provider_adapter.rs:232-240` (`VoxLocalGenerateRequest`), `:263-331` (`VoxLocalAdapter::infer`)
- Test: same file, extending `vox_local_generate_request_posts_the_catalog_id` at `:442-452`

**Interfaces:**
- Consumes: `InferRequest { user_prompt, system_prompt, max_tokens, temperature, .. }` (`provider_adapter.rs:27-36`) — `system_prompt` already exists on the struct and is currently ignored.
- Produces: a request body the server's `GenerateRequest` (`crates/vox-ml-cli/src/commands/ai/serve/schema.rs:8-27`) actually reads.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn vox_local_request_carries_max_tokens_and_system_prompt() {
    let req = InferRequest {
        user_prompt: "write a vox function".into(),
        system_prompt: Some("You are a Vox expert.".into()),
        max_tokens: Some(4096),
        temperature: Some(0.2),
        ..Default::default()
    };
    let body = build_vox_local_request(&req, "mens/demo");
    let v = serde_json::to_value(&body).unwrap();
    assert_eq!(v["max_tokens"], 4096, "the 256 default truncates every reply");
    assert!(
        v["prompt"].as_str().unwrap().contains("You are a Vox expert."),
        "the system prompt must reach the model, not be dropped"
    );
    assert_eq!(v["temperature"], 0.2);
    assert!(v.get("validate").is_none(), "the server has no `validate` field");
}
```

- [ ] **Step 2: Run it and confirm it fails**

Run: `cargo test -p vox-orchestrator-mcp -- vox_local_request_carries`
Expected: FAIL — `build_vox_local_request` does not exist; the current body is built inline at `provider_adapter.rs:279-284`.

- [ ] **Step 3: Extract and fix the body builder**

Replace the inline construction with a named function. Drop the dead `validate` field (the server has no such knob — serde silently ignores it today, so removing it changes nothing on the wire and stops the code claiming a behavior it does not get). Compose the system prompt into `prompt`, since `GenerateRequest` has no separate system field:

```rust
pub(crate) fn build_vox_local_request(
    req: &InferRequest,
    model_id: &str,
) -> VoxLocalGenerateRequest {
    // `GenerateRequest` (serve/schema.rs) has no system-prompt field, so the
    // system prompt is composed into the prompt rather than dropped.
    let prompt = match req.system_prompt.as_deref() {
        Some(sys) if !sys.is_empty() => format!("{sys}\n\n{}", extract_prompt_text(req)),
        _ => extract_prompt_text(req),
    };
    VoxLocalGenerateRequest {
        prompt,
        model: model_id.to_string(),
        max_tokens: req.max_tokens.unwrap_or(2048),
        temperature: req.temperature,
        max_retries: 3,
    }
}
```

Update `VoxLocalGenerateRequest` at `:232-240` to match: remove `validate`, add `max_tokens: u32` and `temperature: Option<f32>`.

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test -p vox-orchestrator-mcp -- vox_local`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-orchestrator-mcp/src/llm_bridge/provider_adapter.rs
git commit -m "fix(chat): stop truncating and unsteering MENS replies

max_tokens was never sent, so every MENS chat reply was capped at the
server's 256 default while the catalog advertised 8192. The system prompt
was dropped on this path only. `validate` was a field the server does not
have."
```

### Task 1.2: Close the "Set active" → daemon gap

**Files:**
- Modify: `crates/vox-gui/src/commands/models.rs:236-255` (`set_active_model`)
- Test: same file, alongside the existing DTO tests at `:724-739`

**Interfaces:**
- Consumes: `OrchDaemonClient` (already used by `crates/vox-gui/src/commands/chat_turn.rs:287-293`).

**Context the implementer needs:** `set_active_model` currently does `std::env::set_var("VOX_MODEL", ...)` **inside the GUI process** and writes a `user_preference` DB row. The orchestrator daemon is a separate sidecar process and never reads the GUI's environment, so clicking "Set active" in `ModelsView` has no effect on chat routing. Only the composer's picker (`model_override`, `App.tsx:1163` → `buildChatTurn.ts:83`) actually steers the daemon.

- [ ] **Step 1: Write the failing test** asserting `set_active_model` issues a daemon call (not just an env/DB write). Mock `OrchDaemonClient` the way the existing tests in this file mock their dependencies.

- [ ] **Step 2: Run it, confirm it fails.**

- [ ] **Step 3: Make `set_active_model` call the daemon's `vox_set_active_model` MCP tool** (the same tool that already sets `mcp_chat_model_override`), in addition to the existing DB write. Keep the DB write — it is what survives a restart.

- [ ] **Step 4: Run the test, confirm it passes.**

- [ ] **Step 5: Commit.**

### Task 1.3: Add the missing `mens serve` card

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx:62-70`
- Test: `crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.test.tsx`

**Context:** the surface ships `mens status`, `mens models`, `mens probe` and no serve card. A user who picks `mens/<run>` in the composer with no serve process running gets `probe_vox_local_health`'s error (`probe.rs:162`, correct text) and no button to act on it. The `execute_command` seam already shells `vox <path…> --key value` (`crates/vox-gui/src/commands/execute.rs:15,80`), so this is a card definition, not new plumbing.

**Critical detail:** `CommandCardsView` runs every card's command **on mount** (a `useEffect`), which is why the probe card was previously changed to avoid triggering a multi-GB download. A serve card must **not** follow that pattern — starting a server on surface mount is wrong. Give it an explicit click-to-run affordance, or gate it behind a confirm.

- [ ] **Step 1: Write the failing test** asserting the rendered surface offers a serve action for the active model and does **not** invoke it on mount.
- [ ] **Step 2: Run it, confirm it fails.**
- [ ] **Step 3: Add the card** with `path: ['mens','serve']` and argv carrying `--model <activeModel> --host 127.0.0.1 --port 11435`. Use **11435**, not 11434: every existing e2e script uses 11435 because Ollama commonly owns 11434, and `drive_child_env` (`crates/vox-cli/src/commands/gui/drive.rs:26-102`) forwards `VOX_LOCAL_ENDPOINT` for exactly this reason.
- [ ] **Step 4: Run the test, confirm it passes.** `cd crates/vox-gui/ui && npx vitest run src/components/surfaces`
- [ ] **Step 5: Commit.**

---

## Phase 2 — From "chats" to "writes code"

This is the phase that answers the actual ask. Two tasks: open the agent loop, then prove the output compiles.

### Task 2.1: Open the agent loop to VoxLocal

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:70-118` (`model_spec_to_llm_config`)
- Modify: `crates/vox-ml-cli/src/commands/ai/serve/mod.rs:118-123` (route table), `crates/vox-ml-cli/src/commands/ai/serve/handlers.rs`, `crates/vox-ml-cli/src/commands/ai/serve/schema.rs`
- Test: `agent_loop.rs` same-file tests; `crates/vox-ml-cli/src/commands/ai/serve/` handler tests

**Interfaces:**
- Consumes: the existing Ollama arm of `model_spec_to_llm_config` (`agent_loop.rs:83-106`) as the template — it is the cheapest correct model for what VoxLocal needs.
- Produces: a `VoxLocal` arm returning `Some(LlmConfig)`, so `try_run_agent_turn` proceeds instead of `?`-returning `None`.

**Decision the implementer must make first, and record in the commit message:** there are two routes and they are not equivalent.

- **Route A — add an OpenAI-compatible `/v1/chat/completions` to `vox mens serve`.** The Ollama arm at `agent_loop.rs:83-106` already speaks this shape, so the orchestrator side becomes a near-copy. Cost: a new route, a messages-array request type, and a `tool_calls` response channel on the server.
- **Route B — add a bespoke VoxLocal arm plus tool-call parsing** against the existing `/generate`. Cheaper on the server, more custom code in the bridge, and it invents a tool-call encoding this codebase would then have to own.

**Prefer Route A.** It reuses a wire format the bridge already handles, and an OpenAI-compatible endpoint is independently useful (it makes `vox mens serve` drop-in for other tooling). Route B's only advantage is avoiding a new route, which is not worth owning a bespoke tool-call format.

- [ ] **Step 1: Write the failing test** — a MENS `ModelSpec` produces `Some(LlmConfig)`:

```rust
#[test]
fn a_mens_model_gets_an_llm_config_so_the_agent_loop_runs() {
    let spec = model_spec("mens/demo-run", ProviderType::VoxLocal);
    assert!(
        model_spec_to_llm_config(&spec).is_some(),
        "VoxLocal returning None closes the agent loop, so a local model can \
         never call a tool or write a file"
    );
}
```

`model_spec(...)` is the existing test helper at `agent_loop.rs:1375`.

- [ ] **Step 2: Run it, confirm it fails.** Run: `cargo test -p vox-orchestrator-mcp -- a_mens_model_gets_an_llm_config`

- [ ] **Step 3: Add `/v1/chat/completions` to `vox mens serve`** — request with a `messages: Vec<ChatMessage>` array and optional `tools`; response with `choices[].message.tool_calls`. Model it on the existing `do_generate` handler and the `GenerateResponse` shape in `schema.rs:43-63`.

- [ ] **Step 4: Add the VoxLocal arm** to `model_spec_to_llm_config`, pointed at that route.

- [ ] **Step 5: Run the test, confirm it passes.**

- [ ] **Step 6: Verify by mutation** — revert the VoxLocal arm to `None`, confirm the test fails, restore.

- [ ] **Step 7: Prove a tool actually dispatches.** Add an integration test (or extend `scripts/axis-drive-metal-e2e.vox`) asserting that a MENS-pinned turn whose prompt requires a tool produces a `tool_calls` dispatch, not just text. This is the assertion that distinguishes this task from "it compiles."

- [ ] **Step 8: Commit.**

### Task 2.2: Make `valid` mean "compiles as Vox"

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/ai/serve/handlers.rs:167` (`validate_structured_output_with_reason` call site), `crates/vox-ml-cli/src/commands/ai/serve/prompt.rs`
- Test: `crates/vox-ml-cli/src/commands/ai/serve/` handler tests

**Context:** today `valid`/`errors` mean "parsed as JSON per `output_mode`". Nothing on this path runs `vox check`. Meanwhile `ModelSpec.capabilities.writes_vox = true` is asserted unconditionally for every MENS run (`catalog.rs:684`) — a claim nothing verifies. Adding a `vox_source` output mode that actually compiles the emitted code closes both gaps and makes the existing repair-retry loop (handlers.rs:116-125) genuinely useful: a compile error is a far better repair signal than a JSON parse error.

- [ ] **Step 1: Write the failing test** — a `vox_source` request whose model emits non-compiling Vox comes back `valid: false` with the compiler's diagnostic in `errors`.
- [ ] **Step 2: Run it, confirm it fails.**
- [ ] **Step 3: Add the `vox_source` variant** to `parse_output_mode_label` (handlers.rs:19-35) and a validator that invokes the compiler the same way `vox check` does. Check `crates/vox-doc-pipeline`'s doctest runner first — it already compiles Vox snippets programmatically and is the reuse candidate rather than shelling out.
- [ ] **Step 4: Run the test, confirm it passes.**
- [ ] **Step 5: Have the chat path request it.** In `build_vox_local_request` (Task 1.1), set `output_mode: "vox_source"` when the turn is a codegen turn. The routing policy already distinguishes CodeGen (`llm_bridge/model_route_policy/tests.rs:403-404` asserts CodeGen prefers VoxLocal) — use that signal.
- [ ] **Step 6: Commit.**

### Task 2.3: Extend the e2e script to assert code, not just text

**Files:**
- Modify: `scripts/axis-drive-metal-e2e.vox`

**Context:** this script already does the hard part — `drive start --profile metal-e2e`, pins `model_override=mens/<run>`, sends a prompt, waits for `reply_ok`, asserts `plane=="live"`, asserts a non-empty assistant bubble, and asserts **no `cost_incurred` event with `provider=openrouter`** (the honesty check that the pin did not silently bill cloud). What it does not assert is that the reply is usable code.

- [ ] **Step 1: Add a codegen turn** — send `"Write a Vox function named add that takes two ints and returns their sum."`
- [ ] **Step 2: Assert the reply compiles** — extract the code from the assistant bubble and run it through `vox check`, asserting success.
- [ ] **Step 3: Assert a tool dispatched** (depends on Task 2.1) — a turn that asks the model to write a file produces a real file on disk.
- [ ] **Step 4: Run the script end to end** and confirm all assertions pass.
- [ ] **Step 5: Commit.**

---

## Phase 3 — Validate the CUDA lane on real hardware

Per D-2, `blaptop04`'s Quadro T1000 (4 GB) is the only reachable NVIDIA device and fits exactly the `qwen3_code` Qwen3-0.6B rung. This phase closes the single largest evidence gap in the program: **no CUDA claim in this codebase has ever been checked against a real CUDA device.**

### Task 3.1: Stand up the repo and toolchain on blaptop04

**Files:** none in-repo — this is host setup, recorded in a doc at the end.

**Context:** the box has no repo, no `vox`, no `cargo`, no `git` on `PATH` (verified: `NO_REPO`, all three `where` lookups failed). Its default SSH shell is `cmd.exe` and `powershell` is not on that `PATH` — use the full path `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`.

- [ ] **Step 1: Install prerequisites over SSH** — git, rustup, and the MSVC toolchain.
- [ ] **Step 2: Clone the repo** to `C:\Users\iacch\dev\vox`.
- [ ] **Step 3: Read the documented CUDA build footgun before building.** `docs/src/reference/mens-training.md:171`: building `candle-kernels` **fails entirely through a nested subshell** (`cmd.exe /c "vcvars64.bat && cargo build"`). You must run `vcvars64.bat` in a persistent PowerShell session, then build in that same session. Over SSH this means one persistent remote shell, not a chain of one-shot commands.
- [ ] **Step 4: Build** `cargo build -p vox-ml-cli --features gpu,execution-api,mens-candle-cuda` — note `cuda` is **off by default** on `crates/vox-plugin-mens-candle-cuda` (its manifest says so explicitly: without it the plugin compiles against CPU candle and GPU ops return a runtime error), so this is exactly the mis-measurement trap the Metal lane already hit at 9× slower.
- [ ] **Step 5: Verify the GPU is actually in use** — `nvidia-smi` during a run should show the process and non-trivial VRAM. Record tokens/sec with and without the feature, as the Metal demo did, to prove the kernel path is live.
- [ ] **Step 6: Write the host-setup doc** at `docs/src/architecture/mens-cuda-host-blaptop04-2026-09.md` with frontmatter, recording the exact SSH invocation (`iacch@blaptop04.tail4f69a0.ts.net`), the GPU, the vcvars footgun, and the feature flags.
- [ ] **Step 7: Commit the doc.**

### Task 3.2: Take the first real CUDA calibration measurement

**Files:**
- Modify: `contracts/mens/memory-model.v1.yaml` (add a measured `candle-cuda` row)
- Modify: `crates/vox-populi/src/mens/tensor/memory_model.rs` (any test asserting the CUDA row is `seeded`)

**Context:** the shipped contract has exactly one lane row — `candle-cuda` with `gradient_checkpointing: true`, `act_bytes_per_lht: 186.3620764597089`, `source: seeded`. Seeded means derived from constants that were themselves deleted; the file's own header says "Replace with a real measured row once `vox mens probe --measure` has run for candle-cuda." Gradient checkpointing auto-enables only at `params_b >= 2.9`, so a 0.6B run produces the **`gc=false` cell, which has no row at all** — currently a `NoMeasurement` warn-and-proceed on every small CUDA model.

- [ ] **Step 1: Run the measurement** on blaptop04 with `qwen3_code`'s Qwen3-0.6B, using `PeakSampler` the way the Metal measurement did.
- [ ] **Step 2: Take a second point** at a different `tokens_per_step` — `fit_a_lane` refuses to fit a slope from one point by design, and will correctly refuse here too.
- [ ] **Step 3: Fit the lane** and add the `(candle-cuda, gc=false)` row with `source: measured` and the two data points cited in the provenance comment.
- [ ] **Step 4: Update any test** asserting that cell is uncalibrated. Do this deliberately, with a commit message explaining the row now exists for real.
- [ ] **Step 5: Run** `cargo test -p vox-populi --features mens-train --lib` and confirm green.
- [ ] **Step 6: Commit.**

### Task 3.3: Prove cross-machine serve

**Files:**
- Create: `scripts/mens-cross-machine-e2e.vox`

**Context:** `vox mens serve` binds `127.0.0.1` by default and **has no authentication** (`serve/mod.rs` builds the router with no auth layer). Binding `0.0.0.0` on an open network is an open inference endpoint. The tailnet is the containment boundary — `blaptop04` is already reachable at `blaptop04.tail4f69a0.ts.net`, and its firewall rule `sshd-tailscale` is scoped to `100.64.0.0/10`.

- [ ] **Step 1: Serve on blaptop04**, bound to its tailnet address only.
- [ ] **Step 2: From the Mac**, send a real prompt to it and assert a real response.
- [ ] **Step 3: Point the GUI at it** — `VOX_LOCAL_ENDPOINT` replaces the probe candidate list entirely (`crates/vox-config/src/inference.rs:201-205`), so set it to the remote base and confirm the GUI chats against the remote CUDA box.
- [ ] **Step 4: Record the result** in the Task 3.1 doc.
- [ ] **Step 5: Commit.**

### Task 3.4: Decide the mesh question explicitly

**Files:**
- Modify: `docs/src/architecture/mesh-mens-distributed-training-and-execution-plan-2026.md`

**Context, so nobody re-derives it:** the iroh mesh is real and macOS↔`blaptop04` pairing was already proven in the ADR-047 spike (~10-12 ms dial). But `interp_executor.rs:478` hard-refuses any `kind != TaskKind::VoxScript`, so `TaskKind::TrainQLoRA` is a wire value with **no executor**; and the only dispatch client, `execute_mesh_dispatch` in `vox-workflow-runtime/src/workflow/populi.rs`, calls `dispatchable_source()` which unconditionally returns `Err`. Mesh dispatch is dead code end-to-end. Distributed training is `world_size = 1` with no all-reduce and no rank discovery. The mesh-MENS plan document predates ADR-047 and assumes the retired HTTP control plane as its substrate.

- [ ] **Step 1: Add a correction banner** to that plan doc stating its transport assumption is superseded by ADR-047 and that Tasks Mn-T1/T2/T11's crates were never created.
- [ ] **Step 2: State the decision** — SSH-over-tailnet is the supported cross-machine path (proven in 3.3); the mesh provides peer directory and A2A signalling, not job execution. Anything else is unbuilt.
- [ ] **Step 3: Commit.**

---

## Phase 4 — Per-spoke corpora and a multi-spoke runner

Per the spoke research: only **four** spokes are trainable (`vox-lang`, `rust`, `tool-selection`, `argument-generation`); the other five in `mens/config/domain-profiles.yaml` have no `base:` and are explicitly retired from fine-tuning. **Every per-spoke corpus source file is missing from disk.** Corpus-building, not training, is the load-bearing prerequisite.

### Task 4.1: Fix `--domain` so it actually selects the spoke's base and preset

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` (~lines 395-450)
- Test: same file

**Context:** `vox mens train --domain rust` applies only *hyperparameters* from the profile — `min_rating`, `ce_last_k`, `seq_len`, `curriculum`, `mix_config`, `adapter_tag` and friends. It **never reads `base.model` or `base.preset`**, so it silently trains whatever `--model`/`--preset` you passed, or the trainer's `DEFAULT_MODEL_ID`. Only `vox mens pipeline --profile` routes through `resolve_training_selection` (`crates/vox-ml-cli/src/commands/mens/training_selection.rs`). This makes "train spoke X" mean two different things depending on which command you use — a trap for every task in this phase.

- [ ] **Step 1: Write the failing test** — `--domain rust` with no `--model` resolves `strong_code_default` and preset `qwen3_16g`.
- [ ] **Step 2: Run it, confirm it fails.**
- [ ] **Step 3: Route `train_arm.rs` through `resolve_training_selection`**, preserving precedence: explicit `--model`/`--preset` still win.
- [ ] **Step 4: Run the test, confirm it passes.**
- [ ] **Step 5: Commit.**

### Task 4.2: Build the `rust` spoke corpus

**Files:**
- Create: `target/dogfood/rust_authoring.validated.jsonl` (via corpus commands, not hand-authored)
- Modify: `mens/config/mix-rust.yaml` if source weights need adjusting

**Context:** `rust`'s mix declares `target/dogfood/rust_authoring.validated.jsonl` with `optional: false` — **strict**, and it does not exist. Separately, `mens/data/mix_sources/rust_source.jsonl` holds 19,664 lines but emitted **0** into the last mix with `skipped_reason: "no_lines_passed_filters"` — diagnose that before generating anything new, because 19k lines that fail filters is a bigger lead than 19k lines that do not exist.

- [ ] **Step 1: Diagnose why `rust_source.jsonl` emits zero lines** — run `vox mens corpus mix` with the rust config and read the mix report's `skipped_reason` chain.
- [ ] **Step 2: Fix the filter or the source** so real lines pass.
- [ ] **Step 3: Produce `rust_authoring.validated.jsonl`** via `vox mens corpus validate`/`pairs`.
- [ ] **Step 4: Confirm the corpus clears the floor.** `MIN_CORPUS_PAIRS = 100` (`crates/vox-ml-cli/src/commands/schola/train/run_train.rs:300`) is the enforced gate, but it inspects only `data_dir/train.jsonl`, and the research it cites (`docs/src/architecture/research-cl-qlora-minimum-corpus-2026.md`) argues <500 pairs guarantees catastrophic overfitting. **Target ≥500, not ≥100.**
- [ ] **Step 5: Run `vox ci spoke-check`** to confirm the spoke validates.
- [ ] **Step 6: Commit** the config changes (not the corpus data, unless the repo tracks it).

### Task 4.3: Train and gate the `rust` spoke

- [ ] **Step 1: Train** — `vox mens pipeline --profile rust` (which routes base/preset correctly) or `vox mens train --domain rust` after Task 4.1.
- [ ] **Step 2: Produce a real baseline** — `vox mens baseline --spoke rust --base-eval-dir <d>`; without it, the eval gate's beat-base check **silently skips**.
- [ ] **Step 3: Run the gate** — `vox mens eval-gate --run-dir <d> --policy mens/config/eval-gates-rust.yaml`.
- [ ] **Step 4: Run the real collateral-damage eval** (Task 0.1) and confirm the gate can fail.
- [ ] **Step 5: Chat with it through the GUI** using Phase 2's e2e script, asserting it writes compiling Rust-adjacent Vox.
- [ ] **Step 6: Commit** the run card and gate receipt.

### Task 4.4: Repeat for `tool-selection` and `argument-generation`

Same shape as 4.2-4.3. Their missing sources are `mens/data/mix_sources/tool_selection_synth.jsonl` and `mens/data/mix_sources/argument_generation_synth.jsonl`; both use `mens/config/eval-gates-agents.yaml`. `tool-selection` is the spoke whose quality is most directly measurable by Phase 2's tool-dispatch assertion — train it after Task 2.1 lands so the eval is meaningful.

### Task 4.5: Build the multi-spoke runner

**Files:**
- Create: `scripts/mens/train-all-spokes.vox`

**Context:** no multi-spoke orchestration exists. `vox mens pipeline` takes a singular `--profile`. `vox mens dogfood` hardcodes `device = "cuda"` and cannot run on a Mac. `scripts/mens/full-pipeline.vox` is single-domain via `VOX_MENS_DOMAIN` and still carries a dead `if domain == "rust-expert"` branch for a spoke that was renamed to `rust`.

- [ ] **Step 1: Write the runner** — loop the four trainable spokes, each through pipeline → baseline → eval-gate → collateral-damage, halting the spoke (not the run) on gate failure and reporting a per-spoke summary at the end.
- [ ] **Step 2: Delete the dead `rust-expert` branch** in `full-pipeline.vox` while you are here.
- [ ] **Step 3: `vox check scripts/mens/train-all-spokes.vox`** — must pass with 0 warnings.
- [ ] **Step 4: Run it for real** on at least two spokes.
- [ ] **Step 5: Commit.**

---

## Phase 5 — Distribution over Xet

Per the distribution research: **nothing needs to be added to `Cargo.toml`.** `hf-hub 1.0.0` declares `hf-xet 1.5.3` as a plain non-optional dependency (resolved to 1.6.0), so Vox already *downloads* over Xet today with zero Vox-side code. And `hf-hub` exposes the full upload surface: `HFClient::create_repository` (`repository/mod.rs:911`, blocking `:1532`), `HFRepository::upload_file` (`repository/upload.rs:1005`), `upload_folder` (`:1035`), `create_commit` (`:978`), `create_tag` (`commits.rs:561`) — with LFS batch negotiation advertising `"transfers": ["basic","multipart","xet"]` (`upload.rs:550`) and `upload_lfs_files_via_xet` (`upload.rs:475`).

### Task 5.1: Fix the license bug before publishing anything

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs:375` and `render_ollama_modelfile`
- Modify: `crates/vox-populi/src/mens/tensor/model_card.rs`
- Test: both files

**Context — this is a compliance bug, not a cosmetic one.** `render_ollama_modelfile(8192, "apache-2.0")` **hardcodes** `LICENSE """apache-2.0"""` at the call site. That string is wrong for a Qwen3 derivative. Meanwhile `license_class` and `attribution_required` are plumbed all the way from the CLI (`action_populi_enum.rs:206-215`) through `finetune_contract.rs:41-42` (and are *hashed into the contract identity* at `:268-269`) into the manifest (`manifest/mod.rs:96,99`) — but **every non-test call site passes `None`/`false`** (`pipeline.rs:496-499`, `dispatch.rs:126-129`, `execution_planner.rs:251-252`, `preset_schema.rs:1208-1209`). Nothing anywhere reads `attribution_required`. No `LICENSE`/`NOTICE` file is ever written. And `MODEL_CARD.md` has no license field and no YAML frontmatter, so it is not an HF-renderable card.

- [ ] **Step 1: Write the failing test** — a merge whose `license_class` is `"qwen"` must not emit `apache-2.0`, and a card for an `attribution_required` base must carry the attribution.
- [ ] **Step 2: Run it, confirm it fails.**
- [ ] **Step 3: Thread the real `license_class`** from the manifest into `render_ollama_modelfile` instead of the literal. Populate the field at the real call sites.
- [ ] **Step 4: Give `MODEL_CARD.md` HF frontmatter** — `license`, `base_model`, `tags` — so the Hub renders it.
- [ ] **Step 5: Emit a `LICENSE`/`NOTICE`** beside published artifacts when `attribution_required`.
- [ ] **Step 6: Run the tests, confirm they pass.**
- [ ] **Step 7: Commit.**

### Task 5.2: Build `vox mens publish`

**Files:**
- Create: `crates/vox-ml-cli/src/commands/mens/publish.rs`
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs` (new subcommand), `dispatch.rs`
- Modify: `infra/containers/entrypoints/populi-entrypoint.vox:149` (replace the Python CLI call)
- Test: `publish.rs`

**Interfaces:**
- Consumes: `hf_hub::HFClient::create_repository`, `HFRepository::upload_folder`, `create_tag`; `vox_secrets::resolve_secret(SecretId::HuggingFaceToken)` (`crates/vox-secrets/src/spec/registry/llm.rs:370`, canonical env `HF_TOKEN`).
- Produces: a published repo containing the adapter + manifest + model card, and a printed repo URL.

- [ ] **Step 1: Write the failing test** — the publish plan for a run directory selects the right file set and refuses when the token is absent or read-only.
- [ ] **Step 2: Run it, confirm it fails.**
- [ ] **Step 3: Implement `vox mens publish --run-dir <d> --repo <owner/name> [--tag <t>]`** on `upload_folder`, which gets Xet dedup for free.
- [ ] **Step 4: Decide and document the artifact-selection story** — adapter-only repo (small, needs the base at load) vs. merged vs. GGUF. Note that `hub.rs`'s `download_model`/`is_model_cached` currently assume a `*.safetensors` base-model layout, **not** an adapter or GGUF repo, so the consume side needs a matching resolver or publishing an adapter repo produces something Vox cannot load back.
- [ ] **Step 5: Replace the Python call** in `populi-entrypoint.vox:149` (`huggingface-cli upload`) with `vox mens publish`, removing the `pip3 install "huggingface-hub[cli]"` line from `infra/containers/Dockerfile.populi`. This also settles the standing VoxScript-first violation.
- [ ] **Step 6: Handle write-scope tokens** — the existing `SecretSpec` is `optional_skip` and scoped "private model access" (read). Fail closed with a clear message on a read-only token rather than a 403 mid-upload.
- [ ] **Step 7: Run the tests, confirm they pass.**
- [ ] **Step 8: Commit.**

### Task 5.3: Prove the download side round-trips

- [ ] **Step 1: Publish** a real trained spoke adapter to a scratch repo.
- [ ] **Step 2: From a clean cache**, download it back through `vox mens` and serve it.
- [ ] **Step 3: Assert the served model behaves as the adapter-bearing model** — reuse the M4 comparison technique (`docs/src/architecture/mens-m4-live-demo-2026-09-12.md`): same prompt, greedy decoding, output differs from base in the way the adapter predicts.
- [ ] **Step 4: Record transfer timings** with Xet dedup, to quantify the end-user saving that motivates this phase.
- [ ] **Step 5: Write the distribution doc** at `docs/src/architecture/mens-model-distribution-2026-09.md` with frontmatter.
- [ ] **Step 6: Commit.**

---

## Phase 6 — Maintenance the program will need

### Task 6.1: Triage the LoC-audit fallout

Per [`mens-loc-audit-2026-09.md`](../../src/architecture/mens-loc-audit-2026-09.md): 26 files exceed this repo's own `arch/god_object` 500-line error threshold, **23 of them unsuppressed** (no ledger entry, no `toestub-ignore`); and 23 files (~3,000 lines) are byte-for-byte identical between `vox-plugin-mens-candle-cuda` and `vox-plugin-mens-candle-metal` with **no `// vox:defactored-from` marker**, several far exceeding the ~50-line defactor carve-out (e.g. `qlora_preflight.rs` at 720 lines).

- [ ] **Step 1: Split or suppress** the 23 God-Object files, with real reasons in the suppression entries.
- [ ] **Step 2: Mark or fold** the duplicated plugin files. The `inference.rs` divergence is already a live hazard — the Metal lane was missing `synthesize_rope_inv_freq` and `compute_dtype_for_device` until the M4 demo caught it, a bug that existed *because* the two files drifted.
- [ ] **Step 3: Commit.**

### Task 6.2: Reconcile the corpus documentation with reality

- [ ] **Step 1:** `mens/config/training_contract.yaml` claims `validated_mixed.jsonl` is "~20K lines, all domains"; it is **1,733** — an 11× overstatement. Correct it.
- [ ] **Step 2:** `MIN_CORPUS_PAIRS = 100` contradicts the research it cites (500). Either raise the constant or document why 100 is the enforced floor.
- [ ] **Step 3:** Note that the primary corpus file is hand-curated with **no automated producer** (CI at `.github/workflows/ci.yml:1494-1516` tolerates its absence with a warning) — that is a real single point of failure for every future training run.
- [ ] **Step 4: Commit.**

---

## Self-review

**Spec coverage.** Against the audit's IDs: R1 ✅ done; R2a/R2b ✅ done; R3 — superseded by design, Phase 3.2 adds the measured rows that make the live model trustworthy; R4/R5 ✅ done; R6 needs the second Metal point (carried, not scheduled here — it is a time problem, not a code problem, and Phase 3 buys the CUDA half instead). D1 partially closed by 3.2; D2 ✅ live-verified; D3 ✅; D4 ✅, fallout in 6.1; D5 closed for CUDA by 3.2, Metal still one-celled; D6 ✅; D7 ⚠️ cosmetic gaps stand; **D8 — the "best config for this Mac" memo — is still unscheduled**, and should be written after Phase 3 produces real numbers; D9 ✅. M1-M3 delivered by the audit doc; M4 ✅ for text, and Phase 2 is what makes it true for *code*.

**Known gaps this plan does not close, stated rather than hidden:**
1. **The second Metal calibration point.** Two bounded attempts hit genuine GPU wall-clock limits. Phase 4's real spoke training is the natural place to harvest it as a byproduct — schedule it there rather than as a standing errand that keeps timing out.
2. **`bdesktop`/`bdesktop2`** remain offline. Phase 3 deliberately targets the 4 GB T1000 instead of waiting for them.
3. **Distributed/multi-rank training** stays unbuilt. `world_size = 1`, no all-reduce, no rank discovery. Task 3.4 writes that down rather than planning around a capability that does not exist.
4. **`vox mens serve` has no auth.** Phase 3.3 contains it on the tailnet; it does not fix it. If serving ever needs to leave a trusted network, that is its own plan.
5. **`vox mens export-gguf`** is declared in the arg enum and documented "not yet implemented"; the working GGUF path is `merge-qlora --gguf-out --llama-cpp`.

**Type consistency.** `build_report_from_scores` (0.1) → used only within `eval_collateral.rs`. `build_vox_local_request` (1.1) → consumed by Task 2.2 Step 5. `model_spec_to_llm_config`'s VoxLocal arm (2.1) → precondition for 2.3's tool-dispatch assertion and for `tool-selection` in 4.4. `resolve_training_selection` (4.1) → precondition for 4.2-4.5. Task 5.1's license threading → precondition for 5.2's card emission.
