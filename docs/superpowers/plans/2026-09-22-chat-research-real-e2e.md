# Chat Research & Deep Research — Real End-to-End Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Axis chat detects when a message needs research, runs quick or deep research against real search with `google/gemini-3.8-flash` over OpenRouter, shows every stage under the reply, and never returns template text — proven by a live run, not mocks.

**Architecture:** Research becomes a typed step inside the daemon's `vox_chat_message` (sync path). A pure intent classifier decides None/Quick/Deep; Quick injects numbered real sources into the chat model's prompt (Gemini is the synthesizer) and checks citations afterwards; Deep runs the existing Scientia pipeline inline on the Deep lane with its template/judge/cache bugs fixed. Every turn returns one `research_trace` event in the existing `events` array, rendered by a new `ResearchTracePanel` in the chat transcript. One config knob, `VOX_MODEL_FORCE`, pins every LLM call strictly.

**Tech Stack:** Rust (tokio, serde, wiremock tests), React 19 + TypeScript (vitest, Playwright), SearXNG in Docker via colima, OpenRouter.

**Spec:** `docs/superpowers/specs/2026-09-21-chat-research-real-e2e-design.md` — read it first; defect ids D1–D15 below refer to its §2 table.

## Global Constraints

- Model: `google/gemini-3.8-flash` via OpenRouter. **No** MENS / VoxLocal / Ollama / free-floor substitution anywhere in chat or research when `VOX_MODEL_FORCE` is set.
- **No-template rule:** no code path may produce answer prose that did not come from an LLM response. Failures surface as errors/failed stages with the real error text.
- Done = spec §8 passes against the real stack. Mocked green is not done.
- Work in worktree `/Users/brbrainerd/dev/vox/.claude/worktrees/research-detection-chat-gui-05d826`, branch `claude/research-detection-chat-gui-05d826`. Do not push. Stage files by explicit path (other agents share the repo).
- Test-first: every new `pub fn` gets a failing test before implementation (repo policy, `tdd-guard` hook).
- Never read `docs/src/archive/`. Never run `cargo fmt --all`; format with `cargo fmt -p <crate>` or `vox run scripts/fmt.vox`.
- No new workspace crate edges (`vox ci crate-edges`). Every crate used below already depends on what it needs.
- Never print secret values. The key resolves from the Clavis vault (`vox secrets get OPENROUTER_API_KEY` shows `sk-o…`).
- Run cargo from the worktree root with plain `cargo` (the build broker shim). If `target/` is missing, rebuild: `RUSTC_WRAPPER=/opt/homebrew/bin/sccache cargo build -p vox-orchestrator-d -p vox-cli`.
- Before each commit: `cargo clippy -p <touched-crates> --all-targets -- -D warnings` must be clean.

## File Map

| File | Responsibility | Task |
|---|---|---|
| `crates/vox-secrets/src/backend/vox_vault.rs` | `run_secrets_future` works on any runtime flavor | 1 |
| `docker/searxng/settings.yml` | enable JSON output | 2 |
| `crates/vox-config/src/inference.rs` | `forced_model()` single pin knob | 3 |
| `crates/vox-actor-runtime/src/llm/cascade.rs` | forced → exactly one research candidate | 3 |
| `crates/vox-research-shim/src/research/orchestrator/model_dispatch.rs` | forced → bypass `decide()` | 3 |
| `crates/vox-orchestrator/src/models/select.rs` | read pin via `forced_model()` | 3 |
| `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs` | strict chat pin | 3 |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_intent.rs` (new) | pure intent classifier | 4 |
| `crates/vox-search/src/web_dispatcher.rs` | `search_with_report` + per-provider outcomes | 5 |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs` (new) | trace types, quick path, citation check, deep adapter | 6, 8 |
| `crates/vox-research-shim/src/research/orchestrator/{stages,pipeline,pipeline_cache,config}.rs`, `research/types.rs` | delete template, honest judge, metadata fields, cache honesty | 7 |
| `crates/vox-orchestrator-mcp/src/memory_tools/{params,handlers_memory}.rs` | `lane` param honored | 7 |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`, `memory_tools/{retrieval,mod}.rs` | wire classifier + trace; delete old gate | 8 |
| `crates/vox-gui/ui/src/lib/buildChatTurn.ts` | research slash stays Sync, raw text | 9 |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ResearchTracePanel.tsx` (new), `ChatTurnEventRow.tsx` | render trace | 9 |
| `crates/vox-gui/src/commands/search_probe.rs` | DDG probe reports "not implemented" | 9 |
| `scripts/axis-drive-research-e2e.vox` (new) | live E2E via Drive | 10 |
| `crates/vox-gui/ui/e2e/fixtures/live-research/*.json` (recorded), `e2e/chat-research-trace.spec.ts` (new) | replay screenshots | 11 |

---

### Task 1: Secrets resolve on current-thread runtimes (D13)

**Files:**
- Modify: `crates/vox-secrets/src/backend/vox_vault.rs:1462-1480` (`run_secrets_future`) and its test module in the same file.

**Interfaces:**
- Produces: `run_secrets_future` keeps its name; bounds become `F: Future<Output = Result<T, SecretError>> + Send, T: Send`. Behavior: works under multi-thread runtime, current-thread runtime, and no runtime.

- [ ] **Step 1: Write the failing tests** — append to the `#[cfg(test)]` module nearest `run_secrets_future` (e.g. `semcov_wave2_tests`):

```rust
    #[test]
    fn run_secrets_future_inside_current_thread_runtime_does_not_fail() {
        // D13: ModelRegistry::maybe_refresh_catalogs resolves secrets on a
        // current_thread runtime; block_in_place panics there and the panic
        // was converted into a silent "missing secret".
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        let got = rt.block_on(async { run_secrets_future(async { Ok::<_, SecretError>(7) }) });
        assert_eq!(got.expect("must resolve on current_thread runtime"), 7);
    }

    #[test]
    fn run_secrets_future_without_runtime_resolves() {
        let got = run_secrets_future(async { Ok::<_, SecretError>(9) });
        assert_eq!(got.expect("must resolve with no ambient runtime"), 9);
    }

    #[test]
    fn run_secrets_future_inside_multi_thread_runtime_resolves() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("rt");
        let got = rt.block_on(async { run_secrets_future(async { Ok::<_, SecretError>(3) }) });
        assert_eq!(got.expect("multi-thread path unchanged"), 3);
    }
```

- [ ] **Step 2: Run to verify the first two fail**

Run: `cargo test -p vox-secrets run_secrets_future -- --nocapture`
Expected: `..._inside_current_thread_runtime_does_not_fail` FAILS with `BackendMisconfigured("failed to execute secrets async operation from active runtime")`; `..._without_runtime_resolves` FAILS with `requires an active Tokio runtime`; multi-thread PASSES.

- [ ] **Step 3: Implement** — replace the body of `run_secrets_future`:

```rust
fn run_secrets_future<F, T>(future: F) -> Result<T, SecretError>
where
    F: Future<Output = Result<T, SecretError>> + Send,
    T: Send,
{
    use tokio::runtime::{Handle, RuntimeFlavor};
    match Handle::try_current() {
        // block_in_place is only legal on the multi-thread scheduler.
        Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        // current_thread runtime (block_in_place would panic — D13) or no runtime:
        // drive the future on a dedicated thread with its own runtime.
        _ => std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| {
                            SecretError::BackendMisconfigured(format!("secrets runtime: {e}"))
                        })?
                        .block_on(future)
                })
                .join()
                .map_err(|_| {
                    SecretError::BackendMisconfigured(
                        "secrets worker thread panicked".to_string(),
                    )
                })?
        }),
    }
}
```

If a caller fails to compile because its future is not `Send`, fix that caller's future (e.g. clone owned data into it) — do **not** drop the `Send` bound. Remove the now-unused `panic` import if clippy flags it. If an existing test asserts the old "requires an active Tokio runtime" error, delete that assertion (the behavior it pinned is the bug).

- [ ] **Step 4: Run tests**

Run: `cargo test -p vox-secrets` then `cargo clippy -p vox-secrets --all-targets -- -D warnings`
Expected: all PASS, clippy clean.

- [ ] **Step 5: Live check** — rebuild the daemon (`cargo build -p vox-orchestrator-d`), run the probe below 3 times, and confirm `grep -c panicked` is 0 every time and the reply has `model: google/gemini-3.8-flash`. (The original panic only fires when the catalog refresh interval has elapsed, so the unit tests in Step 1 are the deterministic guard; this is the live sanity check.) `$SCRATCH` = the session scratchpad directory:

```bash
REQ='{"id":"p","method":"orch.tool_call","params":{"name":"vox_chat_message","args":{"prompt":"Reply with exactly: ok","session_id":"t1-'$RANDOM'","model_override":"google/gemini-3.8-flash"}}}'
(echo "$REQ"; sleep 40) | RUST_BACKTRACE=1 VOX_ORCHESTRATOR_DAEMON_SOCKET=stdio timeout 45s ./target/debug/vox-orchestrator-d 2>"$SCRATCH/t1.err" | grep -m1 '"id":"p"' | jq -c '.payload.value | {success, model: .data.model_used, content: .data.message.content}'
grep -c panicked "$SCRATCH/t1.err"
```

- [ ] **Step 6: Commit**

```bash
git add crates/vox-secrets/src/backend/vox_vault.rs
git commit -m "fix(secrets): resolve on current-thread runtimes instead of reporting missing

run_secrets_future called block_in_place, which panics on a current_thread
runtime; catch_unwind turned that into BackendMisconfigured, so callers such as
ModelRegistry::maybe_refresh_catalogs saw the OpenRouter key as absent.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: SearXNG actually serves JSON (D14, D10)

**Files:**
- Modify: `docker/searxng/settings.yml`
- Create: `~/.vox/config.toml` entry (user config, not committed) — see Step 5.

**Interfaces:**
- Produces: SearXNG at `http://localhost:8080` answering `/search?q=…&format=json` with results; `VOX_SEARCH_SEARXNG_URL=http://localhost:8080` resolvable by the daemon.

- [ ] **Step 1: Start colima and the container** (user approved starting colima)

```bash
colima start --cpu 2 --memory 4
docker compose -f docker/searxng/compose.yml up -d
```

- [ ] **Step 2: Demonstrate the failure**

```bash
curl -s -o /dev/null -w '%{http_code}\n' 'http://localhost:8080/search?q=gemini+flash&format=json'
```
Expected: `403` (JSON format not enabled). If it is already 200, skip to Step 4 and record that D14 was wrong.

- [ ] **Step 3: Fix the config** — in `docker/searxng/settings.yml`, delete the invalid top-level block

```yaml
format:
  json: true
```

and add under the existing `search:` section (create the section if absent):

```yaml
search:
  formats:
    - html
    - json
```

Then `docker compose -f docker/searxng/compose.yml restart`.

- [ ] **Step 4: Verify real results**

```bash
curl -s 'http://localhost:8080/search?q=gemini+3.8+flash+openrouter&format=json' | jq '{n: (.results|length), first: .results[0].url, engines: [.results[].engine] | unique}'
```
Expected: `n` ≥ 5, a real `https://` URL, at least one general web engine. Record p95-ish latency with 5 runs of `curl -w '%{time_total}\n' -o /dev/null -s '…'`; if > 3.5 s, note it in the plan's Task 6 and raise `deep_timeout_ms` usage accordingly.

- [ ] **Step 5: Point the daemon at it** — append to `~/.vox/config.toml` (user file, not committed):

```toml
VOX_SEARCH_SEARXNG_URL = "http://localhost:8080"
```

Verify it resolves: `vox secrets status | grep -i searxng` shows present (it is a registry item read through `resolve_secret`; if `resolve_secret` does not read `~/.vox/config.toml`, use `printf 'http://localhost:8080' | vox secrets set VOX_SEARCH_SEARXNG_URL --stdin` instead and note which worked).

- [ ] **Step 6: Commit**

```bash
git add docker/searxng/settings.yml
git commit -m "fix(searxng): enable JSON output via search.formats

format: json: true is not a SearXNG setting, so every format=json request from
the web dispatcher got 403 and SearXNG never contributed results.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: One strict model pin — `VOX_MODEL_FORCE` (D9, D15)

**Files:**
- Modify: `crates/vox-config/src/inference.rs` (add `forced_model`, `forced_model_from`)
- Modify: `crates/vox-actor-runtime/src/llm/cascade.rs:207` (`cascade_for_research_stage`)
- Modify: `crates/vox-research-shim/src/research/orchestrator/model_dispatch.rs:31` (`primary_candidate_for_intent`)
- Modify: `crates/vox-orchestrator/src/models/select.rs:711` (`select_inner`)
- Modify: `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs:320-327`
- Test: same files' test modules; `resolve.rs` tests go in `model_route_policy/tests.rs`.

**Interfaces:**
- Produces: `vox_config::inference::forced_model() -> Option<String>` (env `VOX_MODEL_FORCE`, then `~/.vox/config.toml`); `vox_config::inference::forced_model_from(raw: &str) -> Option<String>` (pure, trims, empty → None); `vox_actor_runtime::llm::cascade::pinned_research_candidates(stage: ResearchStage, model: &str) -> Vec<LlmConfig>`; `resolve::check_strict_pin(pin: &str, found: Option<&ModelSpec>, gates_ok: bool) -> Result<(), String>`.

- [ ] **Step 1: Failing tests**

In `crates/vox-config/src/inference.rs` test module:

```rust
    #[test]
    fn forced_model_from_trims_and_rejects_blank() {
        assert_eq!(super::forced_model_from("  google/gemini-3.8-flash \n"), Some("google/gemini-3.8-flash".to_string()));
        assert_eq!(super::forced_model_from("   "), None);
        assert_eq!(super::forced_model_from(""), None);
    }
```

In `crates/vox-actor-runtime/src/llm/cascade.rs` test module:

```rust
    #[test]
    fn pinned_research_candidates_is_exactly_the_pin() {
        let c = pinned_research_candidates(ResearchStage::Synthesis, "google/gemini-3.8-flash");
        assert_eq!(c.len(), 1, "a pin must not append free-floor or local candidates");
        assert_eq!(c[0].model, "google/gemini-3.8-flash");
        assert_eq!(c[0].provider, LlmConfig::openrouter("x").provider);
    }
```

In `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/tests.rs`:

```rust
#[test]
fn strict_pin_missing_from_registry_is_an_error_naming_the_pin() {
    let err = super::resolve::check_strict_pin("google/gemini-3.8-flash", None, true).unwrap_err();
    assert!(err.contains("google/gemini-3.8-flash"), "{err}");
    assert!(err.contains("not in the model registry"), "{err}");
}

#[test]
fn strict_pin_blocked_by_gate_is_an_error() {
    let spec = crate::llm_bridge::infer_test_stub::stub_plan_model_spec();
    let err = super::resolve::check_strict_pin(&spec.id, Some(&spec), false).unwrap_err();
    assert!(err.contains("not allowed"), "{err}");
}

#[test]
fn strict_pin_ok_when_found_and_allowed() {
    let spec = crate::llm_bridge::infer_test_stub::stub_plan_model_spec();
    assert!(super::resolve::check_strict_pin(&spec.id, Some(&spec), true).is_ok());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p vox-config forced_model_from; cargo test -p vox-actor-runtime pinned_research_candidates; cargo test -p vox-orchestrator-mcp strict_pin`
Expected: compile errors — functions not defined.

- [ ] **Step 3: Implement**

`crates/vox-config/src/inference.rs`:

```rust
/// Strict model pin shared by chat and research: `VOX_MODEL_FORCE`, read from the
/// environment first, then `~/.vox/config.toml`. `None` when unset or blank.
#[must_use]
pub fn forced_model() -> Option<String> {
    forced_model_from(&crate::env_parse::resolve_config_str("VOX_MODEL_FORCE", ""))
}

/// Pure half of [`forced_model`]: trims, and treats blank as unset.
#[must_use]
pub fn forced_model_from(raw: &str) -> Option<String> {
    let v = raw.trim();
    (!v.is_empty()).then(|| v.to_string())
}
```

`crates/vox-actor-runtime/src/llm/cascade.rs` — add, and short-circuit the top of `cascade_for_research_stage`:

```rust
/// Exactly one OpenRouter candidate for a pinned model: no local lane, no free floor.
#[must_use]
pub fn pinned_research_candidates(stage: ResearchStage, model: &str) -> Vec<LlmConfig> {
    let mut c = LlmConfig::openrouter(model.to_string());
    apply_stage_defaults(stage, &mut c);
    vec![c]
}
```

```rust
pub fn cascade_for_research_stage(stage: ResearchStage, input: &RouteResolutionInput) -> Vec<LlmConfig> {
    if let Some(forced) = vox_config::inference::forced_model() {
        return pinned_research_candidates(stage, &forced);
    }
    // ... existing body unchanged ...
```

`model_dispatch.rs` — first line of `primary_candidate_for_intent`:

```rust
    if let Some(forced) = vox_config::inference::forced_model() {
        return Some(LlmConfig::openrouter(forced));
    }
```

Then in `stages.rs::chat_stage`, after building `candidates`, drop exact duplicates so a pin is tried once:

```rust
    candidates.dedup_by(|a, b| a.provider == b.provider && a.model == b.model);
```

`select.rs::select_inner` — replace `if let Ok(force) = std::env::var("VOX_MODEL_FORCE") { let force = force.trim().to_string(); if !force.is_empty() && let Some(model) = registry.get(&force)` with:

```rust
    if let Some(force) = vox_config::inference::forced_model() {
        if let Some(model) = registry.get(&force) {
            return Some(SelectionOutcome { /* unchanged fields, model_id: force, model_spec: model, ... */ });
        }
        tracing::warn!(pin = %force, "VOX_MODEL_FORCE model not in registry; selection continues unpinned");
    }
```
(keep the existing `SelectionOutcome` construction verbatim).

`resolve.rs` — add the pure checker and replace the hard-pin block at `:320`:

```rust
pub(crate) fn check_strict_pin(pin: &str, found: Option<&ModelSpec>, gates_ok: bool) -> Result<(), String> {
    match found {
        None => Err(format!(
            "pinned model {pin} (VOX_MODEL_FORCE / VOX_ROUTING_HARD_PIN_MODEL) is not in the model registry; \
             refresh the catalog (`vox model`) or fix the pin"
        )),
        Some(_) if !gates_ok => Err(format!(
            "pinned model {pin} is not allowed for this request (local/routing/capability gate)"
        )),
        Some(_) => Ok(()),
    }
}
```

```rust
    let strict_pin = routing_policy
        .hard_pin_model_id
        .clone()
        .or_else(vox_config::inference::forced_model);
    if let Some(pin) = strict_pin.as_deref() {
        let found = registry.get(pin);
        let gates_ok = found
            .as_ref()
            .is_some_and(|m| mcp_local_model_allowed(m) && routing_allows(m) && caps_ok(m));
        check_strict_pin(pin, found.as_ref(), gates_ok)?;
        let m = found.expect("checked above");
        let enforced = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
        if enforced.id != m.id {
            return Err(format!(
                "pinned model {pin} is paid but this turn is free-tier-only (tier=local, Free clutch, or spend cap)"
            ));
        }
        *rationale_out = Some(format!("strict pin: {pin}"));
        return Ok((m.clone(), m.is_free));
    }
```
(`registry.get` returns `Option<ModelSpec>`; adjust `.as_ref()` usage if it returns a reference. `rationale_out` is the `&mut Option<String>` parameter of `resolve_mcp_chat_model_sync_inner`.)

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p vox-config -p vox-actor-runtime -p vox-research-shim -p vox-orchestrator-mcp -p vox-orchestrator 2>&1 | tail -30` and `cargo clippy -p vox-config -p vox-actor-runtime -p vox-research-shim -p vox-orchestrator -p vox-orchestrator-mcp --all-targets -- -D warnings`
Expected: new tests PASS; no regressions (if an existing test relied on the silent hard-pin fallthrough, it asserted the bug — update it to expect the error and say so in the commit body).

- [ ] **Step 5: Persist the pin for the user** (not committed): append to `~/.vox/config.toml`

```toml
VOX_MODEL_FORCE = "google/gemini-3.8-flash"
```

- [ ] **Step 6: Commit**

```bash
git add crates/vox-config/src/inference.rs crates/vox-actor-runtime/src/llm/cascade.rs \
  crates/vox-research-shim/src/research/orchestrator/model_dispatch.rs \
  crates/vox-research-shim/src/research/orchestrator/stages.rs \
  crates/vox-orchestrator/src/models/select.rs \
  crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs \
  crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/tests.rs
git commit -m "feat(routing): strict VOX_MODEL_FORCE pin for chat and research

One knob (env or ~/.vox/config.toml) now pins every research stage to exactly one
OpenRouter candidate and makes the chat pin fail loudly instead of silently
falling back to auto-selection, free-floor, or local models.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: Research intent classifier (D1)

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_intent.rs`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs` (add `pub mod research_intent;`)

**Interfaces:**
- Produces:
```rust
pub enum ResearchMode { None, Quick, Deep }            // serde: "none"|"quick"|"deep"
pub struct ResearchIntent { pub mode: ResearchMode, pub explicit: bool, pub reasons: Vec<String>, pub query: String }
pub fn classify_research_intent(prompt: &str, force: Option<bool>, scope: Option<&str>) -> ResearchIntent
```
`query` is the prompt with any `/research` or `/deepresearch` prefix stripped and trimmed; for non-slash prompts it is the trimmed prompt.

- [ ] **Step 1: Write the failing table test** — create `research_intent.rs` containing only this test module (no implementation yet), so it fails to compile:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn mode(p: &str) -> ResearchMode {
        classify_research_intent(p, None, None).mode
    }

    #[test]
    fn skips_small_talk_and_code_work() {
        for p in [
            "hi", "hello!", "thanks", "thank you so much", "ok", "how are you?",
            "fix the bug in crates/vox-cli/src/main.rs",
            "refactor this function to use iterators",
            "write a test for parse_config",
            "what does ```let x = 1;``` do in this file",
            "explain @crates/vox-search/src/rrf.rs",
            "rename foo to bar",
        ] {
            assert_eq!(mode(p), ResearchMode::None, "{p:?}");
        }
    }

    #[test]
    fn quick_for_time_sensitive_or_evidence_questions() {
        for p in [
            "What is the latest Gemini Flash model on OpenRouter and when was it released?",
            "what's the current version of tokio?",
            "who won the 2026 world cup?",
            "Is there any news about the Rust 2027 edition?",
            "look up the pricing for Tavily search",
            "find sources on retrieval augmented generation evaluation",
        ] {
            assert_eq!(mode(p), ResearchMode::Quick, "{p:?}");
        }
    }

    #[test]
    fn deep_for_comparative_or_survey_questions() {
        for p in [
            "compare SearXNG and Tavily for agent web search",
            "SearXNG vs Tavily for an agent: which is better?",
            "what are the trade-offs between RRF and learned rerankers?",
            "give me a deep dive on speculative decoding",
            "state of the art in long-context retrieval",
        ] {
            assert_eq!(mode(p), ResearchMode::Deep, "{p:?}");
        }
    }

    #[test]
    fn none_for_timeless_or_repo_questions() {
        for p in [
            "how do I reverse a vector in rust",
            "what does this orchestrator do",
            "explain how the chat composer works",
        ] {
            assert_eq!(mode(p), ResearchMode::None, "{p:?}");
        }
    }

    #[test]
    fn slash_commands_are_explicit_and_stripped() {
        let q = classify_research_intent("/research latest tokio release", None, None);
        assert_eq!((q.mode, q.explicit, q.query.as_str()), (ResearchMode::Quick, true, "latest tokio release"));
        let d = classify_research_intent("  /deepresearch compare a and b ", None, None);
        assert_eq!((d.mode, d.explicit, d.query.as_str()), (ResearchMode::Deep, true, "compare a and b"));
        // /research search stays on the local KB path, not web research
        assert_eq!(mode("/research search rrf fusion"), ResearchMode::None);
        assert_eq!(mode("/research-search rrf fusion"), ResearchMode::None);
    }

    #[test]
    fn force_flag_overrides_heuristics() {
        assert_eq!(classify_research_intent("hi", Some(true), None).mode, ResearchMode::Quick);
        assert_eq!(classify_research_intent("hi", Some(true), Some("deep")).mode, ResearchMode::Deep);
        assert_eq!(
            classify_research_intent("what is the latest tokio?", Some(false), None).mode,
            ResearchMode::None
        );
    }

    #[test]
    fn every_decision_carries_a_reason() {
        for p in ["hi", "latest tokio version?", "compare a vs b please", "how do I sort"] {
            assert!(!classify_research_intent(p, None, None).reasons.is_empty(), "{p:?}");
        }
    }
}
```

- [ ] **Step 2: Add the module line, run, verify failure**

Add `pub mod research_intent;` to `chat_tools/chat/mod.rs`.
Run: `cargo test -p vox-orchestrator-mcp research_intent`
Expected: compile error — `classify_research_intent` not found.

- [ ] **Step 3: Implement** — above the test module:

```rust
//! Deterministic research-intent classifier for chat turns (spec §4.1).
//! Replaces the confidence-gate heuristic that fired web research on "hi" (D1).

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResearchMode {
    None,
    Quick,
    Deep,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResearchIntent {
    pub mode: ResearchMode,
    pub explicit: bool,
    pub reasons: Vec<String>,
    pub query: String,
}

const GREETINGS: &[&str] = &[
    "hi", "hello", "hey", "yo", "sup", "thanks", "thank you", "thx", "ok", "okay",
    "cool", "nice", "bye", "good morning", "good night", "how are you",
];
const CODE_VERBS: &[&str] = &[
    "fix ", "refactor ", "implement ", "write a function", "write a test", "add a test",
    "rename ", "debug ", "edit ", "update the code", "change the code",
];
const DEEP_CUES: &[&str] = &[
    "compare", "comparison", " vs ", " vs. ", "versus", "trade-off", "tradeoff",
    "pros and cons", "state of the art", "literature review", "deep dive",
    "comprehensive overview", "survey of",
];
const TIME_CUES: &[&str] = &[
    "latest", "current", "currently", "newest", "recent", "recently", "today",
    "this week", "this month", "this year", "right now", "version", "release",
    "released", "price", "pricing", "news", "announced", "as of",
];
const EVIDENCE_CUES: &[&str] = &[
    "look up", "search for", "search the web", "find sources", "sources on",
    "cite ", "citations", "according to",
];
const QUESTION_WORDS: &[&str] = &[
    "what", "what's", "whats", "who", "when", "where", "which", "is", "are", "does",
    "did", "how many", "how much",
];

fn intent(mode: ResearchMode, explicit: bool, reason: String, query: &str) -> ResearchIntent {
    ResearchIntent { mode, explicit, reasons: vec![reason], query: query.trim().to_string() }
}

/// `"/research foo"` → `Some("foo")`; requires a word boundary after the command.
fn strip_command<'a>(prompt: &'a str, cmd: &str) -> Option<&'a str> {
    let rest = prompt.strip_prefix(cmd)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

fn looks_like_path(word: &str) -> bool {
    const EXTS: &[&str] = &[".rs", ".ts", ".tsx", ".js", ".py", ".md", ".toml", ".json", ".vox", ".yml", ".yaml"];
    word.starts_with('@')
        || word.starts_with("./")
        || word.starts_with("crates/")
        || EXTS.iter().any(|e| word.trim_end_matches(|c: char| !c.is_alphanumeric()).ends_with(e))
}

fn has_recent_year(lower: &str) -> bool {
    lower
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|t| (t.len() == 4).then(|| t.parse::<u32>().ok()).flatten())
        .any(|y| (2020..=2100).contains(&y))
}

pub fn classify_research_intent(prompt: &str, force: Option<bool>, scope: Option<&str>) -> ResearchIntent {
    let trimmed = prompt.trim();
    let lower = trimmed.to_ascii_lowercase();

    // 1. Explicit.
    if let Some(rest) = strip_command(trimmed, "/deepresearch") {
        return intent(ResearchMode::Deep, true, "explicit: /deepresearch".into(), rest);
    }
    if lower.starts_with("/research search")
        || lower.starts_with("/research-search")
        || lower.starts_with("/research --search")
    {
        return intent(ResearchMode::None, true, "local knowledge-base search (/research search)".into(), trimmed);
    }
    if let Some(rest) = strip_command(trimmed, "/research") {
        return intent(ResearchMode::Quick, true, "explicit: /research".into(), rest);
    }
    match force {
        Some(true) => {
            let deep = scope.is_some_and(|s| s.eq_ignore_ascii_case("deep"));
            let mode = if deep { ResearchMode::Deep } else { ResearchMode::Quick };
            return intent(mode, true, "explicit: force_research".into(), trimmed);
        }
        Some(false) => {
            return intent(ResearchMode::None, true, "explicit: force_research=false".into(), trimmed);
        }
        None => {}
    }

    // 2. Skip.
    let bare: String = lower
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '\'')
        .collect();
    let bare = bare.trim();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if GREETINGS.iter().any(|g| bare == *g || (bare.starts_with(&format!("{g} ")) && words.len() <= 4)) {
        return intent(ResearchMode::None, false, "skip: greeting / small talk".into(), trimmed);
    }
    if words.len() < 3 {
        return intent(ResearchMode::None, false, "skip: fewer than 3 words".into(), trimmed);
    }
    if trimmed.contains("```") || words.iter().any(|w| looks_like_path(w)) {
        return intent(ResearchMode::None, false, "skip: references code or files".into(), trimmed);
    }
    if CODE_VERBS.iter().any(|v| lower.starts_with(v)) {
        return intent(ResearchMode::None, false, "skip: coding / edit request".into(), trimmed);
    }

    // 3. Deep.
    let padded = format!(" {lower} ");
    if let Some(cue) = DEEP_CUES.iter().find(|c| padded.contains(*c)) {
        return intent(ResearchMode::Deep, false, format!("comparative / survey cue: \"{}\"", cue.trim()), trimmed);
    }

    // 4. Quick.
    if let Some(cue) = EVIDENCE_CUES.iter().find(|c| padded.contains(*c)) {
        return intent(ResearchMode::Quick, false, format!("evidence request: \"{}\"", cue.trim()), trimmed);
    }
    let is_question = trimmed.ends_with('?')
        || QUESTION_WORDS.iter().any(|q| lower.starts_with(&format!("{q} ")));
    let time_cue = TIME_CUES
        .iter()
        .find(|c| padded.contains(&format!(" {c} ")) || padded.contains(&format!(" {c}?")))
        .map(|c| (*c).to_string())
        .or_else(|| has_recent_year(&lower).then(|| "a recent year".to_string()));
    if let (true, Some(cue)) = (is_question, time_cue) {
        return intent(ResearchMode::Quick, false, format!("question + time-sensitive cue: \"{cue}\""), trimmed);
    }

    intent(ResearchMode::None, false, "no research cue".into(), trimmed)
}
```

- [ ] **Step 4: Run until green**

Run: `cargo test -p vox-orchestrator-mcp research_intent -- --nocapture`
Expected: PASS. If a table row fails, fix the **rule**, not the table — unless the row itself is wrong per spec §4.1; note any table change in the commit body.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-orchestrator-mcp/src/chat_tools/chat/research_intent.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs
git commit -m "feat(chat): deterministic research intent classifier

Explicit slash/force flags first, then skip greetings and code work, then
comparative cues (deep) and time-sensitive or evidence questions (quick). Every
decision carries a human-readable reason for the research trace.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: Per-provider search report (D12)

**Files:**
- Modify: `crates/vox-search/src/web_dispatcher.rs:86-446`
- Create: `crates/vox-search/tests/search_report_test.rs`

**Interfaces:**
- Produces (in `vox_search::web_dispatcher`):
```rust
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProviderStatus { Ok { hits: usize }, Timeout, Error { message: String }, NotConfigured, Disabled, CircuitOpen }
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderOutcome { pub provider: &'static str, pub status: ProviderStatus, pub elapsed_ms: u64 }
#[derive(Debug, Clone, Default)]
pub struct SearchReport { pub hits: Vec<HybridSearchHit>, pub providers: Vec<ProviderOutcome> }
impl WebSearchDispatcher {
    pub async fn search_with_report(query: &str, lane: ResearchLane, policy: &SearchPolicy) -> SearchReport;
}
```
- `search_with_lane_and_registry` keeps its signature and returns `Ok(report.hits)`.

- [ ] **Step 1: Failing integration test** — `crates/vox-search/tests/search_report_test.rs`:

```rust
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::{ProviderStatus, WebSearchDispatcher};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn status_of<'a>(r: &'a vox_search::web_dispatcher::SearchReport, p: &str) -> &'a ProviderStatus {
    &r.providers.iter().find(|o| o.provider == p).unwrap_or_else(|| panic!("no outcome for {p}")).status
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_distinguishes_ok_error_timeout_disabled_and_unconfigured() {
    let wiki = MockServer::start().await;
    Mock::given(method("GET")).and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": {"search": [{"title": "Gemini (language model)", "pageid": 7, "snippet": "Gemini Flash"}]}
        })))
        .mount(&wiki).await;
    let openalex = MockServer::start().await;
    Mock::given(method("GET")).and(path("/works"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&openalex).await;

    let mut policy = SearchPolicy::default();
    policy.deep_timeout_ms = 1500;
    policy.tavily_enabled = false;
    policy.searxng_url = None;
    policy.enable_arxiv = false;
    policy.wikipedia_api_url = Some(format!("{}/w/api.php", wiki.uri()));
    policy.openalex_api_url = Some(openalex.uri());

    let r = WebSearchDispatcher::search_with_report("gemini flash", ResearchLane::Deep, &policy).await;

    assert_eq!(status_of(&r, "wikipedia"), &ProviderStatus::Ok { hits: 1 });
    assert!(matches!(status_of(&r, "openalex"), ProviderStatus::Error { .. }), "{:?}", r.providers);
    assert_eq!(status_of(&r, "arxiv"), &ProviderStatus::Disabled);
    assert_eq!(status_of(&r, "searxng"), &ProviderStatus::NotConfigured);
    assert_eq!(r.providers.len(), 5, "every provider is reported, including tavily");
    assert_eq!(r.hits.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_marks_slow_provider_as_timeout() {
    let slow = MockServer::start().await;
    Mock::given(method("GET")).and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(3000)))
        .mount(&slow).await;
    let mut policy = SearchPolicy::default();
    policy.fast_timeout_ms = 300;
    policy.tavily_enabled = false;
    policy.searxng_url = None;
    policy.enable_arxiv = false;
    policy.enable_openalex = false;
    policy.wikipedia_api_url = Some(format!("{}/w/api.php", slow.uri()));

    let r = WebSearchDispatcher::search_with_report("x y z", ResearchLane::Fast, &policy).await;
    assert_eq!(status_of(&r, "wikipedia"), &ProviderStatus::Timeout);
    assert!(r.hits.is_empty());
}
```

(If the OpenAlex client does not surface an HTTP 500 as `Err`, change the mock to return invalid JSON body `"not json"` with 200 — whichever the client turns into an error; the assertion is that failures are reported as `Error`, not `Ok{0}`.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p vox-search --test search_report_test`
Expected: compile error — `ProviderStatus`, `search_with_report` not found.

- [ ] **Step 3: Implement** — in `web_dispatcher.rs`:

1. Add the three public types from **Interfaces** above, plus a private run enum and timer:

```rust
enum ProviderRun {
    Hits(Vec<crate::searxng::SearxngResult>),
    Failed(String),
    Skipped(ProviderStatus),
}

async fn timed(
    provider: &'static str,
    deadline: std::time::Duration,
    run: impl std::future::Future<Output = ProviderRun>,
) -> (Vec<crate::searxng::SearxngResult>, ProviderOutcome) {
    let started = std::time::Instant::now();
    let (hits, status) = match tokio::time::timeout(deadline, run).await {
        Ok(ProviderRun::Hits(h)) => {
            let n = h.len();
            (h, ProviderStatus::Ok { hits: n })
        }
        Ok(ProviderRun::Failed(message)) => (Vec::new(), ProviderStatus::Error { message }),
        Ok(ProviderRun::Skipped(s)) => (Vec::new(), s),
        Err(_) => (Vec::new(), ProviderStatus::Timeout),
    };
    (hits, ProviderOutcome { provider, status, elapsed_ms: started.elapsed().as_millis() as u64 })
}
```

2. Rename the body of `search_with_lane_and_registry` into a new `pub async fn search_with_report_and_registry(query, lane, policy, registry) -> SearchReport`, and in each of the five provider tasks change only the return values:
   - success arm `hits` → `ProviderRun::Hits(hits)`
   - failure arm `Vec::new()` after `warn!` → `ProviderRun::Failed(e.to_string())` (keep the `warn!` and circuit-breaker calls)
   - disabled branch (`enable_*` false / `wikipedia_fallback_enabled` false) → `ProviderRun::Skipped(ProviderStatus::Disabled)`
   - SearXNG with no URL, Tavily with no client (and the `cfg(not(feature = "tavily"))` path) → `ProviderRun::Skipped(ProviderStatus::NotConfigured)`
   - SearXNG circuit open → `ProviderRun::Skipped(ProviderStatus::CircuitOpen)`
3. Replace the `tokio::join!` of `tokio::time::timeout(...)` and the `unwrap_timed` closure with:

```rust
        let (wiki, openalex, arxiv, searxng, tavily) = tokio::join!(
            timed("wikipedia", deadline, wiki_task),
            timed("openalex", deadline, openalex_task),
            timed("arxiv", deadline, arxiv_task),
            timed("searxng", deadline, searxng_task),
            timed("tavily", deadline, tavily_task),
        );
        for (outcome, id) in [
            (&searxng.1, crate::search_circuit_breaker::SearchProviderId::Searxng),
            (&tavily.1, crate::search_circuit_breaker::SearchProviderId::Tavily),
        ] {
            if outcome.status == ProviderStatus::Timeout {
                registry.record_failure(id, false);
            }
        }
        let providers = vec![arxiv.1.clone(), openalex.1.clone(), wiki.1.clone(), searxng.1.clone(), tavily.1.clone()];
        let provider_lists = vec![arxiv.0, openalex.0, wiki.0, searxng.0, tavily.0];
```

4. Every former `return Ok(Vec::new())` becomes `return SearchReport { hits: Vec::new(), providers }`, and the final `Ok(final_hits)` (both `cfg` arms) becomes `SearchReport { hits: final_hits, providers }`. The empty-query early return becomes `SearchReport::default()`.
5. Wrappers:

```rust
    pub async fn search_with_report(query: &str, lane: ResearchLane, policy: &SearchPolicy) -> SearchReport {
        Self::search_with_report_and_registry(
            query, lane, policy,
            crate::search_circuit_breaker::SearchProviderCircuitRegistry::global(),
        ).await
    }

    pub async fn search_with_lane_and_registry(
        query: &str,
        lane: ResearchLane,
        policy: &SearchPolicy,
        registry: &crate::search_circuit_breaker::SearchProviderCircuitRegistry,
    ) -> anyhow::Result<Vec<crate::memory_hybrid::HybridSearchHit>> {
        Ok(Self::search_with_report_and_registry(query, lane, policy, registry).await.hits)
    }
```

- [ ] **Step 4: Run all vox-search tests** (existing lane/partial-harvest tests must stay green — they pin unchanged hit behavior)

Run: `cargo test -p vox-search` and `cargo clippy -p vox-search --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-search/src/web_dispatcher.rs crates/vox-search/tests/search_report_test.rs
git commit -m "feat(search): per-provider outcome report from the web dispatcher

search_with_report returns hits plus ok/timeout/error/disabled/not-configured
per provider, so the chat research trace can show what actually ran. Existing
callers keep identical hit behavior through the old signature.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: Research trace + quick research building blocks

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs` (`pub mod research_turn;`)

**Interfaces:**
- Consumes: `research_intent::{ResearchIntent, ResearchMode}` (Task 4); `vox_search::web_dispatcher::{WebSearchDispatcher, SearchReport, ProviderStatus}` (Task 5).
- Produces:
```rust
pub struct StageRecord { pub stage: &'static str, pub status: &'static str, pub elapsed_ms: Option<u64>, pub summary: String, pub detail: serde_json::Value }
pub struct Source { pub n: usize, pub url: String, pub title: String, pub engine: String, pub snippet: String }
pub struct ResearchTrace { pub intent: ResearchIntent, pub stages: Vec<StageRecord>, pub sources: Vec<Source>, pub model: Option<String>, started: std::time::Instant }
impl ResearchTrace { pub fn new(intent: ResearchIntent) -> Self; pub fn push(&mut self, s: StageRecord); pub fn to_event(&self) -> serde_json::Value; }
pub fn sources_from_hits(hits: &[vox_search::memory_hybrid::HybridSearchHit], max: usize) -> Vec<Source>
pub fn sources_context_block(sources: &[Source]) -> String
pub struct CitationCheck { pub cited: Vec<usize>, pub invalid: Vec<usize> }
pub fn check_citations(answer: &str, source_count: usize) -> CitationCheck
pub fn citation_stage(check: &CitationCheck, source_count: usize) -> StageRecord
pub async fn run_quick(state: &crate::ServerState, trace: &mut ResearchTrace) -> String   // returns the context block
```
Stage `status` vocabulary: `"ok" | "empty" | "skipped" | "degraded" | "failed"`. `to_event` shape (spec §4.6):
`{"kind":"research_trace","mode","explicit","reasons","query","source_count","model","total_ms","status","stages":[…],"sources":[{n,url,title,engine}]}` where `status` is `"failed"` if any stage failed, `"degraded"` if any degraded, else `"ok"`; `"none"` mode → `"skipped"`.

- [ ] **Step 1: Failing unit tests** (bottom of the new file):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_tools::chat::research_intent::classify_research_intent;
    use vox_search::memory_hybrid::HybridSearchHit;

    fn hit(url: &str, engine: &str) -> HybridSearchHit {
        HybridSearchHit {
            path: url.into(),
            title: format!("t {url}"),
            content_snippet: "snippet ".repeat(200),
            score: 0.5,
            provenance: vec!["WebResearch".into(), format!("engine:{engine}")],
            potential_contradiction: false,
        }
    }

    #[test]
    fn sources_are_numbered_deduped_web_only_and_capped() {
        let hits = vec![
            hit("https://a.example/1", "searxng"),
            hit("https://a.example/1", "wikipedia"),
            hit("docs/internal.md", "bm25"),
            hit("https://b.example/2", "wikipedia"),
            hit("https://c.example/3", "arxiv"),
        ];
        let s = sources_from_hits(&hits, 2);
        assert_eq!(s.iter().map(|x| (x.n, x.url.as_str())).collect::<Vec<_>>(),
                   vec![(1, "https://a.example/1"), (2, "https://b.example/2")]);
        assert_eq!(s[0].engine, "searxng");
        assert!(s[0].snippet.chars().count() <= 600);
    }

    #[test]
    fn context_block_numbers_sources_and_demands_citations() {
        let s = sources_from_hits(&[hit("https://a.example/1", "searxng")], 8);
        let b = sources_context_block(&s);
        assert!(b.starts_with("[WEB RESEARCH — 1 SOURCES]"), "{b}");
        assert!(b.contains("[1] t https://a.example/1 — https://a.example/1 (searxng)"), "{b}");
        assert!(b.contains("cite"), "{b}");
    }

    #[test]
    fn empty_context_block_forbids_invented_citations() {
        let b = sources_context_block(&[]);
        assert!(b.starts_with("[WEB RESEARCH — 0 SOURCES]"), "{b}");
        assert!(b.contains("do not invent citations"), "{b}");
    }

    #[test]
    fn citation_check_finds_valid_and_invalid_markers() {
        let c = check_citations("Gemini 3.8 Flash [1][3]. Also [2, 9] and [x] and [10].", 3);
        assert_eq!(c.cited, vec![1, 2, 3]);
        assert_eq!(c.invalid, vec![9, 10]);
    }

    #[test]
    fn citation_stage_fails_on_uncited_answer_with_sources() {
        let st = citation_stage(&check_citations("no markers here", 4), 4);
        assert_eq!(st.status, "failed");
        let ok = citation_stage(&check_citations("fact [1]", 4), 4);
        assert_eq!(ok.status, "ok");
    }

    #[test]
    fn trace_event_for_no_research_is_skipped_with_detection_stage() {
        let t = ResearchTrace::new(classify_research_intent("hi", None, None));
        let e = t.to_event();
        assert_eq!(e["kind"], "research_trace");
        assert_eq!(e["mode"], "none");
        assert_eq!(e["status"], "skipped");
        assert_eq!(e["stages"][0]["stage"], "detection");
        assert!(e["reasons"][0].as_str().unwrap().contains("greeting"));
    }

    #[test]
    fn trace_status_is_failed_when_any_stage_failed() {
        let mut t = ResearchTrace::new(classify_research_intent("/research x y z", None, None));
        t.push(StageRecord::new("retrieval", "failed", Some(3), "boom".into(), serde_json::json!({})));
        assert_eq!(t.to_event()["status"], "failed");
    }
}
```

- [ ] **Step 2: Run, verify failure**

Run: `cargo test -p vox-orchestrator-mcp research_turn`
Expected: compile error.

- [ ] **Step 3: Implement** (top of `research_turn.rs`):

```rust
//! Chat research turn: typed trace, quick-research retrieval, citation check (spec §4.3, §4.6).

use std::collections::{BTreeSet, HashSet};
use std::time::Instant;

use serde::Serialize;
use serde_json::{Value, json};
use vox_search::memory_hybrid::HybridSearchHit;
use vox_search::web_dispatcher::{ProviderStatus, WebSearchDispatcher};

use super::research_intent::{ResearchIntent, ResearchMode};

#[derive(Debug, Clone, Serialize)]
pub struct StageRecord {
    pub stage: &'static str,
    pub status: &'static str,
    pub elapsed_ms: Option<u64>,
    pub summary: String,
    pub detail: Value,
}

impl StageRecord {
    pub fn new(stage: &'static str, status: &'static str, elapsed_ms: Option<u64>, summary: String, detail: Value) -> Self {
        Self { stage, status, elapsed_ms, summary, detail }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub n: usize,
    pub url: String,
    pub title: String,
    pub engine: String,
    pub snippet: String,
}

#[derive(Debug)]
pub struct ResearchTrace {
    pub intent: ResearchIntent,
    pub stages: Vec<StageRecord>,
    pub sources: Vec<Source>,
    pub model: Option<String>,
    started: Instant,
}

impl ResearchTrace {
    pub fn new(intent: ResearchIntent) -> Self {
        let detection = StageRecord::new(
            "detection",
            "ok",
            None,
            format!("{} — {}", mode_str(intent.mode), intent.reasons.join("; ")),
            json!({ "explicit": intent.explicit, "reasons": intent.reasons }),
        );
        Self { intent, stages: vec![detection], sources: Vec::new(), model: None, started: Instant::now() }
    }

    pub fn push(&mut self, s: StageRecord) {
        self.stages.push(s);
    }

    pub fn to_event(&self) -> Value {
        let status = if self.intent.mode == ResearchMode::None {
            "skipped"
        } else if self.stages.iter().any(|s| s.status == "failed") {
            "failed"
        } else if self.stages.iter().any(|s| s.status == "degraded") {
            "degraded"
        } else {
            "ok"
        };
        json!({
            "kind": "research_trace",
            "mode": mode_str(self.intent.mode),
            "explicit": self.intent.explicit,
            "reasons": self.intent.reasons,
            "query": self.intent.query,
            "source_count": self.sources.len(),
            "model": self.model,
            "total_ms": self.started.elapsed().as_millis() as u64,
            "status": status,
            "stages": self.stages,
            "sources": self.sources.iter().map(|s| json!({"n": s.n, "url": s.url, "title": s.title, "engine": s.engine})).collect::<Vec<_>>(),
        })
    }
}

fn mode_str(m: ResearchMode) -> &'static str {
    match m {
        ResearchMode::None => "none",
        ResearchMode::Quick => "quick",
        ResearchMode::Deep => "deep",
    }
}

pub fn sources_from_hits(hits: &[HybridSearchHit], max: usize) -> Vec<Source> {
    let mut seen = HashSet::new();
    hits.iter()
        .filter(|h| h.path.starts_with("http://") || h.path.starts_with("https://"))
        .filter(|h| seen.insert(h.path.clone()))
        .take(max)
        .enumerate()
        .map(|(i, h)| Source {
            n: i + 1,
            url: h.path.clone(),
            title: h.title.clone(),
            engine: h.provenance.iter().find_map(|p| p.strip_prefix("engine:")).unwrap_or("unknown").to_string(),
            snippet: h.content_snippet.chars().take(600).collect(),
        })
        .collect()
}

pub fn sources_context_block(sources: &[Source]) -> String {
    if sources.is_empty() {
        return "[WEB RESEARCH — 0 SOURCES]\nWeb research ran for this question and returned no sources. \
                Tell the user plainly that the search found nothing; do not invent citations or claim \
                to have found evidence.\n"
            .to_string();
    }
    let mut out = format!(
        "[WEB RESEARCH — {} SOURCES]\nAnswer the user's question from these sources. Cite every factual \
         claim inline as [n] using the numbers below. If the sources do not answer the question, say so \
         explicitly instead of guessing.\n",
        sources.len()
    );
    for s in sources {
        out.push_str(&format!("\n[{}] {} — {} ({})\n{}\n", s.n, s.title, s.url, s.engine, s.snippet.replace('\n', " ")));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CitationCheck {
    pub cited: Vec<usize>,
    pub invalid: Vec<usize>,
}

pub fn check_citations(answer: &str, source_count: usize) -> CitationCheck {
    let mut cited = BTreeSet::new();
    let mut invalid = BTreeSet::new();
    let mut rest = answer;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else { break };
        let inner = &after[..close];
        let nums: Vec<Option<usize>> = inner.split(',').map(|t| t.trim().parse::<usize>().ok()).collect();
        if !nums.is_empty() && nums.iter().all(Option::is_some) {
            for n in nums.into_iter().flatten() {
                if (1..=source_count).contains(&n) { cited.insert(n); } else { invalid.insert(n); }
            }
        }
        rest = &after[close + 1..];
    }
    CitationCheck { cited: cited.into_iter().collect(), invalid: invalid.into_iter().collect() }
}

pub fn citation_stage(check: &CitationCheck, source_count: usize) -> StageRecord {
    let status = if !check.invalid.is_empty() || (source_count > 0 && check.cited.is_empty()) {
        "failed"
    } else {
        "ok"
    };
    StageRecord::new(
        "citation_check",
        status,
        None,
        format!(
            "cited {}/{} sources{}",
            check.cited.len(),
            source_count,
            if check.invalid.is_empty() { String::new() } else { format!(", invalid markers {:?}", check.invalid) }
        ),
        json!(check),
    )
}

/// Quick research: one retrieval wave on the Deep-lane deadline, numbered sources.
/// Returns the context block to inject into the chat prompt.
pub async fn run_quick(state: &crate::ServerState, trace: &mut ResearchTrace) -> String {
    let policy = {
        let cfg = state.orchestrator.config_handle();
        vox_orchestrator::sync_lock::rw_read(&*cfg).effective_search_policy()
    };
    let query = trace.intent.query.clone();
    trace.push(StageRecord::new("queries", "ok", None, format!("1 query: {query}"), json!({ "queries": [query] })));

    let t = Instant::now();
    // Deep-lane deadline: SearXNG does not fit the 1.5 s fast lane (spec §9).
    let report = WebSearchDispatcher::search_with_report(&query, vox_search::policy::ResearchLane::Deep, &policy).await;
    let answered = report.providers.iter().filter(|p| matches!(p.status, ProviderStatus::Ok { hits } if hits > 0)).count();
    let failed = report.providers.iter().any(|p| matches!(p.status, ProviderStatus::Error { .. } | ProviderStatus::Timeout));
    let status = match (report.hits.is_empty(), failed) {
        (true, true) => "failed",
        (true, false) => "empty",
        (false, true) => "degraded",
        (false, false) => "ok",
    };
    trace.push(StageRecord::new(
        "retrieval",
        status,
        Some(t.elapsed().as_millis() as u64),
        format!("{} hits from {answered}/{} providers", report.hits.len(), report.providers.len()),
        json!({ "providers": report.providers }),
    ));

    trace.sources = sources_from_hits(&report.hits, 8);
    trace.push(StageRecord::new(
        "sources",
        if trace.sources.is_empty() { "empty" } else { "ok" },
        None,
        format!("{} web sources kept", trace.sources.len()),
        json!({ "sources": trace.sources }),
    ));
    sources_context_block(&trace.sources)
}
```

(If `config_handle()`/`effective_search_policy()` are named differently on `state.orchestrator`, use the accessor `perform_autonomous_research` uses in `research_dispatch.rs:655-658` — `self.config` + `effective_search_policy()` — via the public equivalent on `Orchestrator`.)

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p vox-orchestrator-mcp research_turn && cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs
git commit -m "feat(chat): research trace, quick retrieval, and citation check

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 7: Deep pipeline honesty — no template, real judge, lane, cache (D5–D8)

**Files:**
- Modify: `crates/vox-research-shim/src/research/orchestrator/stages.rs` (synthesis, judge, delete template + its test at ~`:680-705`)
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs` (`:65-69` cache, `:845-870` synthesis/judge, `:993-1012` metadata)
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline_cache.rs` (`research_cache_key`, metadata literal `:118`)
- Modify: `crates/vox-research-shim/src/research/orchestrator/config.rs:142` (`judge_max_tokens`)
- Modify: `crates/vox-research-shim/src/research/types.rs:209-239` (`ResearchMetadata`)
- Modify: every other `ResearchMetadata { … }` literal the compiler reports (`pipeline.rs:~1362`, `discovery_bridge.rs:~292`)
- Modify: `crates/vox-orchestrator-mcp/src/memory_tools/params.rs` (`ResearchStartParams`, `ResearchRunParams`: `lane: Option<String>`), `handlers_memory.rs:300,398`, and `input_schemas.rs` entries for these tools if they are `additionalProperties: false`.

**Interfaces:**
- Produces: `synthesize_answer_with_llm(..) -> anyhow::Result<String>`; `judge_quality(..) -> Result<i32, String>` (no `fallback_score` field); `ResearchMetadata` gains `#[serde(default)] subqueries: Vec<String>`, `#[serde(default)] synthesis_model: String`, `#[serde(default)] judge_error: Option<String>`, `#[serde(default)] served_from_cache: bool`; `research_start`/`research_run` honor `lane: "deep" | "fast"`.

- [ ] **Step 1: Failing tests** in `stages.rs` test module (runtime feature is on for the crate's tests; if not, gate with `#[cfg(feature = "runtime")]`):

```rust
    #[tokio::test]
    #[allow(unsafe_code)]
    async fn synthesis_failure_is_an_error_not_a_template() {
        // No key + unreachable pinned model => every candidate fails.
        unsafe { std::env::set_var("VOX_MODEL_FORCE", "nonexistent/model-for-test"); }
        unsafe { std::env::set_var("OPENROUTER_BASE_URL", "http://127.0.0.1:9"); }
        let hits = vec![crate::research::types::ResearchHit {
            url: "https://a.example".into(), title: "a".into(), snippet: "s".into(),
            score: 1.0, http_status: 200, trust_score: 1.0, raw_content: String::new(),
        }];
        let r = super::synthesize_answer_with_llm(super::SynthesisParams {
            query: "q", hits: &hits, verdicts: &[], endpoint: None, api_key: None,
            model: "nonexistent/model-for-test", temperature: 0.2, max_tokens: 100, context_max_chars: 4000,
        }).await;
        unsafe { std::env::remove_var("VOX_MODEL_FORCE"); std::env::remove_var("OPENROUTER_BASE_URL"); }
        let err = r.expect_err("a failed synthesis must be Err — never template prose");
        assert!(err.to_string().contains("synthesis failed"), "{err}");
    }

    #[test]
    fn judge_budget_fits_its_schema() {
        let c = super::super::config::ResearchConfig::default();
        assert!(c.judge_max_tokens >= 400, "judge_max_tokens {} truncates the judge JSON (D7)", c.judge_max_tokens);
    }
```

And in `pipeline_cache.rs` tests:

```rust
    #[test]
    fn cache_key_distinguishes_lanes() {
        let mut q = test_query("same question");   // use this module's existing query builder; if none, build ResearchQuery literally
        q.lane = vox_search::policy::ResearchLane::Fast;
        let fast = research_cache_key(&q);
        q.lane = vox_search::policy::ResearchLane::Deep;
        assert_ne!(fast, research_cache_key(&q));
    }
```

- [ ] **Step 2: Run, verify failure**

Run: `cargo test -p vox-research-shim synthesis_failure_is_an_error judge_budget cache_key_distinguishes`
Expected: compile error (return type is `String`), then after that: `judge_budget_fits_its_schema` fails with 16, `cache_key_distinguishes_lanes` fails.

- [ ] **Step 3: Implement**

`stages.rs`:

```rust
/// LLM-backed synthesis. There is deliberately no template fallback (spec §5):
/// a failed synthesis is an error the caller must surface.
pub(super) async fn synthesize_answer_with_llm(params: SynthesisParams<'_>) -> anyhow::Result<String> {
    call_synthesis_llm(&params).await.map_err(|e| anyhow::anyhow!("synthesis failed: {e}"))
}
```
Delete `fn synthesize_answer_template` and the test that calls it. Change `judge_quality` to return `Result<i32, String>`: the `chat_stage` error → `Err(format!("judge call failed: {e}"))`; parse failure → `Err(format!("judge returned unparseable JSON: {}", content.chars().take(200).collect::<String>()))`; `total_score <= 0` → `Err("judge returned no score".into())`; success → `Ok(parsed.total_score.clamp(1, 100))`. Remove `fallback_score` from `JudgeParams`.

`config.rs:142`: `judge_max_tokens: 400,`. If `fallback_quality_score` then has no reader outside its own default, delete the field and its default.

`types.rs` `ResearchMetadata` — append:

```rust
    #[serde(default)]
    pub subqueries: Vec<String>,
    /// Model that produced `answer` (the pin when `VOX_MODEL_FORCE` is set).
    #[serde(default)]
    pub synthesis_model: String,
    /// Set when the judge could not score the answer; `quality_score` is then 0, never a synthetic default.
    #[serde(default)]
    pub judge_error: Option<String>,
    #[serde(default)]
    pub served_from_cache: bool,
```

`pipeline.rs` synthesis/judge (`:845-870`):

```rust
    let answer = match synthesize_answer_with_llm(SynthesisParams { /* unchanged fields */ }).await {
        Ok(a) => a,
        Err(e) => {
            set_session_stage(db, session_id, ResearchStage::Failed).await;
            return Err(e);
        }
    };
    let (quality_score, judge_error) = match judge_quality(JudgeParams { /* unchanged minus fallback_score */ }).await {
        Ok(s) => (s, None),
        Err(e) => {
            tracing::warn!(error = %e, "research judge failed");
            (0, Some(e))
        }
    };
```
Metadata literal (`:993`): add `subqueries: plan.subqueries.clone(), synthesis_model: vox_config::inference::forced_model().unwrap_or_else(|| resolved_llm.synthesis_model.clone()), judge_error, served_from_cache: false,`. Add the four fields with defaults (`Vec::new()`, `String::new()`, `None`, `false`) to every other literal the compiler reports.

Cache (`pipeline.rs:65-69`):

```rust
    if let Some(db) = db
        && let Some(mut cached) = research_cache_short_circuit(&query, db, config).await
    {
        cached.research_metadata.served_from_cache = true;
        if let Some(id) = precreated_session_id {
            set_session_stage(Some(db), id, ResearchStage::Completed).await;
        }
        return Ok(cached);
    }
```
`pipeline_cache.rs::research_cache_key`: add `query.lane` to the formatted key (`"{}|{:?}|{}|{}|{:?}"` … `, query.lane`).

Lane plumbing (`memory_tools/params.rs`): add `#[serde(default)] pub lane: Option<String>` to `ResearchStartParams` and `ResearchRunParams`; in `handlers_memory.rs` replace both `lane: vox_search::policy::ResearchLane::default(),` with

```rust
        lane: match params.lane.as_deref() {
            Some(l) if l.eq_ignore_ascii_case("deep") => vox_search::policy::ResearchLane::Deep,
            _ => vox_search::policy::ResearchLane::Fast,
        },
```
(for `research_start`, read `params.lane` before `params` is moved into the spawned task). If `input_schemas.rs` declares these tools `additionalProperties: false`, add `"lane":{"type":"string","enum":["fast","deep"]}`.

- [ ] **Step 4: Mutation check of the no-template guard** — temporarily make `synthesize_answer_with_llm` return `Ok(format!("# Executive Summary\n{}", params.query))` on error; run `cargo test -p vox-research-shim synthesis_failure_is_an_error`; it MUST fail. Restore; confirm with `git diff --stat` that only intended changes remain; re-run — PASS.

- [ ] **Step 5: Run everything touched**

Run: `cargo test -p vox-research-shim -p vox-orchestrator-mcp 2>&1 | tail -30` and clippy on both.
Expected: PASS. Any existing test asserting template text or a score of 80 was pinning the bug — delete/update it and list it in the commit body.

- [ ] **Step 6: Commit**

```bash
git add crates/vox-research-shim/src/research crates/vox-orchestrator-mcp/src/memory_tools/params.rs crates/vox-orchestrator-mcp/src/memory_tools/handlers_memory.rs crates/vox-orchestrator-mcp/src/input_schemas.rs
git commit -m "fix(research): no template answers, real judge score, honored lane, honest cache

Synthesis failure is now an error (template deleted); the judge gets a token
budget that fits its schema and reports failure instead of a synthetic 80;
research_start/research_run honor lane=deep so the planner runs; cache hits are
flagged and close their session.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 8: Wire research into `vox_chat_message` (D2, D3)

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs` (add `run_deep`, `deep_stages`)
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` (`:17` import, `:750-833` research block, `:891-895` user prompt, `:966-976` precomputed reply, before `:1591` events)
- Modify: `crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs:408-453` and its two tests (delete `should_trigger_autonomous_research`), `memory_tools/mod.rs:27` (re-export)

**Interfaces:**
- Consumes: Tasks 4, 6, 7.
- Produces: `pub async fn run_deep(state: &crate::ServerState, trace: &mut ResearchTrace) -> Result<String, String>` (Ok = answer text; sets `trace.model`, `trace.sources`); every `vox_chat_message` envelope's `data.events[0]` is the `research_trace` event.

- [ ] **Step 1: Failing test** in `message.rs` tests (next to `chat_message_envelope_includes_latency_ms`, same harness):

```rust
    #[tokio::test]
    #[allow(unsafe_code)]
    #[allow(clippy::await_holding_lock)]
    async fn every_turn_carries_a_research_trace_event() {
        let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(plain_response_body("hello back")))
            .mount(&server)
            .await;
        let prev_base = std::env::var("OPENROUTER_BASE_URL").ok();
        let prev_key = std::env::var("OPENROUTER_API_KEY").ok();
        unsafe {
            std::env::set_var("OPENROUTER_BASE_URL", server.uri());
            std::env::set_var("OPENROUTER_API_KEY", "test-key");
        }
        vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);
        let state = test_state();
        let model_id = "test-openrouter-model-trace";
        {
            let handle = state.orchestrator.models_handle();
            handle.write().expect("models lock").register(model_spec(ProviderType::OpenRouter, model_id));
        }
        *state.mcp_chat_model_override.write() = Some(model_id.to_string());

        let params: ChatMessageParams =
            serde_json::from_value(serde_json::json!({ "prompt": "hi" })).expect("params");
        let response_json = chat_message(&state, params).await;

        unsafe {
            match prev_base { Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v), None => std::env::remove_var("OPENROUTER_BASE_URL") }
            match prev_key { Some(v) => std::env::set_var("OPENROUTER_API_KEY", v), None => std::env::remove_var("OPENROUTER_API_KEY") }
        }
        vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

        let parsed: serde_json::Value = serde_json::from_str(&response_json).expect("json");
        assert_eq!(parsed["success"], true, "{response_json}");
        let ev = &parsed["data"]["events"][0];
        assert_eq!(ev["kind"], "research_trace", "{response_json}");
        assert_eq!(ev["mode"], "none");
        assert_eq!(ev["stages"][0]["stage"], "detection");
        // D1 regression: a greeting must not reach web retrieval.
        assert!(ev["stages"].as_array().unwrap().iter().all(|s| s["stage"] != "retrieval"), "{ev}");
    }
```

- [ ] **Step 2: Run, verify failure**

Run: `cargo test -p vox-orchestrator-mcp every_turn_carries_a_research_trace_event`
Expected: FAIL — `events[0]` is null.

- [ ] **Step 3: Implement `run_deep`** in `research_turn.rs`:

```rust
/// Deep research: the Scientia pipeline inline on the Deep lane (spec §4.4).
/// Ok = the pipeline's answer (LLM-synthesized; the template path no longer exists).
pub async fn run_deep(state: &crate::ServerState, trace: &mut ResearchTrace) -> Result<String, String> {
    use std::sync::{Arc, Mutex};
    use vox_research_shim::research::{
        BroadcastEmitter, ResearchConfig, ResearchDomainMode, ResearchQuery, ResearchScope,
        run_research_with_context,
    };
    let progress: Arc<Mutex<Vec<(String, Option<f32>, u64)>>> = Arc::default();
    let sink = progress.clone();
    let t = Instant::now();
    let config = ResearchConfig {
        event_emitter: Some(Arc::new(BroadcastEmitter::new(state.research_events.clone()))),
        progress_callback: Some(Arc::new(move |msg: String, pct: Option<f32>| {
            if let Ok(mut v) = sink.lock() {
                v.push((msg, pct, t.elapsed().as_millis() as u64));
            }
        })),
        ..ResearchConfig::default()
    };
    let rq = ResearchQuery {
        query: trace.intent.query.clone(),
        scope: ResearchScope::Web,
        max_sources: 10,
        persist_to_docs: false,
        verify_claims: true,
        site_scope: None,
        domain_mode: ResearchDomainMode::General,
        waves: 1,
        lane: vox_search::policy::ResearchLane::Deep,
    };
    let ctx = vox_search::SearchRuntimeContext::new(
        state.repository.root.clone(),
        state.db.clone(),
        state.orchestrator_config.memory.log_dir.clone(),
        state.orchestrator_config.memory.memory_md_path.clone(),
    );
    let outcome = run_research_with_context(rq, Some(&ctx), state.db.as_deref(), &config).await;
    let timeline = progress.lock().map(|v| v.clone()).unwrap_or_default();
    trace.push(StageRecord::new(
        "pipeline_progress",
        "ok",
        Some(t.elapsed().as_millis() as u64),
        format!("{} progress events", timeline.len()),
        json!(timeline.iter().map(|(m, p, ms)| json!({"message": m, "pct": p, "at_ms": ms})).collect::<Vec<_>>()),
    ));
    match outcome {
        Ok(r) => {
            for s in deep_stages(&r) {
                trace.push(s);
            }
            trace.sources = r.sources.iter().enumerate().map(|(i, h)| Source {
                n: i + 1, url: h.url.clone(), title: h.title.clone(),
                engine: "pipeline".into(), snippet: h.snippet.chars().take(600).collect(),
            }).collect();
            trace.model = Some(r.research_metadata.synthesis_model.clone());
            Ok(r.answer)
        }
        Err(e) => {
            trace.push(StageRecord::new("deep_pipeline", "failed", Some(t.elapsed().as_millis() as u64), e.to_string(), json!({})));
            Err(e.to_string())
        }
    }
}

/// Map a completed pipeline result onto trace stages.
pub fn deep_stages(r: &vox_research_shim::research::ResearchResult) -> Vec<StageRecord> {
    use vox_research_shim::research::verifier::Verdict;
    let m = &r.research_metadata;
    let count = |v: Verdict| m.claim_verdicts.iter().filter(|c| c.verdict == v).count();
    let mut out = Vec::new();
    if m.served_from_cache {
        out.push(StageRecord::new("cache", "degraded", None, "served from cache (≤1h old); pipeline did not re-run".into(), json!({})));
    }
    out.push(StageRecord::new(
        "planning",
        if m.planner_degraded { "degraded" } else { "ok" },
        None,
        format!("{} subqueries{}", m.subqueries.len(), if m.planner_degraded { " (planner failed — passthrough)" } else { "" }),
        json!({ "subqueries": m.subqueries }),
    ));
    out.push(StageRecord::new(
        "retrieval",
        if m.source_count == 0 { "empty" } else { "ok" },
        None,
        format!("{} sources, {} distinct domains", m.source_count, m.retrieval_diagnostics.distinct_domain_count),
        json!(m.retrieval_diagnostics),
    ));
    out.push(StageRecord::new(
        "claims",
        if m.claim_verdicts.is_empty() { "empty" } else { "ok" },
        None,
        format!(
            "{} claims: {} supported, {} contested, {} contradicted, {} unverified",
            m.claim_verdicts.len(), count(Verdict::Supported), count(Verdict::Contested),
            count(Verdict::Contradicted), count(Verdict::Unverified)
        ),
        json!(m.claim_verdicts.iter().map(|c| json!({"claim": c.claim, "verdict": c.verdict.to_string(), "confidence": c.confidence})).collect::<Vec<_>>()),
    ));
    out.push(StageRecord::new("synthesis", "ok", None, format!("synthesized by {}", m.synthesis_model), json!({ "model": m.synthesis_model })));
    out.push(match &m.judge_error {
        None => StageRecord::new("judge", "ok", None, format!("quality {}/100", m.quality_score), json!({ "quality_score": m.quality_score })),
        Some(e) => StageRecord::new("judge", "failed", None, format!("judge failed: {e}"), json!({ "error": e })),
    });
    if let Some(a) = &m.citation_audit {
        out.push(StageRecord::new(
            "citation_audit",
            if a.unsupported_citation_indices.is_empty() { "ok" } else { "degraded" },
            None,
            format!("{}/{} citations supported (precision {:.2})", a.supported_citations, a.checked_citations, a.precision),
            json!({ "unsupported": a.unsupported_citation_indices, "precision": a.precision }),
        ));
    }
    out
}
```
(Adjust paths if `Verdict`/`ResearchResult`/`BroadcastEmitter` are re-exported elsewhere; `Verdict` needs `PartialEq` — derive it if missing. Import `json`/`Source` already in scope.)

- [ ] **Step 4: Wire into `chat_message`**

1. Remove `should_trigger_autonomous_research` from the `use` at `message.rs:17`; delete the function and its two tests in `retrieval.rs`, and its re-export in `memory_tools/mod.rs:27`.
2. Delete the whole "Check if autonomous deep research should be triggered" block (`message.rs:750-833`, from `let is_research_slash` through the closing `}` of the `if vox_orchestrator::is_chat_research_enabled() …`). Local retrieval context injection above it stays.
3. Directly **after** the local-retrieval `if explicit_search_result.is_none() { … }` block (i.e. outside the `Ok(bundle)` arm — fixes D3), insert:

```rust
    // Research (spec §4): classify, then run quick/deep; every turn gets a trace.
    let intent = super::research_intent::classify_research_intent(
        &expanded_prompt,
        params.force_research,
        params.research_scope.as_deref(),
    );
    let mut research_trace = super::research_turn::ResearchTrace::new(intent.clone());
    let mut deep_answer: Option<Result<String, String>> = None;
    if explicit_search_result.is_none() && vox_orchestrator::is_chat_research_enabled() {
        match intent.mode {
            super::research_intent::ResearchMode::None => {}
            super::research_intent::ResearchMode::Quick => {
                let block = super::research_turn::run_quick(state, &mut research_trace).await;
                context_parts.push(block);
            }
            super::research_intent::ResearchMode::Deep => {
                deep_answer = Some(super::research_turn::run_deep(state, &mut research_trace).await);
            }
        }
    }
    // Slash commands: the model sees the question, not "/research …".
    let expanded_prompt = if intent.explicit && intent.mode != super::research_intent::ResearchMode::None {
        intent.query.clone()
    } else {
        expanded_prompt
    };
```
4. At the reply match (`message.rs:966`), before `if let Some(local_res) = explicit_search_result`, add a deep branch so the LLM is not called again:

```rust
    let (response_text, model_used, tokens, selection_reason, mut events) = if let Some(deep) = deep_answer {
        match deep {
            Ok(answer) => (
                answer,
                research_trace.model.clone().unwrap_or_default(),
                0u64,
                Some("deep research pipeline synthesis".to_string()),
                vec![],
            ),
            Err(e) => (
                // Status message, not an answer: the trace shows the failing stage and the sources.
                format!("Deep research failed: {e}\n\nNo answer was generated. The research trace below shows which stage failed and what was retrieved."),
                "none".to_string(),
                0u64,
                Some("deep research failed".to_string()),
                vec![],
            ),
        }
    } else if let Some(local_res) = explicit_search_result {
```
(keep the rest of the chain as-is; make the binding `mut events`.)
5. Just before the envelope is built (`"events": events` at `~:1591`):

```rust
    if intent.mode == super::research_intent::ResearchMode::Quick {
        research_trace.model = Some(model_used.clone());
        let check = super::research_turn::check_citations(&response_text, research_trace.sources.len());
        research_trace.push(super::research_turn::citation_stage(&check, research_trace.sources.len()));
    }
    events.insert(0, research_trace.to_event());
```

- [ ] **Step 5: Run tests + clippy**

Run: `cargo test -p vox-orchestrator-mcp 2>&1 | tail -30` and `cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings`
Expected: PASS, including the new test and the existing `chat_message_*` tests.

- [ ] **Step 6: Live daemon smoke** (real Gemini, real SearXNG from Task 2) — rebuild `vox-orchestrator-d`, then run the stdio probe from Task 1 Step 5 three times with prompts `hi`, `What is the latest Gemini Flash model on OpenRouter and when was it released?`, and `/deepresearch compare SearXNG and Tavily for agent web search` (use `sleep 180` / `timeout 190s` for deep). Save each raw line to `$SCRATCH/live-<n>.json` and print:

```bash
jq -c '.payload.value | {success, model: .data.model_used, head: (.data.message.content[:300]), trace: (.data.events[0] | {mode, status, source_count, model, stages: [.stages[] | {stage, status, summary}]})}' "$SCRATCH/live-2.json"
```
Expected: `hi` → mode none, no retrieval stage. Quick → `searxng` Ok with hits>0, ≥3 sources, citation_check ok, answer mentions `3.8`, model `google/gemini-3.8-flash`. Deep → planning ≥2 subqueries, claims stage present, judge ok with a model score, synthesis model `google/gemini-3.8-flash`. **If any expectation fails, stop and debug (superpowers:systematic-debugging) before Task 9.** Paste the three outputs into the task report verbatim.

- [ ] **Step 7: Commit**

```bash
git add crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs crates/vox-orchestrator-mcp/src/memory_tools/mod.rs crates/vox-research-shim/src/research/verifier.rs
git commit -m "feat(chat): classifier-driven quick/deep research with a trace on every turn

Replaces the inverted confidence gate and the raw-snippet 'SYNTHESIS SUMMARY'
injection: quick research gives Gemini numbered sources and checks its [n]
citations; deep research runs the Scientia pipeline on the Deep lane and returns
its answer directly. Every envelope carries a research_trace event.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 9: GUI — keep research on the sync path and render the trace (D4, D11)

**Files:**
- Modify: `crates/vox-gui/ui/src/lib/buildChatTurn.ts:74-112` and `lib/buildChatTurn.test.ts`
- Create: `crates/vox-gui/ui/src/components/surfaces/Chat/ResearchTracePanel.tsx`, `ResearchTracePanel.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx`
- Delete (only if `rg -l "ResearchSummaryCard|components/chat/ChatMessage" crates/vox-gui/ui/src` lists nothing but their own files and tests): `crates/vox-gui/ui/src/components/chat/ChatMessage.tsx`, `ResearchSummaryCard.tsx` and their tests
- Modify: `crates/vox-gui/src/commands/search_probe.rs` (DDG row → `not_implemented`, no HTTP call)

**Interfaces:**
- Consumes: the `research_trace` event shape from Task 6.
- Produces: `data-testid`s: `research-trace` (panel), `research-trace-toggle`, `research-stage-<stage>` (row), `research-source-<n>` (link).

- [ ] **Step 1: Failing tests**

`buildChatTurn.test.ts` — add:

```ts
  it('keeps research slash commands on the sync path with the raw text', () => {
    const t = buildChatTurn({ description: '/deepresearch compare a and b' } as ChatTurnSource, { sessionId: 's1' } as BuildChatTurnCtx);
    expect(t.execution).toBe('sync');
    expect(t.content).toBe('/deepresearch compare a and b');
    expect(t.research_scope ?? null).toBeNull();
  });
```
(Replace any existing test asserting `execution: 'background'` / `research_scope: 'deep'` for slash commands — it pinned D4.)

`ResearchTracePanel.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ResearchTracePanel } from './ResearchTracePanel';

const quick = {
  kind: 'research_trace', mode: 'quick', explicit: false, status: 'ok', source_count: 2,
  model: 'google/gemini-3.8-flash', total_ms: 4200, reasons: ['question + time-sensitive cue: "latest"'],
  stages: [
    { stage: 'detection', status: 'ok', summary: 'quick — question + time-sensitive cue', detail: {} },
    { stage: 'retrieval', status: 'degraded', elapsed_ms: 3100, summary: '9 hits from 2/5 providers',
      detail: { providers: [{ provider: 'searxng', status: { state: 'ok', hits: 8 }, elapsed_ms: 900 },
                            { provider: 'openalex', status: { state: 'timeout' }, elapsed_ms: 4000 }] } },
    { stage: 'citation_check', status: 'failed', summary: 'cited 0/2 sources', detail: {} },
  ],
  sources: [{ n: 1, url: 'https://openrouter.ai/google/gemini-3.8-flash', title: 'Gemini 3.8 Flash', engine: 'searxng' },
            { n: 2, url: 'https://ai.google.dev/', title: 'Gemini API', engine: 'searxng' }],
};

describe('ResearchTracePanel', () => {
  it('shows a header with mode, source count, model and time', () => {
    render(<ResearchTracePanel trace={quick} />);
    const header = screen.getByTestId('research-trace-toggle');
    expect(header).toHaveTextContent('Quick research');
    expect(header).toHaveTextContent('2 sources');
    expect(header).toHaveTextContent('google/gemini-3.8-flash');
    expect(header).toHaveTextContent('4.2s');
  });

  it('expands to every stage, provider outcome and source link', () => {
    render(<ResearchTracePanel trace={quick} />);
    fireEvent.click(screen.getByTestId('research-trace-toggle'));
    expect(screen.getByTestId('research-stage-retrieval')).toHaveTextContent('openalex');
    expect(screen.getByTestId('research-stage-retrieval')).toHaveTextContent('timeout');
    expect(screen.getByTestId('research-source-1')).toHaveAttribute('href', 'https://openrouter.ai/google/gemini-3.8-flash');
  });

  it('marks failed stages visibly', () => {
    render(<ResearchTracePanel trace={quick} />);
    fireEvent.click(screen.getByTestId('research-trace-toggle'));
    expect(screen.getByTestId('research-stage-citation_check')).toHaveAttribute('data-status', 'failed');
  });

  it('renders a compact "no research" line for mode none', () => {
    render(<ResearchTracePanel trace={{ kind: 'research_trace', mode: 'none', status: 'skipped', reasons: ['skip: greeting / small talk'], stages: [], sources: [], source_count: 0 }} />);
    expect(screen.getByTestId('research-trace-toggle')).toHaveTextContent('No research');
    expect(screen.getByTestId('research-trace-toggle')).toHaveTextContent('greeting');
  });
});
```

`ChatTurnEventRow.test.tsx` — add:

```tsx
  it('renders the research trace panel for research_trace events', () => {
    render(<ChatTurnEventRow event={{ kind: 'research_trace', mode: 'none', status: 'skipped', reasons: ['no research cue'], stages: [], sources: [], source_count: 0 }} />);
    expect(screen.getByTestId('research-trace')).toBeInTheDocument();
  });
```

- [ ] **Step 2: Run, verify failure**

Run: `pnpm --dir crates/vox-gui/ui exec vitest run src/lib/buildChatTurn.test.ts src/components/surfaces/Chat/`
Expected: FAIL (execution is `background`; `ResearchTracePanel` missing).

- [ ] **Step 3: Implement**

`buildChatTurn.ts` — keep the slash text and the sync path (the daemon classifier owns research routing now):

```ts
  const researchSlash = parseResearchSlashCommand(payload.description);
  const content = payload.description;
  ...
    execution:
      payload.execution_mode === 'plan'
        ? 'plan'
        : payload.execution_mode === 'task' ? 'background' : 'sync',
  ...
    force_research: payload.force_research ?? null,
    research_scope: payload.research_scope ?? null,
    domain_mode: payload.domain_mode ?? researchSlash?.domainMode ?? null,
    site_scope: payload.site_scope ?? researchSlash?.siteScope ?? null,
```
(Remove the now-unused `isResearchSlash`.)

`ResearchTracePanel.tsx`:

```tsx
import React, { useState } from 'react';

type Stage = { stage: string; status: string; elapsed_ms?: number | null; summary: string; detail?: unknown };
type Src = { n: number; url: string; title: string; engine: string };
type Provider = { provider: string; status: { state: string; hits?: number; message?: string }; elapsed_ms: number };
export type ResearchTrace = {
  kind: 'research_trace'; mode: 'none' | 'quick' | 'deep' | string; status: string;
  reasons?: string[]; stages: Stage[]; sources: Src[]; source_count: number;
  model?: string | null; total_ms?: number;
};

const STATUS_CLASS: Record<string, string> = {
  ok: 'text-text-secondary', empty: 'text-text-muted', skipped: 'text-text-muted',
  degraded: 'text-amber-500', failed: 'text-red-500',
};
const MODE_LABEL: Record<string, string> = { quick: 'Quick research', deep: 'Deep research', none: 'No research' };

function providersOf(detail: unknown): Provider[] {
  const p = (detail as { providers?: unknown } | undefined)?.providers;
  return Array.isArray(p) ? (p as Provider[]) : [];
}

export function ResearchTracePanel({ trace }: { trace: ResearchTrace }) {
  const [open, setOpen] = useState(false);
  const seconds = trace.total_ms != null ? `${(trace.total_ms / 1000).toFixed(1)}s` : '';
  const header = trace.mode === 'none'
    ? `No research · ${trace.reasons?.[0] ?? ''}`
    : [MODE_LABEL[trace.mode] ?? trace.mode, `${trace.source_count} sources`, trace.model ?? 'no model', seconds, trace.status]
        .filter(Boolean).join(' · ');
  return (
    <div data-testid="research-trace" data-status={trace.status}
         className="self-start w-full max-w-[720px] rounded-md border border-border-subtle bg-overlay-subtle font-mono text-[11px]">
      <button type="button" data-testid="research-trace-toggle" aria-expanded={open}
              onClick={() => setOpen(o => !o)}
              className={`w-full px-2 py-1 text-left ${STATUS_CLASS[trace.status] ?? ''}`}>
        {open ? '▾' : '▸'} {header}
      </button>
      {open && (
        <div className="border-t border-border-subtle px-2 py-1 space-y-1">
          {trace.stages.map((s, i) => (
            <div key={`${s.stage}-${i}`} data-testid={`research-stage-${s.stage}`} data-status={s.status}
                 className={STATUS_CLASS[s.status] ?? ''}>
              <span className="font-semibold">{s.stage}</span> · {s.status}
              {s.elapsed_ms != null && ` · ${s.elapsed_ms}ms`} — {s.summary}
              {providersOf(s.detail).length > 0 && (
                <ul className="ml-4">
                  {providersOf(s.detail).map(p => (
                    <li key={p.provider}>
                      {p.provider}: {p.status.state}
                      {p.status.hits != null && ` (${p.status.hits})`}
                      {p.status.message && ` — ${p.status.message}`} · {p.elapsed_ms}ms
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ))}
          {trace.sources.length > 0 && (
            <ol className="ml-4 list-none">
              {trace.sources.map(s => (
                <li key={s.n}>
                  [{s.n}] <a data-testid={`research-source-${s.n}`} href={s.url} target="_blank" rel="noreferrer"
                             className="underline">{s.title || s.url}</a> <span className="text-text-muted">({s.engine})</span>
                </li>
              ))}
            </ol>
          )}
        </div>
      )}
    </div>
  );
}
```

`ChatTurnEventRow.tsx` — before `return null;`:

```tsx
  if (event.kind === 'research_trace') {
    return <ResearchTracePanel trace={event as unknown as ResearchTrace} />;
  }
```
with `import { ResearchTracePanel, type ResearchTrace } from './ResearchTracePanel';`.

`search_probe.rs`: make the DuckDuckGo probe return status `not_implemented` with detail `"DuckDuckGo client is a stub (vox-search/src/duckduckgo.rs); not used by research"` instead of calling the stub. Update its unit test to expect that.

- [ ] **Step 4: Run tests + typecheck**

Run: `pnpm --dir crates/vox-gui/ui exec vitest run && pnpm --dir crates/vox-gui/ui typecheck` and `cargo test -p vox-gui search_probe`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/lib/buildChatTurn.ts crates/vox-gui/ui/src/lib/buildChatTurn.test.ts \
  crates/vox-gui/ui/src/components/surfaces/Chat/ResearchTracePanel.tsx \
  crates/vox-gui/ui/src/components/surfaces/Chat/ResearchTracePanel.test.tsx \
  crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx \
  crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.test.tsx \
  crates/vox-gui/src/commands/search_probe.rs
# plus `git rm` of the dead ChatMessage/ResearchSummaryCard files if Step "Delete" applied
git commit -m "feat(gui): research trace panel under chat replies; research slash stays sync

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 10: Live end-to-end through the real Axis app (spec §8)

**Files:**
- Create: `scripts/axis-drive-research-e2e.vox` (model on `scripts/axis-drive-openrouter-e2e.vox`)
- Create (recorded output): `crates/vox-gui/ui/e2e/fixtures/live-research/{hi,quick,deep}.json`

**Interfaces:**
- Consumes: everything above; `vox gui drive start|set|send|wait|state` (see `crates/vox-cli/src/commands/gui/drive.rs`).
- Produces: three recorded `state` JSON files, each with a `_recorded` header object `{date, model, commit, prompt}`; native screenshots in `crates/vox-gui/ui/review-bundle/latest/live-research-*.png`.

- [ ] **Step 1: Build the real app from this worktree**

```bash
vox run scripts/gui-build.vox          # sidecar vox binary for Tauri
pnpm --dir crates/vox-gui/ui install && pnpm --dir crates/vox-gui/ui build
cargo build -p vox-gui -p vox-cli -p vox-orchestrator-d
```
Confirm SearXNG is up (`curl -s -o /dev/null -w '%{http_code}' 'http://localhost:8080/search?q=x&format=json'` → 200) and `~/.vox/config.toml` has `VOX_MODEL_FORCE` and `VOX_SEARCH_SEARXNG_URL`. Kill any stale `vox-orchestrator-d` (`pkill -x vox-orchestrator-d`) so Drive spawns this worktree's daemon; verify with `lsof -p <pid> | grep cwd` / binary path that the running daemon is `./target/debug/vox-orchestrator-d`.

- [ ] **Step 2: Write the script** — `scripts/axis-drive-research-e2e.vox`: reuse `vox_binary_path`, `fail`, `run_json`, `state_view`, `string_field` from the OpenRouter script verbatim; then for each case send, wait, read state, assert, and write the fixture:

```vox
fn run_case(bin: str, name: str, prompt: str, wait_secs: str) to Json {
    let _ = run_json(bin, ["gui", "drive", "send", prompt])
    let _ = run_json(bin, ["gui", "drive", "wait", "--until", "reply_ok", "--timeout", wait_secs])
    let st = state_view(run_json(bin, ["gui", "drive", "state"]))
    fs.write("crates/vox-gui/ui/e2e/fixtures/live-research/" + name + ".json", json.stringify_pretty(st))
    return st
}
```
Assertions per spec §8 (read the last assistant message and its `events[0]` from `st` — inspect one real `state` payload first and adapt the field paths; do not guess):
- `hi`: trace `mode == "none"`; no stage named `retrieval`.
- quick: `mode == "quick"`; `retrieval.detail.providers` has `searxng` with `state == "ok"` and `hits > 0`; `source_count >= 3`; every source url starts with `http`; reply contains `3.8`; `citation_check.status == "ok"`; `model == "google/gemini-3.8-flash"`.
- deep: `mode == "deep"`; `planning.detail.subqueries` length ≥ 2; `claims` stage present; `judge.status == "ok"`; `synthesis.detail.model == "google/gemini-3.8-flash"`; reply mentions both `SearXNG` and `Tavily`.
- Anti-template on quick + deep replies: must not contain `Key architectural considerations based on gathered evidence`, `No contested findings identified`, or `[autonomous_research:`; the two replies share no identical paragraph ≥ 80 chars.

Use `vox check scripts/axis-drive-research-e2e.vox` until it type-checks.

- [ ] **Step 3: Run it, capture evidence**

```bash
vox run scripts/axis-drive-research-e2e.vox 2>&1 | tee "$SCRATCH/live-e2e.log"
```
While each reply is on screen with the trace expanded (use `vox gui drive show` or click the toggle), capture the real window:

```bash
screencapture -l "$(osascript -e 'tell app "System Events" to id of first window of (first process whose name contains "Axis")' 2>/dev/null)" crates/vox-gui/ui/review-bundle/latest/live-research-quick.png || screencapture -x crates/vox-gui/ui/review-bundle/latest/live-research-quick.png
```
Expected: script exits 0. **If any assertion fails: report the failing assertion with the raw state JSON, debug with superpowers:systematic-debugging, fix at the root, re-run. Do not relax an assertion to get green; do not declare done.**

- [ ] **Step 4: Commit the script and recorded fixtures**

```bash
git add scripts/axis-drive-research-e2e.vox crates/vox-gui/ui/e2e/fixtures/live-research/
git commit -m "test(e2e): live Drive run of chat research against Gemini 3.8 Flash + SearXNG

Fixtures are recorded from the real run (see _recorded headers), not hand-written.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 11: Playwright replay screenshots of every stage

**Files:**
- Create: `crates/vox-gui/ui/e2e/chat-research-trace.spec.ts`
- Modify (only if needed to return a canned `chat_turn`): `crates/vox-gui/ui/e2e/lib/tauriMockShared.ts`

**Interfaces:**
- Consumes: Task 10 fixtures; `installTauriMock` / `seedMockEnvironment` from `e2e/lib`.
- Produces: `crates/vox-gui/ui/review-bundle/latest/research-trace-{hi,quick,deep}-{collapsed,expanded}.png`.

- [ ] **Step 1: Write the spec** — for each fixture: install the mock so `chat_turn` resolves to the recorded assistant turn (`content`, `model_id`, `events`) from the fixture file (read with `fs.readFileSync`, never inline literals); type the fixture's prompt into the composer and press Enter; assert `research-trace` is visible and its toggle text contains the recorded mode label; screenshot collapsed; click `research-trace-toggle`; assert one `research-stage-*` row per recorded stage and one `research-source-*` link per recorded source; screenshot expanded (full page, viewport 1440×900).

```ts
import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { installTauriMock } from './lib/tauriMock';

const FIX = path.join(__dirname, 'fixtures', 'live-research');
const OUT = path.join(__dirname, '..', 'review-bundle', 'latest');

for (const name of ['hi', 'quick', 'deep'] as const) {
  test(`research trace replay (${name}) — recorded live payload`, async ({ page }) => {
    const rec = JSON.parse(fs.readFileSync(path.join(FIX, `${name}.json`), 'utf8'));
    const turn = rec.last_turn ?? rec; // adapt to the recorded shape from Task 10
    await installTauriMock(page, { overrides: { chat_turn: turn } });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.getByRole('textbox').first().fill(rec._recorded.prompt);
    await page.keyboard.press('Enter');
    const panel = page.getByTestId('research-trace').last();
    await expect(panel).toBeVisible();
    await page.screenshot({ path: path.join(OUT, `research-trace-${name}-collapsed.png`), fullPage: true });
    await panel.getByTestId('research-trace-toggle').click();
    for (const s of turn.events[0].stages) {
      await expect(panel.getByTestId(`research-stage-${s.stage}`).first()).toBeVisible();
    }
    for (const src of turn.events[0].sources) {
      await expect(panel.getByTestId(`research-source-${src.n}`)).toHaveAttribute('href', src.url);
    }
    await page.screenshot({ path: path.join(OUT, `research-trace-${name}-expanded.png`), fullPage: true });
  });
}
```
(If `installTauriMock` has no `overrides` option, add a minimal one in `tauriMockShared.ts`: a map consulted first in the invoke handler. Keep it generic.)

- [ ] **Step 2: Run**

Run: `pnpm --dir crates/vox-gui/ui test:e2e e2e/chat-research-trace.spec.ts --project=chromium`
Expected: 3 PASS, 6 PNGs written. Open each PNG and look at it; report anything visually wrong (overlap, clipping, unreadable status colors) and fix before committing.

- [ ] **Step 3: Commit**

```bash
git add crates/vox-gui/ui/e2e/chat-research-trace.spec.ts crates/vox-gui/ui/e2e/lib/tauriMockShared.ts
git commit -m "test(e2e): Playwright replay of recorded live research traces with stage screenshots

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 12: Close-out — review, docs, report

**Files:**
- Modify: `docs/superpowers/specs/2026-09-21-chat-research-real-e2e-design.md` (`status: "implemented"`, note any deviations)
- Modify: `docs/src/architecture/where-things-live.md` (row: "Chat research intent / trace" → `vox-orchestrator-mcp/src/chat_tools/chat/research_{intent,turn}.rs`)

- [ ] **Step 1:** `cargo clippy` on every touched crate with `-D warnings`; `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/superpowers/specs/2026-09-21-chat-research-real-e2e-design.md docs/src/architecture/where-things-live.md`; `vox ci pre-push --complete` (report output verbatim, including failures unrelated to this work).
- [ ] **Step 2:** `/code-review high` over `main..HEAD`; fix confirmed findings.
- [ ] **Step 3:** Update the spec status and where-things-live row; commit.
- [ ] **Step 4:** Report to the user: the three live outputs (trace summaries + reply excerpts), the native screenshots and Playwright PNGs, test results, anything that failed or was skipped — stated plainly.

---

### Task 13: Per-role model config + honest alias resolution

**Why:** `VOX_MODEL_FORCE` is a single global pin, so every stage uses one model. Live pricing (probed 2026-09-22) makes role-splitting worth real money: `~deepseek/deepseek-flash-latest` is $0.10/$0.50 per 1M with a 943k output ceiling and no separate reasoning charge, vs `google/gemini-3.8-flash` at $0.75/$3.75 with reasoning billed at $3.75/M and a 65k output ceiling — about 8× cheaper for a ~5-call deep run at comparable agentic-coding benchmark scores. Gemini stays the chat model because it is the only candidate accepting audio/video/file input. Second problem: a `~vendor/model-latest` alias makes the trace record "latest" instead of the version that actually answered, which violates spec §5's honesty rule.

**Files:**
- Modify: `crates/vox-config/src/inference.rs` (add role-scoped pin readers next to `forced_model`)
- Modify: `crates/vox-research-shim/src/research/orchestrator/model_dispatch.rs`, `crates/vox-actor-runtime/src/llm/cascade.rs` (consume the research-role pin)
- Modify: `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs` (chat-role pin)
- Modify: `crates/vox-actor-runtime/src/llm/types.rs` or `chat.rs` (surface the concrete model id already present on `LlmResponse`)
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs` (trace records resolved id)
- Test: the same crates' test modules

**Interfaces:**
- Produces: `vox_config::inference::forced_model_for(role: ModelRole) -> Option<String>` where `pub enum ModelRole { Chat, Research, Judge }`; resolution order per role: `VOX_MODEL_FORCE_<ROLE>` → `VOX_MODEL_FORCE` → `None`, all read through `resolve_config_str` (env, then `~/.vox/config.toml`).
- Keeps: `forced_model()` as `forced_model_for(ModelRole::Chat)`'s unscoped fallback, so existing call sites behave identically when only `VOX_MODEL_FORCE` is set.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn role_pin_falls_back_to_the_global_pin() {
        assert_eq!(
            super::resolve_role_pin(Some("deepseek/x"), Some("google/y")),
            Some("deepseek/x".to_string()),
            "role-specific pin wins"
        );
        assert_eq!(
            super::resolve_role_pin(None, Some("google/y")),
            Some("google/y".to_string()),
            "falls back to the global pin"
        );
        assert_eq!(super::resolve_role_pin(None, None), None);
        assert_eq!(super::resolve_role_pin(Some("  "), Some("google/y")), Some("google/y".to_string()));
    }
```

And in `research_turn.rs`'s test module:

```rust
    #[test]
    fn trace_records_the_resolved_model_not_the_alias_when_they_differ() {
        let mut t = ResearchTrace::new(classify_research_intent("/research x y z", None, None));
        t.set_model("~deepseek/deepseek-flash-latest", Some("deepseek/deepseek-v4.1-flash"));
        let e = t.to_event();
        assert_eq!(e["model"], "deepseek/deepseek-v4.1-flash");
        assert_eq!(e["model_alias"], "~deepseek/deepseek-flash-latest");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p vox-config role_pin; cargo test -p vox-orchestrator-mcp trace_records_the_resolved_model`
Expected: compile errors — `resolve_role_pin` / `set_model` not defined.

- [ ] **Step 3: Implement**

```rust
/// Pure half of the role-scoped pin: role value wins, else the global pin; blanks are unset.
#[must_use]
pub fn resolve_role_pin(role_value: Option<&str>, global: Option<&str>) -> Option<String> {
    forced_model_from(role_value.unwrap_or("")).or_else(|| forced_model_from(global.unwrap_or("")))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRole { Chat, Research, Judge }

impl ModelRole {
    const fn env_key(self) -> &'static str {
        match self {
            Self::Chat => "VOX_MODEL_FORCE_CHAT",
            Self::Research => "VOX_MODEL_FORCE_RESEARCH",
            Self::Judge => "VOX_MODEL_FORCE_JUDGE",
        }
    }
}

#[must_use]
pub fn forced_model_for(role: ModelRole) -> Option<String> {
    resolve_role_pin(
        Some(&crate::env_parse::resolve_config_str(role.env_key(), "")),
        Some(&crate::env_parse::resolve_config_str("VOX_MODEL_FORCE", "")),
    )
}
```
Then: `cascade_for_research_stage` and `primary_candidate_for_intent` call `forced_model_for(ModelRole::Research)` (Judge stage uses `ModelRole::Judge`, falling back through Research to global); `resolve.rs`'s `strict_pin` uses `forced_model_for(ModelRole::Chat)`. Strictness is unchanged: a pinned model that is missing or gated still errors.

For the trace: `LlmResponse.model` already carries the concrete id the provider returned (Task 7 fix round 1 threaded it for synthesis). Add `ResearchTrace::set_model(alias, resolved)` storing both, and emit `model` = resolved (falling back to the alias when the provider echoes the alias) plus `model_alias`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p vox-config -p vox-actor-runtime -p vox-research-shim -p vox-orchestrator-mcp 2>&1 | tail -20`, then clippy `--no-deps` on each.
Expected: PASS.

- [ ] **Step 5: Live check** — with `~/.vox/config.toml` holding `VOX_MODEL_FORCE = "google/gemini-3.8-flash"` and `VOX_MODEL_FORCE_RESEARCH = "~deepseek/deepseek-flash-latest"`, run the three live prompts from Task 8 Step 6. Assert: the chat reply's `model_used` is the Gemini pin, the trace's research stages report the DeepSeek model, and the deep answer is still complete with a real judge score. Record the outputs.

- [ ] **Step 6: Commit**

```bash
git add crates/vox-config/src/inference.rs crates/vox-actor-runtime/src/llm/cascade.rs crates/vox-research-shim/src/research/orchestrator/model_dispatch.rs crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
git commit -m "feat(routing): per-role model pins and alias-resolved model in the research trace

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 14: Retire hardcoded model ids repo-wide

**Why:** hardcoded vendor model ids are scattered across code and contracts, many of them stale (`gemini-1.5-pro`, `gemini-2.5-flash`, `gemini-2.0-flash-lite`, `gemini-3.1-pro`). Each is a silent default that overrides intent when selection fails, exactly the failure class this plan exists to remove. The 2026-09-20 memory note (`openrouter-free-slugs-churn`) records a case where every entry in such a list was dead, and research silently fell back to raw snippets.

**Files:**
- Create: `contracts/orchestration/model-defaults.v1.yaml` (single source for every default/fallback model id, with a `role` per entry)
- Modify: `crates/vox-config/src/bootstrap_inference.rs` (`RESEARCH_FLASH_FALLBACK`, `OPENROUTER_FREE_FALLBACK_MODELS`), `crates/vox-config/src/routing_policy.rs` (`gemini_route_targets_from_env` defaults), `crates/vox-config/src/config_registry.rs` + `operator_registry.rs` (`gemini-1.5-pro`), `crates/vox-research-shim/src/research/model_select.rs` (planner/claim/judge fallbacks), `crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs` (Lane G `google/gemini-3.1-pro`), `crates/vox-code-audit/src/ai_analyze.rs` (`default_gemini_model`), `contracts/orchestration/model-pins.v1.yaml` (`premium_alias`, `fallback`), `contracts/orchestration/model-routing.v1.yaml` (`premium_alias`), `contracts/orchestration/model-catalog.bootstrap.v1.json`, `apps/editor/vox-vscode/package.json` + `src/core/ConfigManager.ts`
- Test: `crates/vox-config/tests/` (new drift test)

**Interfaces:**
- Produces: `vox_config::model_defaults::{default_for(role) -> &'static str, all() -> &'static [ModelDefault]}`, generated from or validated against `contracts/orchestration/model-defaults.v1.yaml`.

**Do NOT touch:** `assets/skills/claude-api/**` (vendored Anthropic docs), `contracts/reports/**` and `contracts/**/_snapshot/**` (historical run records), `docs/src/archive/**`, or any test fixture whose literal id is the thing under test. List anything you deliberately skip in the report.

- [ ] **Step 1: Inventory** — produce the checklist first, so the sweep is auditable:

```bash
rg -n --glob '!docs/src/archive/**' --glob '!assets/skills/**' --glob '!contracts/reports/**' \
  -e 'google/gemini-[0-9]' -e '"gemini-[0-9]' -e 'anthropic/claude-[a-z]+-[0-9]' -e 'openai/gpt-[0-9]' \
  crates contracts apps scripts > /tmp/model-id-inventory.txt
wc -l /tmp/model-id-inventory.txt
```
Classify every line as: (a) a default/fallback to migrate, (b) a catalog/bootstrap entry (keep, but refresh stale ids), (c) a test fixture (keep), (d) documentation (update only if it states current policy). Put the classified table in the report.

- [ ] **Step 2: Write the failing drift test**

```rust
#[test]
fn no_hardcoded_vendor_model_ids_outside_the_defaults_contract() {
    // Guards the class of bug in the 2026-09-20 `openrouter-free-slugs-churn` note:
    // a stale hardcoded id silently replaces the operator's intent.
    let offenders = vox_config::model_defaults::scan_workspace_for_hardcoded_ids();
    assert!(
        offenders.is_empty(),
        "hardcoded vendor model ids must live in contracts/orchestration/model-defaults.v1.yaml: {offenders:#?}"
    );
}
```
If a workspace-walking test is too slow or fragile for CI, implement the same check as a `vox ci` guard instead and have this test assert the guard's allowlist is empty; say which you chose and why.

- [ ] **Step 3: Run to verify failure**

Run: `cargo test -p vox-config no_hardcoded_vendor_model_ids`
Expected: FAIL, listing the (a)-class sites from Step 1.

- [ ] **Step 4: Implement** — write `model-defaults.v1.yaml` with one entry per role (`chat`, `research`, `judge`, `claim_extraction`, `free_floor`), each `{role, model, rationale, verified_on}`. Seed it from the live catalog values verified 2026-09-22: chat `google/gemini-3.8-flash`, research `~deepseek/deepseek-flash-latest`, judge `~deepseek/deepseek-pro-latest`. Replace every (a)-class hardcoded literal with a `model_defaults::default_for(role)` call. Refresh (b)-class catalog entries that name dead ids. Leave (c) and (d) alone unless the id is factually wrong.

- [ ] **Step 5: Verify the ids are real** — every id in the new contract must exist in the live catalog:

```bash
curl -s https://openrouter.ai/api/v1/models | jq -r '.data[].id' > /tmp/live-ids.txt
for m in $(rg -o '^\s+model: "([^"]+)"' -r '$1' contracts/orchestration/model-defaults.v1.yaml); do
  grep -qx "$m" /tmp/live-ids.txt && echo "OK   $m" || echo "DEAD $m"
done
```
Every line must read OK. Paste the output into the report.

- [ ] **Step 6: Run the suites + commit**

Run: `cargo test -p vox-config -p vox-orchestrator -p vox-research-shim -p vox-code-audit 2>&1 | tail -20`; clippy `--no-deps` per crate; `pnpm --dir apps/editor/vox-vscode test` if that package has tests.

```bash
git add contracts/orchestration/model-defaults.v1.yaml crates/vox-config crates/vox-research-shim crates/vox-orchestrator crates/vox-code-audit contracts/orchestration/model-pins.v1.yaml contracts/orchestration/model-routing.v1.yaml contracts/orchestration/model-catalog.bootstrap.v1.json apps/editor/vox-vscode
git commit -m "refactor(models): single defaults contract; retire hardcoded vendor model ids

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```
