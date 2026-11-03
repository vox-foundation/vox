# Chat Turn Trace: One Collapsed, Honest Account per Assistant Turn — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task docs/superpowers/plans/2026-09-28-chat-turn-trace.md <N>`, where `<N>` is `1`, `2a`, `2b`, `3` … `9`),
> per [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, reviews the diff and commits. Steps use checkbox
> (`- [ ]`) syntax. Code blocks are transcribed exactly.
>
> **Revision 2 (2026-09-28)** after review (no Critical defects; T2/T3/T6/T7 WARN). Changes are marked
> `<!-- AMENDED: T<n> — reason -->`. Summary: the engine-events half is cut (no emitter or no route to a chat turn today;
> see Deferred); Task 2 is split into 2a/2b; `resolved_from` derives from `PricingSource`; `reason` never echoes a
> requested model id; `mode`/`objective` are omitted when the composer sent no clutch; replies without a routing event keep
> their model attribution; the research row is proven not to render `query`; `first_diff` is replaced by `assert_eq!`;
> this plan owns the shared model/mode labels. 10 driven tasks (1, 2a, 2b, 3–9).

**Goal:** Every assistant turn in the chat shows one quiet summary row (resolved model, tools, receipts, delegations,
research, duration) that expands into ordered steps, with an interrupt chip inline only for something a human must act
on. Rust and the GUI agree on every turn event through one contract file checked from both sides, and one golden turn
proves that what the engine emits is exactly what the chat renders.

**Architecture:** A new contract `contracts/gui/turn-event-kinds.v1.json` lists every turn-event kind with its required
fields and one example. Rust checks it in `chat_tools/chat/turn_events.rs` against the real producers
(`turn_event_for_result`, `receipt_turn_event`, and the new `routing_decision_event`); the GUI checks it in
`turnEventContract.test.tsx`. `try_run_agent_turn` (`message.rs`, where the chat model is resolved) prepends one
`routing_decision` to each turn's events, using `models::family::family_key` and a new `models::provenance::resolved_from`
(derived from `PricingSource`). On the GUI side a shared validator and label owner (`lib/turnEvents.ts`) gates
`ChatTurnEventRow`; a pure builder (`lib/turnTrace.ts`) turns a turn's events into summary, steps, interrupts and default
expansion; `TurnTrace.tsx` renders it under the message (replacing `ModelBadge`, falling back to the message's `modelId`
when a reply carries no routing event), and a Quiet / Normal / Verbose control sets the default expansion.

**Tech Stack:** Rust (`vox-orchestrator`, `vox-orchestrator-mcp`, `vox-gui`), TypeScript/React 19 + Tailwind 4
(`crates/vox-gui/ui`), vitest 3, Playwright 1.62, serde_json, wiremock 0.6.

**Spec:** [`docs/src/architecture/chat-surface-design-critique-2026-09-28.md`](../../src/architecture/chat-surface-design-critique-2026-09-28.md)
(matrix rows 17–22 and 39–40, the anti-spam policy, the canonical vocabulary) and tasks 13–18 and 28 of the index plan
[`2026-09-28-chat-surface-trace-and-latest-models.md`](2026-09-28-chat-surface-trace-and-latest-models.md).

**Prerequisites (all committed before Task 1 is driven):** Phase 5 (`.planning/phases/05-*`, through 05-07);
[`2026-09-28-model-routing-latest-and-honest.md`](2026-09-28-model-routing-latest-and-honest.md) Tasks 1–6 (for
`pub mod family` with `pub fn family_key`); and [`2026-09-28-model-routing-chat-lane.md`](2026-09-28-model-routing-chat-lane.md)
Task 1 (the Efficient and Balanced objective text below promises "no flagship while another model fits", which that task
makes true for chat). Pre-flight for Claude: `rg -n "pub mod family" crates/vox-orchestrator/src/models/mod.rs` and
`rg -n "pub fn family_key" crates/vox-orchestrator/src/models/family.rs` each print one line.
<!-- AMENDED: T2 — released_at is no longer consumed (resolved_from uses PricingSource), so only family_key is a prerequisite. -->

## Global Constraints

- No new crate, crate edge or dependency. `vox-orchestrator-mcp` and `vox-gui` already depend on `vox-orchestrator`;
  `wiremock` and `serial_test` are already dev-dependencies of `vox-orchestrator-mcp`.
- No versioned cloud model id literals anywhere (code, tests, mocks, contract). Fixtures use the fictional family
  `acme/widget` (resolved id `acme/widget-5.5`) or local ids like `mens/run-7`.
- Every chat-visible field is a server-derived constant or registry value. Input a model can influence is never
  rendered: `research_milestone.query`, and the requested/pinned model id that `resolve.rs` echoes into two rationales
  (`Fallback: requested \`{id}\`…`, `Sticky VoxLocal: \`{id}\`…`).
- Test-first for every task; record the RED output to `target/turn-trace-t<N>-red.txt` **before** touching
  implementation code. Every guard or filter gets a mutation proof (break it, see the named test fail, restore, check
  the diff).
- Commands are foreground and `timeout`-prefixed: cargo `timeout 1500s`, pnpm/vitest/playwright `timeout 300s`. Cargo
  takes one test filter before `--`, several after it, and is always scoped with `-p`. Format each changed `.rs` file
  with `rustfmt --edition 2024 <file>`; never `cargo fmt`.
- UI commands: `pnpm --dir crates/vox-gui/ui exec vitest run <files>`, `pnpm --dir crates/vox-gui/ui typecheck`,
  `pnpm --dir crates/vox-gui/ui exec playwright test <spec> --project=chromium --reporter=line` (the Vite dev server on
  :1420 is normally already up and is reused).
- Existing tests are not edited except the ones a task names ("sanctioned edit"). Any other existing test that breaks
  is a STOP: report its name and failing assertion.
- The contract file is edited with targeted edits only (never re-serialized). Check `git diff --numstat` against the
  size each task states.
- Files over 500 non-blank lines (`agent_loop.rs` 2401, `message.rs` 2146, `vox-gui/src/commands/models.rs` ~800) get
  only the edits shown; new code goes in new files.
- The agent never runs `git add` / `git commit`; each task's commit block is run by Claude Code after review.

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `contracts/gui/turn-event-kinds.v1.json` | create (T1), edit (T2b, T9) | kinds + required fields + examples; `golden_turn` |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs` | create (T1), modify (T2a, T2b) | contract tests (T1); `routing_decision_event`, `routing_reason`, `test_spec` (T2a); contract arm (T2b) |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs` | modify (T1) | `mod turn_events;` |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` | modify (T1) | `receipt_turn_event` becomes `pub(crate)` |
| `crates/vox-orchestrator/src/models/provenance.rs` | create (T2a) | `ResolvedFrom`, `resolved_from(spec)` from provider + `PricingSource` |
| `crates/vox-orchestrator/src/models/mod.rs` | modify (T2a) | `pub mod provenance;` |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` | modify (T2b) | `try_run_agent_turn` prepends `routing_decision`; declares the test module |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs` | create (T2b), modify (T9) | wiremock tests through `try_run_agent_turn`; golden seam (T9) |
| `crates/vox-gui/src/commands/models.rs` | modify (T3) | `RoutingSummaryDto.{family, resolved_from, reason}` |
| `crates/vox-gui/ui/src/types/tauri.ts` | modify (T3) | TS mirror of the three DTO fields |
| `crates/vox-gui/ui/src/lib/turnEvents.ts` | create (T4) | kind list, `isKnownTurnEvent`, `MODE_NAMES`, `modeLabel`, `routingModelLabel` (the one owner of these labels) |
| `crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx` | create (T4) | TS side of the contract |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx` | modify (T4) | validator gate; delegation, research, routing branches |
| `crates/vox-gui/ui/src/lib/turnTrace.ts` + `.test.ts` | create (T5) | `buildTurnTrace`, `summaryText`, `isInterrupt` |
| `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx` + `.test.tsx` | create (T6), modify test (T9) | the trace component; golden render (T9) |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx` + `.test.tsx` | modify (T6, T7) | trace replaces `ModelBadge` and loose chips; verbosity control |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ModelBadge.tsx` + `.test.tsx` | delete (T6) | superseded by the trace summary |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatVerbosityControl.tsx` + `.test.tsx` | create (T7) | Quiet / Normal / Verbose |
| `crates/vox-gui/ui/src/hooks/useChatVerbosity.ts` | modify (T7) | doc comment only |
| `crates/vox-gui/ui/e2e/chat-turn-trace.spec.ts` | create (T8) | Playwright: collapsed, expanded, interrupt, verbosity |
| `docs/agents/gui-honesty-manifest.json`, `docs/agents/gui-honesty-triage.md` | regenerate / edit (verification sweep, Claude) | drop `ModelBadge.tsx` rows |

<!-- AMENDED: T6(old) — chatCorrelation.ts / sessionChatStore.ts / chatCorrelation.test.ts removed from this plan with the engine-events half. -->

---

### Task 1: The turn-event contract and its Rust-side check

<!-- AMENDED: T1 — engine_events, the AgentEventKind round-trip tests and their mutation proof removed (see Deferred). routing_decision's `mode`/`objective` are optional. -->

**Files:**
- Create: `contracts/gui/turn-event-kinds.v1.json`
- Create: `crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs` (one line)
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` (one word: `receipt_turn_event` visibility)

**Interfaces:**
- Consumes: `agent_loop::turn_event_for_result(tool_name: &str, args: &Value, result_content: &str, success: bool) -> Option<Value>` (existing, `pub(crate)`); `agent_loop::receipt_turn_event(r: &crate::dispatch::ToolReceiptOutcome, verified: bool) -> Value` (existing, made `pub(crate)` here); `crate::dispatch::ToolReceiptOutcome { receipt_id: String, tool_name: String, fulfilled: bool }`; `vox_mcp_registry::TOOL_REGISTRY` (entries with `.name`).
- Produces: the contract file (top-level keys `schema_version`, `description`, `kinds[]` of `{kind, status?, required[], example}`); module `turn_events` (test-only content in this task).

- [ ] **Step 1: Write the failing tests.** Create `crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs` with exactly:

```rust
//! Chat turn events shared with the GUI trace. `contracts/gui/turn-event-kinds.v1.json`
//! is checked from this side (the real producers) and from the GUI side
//! (`crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx`).

#[cfg(test)]
mod contract_tests {
    use serde_json::{Value, json};

    use super::super::agent_loop::{receipt_turn_event, turn_event_for_result};
    use crate::dispatch::ToolReceiptOutcome;

    const CONTRACT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/gui/turn-event-kinds.v1.json"
    ));

    fn contract() -> Value {
        serde_json::from_str(CONTRACT)
            .expect("contracts/gui/turn-event-kinds.v1.json is valid JSON")
    }

    fn contract_kind_names(c: &Value) -> Vec<String> {
        c["kinds"]
            .as_array()
            .expect("`kinds` array")
            .iter()
            .map(|k| k["kind"].as_str().expect("every entry names its kind").to_string())
            .collect()
    }

    fn sample_receipt() -> ToolReceiptOutcome {
        ToolReceiptOutcome {
            receipt_id: "01920000-aaaa-7bbb-8ccc-000000000001".into(),
            tool_name: "vox_git_status".into(),
            fulfilled: true,
        }
    }

    /// Events Rust emits outside `turn_event_for_result`, one per producer.
    fn standalone_events() -> Vec<Value> {
        vec![receipt_turn_event(&sample_receipt(), true)]
    }

    /// What the real producer emits for each contract kind, from fixed inputs.
    fn produced(kind: &str) -> Option<Value> {
        match kind {
            "skill_activated" => {
                turn_event_for_result("vox_skill_use", &json!({"id": "ponytail"}), "", true)
            }
            "delegation_spawned" => turn_event_for_result(
                "vox_submit_task",
                &json!({}),
                r#"{"success":true,"data":{"task_id":812,"agent_id":3}}"#,
                true,
            ),
            "research_milestone" => turn_event_for_result(
                "vox_deep_research",
                &json!({"query": "SQLite JSONB performance"}),
                r#"{"success":true,"data":{"waves_executed":3,"claims_verified":12,"contradictions_resolved":1}}"#,
                true,
            ),
            "tool_receipt" => Some(receipt_turn_event(&sample_receipt(), true)),
            "receipt_claims" => turn_event_for_result(
                "vox_verify_task_claims",
                &json!({}),
                r#"{"success":true,"data":{"valid":["a","b"],"fabricated":["x"],"unverified":[]}}"#,
                true,
            ),
            _ => None,
        }
    }

    /// Kinds in `emitted` that `listed` lacks.
    fn kinds_missing_from_contract(emitted: &[String], listed: &[String]) -> Vec<String> {
        emitted
            .iter()
            .filter(|k| !listed.contains(k))
            .cloned()
            .collect()
    }

    #[test]
    fn every_contract_example_equals_what_rust_emits() {
        let c = contract();
        for entry in c["kinds"].as_array().expect("`kinds` array") {
            let kind = entry["kind"].as_str().expect("kind");
            assert_eq!(
                entry["example"]["kind"], kind,
                "the {kind} example must carry its own kind"
            );
            if entry.get("status").and_then(Value::as_str) == Some("planned") {
                assert!(
                    produced(kind).is_none(),
                    "{kind} has a Rust producer now: delete its \"status\": \"planned\""
                );
                continue;
            }
            let actual =
                produced(kind).unwrap_or_else(|| panic!("contract kind {kind} has no Rust producer"));
            assert_eq!(
                actual, entry["example"],
                "the contract example for {kind} differs from what Rust emits"
            );
            for field in entry["required"].as_array().expect("`required` array") {
                let field = field.as_str().expect("field name");
                assert!(
                    actual.get(field).is_some_and(|v| !v.is_null()),
                    "{kind} must always carry required field {field}"
                );
            }
        }
    }

    #[test]
    fn every_kind_rust_can_emit_is_in_the_contract() {
        // A success body carrying every field any producer arm reads, so each arm fires.
        let body = r#"{"success":true,"data":{"agent_id":1,"task_id":2,"valid":[],"fabricated":[],"unverified":[],"waves_executed":1,"claims_verified":0,"contradictions_resolved":0}}"#;
        let args = json!({"id": "ponytail", "query": "q"});
        let mut emitted: Vec<String> = vox_mcp_registry::TOOL_REGISTRY
            .iter()
            .filter_map(|entry| turn_event_for_result(entry.name, &args, body, true))
            .chain(standalone_events())
            .map(|ev| ev["kind"].as_str().expect("every event names its kind").to_string())
            .collect();
        emitted.sort();
        emitted.dedup();
        assert!(
            emitted.len() >= 5,
            "the sweep must reach every producer arm, got {emitted:?}"
        );
        let missing = kinds_missing_from_contract(&emitted, &contract_kind_names(&contract()));
        assert!(
            missing.is_empty(),
            "Rust emits kinds the contract does not list: {missing:?}"
        );
    }

    #[test]
    fn the_missing_kind_check_flags_an_unlisted_kind() {
        let emitted = vec!["tool_receipt".to_string(), "brand_new_kind".to_string()];
        let listed = vec!["tool_receipt".to_string()];
        assert_eq!(
            kinds_missing_from_contract(&emitted, &listed),
            vec!["brand_new_kind".to_string()]
        );
    }
}
```

In `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs`, add `mod turn_events;` on the line directly after `pub(crate) mod message;`.

In `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`, change `fn receipt_turn_event(` to `pub(crate) fn receipt_turn_event(` (the only change to that file).

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib turn_events > target/turn-trace-t1-red.txt 2>&1; tail -20 target/turn-trace-t1-red.txt`
Expected: FAIL to compile — `couldn't read ... contracts/gui/turn-event-kinds.v1.json`. Any other compile error is a STOP.

- [ ] **Step 3: Create the contract.** Create `contracts/gui/turn-event-kinds.v1.json` with exactly:

```json
{
  "schema_version": 1,
  "description": "Chat turn events: every kind Rust puts in a vox_chat_message reply's data.events, with its required fields and one example. Checked from Rust by crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs and from the GUI by crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx. Examples use a fictional model family (acme/widget), never a real versioned model id. routing_decision carries mode and objective only when the composer sent a mode.",
  "kinds": [
    {
      "kind": "skill_activated",
      "required": ["skill_id"],
      "example": { "kind": "skill_activated", "skill_id": "ponytail" }
    },
    {
      "kind": "delegation_spawned",
      "required": ["tool", "agent_id"],
      "example": { "kind": "delegation_spawned", "tool": "vox_submit_task", "agent_id": 3, "task_id": 812 }
    },
    {
      "kind": "research_milestone",
      "required": ["tool", "waves_executed", "claims_verified", "contradictions_resolved"],
      "example": {
        "kind": "research_milestone",
        "tool": "vox_deep_research",
        "query": "SQLite JSONB performance",
        "waves_executed": 3,
        "claims_verified": 12,
        "contradictions_resolved": 1
      }
    },
    {
      "kind": "tool_receipt",
      "required": ["tool", "receipt_id", "fulfilled", "verified"],
      "example": {
        "kind": "tool_receipt",
        "tool": "vox_git_status",
        "receipt_id": "01920000-aaaa-7bbb-8ccc-000000000001",
        "fulfilled": true,
        "verified": true
      }
    },
    {
      "kind": "receipt_claims",
      "required": ["valid", "fabricated", "unverified"],
      "example": { "kind": "receipt_claims", "valid": 2, "fabricated": 1, "unverified": 0 }
    },
    {
      "kind": "routing_decision",
      "status": "planned",
      "required": ["family", "resolved_id", "resolved_from", "reason"],
      "example": {
        "kind": "routing_decision",
        "family": "acme/widget",
        "resolved_id": "acme/widget-5.5",
        "resolved_from": "catalog",
        "mode": "efficiency",
        "objective": "Most quality per dollar; no flagship while another model fits",
        "reason": "Chosen by the model scorer as the best match for your request"
      }
    }
  ]
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib turn_events 2>&1 | tail -15`
Expected: `3 passed`.

- [ ] **Step 5: Mutation proofs.** Save a copy: `cp contracts/gui/turn-event-kinds.v1.json target/turn-trace-contract.bak`.
  (a) In the `delegation_spawned` example change `"agent_id": 3` to `"agent_id": 4`; run the Step 4 command into `target/turn-trace-t1-mutant-a.txt`; confirm `every_contract_example_equals_what_rust_emits ... FAILED`.
  (b) Restore (`cp target/turn-trace-contract.bak contracts/gui/turn-event-kinds.v1.json`), then change the line that is exactly `      "kind": "receipt_claims",` (six spaces; the entry key on its own line, not the one-line `"example": { "kind": "receipt_claims", … }`) to `      "kind": "receipt_claims_x",`; run into `target/turn-trace-t1-mutant-b.txt`; confirm `every_kind_rust_can_emit_is_in_the_contract ... FAILED`.
  Restore, and confirm `diff -q target/turn-trace-contract.bak contracts/gui/turn-event-kinds.v1.json` prints nothing and `git diff --stat` shows only `agent_loop.rs` (1 line) and `chat/mod.rs` (1 line) among tracked files.

- [ ] **Step 6:** `rustfmt --edition 2024 crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`, then re-run Step 4.

- [ ] **Step 7: Commit (Claude Code)**

```bash
git add -- contracts/gui/turn-event-kinds.v1.json crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs
git commit -m "feat(chat-trace): turn-event contract checked against the Rust producers"
```

---

### Task 2a: Provenance and the `routing_decision` producer (pure functions)

<!-- AMENDED: T2 — split into 2a (pure, ≤4 files) and 2b (wiring). resolved_from derives from PricingSource, not released_at. reason never echoes a requested id. mode/objective optional. -->

**Files:**
- Create: `crates/vox-orchestrator/src/models/provenance.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs` (one line)
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs` (add producer code, `test_spec`, `routing_tests`)

**Interfaces:**
- Consumes: `vox_orchestrator::models::family::family_key(slug: &str) -> String` (model-routing plan Task 2); `vox_orchestrator::models::spec::PricingSource` (`Bootstrap` (default) | `Unknown` | `OpenRouter` | `AnthropicDirect` | `LiteLLM` | `UserConfig` | `Telemetry`, read from `models/spec.rs`); `vox_orchestrator::mode::ClutchProfile` (`Free | Efficiency | Balanced | Genius`, serde `snake_case`); `super::message::SelectionSource` (`UserOverride | Global | AutoRouted | Fallback`).
- Produces: `pub enum vox_orchestrator::models::provenance::ResolvedFrom { Catalog, Bootstrap, Local }` with `pub fn as_str(self) -> &'static str`; `pub fn resolved_from(spec: &ModelSpec) -> ResolvedFrom`; `pub(crate) fn turn_events::routing_decision_event(spec: &ModelSpec, mode: Option<ClutchProfile>, reason: &str) -> serde_json::Value`; `pub(crate) fn turn_events::routing_reason(rationale: Option<&str>, source: SelectionSource) -> String`; `#[cfg(test)] pub(crate) fn turn_events::test_spec(id: &str, provider_type: ProviderType, pricing_source: PricingSource) -> ModelSpec`.

Rules: `resolved_from` is `local` for Ollama / VoxLocal / PopuliMesh, `bootstrap` for `PricingSource::Bootstrap`
(the compiled bootstrap JSON, "may be stale"), else `catalog`. `routing_reason` is a constant for every source except
`AutoRouted`, whose rationale comes from the scorer (`SelectionReason` display: registry/contract values) or the
free-tier router (static text); the two `resolve.rs` rationales that echo a requested id are replaced by constants.
Until Task 2b wires them, the two producer functions are used only by tests (a `dead_code` warning is expected between
2a and 2b; drive them back to back).

- [ ] **Step 1: Write the failing provenance tests.** Create `crates/vox-orchestrator/src/models/provenance.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ModelCapabilities, StrengthTag};

    fn spec(provider_type: ProviderType, pricing_source: PricingSource) -> ModelSpec {
        ModelSpec {
            id: "acme/widget-5.5".into(),
            canonical_slug: "acme/widget-5.5".into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 32_000,
            cost_per_1k: 0.001,
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.001,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn an_openrouter_priced_spec_comes_from_the_catalog() {
        assert_eq!(
            resolved_from(&spec(ProviderType::OpenRouter, PricingSource::OpenRouter)),
            ResolvedFrom::Catalog
        );
    }

    #[test]
    fn a_bootstrap_priced_cloud_spec_comes_from_the_bootstrap_file() {
        assert_eq!(
            resolved_from(&spec(ProviderType::OpenRouter, PricingSource::Bootstrap)),
            ResolvedFrom::Bootstrap
        );
    }

    #[test]
    fn direct_provider_specs_are_catalog_not_bootstrap() {
        assert_eq!(
            resolved_from(&spec(ProviderType::Anthropic, PricingSource::AnthropicDirect)),
            ResolvedFrom::Catalog
        );
        assert_eq!(
            resolved_from(&spec(ProviderType::GoogleDirect, PricingSource::LiteLLM)),
            ResolvedFrom::Catalog
        );
    }

    #[test]
    fn local_backends_are_local_whatever_their_pricing_source() {
        for provider in [
            ProviderType::Ollama,
            ProviderType::VoxLocal,
            ProviderType::PopuliMesh,
        ] {
            for pricing in [PricingSource::Bootstrap, PricingSource::UserConfig] {
                assert_eq!(
                    resolved_from(&spec(provider.clone(), pricing.clone())),
                    ResolvedFrom::Local,
                    "{provider:?} priced by {pricing:?}"
                );
            }
        }
    }

    #[test]
    fn wire_names_match_the_turn_event_contract() {
        assert_eq!(ResolvedFrom::Catalog.as_str(), "catalog");
        assert_eq!(ResolvedFrom::Bootstrap.as_str(), "bootstrap");
        assert_eq!(ResolvedFrom::Local.as_str(), "local");
    }
}
```

In `crates/vox-orchestrator/src/models/mod.rs`, add `pub mod provenance;` on the line directly after `pub mod prompt_profiles;`.

- [ ] **Step 2: Write the failing producer tests.** In `turn_events.rs`, insert this block between the module doc comment and `#[cfg(test)] mod contract_tests`:

```rust
#[cfg(test)]
mod routing_tests {
    use std::collections::HashSet;

    use super::*;
    use vox_orchestrator::models::ProviderType;
    use vox_orchestrator::models::spec::PricingSource;

    const HOSTILE: &str = "<img src=x onerror=alert(1)>";

    fn widget(pricing: PricingSource) -> ModelSpec {
        test_spec("acme/widget-5.5", ProviderType::OpenRouter, pricing)
    }

    #[test]
    fn a_catalog_pick_carries_family_provenance_mode_and_objective() {
        let ev = routing_decision_event(
            &widget(PricingSource::OpenRouter),
            Some(ClutchProfile::Efficiency),
            "why",
        );
        assert_eq!(ev["kind"], "routing_decision");
        assert_eq!(ev["family"], "acme/widget");
        assert_eq!(ev["resolved_id"], "acme/widget-5.5");
        assert_eq!(ev["resolved_from"], "catalog");
        assert_eq!(ev["mode"], "efficiency");
        assert_eq!(
            ev["objective"],
            "Most quality per dollar; no flagship while another model fits"
        );
        assert_eq!(ev["reason"], "why");
    }

    #[test]
    fn a_bootstrap_pick_is_marked_bootstrap() {
        let ev = routing_decision_event(&widget(PricingSource::Bootstrap), None, "why");
        assert_eq!(ev["resolved_from"], "bootstrap");
        assert_eq!(ev["family"], "acme/widget");
    }

    #[test]
    fn without_a_composer_mode_there_is_no_mode_or_objective() {
        let ev = routing_decision_event(&widget(PricingSource::OpenRouter), None, "why");
        assert!(ev.get("mode").is_none(), "no mode was sent: {ev}");
        assert!(ev.get("objective").is_none(), "no mode, no promise: {ev}");
    }

    #[test]
    fn every_mode_has_its_serde_wire_value_and_its_own_objective() {
        let modes = [
            ClutchProfile::Free,
            ClutchProfile::Efficiency,
            ClutchProfile::Balanced,
            ClutchProfile::Genius,
        ];
        let spec = widget(PricingSource::OpenRouter);
        let mut objectives = HashSet::new();
        for mode in modes {
            let ev = routing_decision_event(&spec, Some(mode), "why");
            assert_eq!(
                ev["mode"],
                serde_json::to_value(mode).expect("ClutchProfile serializes"),
                "mode must equal ClutchProfile's serde name (the GUI's ClutchId)"
            );
            objectives.insert(ev["objective"].as_str().expect("objective").to_string());
        }
        assert_eq!(objectives.len(), 4, "each mode states its own objective");
    }

    #[test]
    fn an_auto_routed_rationale_is_used_and_capped() {
        assert_eq!(
            routing_reason(Some("Chosen by the model scorer"), SelectionSource::AutoRouted),
            "Chosen by the model scorer"
        );
        let long = "x".repeat(300);
        assert_eq!(
            routing_reason(Some(&long), SelectionSource::AutoRouted)
                .chars()
                .count(),
            200
        );
        assert_eq!(
            routing_reason(Some("   "), SelectionSource::AutoRouted),
            "No selection reason recorded for this route",
            "a blank rationale is no rationale"
        );
    }

    #[test]
    fn a_hostile_requested_id_is_never_echoed() {
        let fallback = format!("Fallback: requested `{HOSTILE}` is not in the registry");
        let sticky = format!("Sticky VoxLocal: `{HOSTILE}` synthesized (not yet in orch registry)");
        for (rationale, source) in [
            (fallback.as_str(), SelectionSource::AutoRouted),
            (fallback.as_str(), SelectionSource::Fallback),
            (sticky.as_str(), SelectionSource::AutoRouted),
            (sticky.as_str(), SelectionSource::UserOverride),
            (sticky.as_str(), SelectionSource::Global),
        ] {
            let reason = routing_reason(Some(rationale), source);
            assert!(
                !reason.contains("onerror"),
                "{source:?} echoed the requested id: {reason}"
            );
        }
        assert_eq!(
            routing_reason(Some(&fallback), SelectionSource::Fallback),
            "Requested model unavailable; fell back to auto-routing"
        );
    }

    #[test]
    fn without_a_rationale_the_reason_names_the_selection_source() {
        assert_eq!(
            routing_reason(None, SelectionSource::Global),
            "Pinned by the global model override"
        );
        assert_eq!(routing_reason(None, SelectionSource::UserOverride), "Your pick");
        assert_eq!(
            routing_reason(None, SelectionSource::AutoRouted),
            "No selection reason recorded for this route"
        );
        assert_eq!(
            routing_reason(None, SelectionSource::Fallback),
            "Requested model unavailable; fell back to auto-routing"
        );
    }

    /// The prefix checks above mirror two format strings in resolve.rs; if those are reworded,
    /// revisit `routing_reason` before this test is updated.
    #[test]
    fn resolve_rs_still_formats_the_echoing_rationales_this_module_replaces() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/llm_bridge/model_route_policy/resolve.rs"
        ));
        assert!(src.contains("Fallback: requested `{id}`"));
        assert!(src.contains("Sticky VoxLocal: `{id}`"));
    }

    #[test]
    fn an_over_long_reason_is_capped_in_the_event() {
        let ev = routing_decision_event(
            &widget(PricingSource::OpenRouter),
            Some(ClutchProfile::Genius),
            &"y".repeat(500),
        );
        assert_eq!(ev["reason"].as_str().expect("reason").chars().count(), 200);
    }
}
```

- [ ] **Step 3: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::provenance > target/turn-trace-t2a-red.txt 2>&1; timeout 1500s cargo test -p vox-orchestrator-mcp --lib turn_events >> target/turn-trace-t2a-red.txt 2>&1; tail -30 target/turn-trace-t2a-red.txt`
Expected: both FAIL to compile — `cannot find function resolved_from` / `cannot find type ResolvedFrom` / `PricingSource` not in scope, and `cannot find function routing_decision_event` / `test_spec` / `routing_reason`.

- [ ] **Step 4: Implement provenance.** Insert above `#[cfg(test)]` in `provenance.rs`:

```rust
//! Where a resolved model id came from, for chat routing events and the routing DTO.

use crate::models::spec::PricingSource;
use crate::models::{ModelSpec, ProviderType};

/// Provenance of a resolved model id. The GUI shows a concrete version only for
/// [`ResolvedFrom::Catalog`]; a bootstrap pick shows its family marked offline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedFrom {
    /// Priced from a live source (OpenRouter, a direct provider API, LiteLLM, telemetry, user config).
    Catalog,
    /// Still carries the compiled bootstrap JSON's pricing: its version may be stale.
    Bootstrap,
    /// On-device or mesh inference (Ollama, VoxLocal, Populi mesh).
    Local,
}

impl ResolvedFrom {
    /// Wire name used by the `routing_decision` turn event and `RoutingSummaryDto`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::Bootstrap => "bootstrap",
            Self::Local => "local",
        }
    }
}

/// Classify `spec` by backend, then by `PricingSource`: the catalog refresh re-stamps
/// fetched specs from `Bootstrap` to `OpenRouter` (`registry.rs`), so a spec still marked
/// `Bootstrap` never came from a live source.
#[must_use]
pub fn resolved_from(spec: &ModelSpec) -> ResolvedFrom {
    match (&spec.provider_type, &spec.pricing_source) {
        (ProviderType::Ollama | ProviderType::VoxLocal | ProviderType::PopuliMesh, _) => {
            ResolvedFrom::Local
        }
        (_, PricingSource::Bootstrap) => ResolvedFrom::Bootstrap,
        _ => ResolvedFrom::Catalog,
    }
}

```

- [ ] **Step 5: Implement the producer.** In `turn_events.rs`, insert directly after the module doc comment (above `#[cfg(test)] mod routing_tests`):

```rust

use serde_json::{Value, json};
use vox_orchestrator::mode::ClutchProfile;
use vox_orchestrator::models::ModelSpec;
use vox_orchestrator::models::family::family_key;
use vox_orchestrator::models::provenance::resolved_from;

use super::message::SelectionSource;

/// Longest `reason` echoed into a turn event (it renders in system-styled chrome).
const REASON_CAP: usize = 200;
/// Shown instead of any rationale that names the requested model: a requested or pinned id
/// can come from composer input a model influences, so it is never echoed into chrome.
const FALLBACK_REASON: &str = "Requested model unavailable; fell back to auto-routing";
const STICKY_LOCAL_REASON: &str = "Pinned local model, not yet in the registry";

/// Wire value of a mode: `ClutchProfile`'s serde name and the GUI's `ClutchId`.
fn mode_wire(mode: ClutchProfile) -> &'static str {
    match mode {
        ClutchProfile::Free => "free",
        ClutchProfile::Efficiency => "efficiency",
        ClutchProfile::Balanced => "balanced",
        ClutchProfile::Genius => "genius",
    }
}

/// What each mode promises, in the words the trace shows.
fn mode_objective(mode: ClutchProfile) -> &'static str {
    match mode {
        ClutchProfile::Free => "Zero-priced models only",
        ClutchProfile::Efficiency => "Most quality per dollar; no flagship while another model fits",
        ClutchProfile::Balanced => {
            "Cost, speed and quality weighed evenly; no flagship while another model fits"
        }
        ClutchProfile::Genius => "Highest available intelligence",
    }
}

/// The reason a turn shows. Every source but `AutoRouted` maps to a constant; an auto-routed
/// rationale comes from the scorer (`SelectionReason`) or the free-tier router, except the two
/// `resolve.rs` lines that echo a requested id, which become constants too.
// ponytail: the two prefixes mirror resolve.rs format strings (pinned by a test); if resolve.rs
// ever returns a typed rationale, match on that instead.
pub(crate) fn routing_reason(rationale: Option<&str>, source: SelectionSource) -> String {
    let text = match source {
        SelectionSource::Fallback => FALLBACK_REASON,
        SelectionSource::UserOverride => "Your pick",
        SelectionSource::Global => "Pinned by the global model override",
        SelectionSource::AutoRouted => match rationale.map(str::trim).filter(|r| !r.is_empty()) {
            Some(r) if r.starts_with("Fallback: requested") => FALLBACK_REASON,
            Some(r) if r.starts_with("Sticky VoxLocal:") => STICKY_LOCAL_REASON,
            Some(r) => r,
            None => "No selection reason recorded for this route",
        },
    };
    text.chars().take(REASON_CAP).collect()
}

/// The `routing_decision` turn event (`contracts/gui/turn-event-kinds.v1.json`). `mode` and
/// `objective` appear only when the composer sent a mode: with none, the resolver used its own
/// default axes and the event makes no promise about them.
pub(crate) fn routing_decision_event(
    spec: &ModelSpec,
    mode: Option<ClutchProfile>,
    reason: &str,
) -> Value {
    let mut ev = json!({
        "kind": "routing_decision",
        "family": family_key(&spec.id),
        "resolved_id": spec.id,
        "resolved_from": resolved_from(spec).as_str(),
        "reason": reason.chars().take(REASON_CAP).collect::<String>(),
    });
    if let Some(mode) = mode {
        ev["mode"] = json!(mode_wire(mode));
        ev["objective"] = json!(mode_objective(mode));
    }
    ev
}

/// Test-only spec builder shared by this module's tests and `message`'s turn-trace tests.
#[cfg(test)]
pub(crate) fn test_spec(
    id: &str,
    provider_type: vox_orchestrator::models::ProviderType,
    pricing_source: vox_orchestrator::models::spec::PricingSource,
) -> ModelSpec {
    ModelSpec {
        id: id.into(),
        canonical_slug: id.into(),
        provider: "test".into(),
        provider_type,
        max_tokens: 32_000,
        cost_per_1k: 0.001,
        cost_per_1k_input: 0.001,
        cost_per_1k_output: 0.001,
        is_free: false,
        observed_cost_per_1k: None,
        strengths: vec![vox_orchestrator::models::StrengthTag::Generalist],
        capabilities: vox_orchestrator::models::ModelCapabilities::default(),
        cache_creation_cost_per_1k: 0.0,
        cache_read_cost_per_1k: 0.0,
        supports_prompt_caching: false,
        pricing_source,
        supported_parameters: vec![],
    }
}
```

- [ ] **Step 6: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::provenance 2>&1 | tail -8; timeout 1500s cargo test -p vox-orchestrator-mcp --lib turn_events 2>&1 | tail -20`
Expected: provenance `5 passed`; `turn_events` all pass (9 `routing_tests` + 3 `contract_tests`; `routing_decision` is still `planned` and `produced` has no arm for it). A `dead_code` warning for `routing_decision_event` / `routing_reason` is expected until Task 2b.

- [ ] **Step 7: Mutation proofs.** (a) In `provenance.rs` delete the line `(_, PricingSource::Bootstrap) => ResolvedFrom::Bootstrap,`; run `timeout 1500s cargo test -p vox-orchestrator --lib models::provenance > target/turn-trace-t2a-mutant-a.txt 2>&1`; confirm `a_bootstrap_priced_cloud_spec_comes_from_the_bootstrap_file ... FAILED`; restore. (b) In `routing_reason` delete the line `Some(r) if r.starts_with("Fallback: requested") => FALLBACK_REASON,`; run `timeout 1500s cargo test -p vox-orchestrator-mcp --lib turn_events > target/turn-trace-t2a-mutant-b.txt 2>&1`; confirm `a_hostile_requested_id_is_never_echoed ... FAILED`; restore. (c) Replace `SelectionSource::UserOverride => "Your pick",` with `SelectionSource::UserOverride => rationale.unwrap_or("Your pick"),`; run into `target/turn-trace-t2a-mutant-c.txt`; confirm `a_hostile_requested_id_is_never_echoed ... FAILED`; restore. (d) Replace `if let Some(mode) = mode {` with `if let Some(mode) = mode.or(Some(ClutchProfile::Efficiency)) {`; run into `target/turn-trace-t2a-mutant-d.txt`; confirm `without_a_composer_mode_there_is_no_mode_or_objective ... FAILED`; restore. `git diff --stat` lists only `models/mod.rs` and `turn_events.rs` among tracked files (plus the new `provenance.rs`).

- [ ] **Step 8:** `rustfmt --edition 2024 crates/vox-orchestrator/src/models/provenance.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs`; re-run Step 6.

- [ ] **Step 9: Commit (Claude Code)**

```bash
git add -- crates/vox-orchestrator/src/models/provenance.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs
git commit -m "feat(chat-trace): model provenance and a routing_decision producer that never echoes input"
```

---

### Task 2b: Every agent-loop chat turn starts with one `routing_decision`

<!-- AMENDED: T2 — wiring half of the old Task 2; adds the no-clutch wiring test. -->

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` (`try_run_agent_turn` only, plus a 3-line test-module declaration at the end)
- Create: `crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs` (`contract_tests` only)
- Modify: `contracts/gui/turn-event-kinds.v1.json` (delete one line)

**Interfaces:**
- Consumes: `routing_decision_event`, `routing_reason`, `test_spec` (Task 2a); `SelectionSource::classify(requested: Option<&str>, resolved: Option<&str>, global: Option<&str>) -> SelectionSource` (existing, `message.rs`); `try_run_agent_turn(state, system_prompt, user_prompt, session_id, active_skill_id, has_attachment, temperature, top_p, request_model_override, tier, clutch: Option<&str>, risk) -> Option<Result<AgentTurnResult, String>>` (existing, private); `ServerState::hermetic_stub`, `CHAT_MESSAGE_ENV_LOCK` (existing).
- Produces: `AgentTurnResult.events[0]` is the turn's `routing_decision`; test helpers `test_state`, `register_pinned`, `plain_body`, `point_openrouter_at`, `restore_openrouter`, constant `MODEL_ID` in `message_turn_trace_tests.rs` (reused by Task 9).

- [ ] **Step 1: Write the failing wiring tests.** Create `crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs`:

```rust
//! Turn-trace tests that drive `try_run_agent_turn` (the default `vox_chat_message` path)
//! against a wiremock model server. A child module of `message` so it can call the private
//! `try_run_agent_turn` and read `AgentTurnResult` without widening either.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use vox_orchestrator::models::ProviderType;
use vox_orchestrator::models::spec::PricingSource;
use vox_orchestrator::{
    AffinityGroupRegistry, Orchestrator, OrchestratorConfig, SessionConfig, SessionManager,
};
use vox_repository::{RepoCapabilities, RepositoryContext};
use vox_skills::new_registry_arc;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::super::agent_loop::CHAT_MESSAGE_ENV_LOCK;
use super::super::turn_events::test_spec;
use super::try_run_agent_turn;
use crate::server_state::ServerState;

const MODEL_ID: &str = "acme/widget-5.5";

fn test_state() -> ServerState {
    let cfg = OrchestratorConfig::for_testing();
    let orch_cfg = cfg.clone();
    let groups = AffinityGroupRegistry::new(vec![]);
    let session_cfg = SessionConfig {
        persist: false,
        sessions_dir: std::env::temp_dir().join("vox-mcp-turn-trace-test-sessions"),
        ..SessionConfig::default()
    };
    let session_manager = SessionManager::new(session_cfg).expect("session manager");
    let repository = RepositoryContext {
        root: PathBuf::from("."),
        git_root: None,
        repository_id: "turn-trace-test".into(),
        origin_url: None,
        capabilities: RepoCapabilities {
            vox_project: false,
            cargo_workspace: false,
            cargo_package: false,
            node_workspace: false,
            python_project: false,
            go_module: false,
            git: false,
        },
        has_vox_agents_dir: false,
        vox_toml: None,
    };
    ServerState::hermetic_stub(
        cfg,
        repository,
        Arc::new(Orchestrator::with_groups(orch_cfg, groups)),
        Arc::new(Mutex::new(session_manager)),
        new_registry_arc(),
    )
}

/// Register a catalog-priced OpenRouter spec and pin it as the process-global chat model.
fn register_pinned(state: &ServerState, id: &str) {
    {
        let handle = state.orchestrator.models_handle();
        let mut registry = handle.write().expect("models registry lock");
        registry.register(test_spec(id, ProviderType::OpenRouter, PricingSource::OpenRouter));
    }
    *state.mcp_chat_model_override.write() = Some(id.to_string());
}

fn plain_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": content},
            "finish_reason": "stop",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    })
}

/// Point OpenRouter at `uri`; returns the previous values. Callers hold `CHAT_MESSAGE_ENV_LOCK`.
#[allow(unsafe_code)]
fn point_openrouter_at(uri: &str) -> (Option<String>, Option<String>) {
    let prev = (
        std::env::var("OPENROUTER_BASE_URL").ok(),
        std::env::var("OPENROUTER_API_KEY").ok(),
    );
    // SAFETY: callers hold CHAT_MESSAGE_ENV_LOCK, the crate-wide lock for these two vars.
    unsafe {
        std::env::set_var("OPENROUTER_BASE_URL", uri);
        std::env::set_var("OPENROUTER_API_KEY", "test-key");
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);
    prev
}

#[allow(unsafe_code)]
fn restore_openrouter(prev: (Option<String>, Option<String>)) {
    // SAFETY: as in `point_openrouter_at`.
    unsafe {
        match prev.0 {
            Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
            None => std::env::remove_var("OPENROUTER_BASE_URL"),
        }
        match prev.1 {
            Some(v) => std::env::set_var("OPENROUTER_API_KEY", v),
            None => std::env::remove_var("OPENROUTER_API_KEY"),
        }
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);
}

/// Run one plain (no-tool) default-path turn with `clutch`; returns the turn's events.
async fn plain_turn_events(clutch: Option<&str>) -> Vec<serde_json::Value> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(plain_body("no tools needed")))
        .mount(&server)
        .await;
    let prev = point_openrouter_at(&server.uri());
    let state = test_state();
    register_pinned(&state, MODEL_ID);
    let result = try_run_agent_turn(
        &state,
        "system prompt",
        "hello there",
        "trace-session",
        None,
        false,
        None,
        None,
        None,
        None,
        clutch,
        None,
    )
    .await;
    restore_openrouter(prev);
    result
        .expect("an OpenRouter pick runs the agent loop")
        .expect("the turn succeeds")
        .events
}

/// The composer's mode flows into the event (a hard-coded default would say "efficiency").
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn a_default_path_turn_starts_with_exactly_one_routing_decision() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let events = plain_turn_events(Some("genius")).await;
    let routing = events
        .iter()
        .filter(|e| e["kind"] == "routing_decision")
        .count();
    assert_eq!(routing, 1, "exactly one routing_decision per turn: {events:?}");
    let first = &events[0];
    assert_eq!(first["kind"], "routing_decision");
    assert_eq!(first["resolved_id"], MODEL_ID);
    assert_eq!(first["family"], "acme/widget");
    assert_eq!(first["resolved_from"], "catalog");
    assert_eq!(first["mode"], "genius");
    assert_eq!(first["objective"], "Highest available intelligence");
    assert_eq!(first["reason"], "Pinned by the global model override");
}

/// No composer mode: the resolver used its own default axes, so the event claims no mode.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn a_turn_without_a_composer_mode_claims_no_mode() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let events = plain_turn_events(None).await;
    let first = &events[0];
    assert_eq!(first["kind"], "routing_decision");
    assert!(first.get("mode").is_none(), "no mode was sent: {first}");
    assert!(first.get("objective").is_none(), "no mode, no promise: {first}");
    assert_eq!(first["resolved_id"], MODEL_ID);
}
```

Append to the very end of `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`:

```rust

#[cfg(test)]
#[path = "message_turn_trace_tests.rs"]
mod turn_trace_tests;
```

In `turn_events.rs` `contract_tests`, add these three `use` lines under the existing `use serde_json::{Value, json};`:

```rust
    use vox_orchestrator::mode::ClutchProfile;
    use vox_orchestrator::models::ProviderType;
    use vox_orchestrator::models::spec::PricingSource;
```

add this arm to `produced`, directly before `_ => None,`:

```rust
            "routing_decision" => Some(super::routing_decision_event(
                &super::test_spec("acme/widget-5.5", ProviderType::OpenRouter, PricingSource::OpenRouter),
                Some(ClutchProfile::Efficiency),
                "Chosen by the model scorer as the best match for your request",
            )),
```

and replace the body of `standalone_events` with:

```rust
        vec![
            receipt_turn_event(&sample_receipt(), true),
            super::routing_decision_event(
                &super::test_spec("acme/widget-5.5", ProviderType::OpenRouter, PricingSource::OpenRouter),
                None,
                "r",
            ),
        ]
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib -- turn_events turn_trace_tests > target/turn-trace-t2b-red.txt 2>&1; tail -30 target/turn-trace-t2b-red.txt`
Expected (compiles, fails on behaviour): `a_default_path_turn_starts_with_exactly_one_routing_decision ... FAILED` and `a_turn_without_a_composer_mode_claims_no_mode ... FAILED` (no routing event: `message.rs` is not wired), and `every_contract_example_equals_what_rust_emits ... FAILED` with "routing_decision has a Rust producer now". If either wiring test passes here, STOP: something else already emits the event.

- [ ] **Step 3: Wire `try_run_agent_turn`.** In `message.rs`, directly after the line `let selection_reason = choice.rationale;` insert:

```rust
    // One `routing_decision` per assistant turn, first in `events`, built only from the resolved
    // spec, the composer's mode (if it sent one) and a reason that never echoes a requested id
    // (docs/superpowers/plans/2026-09-28-chat-turn-trace.md).
    let routing_event = super::turn_events::routing_decision_event(
        &model,
        clutch.and_then(vox_orchestrator::mode::ClutchProfile::from_label),
        &super::turn_events::routing_reason(
            selection_reason.as_deref(),
            SelectionSource::classify(
                request_model_override
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
                Some(model.id.as_str()),
                global_pref.as_deref(),
            ),
        ),
    );
```

and, in the same function, replace the line `                events: outcome.events,` with:

```rust
                events: std::iter::once(routing_event)
                    .chain(outcome.events)
                    .collect(),
```

- [ ] **Step 4: Flip the contract.** In `contracts/gui/turn-event-kinds.v1.json`, delete the single line `      "status": "planned",` (one targeted edit; the `routing_decision` example is unchanged).

- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib -- turn_events message:: 2>&1 | tail -20`
Expected: all pass, including every pre-existing `message::tests` test and the two `turn_trace_tests`. A pre-existing test that fails is a STOP (name + assertion). If a wiring test fails only on `reason`, paste the actual value and STOP; do not change the expected string. The `dead_code` warning from Task 2a is gone.

- [ ] **Step 6: Mutation proofs.** (a) Put back `events: outcome.events,` in place of the three-line `std::iter::once(...)` expression; run `timeout 1500s cargo test -p vox-orchestrator-mcp --lib turn_trace_tests > target/turn-trace-t2b-mutant-a.txt 2>&1`; confirm both wiring tests FAIL; restore. (b) Replace `clutch.and_then(vox_orchestrator::mode::ClutchProfile::from_label),` with `Some(vox_orchestrator::mode::ClutchProfile::Efficiency),`; run into `target/turn-trace-t2b-mutant-b.txt`; confirm both wiring tests FAIL (mode `efficiency` ≠ `genius`; mode present with no clutch); restore. `git diff --stat` lists only this task's files; the contract shows `1 deletion`.

- [ ] **Step 7:** `rustfmt --edition 2024 crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs`; re-run Step 5.

- [ ] **Step 8: Commit (Claude Code)**

```bash
git add -- crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/turn_events.rs contracts/gui/turn-event-kinds.v1.json
git commit -m "feat(chat-trace): every agent-loop chat turn emits one server-derived routing_decision"
```

---

### Task 3: `RoutingSummaryDto` carries family, provenance and reason

<!-- AMENDED: T3 — kept as is, except the test spec now varies PricingSource (Task 2a's rule) instead of released_at. -->

**Files:**
- Modify: `crates/vox-gui/src/commands/models.rs` (`RoutingSummaryDto`, `get_routing_summary`, new private `routing_provenance`, tests)
- Modify: `crates/vox-gui/ui/src/types/tauri.ts` (`RoutingSummary`)

The brief placed this DTO in `vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs`; it is not there. `RoutingSummaryDto`
is defined in `crates/vox-gui/src/commands/models.rs` and mirrored by `RoutingSummary` in `types/tauri.ts`. The HTTP
`/routing/summary` endpoint builds its own JSON and is left alone. This task pairs one Rust DTO with its 3-line TS mirror
because they are one seam; `typecheck` is the TS check.

**Interfaces:**
- Consumes: `models::family::family_key`, `models::provenance::resolved_from` (Task 2a), `models::select::SelectionReason` (existing `Display`), `ModelSelectionDecision.outcome.{model_spec, reason}` (existing).
- Produces: `RoutingSummaryDto { …, family: Option<String>, resolved_from: Option<String>, reason: Option<String> }`; TS `RoutingSummary.family?: string | null; resolved_from?: 'catalog' | 'bootstrap' | 'local' | null; reason?: string | null`.

- [ ] **Step 1: Write the failing tests.** Append inside `mod tests` in `crates/vox-gui/src/commands/models.rs`:

```rust
    fn provenance_spec(
        pricing_source: vox_orchestrator::models::spec::PricingSource,
    ) -> ModelSpec {
        ModelSpec {
            id: "acme/widget-5.5".into(),
            canonical_slug: "acme/widget-5.5".into(),
            provider: "test".into(),
            provider_type: vox_orchestrator::models::ProviderType::OpenRouter,
            max_tokens: 32_000,
            cost_per_1k: 0.001,
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.001,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![vox_orchestrator::models::StrengthTag::Generalist],
            capabilities: vox_orchestrator::models::ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn routing_provenance_reports_family_catalog_source_and_reason() {
        let (family, from, reason) = routing_provenance(
            &provenance_spec(vox_orchestrator::models::spec::PricingSource::OpenRouter),
            &vox_orchestrator::models::select::SelectionReason::Scored,
        );
        assert_eq!(family, "acme/widget");
        assert_eq!(from, "catalog");
        assert_eq!(
            reason,
            "Chosen by the model scorer as the best match for your request"
        );
    }

    #[test]
    fn routing_provenance_marks_a_bootstrap_pick_as_bootstrap() {
        let (_, from, _) = routing_provenance(
            &provenance_spec(vox_orchestrator::models::spec::PricingSource::Bootstrap),
            &vox_orchestrator::models::select::SelectionReason::Scored,
        );
        assert_eq!(from, "bootstrap");
    }

    #[test]
    fn routing_summary_serializes_absent_provenance_as_null() {
        let dto = RoutingSummaryDto {
            active_model: None,
            exploration_spent_usd: 0.0,
            exploration_budget_usd: 0.0,
            routing_priority: sample_priority(),
            arm_count: 0,
            model_count: 0,
            decision_preview: None,
            family: None,
            resolved_from: None,
            reason: None,
        };
        let v = serde_json::to_value(&dto).expect("serialize");
        for key in ["family", "resolved_from", "reason"] {
            assert!(
                v.get(key).is_some_and(serde_json::Value::is_null),
                "{key} must serialize as null, got {v}"
            );
        }
    }
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-gui --bin vox-gui commands::models > target/turn-trace-t3-red.txt 2>&1; tail -20 target/turn-trace-t3-red.txt`
Expected: FAIL to compile — `cannot find function routing_provenance` and `struct RoutingSummaryDto has no field named family`. A failure inside `tauri-build` about a missing resource path is the fresh-worktree sidecar problem (AGENTS.md "Perennial Bug Patterns"): STOP and say so.

- [ ] **Step 3: Implement.** In `RoutingSummaryDto`, directly after `pub decision_preview: Option<DecisionPreviewDto>,` add:

```rust
    /// Family of the preview pick (`models::family::family_key`); `None` without a pick.
    pub family: Option<String>,
    /// `catalog` | `bootstrap` | `local` (`models::provenance::resolved_from`). The GUI shows a
    /// concrete version only for `catalog`.
    pub resolved_from: Option<String>,
    /// `SelectionReason` display text of the preview pick.
    pub reason: Option<String>,
```

Directly above `pub async fn get_routing_summary(` add:

```rust
/// Family, provenance and reason of a routing pick, for [`RoutingSummaryDto`].
fn routing_provenance(
    spec: &ModelSpec,
    reason: &vox_orchestrator::models::select::SelectionReason,
) -> (String, &'static str, String) {
    (
        vox_orchestrator::models::family::family_key(&spec.id),
        vox_orchestrator::models::provenance::resolved_from(spec).as_str(),
        reason.to_string(),
    )
}

```

In `get_routing_summary`, replace the whole `let decision_preview = { ... };` block (from `let decision_preview = {` through its closing `};`) with:

```rust
    let decision = decide(
        &ModelSelectionRequest::from_intent(SelectionIntent::for_task(TaskCategory::CodeGen)),
        &reg,
    );
    let (family, resolved_from, reason) = match &decision {
        Some(d) => {
            let (family, from, reason) =
                routing_provenance(&d.outcome.model_spec, &d.outcome.reason);
            (Some(family), Some(from.to_string()), Some(reason))
        }
        None => (None, None, None),
    };
    let decision_preview = decision.map(|d| DecisionPreviewDto {
        selected_model: d.selected_model,
        discovery_state: d.discovery_state.as_str().to_string(),
        alternatives: d.alternatives,
        rejection_reasons: d.rejection_reasons,
        intelligence_score: d.score_breakdown.intelligence_score,
        efficiency_score: d.score_breakdown.efficiency_score,
        latency_score: d.score_breakdown.latency_score,
    });
```

and in the `Ok(RoutingSummaryDto { ... })` literal at the end of that function, directly after `decision_preview,` add `family,`, `resolved_from,`, `reason,` (one per line).

In `crates/vox-gui/ui/src/types/tauri.ts`, inside `export interface RoutingSummary`, directly after `decision_preview: DecisionPreview | null;` add:

```ts
  /** Family of the preview pick (Rust `models::family::family_key`). */
  family?: string | null;
  /** Where the pick's id came from; show a concrete version only for `catalog`. */
  resolved_from?: 'catalog' | 'bootstrap' | 'local' | null;
  /** `SelectionReason` display text of the preview pick. */
  reason?: string | null;
```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-gui --bin vox-gui commands::models 2>&1 | tail -15 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck`
Expected: all `commands::models::tests` pass (the three new and every existing one); typecheck exits 0.

- [ ] **Step 5: Mutation proof.** In `routing_provenance` replace `vox_orchestrator::models::provenance::resolved_from(spec).as_str(),` with `"catalog",`; run `timeout 1500s cargo test -p vox-gui --bin vox-gui commands::models > target/turn-trace-t3-mutant.txt 2>&1`; confirm `routing_provenance_marks_a_bootstrap_pick_as_bootstrap ... FAILED`; restore; `git diff --stat` shows only the two files.

- [ ] **Step 6:** `rustfmt --edition 2024 crates/vox-gui/src/commands/models.rs`; re-run Step 4.

- [ ] **Step 7: Commit (Claude Code)**

```bash
git add -- crates/vox-gui/src/commands/models.rs crates/vox-gui/ui/src/types/tauri.ts
git commit -m "feat(chat-trace): routing summary reports family, provenance and reason"
```

---

### Task 4: TS side of the contract; `ChatTurnEventRow` renders every kind

<!-- AMENDED: T4 — engine-event validators and TURN_ENGINE_EVENT_TYPES removed; MODE_NAMES exported as the single owner of mode names; routing mode/objective optional; research row proven not to render `query` (sentinel test + mutation). -->

**Files:**
- Create: `crates/vox-gui/ui/src/lib/turnEvents.ts`
- Create: `crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx`
- Regression: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.test.tsx` (not edited)

**Interfaces:**
- Consumes: the contract (Tasks 1–2b; `routing_decision` has no `status`); `TurnEventDto` (`types/dashboard.ts`: `{ kind: string; skill_id?: string; [key: string]: unknown }`).
- Produces (`lib/turnEvents.ts`, the single owner of these labels — the surfaces plan imports them): `export const TURN_EVENT_KINDS: readonly string[]`; `export function isKnownTurnEvent(e: TurnEventDto | null | undefined): e is TurnEventDto`; `export const MODE_NAMES: Readonly<Record<'free' | 'efficiency' | 'balanced' | 'genius', string>>` (`Free`, `Efficient`, `Balanced`, `Genius`); `export function modeLabel(wire: string): string`; `export function routingModelLabel(e: TurnEventDto): string` (`catalog` → `<resolved_id>`; `local` → `<resolved_id> (local)`; `bootstrap` → `<family> (offline)`). `ChatTurnEventRow` test ids `chat-turn-delegation-row`, `chat-turn-research-row`, `chat-turn-routing-row` (with `data-resolved-from`).

- [ ] **Step 1: Write the failing test.** Create `crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ChatTurnEventRow } from './ChatTurnEventRow';
import { isKnownTurnEvent, MODE_NAMES, modeLabel, TURN_EVENT_KINDS } from '../../../lib/turnEvents';
import type { TurnEventDto } from '../../../types/dashboard';

interface ContractKind {
  kind: string;
  status?: string;
  required: string[];
  example: TurnEventDto;
}

const CONTRACT: { kinds: ContractKind[] } = JSON.parse(
  readFileSync(
    join(
      dirname(fileURLToPath(import.meta.url)),
      '../../../../../../../contracts/gui/turn-event-kinds.v1.json',
    ),
    'utf8',
  ),
);

const shipped = CONTRACT.kinds.filter((k) => k.status !== 'planned');

function example(kind: string): TurnEventDto {
  const entry = CONTRACT.kinds.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return { ...entry.example };
}

function without(event: TurnEventDto, field: string): TurnEventDto {
  const copy: TurnEventDto = { ...event };
  delete copy[field];
  return copy;
}

describe('turn-event contract (GUI side)', () => {
  it('the GUI knows exactly the contract kinds', () => {
    expect(CONTRACT.kinds.map((k) => k.kind).sort()).toEqual([...TURN_EVENT_KINDS].sort());
  });

  it.each(shipped.map((k) => [k.kind, k] as const))('%s: its example renders', (_kind, entry) => {
    const { container } = render(<ChatTurnEventRow event={entry.example} />);
    expect(container).not.toBeEmptyDOMElement();
  });

  it.each(shipped.flatMap((k) => k.required.map((field) => [k.kind, field, k] as const)))(
    '%s without %s renders nothing',
    (_kind, field, entry) => {
      const { container } = render(<ChatTurnEventRow event={without(entry.example, field)} />);
      expect(container).toBeEmptyDOMElement();
    },
  );

  it('research_milestone never renders its model-supplied query', () => {
    render(<ChatTurnEventRow event={{ ...example('research_milestone'), query: 'SENTINEL-QUERY-TEXT' }} />);
    const row = screen.getByTestId('chat-turn-research-row');
    expect(row).toHaveTextContent('3 waves');
    expect(row).not.toHaveTextContent('SENTINEL-QUERY-TEXT');
  });

  it('routing_decision without a mode still renders, with no mode text', () => {
    const noMode = without(without(example('routing_decision'), 'mode'), 'objective');
    render(<ChatTurnEventRow event={noMode} />);
    const row = screen.getByTestId('chat-turn-routing-row');
    expect(row).toHaveTextContent('acme/widget-5.5');
    expect(row).not.toHaveTextContent('Efficient');
    expect(row).not.toHaveTextContent('undefined');
  });

  it('mode names are the canonical vocabulary keyed by wire value', () => {
    expect(MODE_NAMES).toEqual({ free: 'Free', efficiency: 'Efficient', balanced: 'Balanced', genius: 'Genius' });
    expect(modeLabel('efficiency')).toBe('Efficient');
    expect(modeLabel('constructor')).toBe('constructor');
  });

  it('a kind that only exists on Object.prototype is not a known event', () => {
    expect(isKnownTurnEvent({ kind: 'constructor' })).toBe(false);
    expect(isKnownTurnEvent({ kind: 'toString' })).toBe(false);
    expect(isKnownTurnEvent(undefined)).toBe(false);
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/turnEventContract.test.tsx > target/turn-trace-t4-red.txt 2>&1; tail -20 target/turn-trace-t4-red.txt`
Expected: FAIL — cannot resolve `../../../lib/turnEvents`.

- [ ] **Step 3: Create the validator and label owner.** Create `crates/vox-gui/ui/src/lib/turnEvents.ts`:

```ts
import type { TurnEventDto } from '../types/dashboard';

/**
 * Turn-event kinds Rust puts in a chat reply's `events` (`contracts/gui/turn-event-kinds.v1.json`;
 * producers in `crates/vox-orchestrator-mcp/src/chat_tools/chat/{agent_loop,turn_events}.rs`).
 */
export const TURN_EVENT_KINDS: readonly string[] = [
  'skill_activated',
  'delegation_spawned',
  'research_milestone',
  'tool_receipt',
  'receipt_claims',
  'routing_decision',
];

const isText = (v: unknown): v is string => typeof v === 'string' && v.length > 0;
const isCount = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const isFlag = (v: unknown): v is boolean => typeof v === 'boolean';
const optionalText = (v: unknown) => v === undefined || isText(v);

const RESOLVED_FROM = new Set(['catalog', 'bootstrap', 'local']);

/** Required-field checks, one per kind above (a Map, so `constructor` is not a kind). */
const VALIDATORS = new Map<string, (e: TurnEventDto) => boolean>([
  ['skill_activated', (e) => isText(e.skill_id)],
  ['delegation_spawned', (e) => isText(e.tool) && isCount(e.agent_id)],
  [
    'research_milestone',
    (e) =>
      isText(e.tool) &&
      isCount(e.waves_executed) &&
      isCount(e.claims_verified) &&
      isCount(e.contradictions_resolved),
  ],
  [
    'tool_receipt',
    (e) => isText(e.tool) && isText(e.receipt_id) && isFlag(e.fulfilled) && isFlag(e.verified),
  ],
  ['receipt_claims', (e) => isCount(e.valid) && isCount(e.fabricated) && isCount(e.unverified)],
  [
    'routing_decision',
    (e) =>
      isText(e.family) &&
      isText(e.resolved_id) &&
      typeof e.resolved_from === 'string' &&
      RESOLVED_FROM.has(e.resolved_from) &&
      isText(e.reason) &&
      optionalText(e.mode) &&
      optionalText(e.objective),
  ],
]);

/** True for a known kind whose required fields are present and well-typed. */
export function isKnownTurnEvent(e: TurnEventDto | null | undefined): e is TurnEventDto {
  if (!e || typeof e.kind !== 'string') return false;
  const check = VALIDATORS.get(e.kind);
  return check !== undefined && check(e);
}

/**
 * Canonical mode names keyed by wire value (`ClutchId`). The one owner of these labels: the
 * composer, rail and status bar import them from here.
 */
export const MODE_NAMES = Object.freeze({
  free: 'Free',
  efficiency: 'Efficient',
  balanced: 'Balanced',
  genius: 'Genius',
} as const);

/** Canonical mode name for a wire value (`efficiency` → `Efficient`); unknown values pass through. */
export function modeLabel(wire: string): string {
  return Object.prototype.hasOwnProperty.call(MODE_NAMES, wire)
    ? MODE_NAMES[wire as keyof typeof MODE_NAMES]
    : wire;
}

/**
 * What a routing decision may show as the model (the one owner of this rule): the catalog id only
 * when it came from a live source, the local id marked local, the family marked offline for the
 * bootstrap fallback. Never a "(latest)" claim.
 */
export function routingModelLabel(e: TurnEventDto): string {
  if (e.resolved_from === 'catalog') return String(e.resolved_id);
  if (e.resolved_from === 'local') return `${String(e.resolved_id)} (local)`;
  return `${String(e.family)} (offline)`;
}
```

- [ ] **Step 4: Run; the row must now fail on behaviour.**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/turnEventContract.test.tsx > target/turn-trace-t4-red2.txt 2>&1; tail -40 target/turn-trace-t4-red2.txt`
Expected: exactly these 8 FAIL — `delegation_spawned: its example renders`, `research_milestone: its example renders`, `routing_decision: its example renders`, `skill_activated without skill_id renders nothing`, `tool_receipt without fulfilled renders nothing`, `tool_receipt without verified renders nothing`, `research_milestone never renders its model-supplied query`, `routing_decision without a mode still renders, with no mode text`; every other case passes. A different failure set is a STOP (paste it).

- [ ] **Step 5: Implement the row.** In `ChatTurnEventRow.tsx`:
  1. Directly after `import type { TurnEventDto } from '../../../types/dashboard';` add:

```tsx
import { isKnownTurnEvent, modeLabel, routingModelLabel } from '../../../lib/turnEvents';

const CHIP =
  'flex items-center gap-2 self-start rounded-full border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[11px] text-text-secondary';
```

  2. Make the first statement inside `export function ChatTurnEventRow({ event, onExcludeSkill }: ChatTurnEventRowProps) {`:

```tsx
  if (!isKnownTurnEvent(event)) return null;
```

  3. Replace the function's final `  return null;` (two-space indent, directly before the closing `}` of the function) with:

```tsx
  if (event.kind === 'delegation_spawned') {
    const taskId = typeof event.task_id === 'number' ? event.task_id : null;
    return (
      <div data-testid="chat-turn-delegation-row" className={CHIP}>
        <span>
          delegated · agent {String(event.agent_id)}
          {taskId != null ? ` · task ${taskId}` : ''}
        </span>
      </div>
    );
  }

  if (event.kind === 'research_milestone') {
    // `query` is model-supplied and never rendered; counts only.
    return (
      <div data-testid="chat-turn-research-row" className={CHIP}>
        <span>
          research · {String(event.waves_executed)} waves · {String(event.claims_verified)} claims
          verified · {String(event.contradictions_resolved)} contradictions resolved
        </span>
      </div>
    );
  }

  if (event.kind === 'routing_decision') {
    const mode = typeof event.mode === 'string' ? event.mode : null;
    return (
      <div
        data-testid="chat-turn-routing-row"
        data-resolved-from={String(event.resolved_from)}
        title={typeof event.objective === 'string' ? event.objective : undefined}
        className={CHIP}
      >
        <span>
          routed to {routingModelLabel(event)}
          {mode ? ` · ${modeLabel(mode)}` : ''} — {String(event.reason)}
        </span>
      </div>
    );
  }

  return null;
```

- [ ] **Step 6: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/turnEventContract.test.tsx src/components/surfaces/Chat/ChatTurnEventRow.test.tsx 2>&1 | tail -15 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck`
Expected: both files pass (every pre-existing `ChatTurnEventRow` test included); typecheck exits 0.

- [ ] **Step 7: Mutation proofs.** (a) Delete the line `  if (!isKnownTurnEvent(event)) return null;`; run the contract test into `target/turn-trace-t4-mutant-a.txt`; confirm the three "without … renders nothing" cases for `skill_id`, `fulfilled` and `verified` FAIL; restore. (b) In the research branch replace `contradictions resolved` with `contradictions resolved · {String(event.query)}`; run into `target/turn-trace-t4-mutant-b.txt`; confirm `research_milestone never renders its model-supplied query` FAILS; restore. `git diff --stat` shows only `ChatTurnEventRow.tsx` among tracked files.

- [ ] **Step 8: Commit (Claude Code)**

```bash
git add -- crates/vox-gui/ui/src/lib/turnEvents.ts crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx
git commit -m "feat(chat-trace): GUI checks the turn-event contract; delegation, research and routing render"
```

---

### Task 5: `turnTrace.ts` — the pure trace builder

<!-- AMENDED: T5 — engine-event interrupts, statuses and describeStep removed; the only live interrupt is a flagged receipt_claims; summary falls back to the message's modelId when no routing_decision exists (routing wins when both exist). -->

**Files:**
- Create: `crates/vox-gui/ui/src/lib/turnTrace.ts`
- Create: `crates/vox-gui/ui/src/lib/turnTrace.test.ts`

**Interfaces:**
- Consumes: `isKnownTurnEvent`, `routingModelLabel` (Task 4); `ChatVerbosity` (`'quiet' | 'normal' | 'verbose'`, `hooks/useChatVerbosity.ts`); contract examples.
- Produces: `export type TraceItemStatus = 'ok' | 'failed' | 'info'`; `export interface TraceItem { event: TurnEventDto; count: number; status: TraceItemStatus }`; `export interface TurnTraceSummary { modelLabel: string | null; tools: number; receiptsVerified: number; receiptsUnverified: number; delegations: number; researchWaves: number; durationMs: number | null }`; `export interface TurnTrace { summary: TurnTraceSummary; steps: TraceItem[]; interrupts: TraceItem[]; inline: TurnEventDto[]; defaultExpanded: boolean }`; `export function buildTurnTrace(events: TurnEventDto[] | undefined, verbosity: ChatVerbosity, meta?: { latencyMs?: number; modelId?: string }): TurnTrace`; `export function isInterrupt(e: TurnEventDto): boolean`; `export function summaryText(s: TurnTraceSummary): string`.

Policy (critique, anti-spam): the only interrupt among live kinds is a flagged `receipt_claims` (fabricated or
unverified > 0). `skill_activated` stays inline (actionable, not urgent). Everything else is a step; identical consecutive
items coalesce. An unverified `tool_receipt` is a failed step. Default expansion: quiet → collapsed; normal → open only
when a step failed or there is an interrupt; verbose → open. The model label comes from `routing_decision` via
`routingModelLabel`; only when the reply has none (history hydrate, attachments, unmapped providers, `cognitive_profile`,
background tasks) does it fall back to the message's `modelId`, shown as the plain id with no version claim.

- [ ] **Step 1: Write the failing tests.** Create `crates/vox-gui/ui/src/lib/turnTrace.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { TurnEventDto } from '../types/dashboard';
import { buildTurnTrace, isInterrupt, summaryText } from './turnTrace';

const CONTRACT: { kinds: Array<{ kind: string; example: TurnEventDto }> } = JSON.parse(
  readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), '../../../../../contracts/gui/turn-event-kinds.v1.json'),
    'utf8',
  ),
);

function example(kind: string): TurnEventDto {
  const entry = CONTRACT.kinds.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return { ...entry.example };
}

const EMPTY = {
  modelLabel: null,
  tools: 0,
  receiptsVerified: 0,
  receiptsUnverified: 0,
  delegations: 0,
  researchWaves: 0,
  durationMs: null,
};

describe('buildTurnTrace', () => {
  it('summarises a clean turn from the contract examples', () => {
    const t = buildTurnTrace(
      [example('routing_decision'), example('tool_receipt'), example('delegation_spawned'), example('research_milestone')],
      'normal',
      { latencyMs: 4100 },
    );
    expect(t.summary).toEqual({
      modelLabel: 'acme/widget-5.5',
      tools: 1,
      receiptsVerified: 1,
      receiptsUnverified: 0,
      delegations: 1,
      researchWaves: 3,
      durationMs: 4100,
    });
    expect(t.steps.map((s) => s.event.kind)).toEqual([
      'routing_decision',
      'tool_receipt',
      'delegation_spawned',
      'research_milestone',
    ]);
    expect(t.interrupts).toEqual([]);
    expect(t.defaultExpanded).toBe(false);
    expect(summaryText(t.summary)).toBe(
      'acme/widget-5.5 · 1 tool · 1 receipt ✓ · 1 delegated · 3 research waves · 4.1s',
    );
  });

  it('shows the family marked offline for a bootstrap pick and the local id for a local pick', () => {
    const offline = buildTurnTrace([{ ...example('routing_decision'), resolved_from: 'bootstrap' }], 'normal');
    expect(offline.summary.modelLabel).toBe('acme/widget (offline)');
    const local = buildTurnTrace(
      [{ ...example('routing_decision'), resolved_from: 'local', resolved_id: 'mens/run-7' }],
      'normal',
    );
    expect(local.summary.modelLabel).toBe('mens/run-7 (local)');
  });

  it('falls back to the message modelId, plainly, only when no routing decision exists', () => {
    const hydrated = buildTurnTrace(undefined, 'normal', { modelId: 'mens/run-7', latencyMs: 1200 });
    expect(hydrated.summary.modelLabel).toBe('mens/run-7');
    expect(summaryText(hydrated.summary)).toBe('mens/run-7 · 1.2s');
    const both = buildTurnTrace([{ ...example('routing_decision'), resolved_from: 'bootstrap' }], 'normal', {
      modelId: 'acme/widget-5.5',
    });
    expect(both.summary.modelLabel).toBe('acme/widget (offline)');
  });

  it('keeps skill activation inline and makes a flagged claims verdict an interrupt, not a step', () => {
    const t = buildTurnTrace([example('skill_activated'), example('receipt_claims'), example('tool_receipt')], 'quiet');
    expect(t.inline.map((e) => e.kind)).toEqual(['skill_activated']);
    expect(t.interrupts.map((i) => i.event.kind)).toEqual(['receipt_claims']);
    expect(t.steps.map((s) => s.event.kind)).toEqual(['tool_receipt']);
    expect(t.defaultExpanded).toBe(false);
  });

  it('a clean claims verdict is a step, not an interrupt', () => {
    const t = buildTurnTrace([{ ...example('receipt_claims'), fabricated: 0, unverified: 0 }], 'normal');
    expect(t.interrupts).toEqual([]);
    expect(t.steps).toHaveLength(1);
    expect(t.steps[0].status).toBe('ok');
  });

  it('an unverified receipt without a claims verdict is a failed step, not an interrupt', () => {
    const t = buildTurnTrace([{ ...example('tool_receipt'), verified: false }], 'normal');
    expect(t.interrupts).toEqual([]);
    expect(t.steps[0].status).toBe('failed');
    expect(t.summary.receiptsUnverified).toBe(1);
    expect(t.defaultExpanded).toBe(true);
  });

  it('coalesces identical consecutive events into one step with a count', () => {
    const r = example('research_milestone');
    const t = buildTurnTrace([r, { ...r }, { ...r }], 'quiet');
    expect(t.steps).toHaveLength(1);
    expect(t.steps[0]).toMatchObject({ count: 3, status: 'ok' });
    expect(t.summary.researchWaves).toBe(9);
  });

  it('does not coalesce events that differ or are not adjacent', () => {
    const r = example('research_milestone');
    const t = buildTurnTrace([r, { ...r, waves_executed: 4 }, r], 'normal');
    expect(t.steps.map((s) => s.count)).toEqual([1, 1, 1]);
  });

  it('drops unknown kinds and events missing a required field', () => {
    const noReceiptId: TurnEventDto = { ...example('tool_receipt') };
    delete noReceiptId.receipt_id;
    const t = buildTurnTrace([{ kind: 'from_the_future' }, noReceiptId, example('routing_decision')], 'normal');
    expect(t.steps.map((s) => s.event.kind)).toEqual(['routing_decision']);
    expect(t.summary.tools).toBe(0);
  });

  it('default expansion follows verbosity', () => {
    const clean = [example('routing_decision'), example('tool_receipt')];
    const failed = [example('routing_decision'), { ...example('tool_receipt'), verified: false }];
    const interrupted = [...clean, example('receipt_claims')];
    expect(['quiet', 'normal', 'verbose'].map((v) => buildTurnTrace(clean, v as 'quiet').defaultExpanded)).toEqual([
      false,
      false,
      true,
    ]);
    expect(buildTurnTrace(failed, 'quiet').defaultExpanded).toBe(false);
    expect(buildTurnTrace(failed, 'normal').defaultExpanded).toBe(true);
    expect(buildTurnTrace(interrupted, 'normal').defaultExpanded).toBe(true);
  });

  it('treats an absent event list as an empty trace', () => {
    const t = buildTurnTrace(undefined, 'verbose');
    expect(t.steps).toEqual([]);
    expect(t.summary).toEqual(EMPTY);
  });
});

describe('isInterrupt', () => {
  it('is only a flagged claims verdict among live kinds', () => {
    expect(isInterrupt(example('receipt_claims'))).toBe(true);
    expect(isInterrupt({ ...example('receipt_claims'), fabricated: 0, unverified: 2 })).toBe(true);
    expect(isInterrupt({ ...example('receipt_claims'), fabricated: 0, unverified: 0 })).toBe(false);
    for (const kind of ['skill_activated', 'delegation_spawned', 'research_milestone', 'tool_receipt', 'routing_decision']) {
      expect(isInterrupt(example(kind)), kind).toBe(false);
    }
  });
});

describe('summaryText', () => {
  it('reports verified out of total when a receipt failed', () => {
    expect(summaryText({ ...EMPTY, tools: 3, receiptsVerified: 2, receiptsUnverified: 1 })).toBe(
      '3 tools · 2/3 receipts verified',
    );
  });

  it('is empty for an empty summary', () => {
    expect(summaryText(EMPTY)).toBe('');
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/turnTrace.test.ts > target/turn-trace-t5-red.txt 2>&1; tail -15 target/turn-trace-t5-red.txt`
Expected: FAIL — cannot resolve `./turnTrace`.

- [ ] **Step 3: Implement.** Create `crates/vox-gui/ui/src/lib/turnTrace.ts`:

```ts
import type { TurnEventDto } from '../types/dashboard';
import type { ChatVerbosity } from '../hooks/useChatVerbosity';
import { isKnownTurnEvent, routingModelLabel } from './turnEvents';

export type TraceItemStatus = 'ok' | 'failed' | 'info';

/** One trace row; `count` > 1 when identical consecutive events were coalesced. */
export interface TraceItem {
  event: TurnEventDto;
  count: number;
  status: TraceItemStatus;
}

export interface TurnTraceSummary {
  /** `routingModelLabel` of the turn's routing decision, else the message's plain `modelId`. */
  modelLabel: string | null;
  tools: number;
  receiptsVerified: number;
  receiptsUnverified: number;
  delegations: number;
  researchWaves: number;
  durationMs: number | null;
}

export interface TurnTrace {
  summary: TurnTraceSummary;
  /** Rows inside the collapsible trace, in arrival order. */
  steps: TraceItem[];
  /** Things a human must act on; shown inline even while the trace is collapsed. */
  interrupts: TraceItem[];
  /** Actionable, non-urgent chips kept inline (skill activation and its "not this one"). */
  inline: TurnEventDto[];
  defaultExpanded: boolean;
}

const OK_KINDS = new Set(['delegation_spawned', 'research_milestone', 'receipt_claims']);

/** Interrupts are only what a human must act on (critique anti-spam policy, rule 1). */
export function isInterrupt(e: TurnEventDto): boolean {
  return e.kind === 'receipt_claims' && ((e.fabricated as number) > 0 || (e.unverified as number) > 0);
}

function statusOf(e: TurnEventDto): TraceItemStatus {
  if (e.kind === 'tool_receipt') return e.verified === true ? 'ok' : 'failed';
  if (OK_KINDS.has(e.kind)) return 'ok';
  return 'info';
}

/** Append `e`, or bump the count when it is identical to the previous item. */
function pushCoalesced(items: TraceItem[], e: TurnEventDto): void {
  const last = items[items.length - 1];
  if (last && JSON.stringify(last.event) === JSON.stringify(e)) {
    last.count += 1;
    return;
  }
  items.push({ event: e, count: 1, status: statusOf(e) });
}

/** Decide what one assistant turn's trace says and whether it opens by default. */
export function buildTurnTrace(
  events: TurnEventDto[] | undefined,
  verbosity: ChatVerbosity,
  meta: { latencyMs?: number; modelId?: string } = {},
): TurnTrace {
  const summary: TurnTraceSummary = {
    modelLabel: null,
    tools: 0,
    receiptsVerified: 0,
    receiptsUnverified: 0,
    delegations: 0,
    researchWaves: 0,
    durationMs:
      typeof meta.latencyMs === 'number' && Number.isFinite(meta.latencyMs) ? meta.latencyMs : null,
  };
  const steps: TraceItem[] = [];
  const interrupts: TraceItem[] = [];
  const inline: TurnEventDto[] = [];
  for (const e of events ?? []) {
    if (!isKnownTurnEvent(e)) continue;
    if (e.kind === 'skill_activated') {
      inline.push(e);
      continue;
    }
    if (isInterrupt(e)) {
      pushCoalesced(interrupts, e);
      continue;
    }
    if (e.kind === 'routing_decision' && summary.modelLabel === null) {
      summary.modelLabel = routingModelLabel(e);
    }
    if (e.kind === 'tool_receipt') {
      summary.tools += 1;
      if (e.verified === true) summary.receiptsVerified += 1;
      else summary.receiptsUnverified += 1;
    }
    if (e.kind === 'delegation_spawned') summary.delegations += 1;
    if (e.kind === 'research_milestone') summary.researchWaves += e.waves_executed as number;
    pushCoalesced(steps, e);
  }
  // No routing decision (history hydrate, attachments, unmapped providers, background tasks):
  // keep the reply's own model id, plainly, with no version or recency claim.
  if (summary.modelLabel === null && typeof meta.modelId === 'string' && meta.modelId.length > 0) {
    summary.modelLabel = meta.modelId;
  }
  const needsALook = steps.some((s) => s.status === 'failed') || interrupts.length > 0;
  const defaultExpanded = verbosity === 'verbose' || (verbosity === 'normal' && needsALook);
  return { summary, steps, interrupts, inline, defaultExpanded };
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** The one-line summary shown on the trace row. */
export function summaryText(s: TurnTraceSummary): string {
  const parts: string[] = [];
  if (s.modelLabel) parts.push(s.modelLabel);
  if (s.tools > 0) parts.push(plural(s.tools, 'tool', 'tools'));
  const receipts = s.receiptsVerified + s.receiptsUnverified;
  if (receipts > 0) {
    parts.push(
      s.receiptsUnverified > 0
        ? `${s.receiptsVerified}/${receipts} receipts verified`
        : `${plural(receipts, 'receipt', 'receipts')} ✓`,
    );
  }
  if (s.delegations > 0) parts.push(`${s.delegations} delegated`);
  if (s.researchWaves > 0) parts.push(plural(s.researchWaves, 'research wave', 'research waves'));
  if (s.durationMs != null) parts.push(`${(s.durationMs / 1000).toFixed(1)}s`);
  return parts.join(' · ');
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/turnTrace.test.ts 2>&1 | tail -10 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck`
Expected: all pass; typecheck exits 0.

- [ ] **Step 5: Mutation proofs.** (a) Change `(verbosity === 'normal' && needsALook)` to `false`; run into `target/turn-trace-t5-mutant-a.txt`; confirm `default expansion follows verbosity` and `an unverified receipt without a claims verdict is a failed step, not an interrupt` FAIL; restore. (b) In `pushCoalesced` change `if (last && JSON.stringify(last.event) === JSON.stringify(e)) {` to `if (false) {`; run into `target/turn-trace-t5-mutant-b.txt`; confirm `coalesces identical consecutive events into one step with a count` FAILS; restore. (c) In the modelId fallback change `if (summary.modelLabel === null && typeof meta.modelId === 'string'` to `if (typeof meta.modelId === 'string'`; run into `target/turn-trace-t5-mutant-c.txt`; confirm `falls back to the message modelId, plainly, only when no routing decision exists` FAILS; restore. `git diff --stat` shows no tracked-file changes (both files are new).

- [ ] **Step 6: Commit (Claude Code)**

```bash
git add -- crates/vox-gui/ui/src/lib/turnTrace.ts crates/vox-gui/ui/src/lib/turnTrace.test.ts
git commit -m "feat(chat-trace): pure turn-trace builder with interrupts, coalescing and verbosity"
```

---

### Task 6: `TurnTrace` replaces the loose chips and the `ModelBadge`

<!-- AMENDED: T6 (was T7) — PhaseChip/engine interrupt chips dropped (no live pav/injection route); no role="status"; replies without events keep model attribution and the "Assistant" header; hydrated-message test + mutation proofs; gui-honesty docs handled in Claude's sweep. -->

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.test.tsx` (one sanctioned replacement, imports, one new test)
- Delete: `crates/vox-gui/ui/src/components/surfaces/Chat/ModelBadge.tsx`, `crates/vox-gui/ui/src/components/surfaces/Chat/ModelBadge.test.tsx`
- Not the agent's: `docs/agents/gui-honesty-manifest.json` and `docs/agents/gui-honesty-triage.md` list `ModelBadge.tsx`; Claude's verification sweep regenerates / edits them.

**Interfaces:**
- Consumes: `buildTurnTrace`, `summaryText`, `TraceItem` (Task 5); `ChatTurnEventRow` (Task 4); `ChatMessage.{events, modelId, latencyMs, status}` (`lib/chatCorrelation.ts`; history hydrate in `App.tsx` restores `modelId`/`latencyMs` but not `events`).
- Produces: `export function TurnTrace(props: { events?: TurnEventDto[]; verbosity: ChatVerbosity; latencyMs?: number; modelId?: string; onExcludeSkill?: (skillId: string) => void })`. Test ids: `chat-trace`, `chat-trace-summary` (a button with `aria-expanded`/`aria-controls` when there are steps; a plain line otherwise), `chat-trace-steps` (`<ol hidden>` when collapsed — always in the DOM when there are steps), `chat-trace-step` (`data-kind`, `data-status`). `MessageBubble` gains `verbosity?: ChatVerbosity` (default `'normal'`), renders the trace for an assistant reply that has events or is done with a `modelId`, and always shows the "Assistant" header.

Every interrupt among live kinds is a flagged `receipt_claims`, rendered through `ChatTurnEventRow` (no extra
`role="status"`: the transcript is already an `aria-live` `role="log"`). The steps list uses the `hidden` attribute rather
than unmounting, so `aria-controls` always resolves and Phase 5's `e2e/chat-trust-chips.spec.ts` (which counts receipt rows)
keeps passing: its first turn has an unverified receipt (a failed step, so Normal opens it) and its second has a flagged
claims verdict (an interrupt, rendered inline once — not also as a step).

- [ ] **Step 1: Preconditions.** Run `rg -n "ModelBadge|SelectionDto" crates/vox-gui/ui/src crates/vox-gui/ui/e2e`. Expected: matches only in `ChatTranscript.tsx`, `ChatTranscript.test.tsx`, `ModelBadge.tsx`, `ModelBadge.test.tsx`. Any other file is a STOP.

- [ ] **Step 2: Write the failing tests.** Create `TurnTrace.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TurnTrace } from './TurnTrace';
import type { TurnEventDto } from '../../../types/dashboard';

interface Contract {
  kinds: Array<{ kind: string; example: TurnEventDto }>;
  golden_turn?: { events: TurnEventDto[] };
}
const CONTRACT: Contract = JSON.parse(
  readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), '../../../../../../../contracts/gui/turn-event-kinds.v1.json'),
    'utf8',
  ),
);

function example(kind: string): TurnEventDto {
  const entry = CONTRACT.kinds.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return { ...entry.example };
}

const CLEAN = [
  example('routing_decision'),
  example('tool_receipt'),
  example('delegation_spawned'),
  example('research_milestone'),
];

describe('TurnTrace', () => {
  it('a clean turn is one collapsed summary row', () => {
    render(<TurnTrace events={CLEAN} verbosity="normal" latencyMs={4100} />);
    const summary = screen.getByTestId('chat-trace-summary');
    expect(summary).toHaveTextContent('acme/widget-5.5 · 1 tool · 1 receipt ✓ · 1 delegated · 3 research waves · 4.1s');
    expect(summary).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
  });

  it('the summary row expands into the steps in arrival order', () => {
    render(<TurnTrace events={CLEAN} verbosity="normal" />);
    fireEvent.click(screen.getByTestId('chat-trace-summary'));
    expect(screen.getByTestId('chat-trace-summary')).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByTestId('chat-trace-steps')).toBeVisible();
    expect(screen.getAllByTestId('chat-trace-step').map((li) => li.getAttribute('data-kind'))).toEqual([
      'routing_decision',
      'tool_receipt',
      'delegation_spawned',
      'research_milestone',
    ]);
    expect(screen.getByTestId('chat-turn-delegation-row')).toBeVisible();
    expect(screen.getByTestId('chat-turn-research-row')).toBeVisible();
  });

  it('an interrupt shows inline while the trace stays collapsed (quiet)', () => {
    render(
      <TurnTrace
        events={[example('routing_decision'), example('tool_receipt'), example('receipt_claims')]}
        verbosity="quiet"
      />,
    );
    expect(screen.getByTestId('chat-turn-claims-row')).toHaveAttribute('data-flagged', 'true');
    expect(screen.getAllByTestId('chat-turn-claims-row')).toHaveLength(1);
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('identical steps coalesce into one row with a count', () => {
    const r = example('research_milestone');
    render(<TurnTrace events={[example('routing_decision'), r, { ...r }, { ...r }]} verbosity="verbose" />);
    const step = screen
      .getAllByTestId('chat-trace-step')
      .find((li) => li.getAttribute('data-kind') === 'research_milestone');
    expect(step).toHaveTextContent('3×');
  });

  it('skill activation stays inline with its "not this one" action', () => {
    const onExcludeSkill = vi.fn();
    render(
      <TurnTrace
        events={[example('skill_activated'), example('routing_decision')]}
        verbosity="quiet"
        onExcludeSkill={onExcludeSkill}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'not this one' }));
    expect(onExcludeSkill).toHaveBeenCalledWith('ponytail');
  });

  it('a reply with no events keeps its model id as a plain, non-expandable summary', () => {
    render(<TurnTrace verbosity="normal" modelId="mens/run-7" latencyMs={1200} />);
    const summary = screen.getByTestId('chat-trace-summary');
    expect(summary).toHaveTextContent('mens/run-7 · 1.2s');
    expect(summary).not.toHaveTextContent('latest');
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('renders nothing when no event is known and there is no model id', () => {
    const { container } = render(<TurnTrace events={[{ kind: 'from_the_future' }]} verbosity="verbose" />);
    expect(container).toBeEmptyDOMElement();
  });

  it('switching to verbose opens a clean trace that normal leaves collapsed', () => {
    const { rerender } = render(<TurnTrace events={CLEAN} verbosity="normal" />);
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
    rerender(<TurnTrace events={CLEAN} verbosity="verbose" />);
    expect(screen.getByTestId('chat-trace-steps')).toBeVisible();
  });
});
```

In `ChatTranscript.test.tsx`: directly after `import { listHarnessIssuesForSession } from '../Scientia/harnessIssuesApi';` add

```tsx
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { TurnEventDto } from '../../../types/dashboard';

const CONTRACT_KINDS: Array<{ kind: string; example: TurnEventDto }> = JSON.parse(
  readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), '../../../../../../../contracts/gui/turn-event-kinds.v1.json'),
    'utf8',
  ),
).kinds;

function example(kind: string): TurnEventDto {
  const entry = CONTRACT_KINDS.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return { ...entry.example };
}
```

and **(sanctioned edit)** replace the whole `describe('MessageBubble local-model badge', () => { ... });` block with:

```tsx
describe('MessageBubble routing trace (replaces ModelBadge)', () => {
  it('shows a local pick as its local id in the trace summary, with no model badge', () => {
    const local = { ...example('routing_decision'), resolved_from: 'local', resolved_id: 'mens/e2e-smoke-metal' };
    render(<MessageBubble message={msg({ modelId: 'mens/e2e-smoke-metal', events: [local] })} />);
    expect(screen.getByTestId('chat-trace-summary')).toHaveTextContent('mens/e2e-smoke-metal (local)');
    expect(screen.queryByRole('button', { name: /Completed by/ })).not.toBeInTheDocument();
  });

  it('a hydrated reply (modelId and latency, no events) keeps its attribution and the Assistant header', () => {
    render(<MessageBubble message={msg({ modelId: 'mens/e2e-smoke-metal', latencyMs: 1200 })} />);
    expect(screen.getByText('Assistant')).toBeInTheDocument();
    const summary = screen.getByTestId('chat-trace-summary');
    expect(summary).toHaveTextContent('mens/e2e-smoke-metal · 1.2s');
    expect(summary).not.toHaveTextContent('latest');
  });
});
```

- [ ] **Step 3: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/TurnTrace.test.tsx src/components/surfaces/Chat/ChatTranscript.test.tsx > target/turn-trace-t6-red.txt 2>&1; tail -30 target/turn-trace-t6-red.txt`
Expected: `TurnTrace.test.tsx` fails to resolve `./TurnTrace`; in `ChatTranscript.test.tsx` exactly the two new `MessageBubble routing trace` tests fail (no `chat-trace-summary`; and for the hydrated one, the "Assistant" header is hidden because `modelId` is set and `ModelBadge` renders instead). Every other `ChatTranscript` test passes.

- [ ] **Step 4: Implement the component.** Create `TurnTrace.tsx`:

```tsx
import React, { useEffect, useId, useMemo, useState } from 'react';
import type { TurnEventDto } from '../../../types/dashboard';
import type { ChatVerbosity } from '../../../hooks/useChatVerbosity';
import { buildTurnTrace, summaryText } from '../../../lib/turnTrace';
import { ChatTurnEventRow } from './ChatTurnEventRow';

interface TurnTraceProps {
  events?: TurnEventDto[];
  verbosity: ChatVerbosity;
  latencyMs?: number;
  /** The reply's model id; shown plainly only when the turn has no routing decision. */
  modelId?: string;
  /** "not this one" on a skill-activation chip — see `ChatTurnEventRow`. */
  onExcludeSkill?: (skillId: string) => void;
}

/**
 * One assistant turn's account: inline skill chips and interrupts (things a human must act on),
 * then one summary row that expands into the ordered steps. Default expansion follows the
 * verbosity control (`buildTurnTrace`). Every row is built from server-derived event fields.
 */
export function TurnTrace({ events, verbosity, latencyMs, modelId, onExcludeSkill }: TurnTraceProps) {
  const trace = useMemo(
    () => buildTurnTrace(events, verbosity, { latencyMs, modelId }),
    [events, verbosity, latencyMs, modelId],
  );
  const [expanded, setExpanded] = useState(trace.defaultExpanded);
  useEffect(() => setExpanded(trace.defaultExpanded), [trace.defaultExpanded]);
  const stepsId = useId();
  const summary = summaryText(trace.summary);

  if (
    trace.steps.length === 0 &&
    trace.interrupts.length === 0 &&
    trace.inline.length === 0 &&
    summary === ''
  ) {
    return null;
  }

  return (
    <div data-testid="chat-trace" className="mt-1 flex flex-col items-start gap-1">
      {trace.inline.map((event, i) => (
        <ChatTurnEventRow key={`inline-${i}`} event={event} onExcludeSkill={onExcludeSkill} />
      ))}
      {trace.interrupts.map((item, i) => (
        <ChatTurnEventRow key={`interrupt-${i}`} event={item.event} />
      ))}
      {trace.steps.length > 0 ? (
        <>
          <button
            type="button"
            data-testid="chat-trace-summary"
            aria-expanded={expanded}
            aria-controls={stepsId}
            onClick={() => setExpanded((v) => !v)}
            className="flex items-center gap-1.5 rounded-md px-1 py-0.5 font-mono text-[11px] text-text-muted hover:text-text-secondary"
          >
            <span aria-hidden="true">{expanded ? '▾' : '▸'}</span>
            <span>{summary || `${trace.steps.length} steps`}</span>
          </button>
          <ol
            id={stepsId}
            data-testid="chat-trace-steps"
            hidden={!expanded}
            className="space-y-1 border-l border-border-subtle pl-3"
          >
            {trace.steps.map((item, i) => (
              <li
                key={i}
                data-testid="chat-trace-step"
                data-kind={item.event.kind}
                data-status={item.status}
                className="flex items-center gap-2"
              >
                <ChatTurnEventRow event={item.event} />
                {item.count > 1 && (
                  <span className="font-mono text-[11px] text-text-muted">{item.count}×</span>
                )}
              </li>
            ))}
          </ol>
        </>
      ) : summary !== '' ? (
        <div data-testid="chat-trace-summary" className="px-1 py-0.5 font-mono text-[11px] text-text-muted">
          {summary}
        </div>
      ) : null}
    </div>
  );
}
```

- [ ] **Step 5: Wire it into the transcript.** In `ChatTranscript.tsx`:
  1. Replace `import { ModelBadge } from './ModelBadge';` with `import { TurnTrace } from './TurnTrace';`.
  2. Replace `import { useChatVerbosity } from '../../../hooks/useChatVerbosity';` with `import { useChatVerbosity, type ChatVerbosity } from '../../../hooks/useChatVerbosity';`.
  3. Change the `MessageBubble` signature from

```tsx
export function MessageBubble({
  message,
  onExcludeSkill,
}: {
  message: ChatMessage;
  onExcludeSkill?: (skillId: string) => void;
}) {
```

to

```tsx
export function MessageBubble({
  message,
  onExcludeSkill,
  verbosity = 'normal',
}: {
  message: ChatMessage;
  onExcludeSkill?: (skillId: string) => void;
  verbosity?: ChatVerbosity;
}) {
```

  4. Replace `      {!isSystem && !isUser && !message.modelId && (` (the "Assistant" header) with `      {!isSystem && !isUser && (` — the badge that used to identify the reply is gone, so the header always shows.
  5. Delete the whole block that starts `      {message.role === 'assistant' && message.status === 'done' && message.modelId && (` and renders `<ModelBadge … />` (through its closing `      )}`).
  6. Replace the events block

```tsx
      {message.role === 'assistant' && message.events && message.events.length > 0 && (
        <div className="mt-1 flex flex-wrap justify-end gap-1">
          {message.events.map((ev, i) => (
            <ChatTurnEventRow key={i} event={ev} onExcludeSkill={onExcludeSkill} />
          ))}
        </div>
      )}
```

with

```tsx
      {message.role === 'assistant' &&
        ((message.events?.length ?? 0) > 0 || (message.status === 'done' && !!message.modelId)) && (
          <TurnTrace
            events={message.events}
            verbosity={verbosity}
            latencyMs={message.latencyMs}
            modelId={message.modelId}
            onExcludeSkill={onExcludeSkill}
          />
        )}
```

  7. Delete the now-unused line `import { ChatTurnEventRow } from './ChatTurnEventRow';`.
  8. In `ChatTranscript`, replace `<MessageBubble key={row.id} message={row.message} onExcludeSkill={onExcludeSkill} />` with `<MessageBubble key={row.id} message={row.message} onExcludeSkill={onExcludeSkill} verbosity={verbosity} />`.

- [ ] **Step 6: Delete the superseded badge.** `rm crates/vox-gui/ui/src/components/surfaces/Chat/ModelBadge.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ModelBadge.test.tsx` (exactly these two paths).

- [ ] **Step 7: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat src/lib 2>&1 | tail -15 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck && timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/chat-trust-chips.spec.ts --project=chromium --reporter=line`
Expected: all vitest pass; typecheck exits 0; `3 passed` for the Phase 5 trust-chips spec (regression). A trust-chips failure is a STOP (paste the assertion).

- [ ] **Step 8: Mutation proofs.** (a) Change `{trace.interrupts.map((item, i) => (` to `{expanded && trace.interrupts.map((item, i) => (`; run the `TurnTrace.test.tsx` vitest into `target/turn-trace-t6-mutant-a.txt`; confirm `an interrupt shows inline while the trace stays collapsed (quiet)` FAILS; restore. (b) In `ChatTranscript.tsx` change `((message.events?.length ?? 0) > 0 || (message.status === 'done' && !!message.modelId))` to `(message.events?.length ?? 0) > 0`; run the `ChatTranscript.test.tsx` vitest into `target/turn-trace-t6-mutant-b.txt`; confirm `a hydrated reply (modelId and latency, no events) keeps its attribution and the Assistant header` FAILS; restore. (c) Put back `!message.modelId && ` in the Assistant-header condition; run into `target/turn-trace-t6-mutant-c.txt`; confirm the same hydrated test FAILS; restore. `git diff --stat` shows `ChatTranscript.tsx`, `ChatTranscript.test.tsx` and the two deletions only.

- [ ] **Step 9: Commit (Claude Code)** — the gui-honesty docs are updated in the verification sweep, not here.

```bash
git add -- crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.test.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ModelBadge.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ModelBadge.test.tsx
git commit -m "feat(chat-trace): one collapsed trace per turn replaces loose chips and the model badge"
```

---

### Task 7: Quiet / Normal / Verbose control

<!-- AMENDED: T7 (was T8) — renumbered; hint text no longer mentions engine failures. -->

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatVerbosityControl.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatVerbosityControl.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.test.tsx` (new tests only)
- Modify: `crates/vox-gui/ui/src/hooks/useChatVerbosity.ts` (doc comment only)

**Interfaces:**
- Consumes: `useChatVerbosity()` → `[ChatVerbosity, setter]` and `CHAT_VERBOSITY_KEY = 'gui.chat.verbosity.v1'` (existing); `TurnTrace` (Task 6); `example()` helper in `ChatTranscript.test.tsx` (Task 6).
- Produces: `export function ChatVerbosityControl(props: { value: ChatVerbosity; onChange: (next: ChatVerbosity) => void })` — `role="radiogroup"` `aria-label="Trace detail"`, `data-testid="chat-verbosity-control"`, three `role="radio"` buttons named `Quiet`, `Normal`, `Verbose`. Rendered by `ChatTranscript` (the hook's only consumer) at the top of the log, so one hook instance drives both the control and every trace.

- [ ] **Step 1: Write the failing tests.** Create `ChatVerbosityControl.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ChatVerbosityControl } from './ChatVerbosityControl';

describe('ChatVerbosityControl', () => {
  it('offers the three levels and marks the current one', () => {
    render(<ChatVerbosityControl value="normal" onChange={() => {}} />);
    expect(screen.getAllByRole('radio').map((r) => r.textContent)).toEqual(['Quiet', 'Normal', 'Verbose']);
    expect(screen.getByRole('radio', { name: 'Normal' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: 'Verbose' })).toHaveAttribute('aria-checked', 'false');
  });

  it('reports the chosen level', () => {
    const onChange = vi.fn();
    render(<ChatVerbosityControl value="normal" onChange={onChange} />);
    fireEvent.click(screen.getByRole('radio', { name: 'Quiet' }));
    expect(onChange).toHaveBeenCalledWith('quiet');
  });

  it('checks nothing for a stale stored value', () => {
    render(<ChatVerbosityControl value={'loud' as 'normal'} onChange={() => {}} />);
    expect(screen.getAllByRole('radio').filter((r) => r.getAttribute('aria-checked') === 'true')).toHaveLength(0);
  });
});
```

Append to `ChatTranscript.test.tsx`:

```tsx
describe('ChatTranscript verbosity control', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.mocked(listHarnessIssuesForSession).mockReset();
  });

  it('Verbose opens a clean trace that Normal leaves collapsed, and persists the choice', () => {
    render(<ChatTranscript messages={[msg({ events: [example('routing_decision'), example('tool_receipt')] })]} />);
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
    fireEvent.click(screen.getByRole('radio', { name: 'Verbose' }));
    expect(screen.getByTestId('chat-trace-steps')).toBeVisible();
    expect(localStorage.getItem('gui.chat.verbosity.v1')).toBe('"verbose"');
  });

  it('Quiet keeps a failed turn collapsed', () => {
    render(
      <ChatTranscript
        messages={[msg({ events: [example('routing_decision'), { ...example('tool_receipt'), verified: false }] })]}
      />,
    );
    expect(screen.getByTestId('chat-trace-steps')).toBeVisible();
    fireEvent.click(screen.getByRole('radio', { name: 'Quiet' }));
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/ChatVerbosityControl.test.tsx src/components/surfaces/Chat/ChatTranscript.test.tsx > target/turn-trace-t7-red.txt 2>&1; tail -30 target/turn-trace-t7-red.txt`
Expected: `ChatVerbosityControl.test.tsx` fails to resolve `./ChatVerbosityControl`; the two new `ChatTranscript verbosity control` tests fail (no radio); every other `ChatTranscript` test passes.

- [ ] **Step 3: Implement.** Create `ChatVerbosityControl.tsx`:

```tsx
import React from 'react';
import type { ChatVerbosity } from '../../../hooks/useChatVerbosity';

const OPTIONS: ReadonlyArray<{ value: ChatVerbosity; label: string; hint: string }> = [
  { value: 'quiet', label: 'Quiet', hint: 'Traces stay collapsed; only things that need you show' },
  { value: 'normal', label: 'Normal', hint: 'A trace opens when a receipt failed or something needs you' },
  { value: 'verbose', label: 'Verbose', hint: 'Every trace opens' },
];

/** How much of each turn's trace opens by default (`buildTurnTrace`). */
export function ChatVerbosityControl({
  value,
  onChange,
}: {
  value: ChatVerbosity;
  onChange: (next: ChatVerbosity) => void;
}) {
  return (
    <div
      role="radiogroup"
      aria-label="Trace detail"
      data-testid="chat-verbosity-control"
      className="flex items-center gap-1 self-end font-mono text-[11px]"
    >
      {OPTIONS.map((o) => {
        const active = value === o.value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={active}
            title={o.hint}
            onClick={() => onChange(o.value)}
            className={`rounded-md border px-2 py-0.5 ${
              active ? 'border-brass/40 text-text-primary' : 'border-transparent text-text-muted hover:text-text-secondary'
            }`}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
```

In `ChatTranscript.tsx`: add `import { ChatVerbosityControl } from './ChatVerbosityControl';` after the `TurnTrace` import; replace `const [verbosity] = useChatVerbosity();` with `const [verbosity, setVerbosity] = useChatVerbosity();`; and insert `<ChatVerbosityControl value={verbosity} onChange={setVerbosity} />` as the first child of `<div className="mx-auto flex w-full max-w-[900px] flex-col gap-2">` (before the `harnessIssues` block).

In `useChatVerbosity.ts`, replace the doc comment above `export function useChatVerbosity()` with:

```ts
/**
 * Global chat-feed verbosity, set by `ChatVerbosityControl`. It decides how much of each turn's
 * trace opens by default (`buildTurnTrace`): quiet keeps traces collapsed, normal opens a trace
 * when a receipt failed or something needs you, verbose opens every trace. Interrupts show at
 * every level. Quiet also hides the per-task "Done · $x" row (`buildChatOnlyTimeline`).
 */
```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat src/hooks 2>&1 | tail -12 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck`
Expected: all pass (including `useChatVerbosity.test.ts`); typecheck exits 0.

- [ ] **Step 5: Mutation proof.** Change `onClick={() => onChange(o.value)}` to `onClick={() => {}}`; run the Step 4 vitest into `target/turn-trace-t7-mutant.txt`; confirm `reports the chosen level` and `Verbose opens a clean trace that Normal leaves collapsed, and persists the choice` FAIL; restore; `git diff --stat` shows only this task's files.

- [ ] **Step 6: Commit (Claude Code)**

```bash
git add -- crates/vox-gui/ui/src/components/surfaces/Chat/ChatVerbosityControl.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatVerbosityControl.test.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.test.tsx crates/vox-gui/ui/src/hooks/useChatVerbosity.ts
git commit -m "feat(chat-trace): Quiet / Normal / Verbose sets how much of each trace opens"
```

---

### Task 8: Playwright visual verification of the trace

<!-- AMENDED: T8 (was T9) — renumbered only. -->

**Files:**
- Create: `crates/vox-gui/ui/e2e/chat-turn-trace.spec.ts`
- Outputs (gitignored): `crates/vox-gui/ui/review-bundle/latest/chat-trace-collapsed.png`, `chat-trace-expanded.png`, `chat-trace-interrupt.png`

**Interfaces:**
- Consumes: `installTauriMock` (`e2e/lib/tauriMock.ts`), `addMockInitScript` (`e2e/lib/tauriMockShared.ts`), the `chat_turn` invoke and `ChatTurnDto` shape (`id, role, content, created_at, task_id, model_id?, latency_ms?, events?`), contract examples as payloads; test ids from Tasks 4, 6, 7.
- Produces: the spec and three screenshots.

No guard is added here; Step 3 proves the spec can see a collapse bug.

- [ ] **Step 1: Write the spec.** Create `crates/vox-gui/ui/e2e/chat-turn-trace.spec.ts`:

```ts
import { test, expect, type Page } from '@playwright/test';
import { mkdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, '..', 'review-bundle', 'latest');
const CONTRACT = JSON.parse(
  readFileSync(join(HERE, '../../../../contracts/gui/turn-event-kinds.v1.json'), 'utf8'),
) as { kinds: Array<{ kind: string; example: Record<string, unknown> }> };

function example(kind: string): Record<string, unknown> {
  const entry = CONTRACT.kinds.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return entry.example;
}

const ROUTING = example('routing_decision');

/**
 * Same shape as `installTrustOverrides` in chat-trust-chips.spec.ts (Playwright does not let one
 * spec import another): answer `chat_turn` with `reply`, fall through to the base mock otherwise.
 */
async function installChatTurn(page: Page, reply: unknown): Promise<void> {
  await addMockInitScript(page, installTauriMock, 'chat');
  await page.addInitScript((chatTurn: unknown) => {
    const internals = (window as any).__TAURI_INTERNALS__;
    const base: ((cmd: string, args?: any) => Promise<unknown>) | undefined = internals?.invoke;
    if (typeof base !== 'function') {
      throw new Error('installChatTurn must run after installTauriMock');
    }
    internals.invoke = async (cmd: string, args?: any) => {
      if (cmd === 'chat_turn') {
        (window as any).__VOX_IPC_ACTIVE_COUNT__ = ((window as any).__VOX_IPC_ACTIVE_COUNT__ || 0) + 1;
        (window as any).__TAURI_CALLS__.push({ cmd, args: args ?? null });
        try {
          return chatTurn;
        } finally {
          (window as any).__VOX_IPC_ACTIVE_COUNT__ = Math.max(
            0,
            ((window as any).__VOX_IPC_ACTIVE_COUNT__ || 1) - 1,
          );
        }
      }
      return base(cmd, args);
    };
  }, reply);
}

function reply(id: number, events: unknown[]) {
  return {
    id,
    role: 'assistant',
    content: 'Checked the repository and delegated the migration.',
    created_at: '2026-09-28T12:00:00.000Z',
    task_id: null,
    model_id: ROUTING.resolved_id,
    latency_ms: 4100,
    events,
  };
}

async function sendTurn(page: Page, prompt: string): Promise<void> {
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
  const chooseModeBtn = page.getByLabel('Choose send mode');
  if (await chooseModeBtn.isVisible()) {
    if ((await chooseModeBtn.getAttribute('aria-expanded')) !== 'true') {
      await chooseModeBtn.click();
    }
    const quickChatBtn = page.getByLabel('Set send mode: Quick chat');
    if (await quickChatBtn.isVisible()) {
      await quickChatBtn.click();
    }
  }
  const composer = page.getByLabel('Task composer');
  await composer.fill(prompt);
  await composer.press('Enter');
  await expect(page.getByTestId('chat-trace-summary')).toBeVisible();
  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
}

test('a clean turn shows one collapsed trace row that expands into its steps', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await installChatTurn(
    page,
    reply(9101, [
      example('routing_decision'),
      example('tool_receipt'),
      example('delegation_spawned'),
      example('research_milestone'),
    ]),
  );
  await sendTurn(page, 'Check the repo and delegate the migration');

  const summary = page.getByTestId('chat-trace-summary');
  await expect(summary).toContainText(String(ROUTING.resolved_id));
  await expect(summary).toContainText('1 tool');
  await expect(summary).toContainText('4.1s');
  await expect(summary).toHaveAttribute('aria-expanded', 'false');
  await expect(page.getByTestId('chat-trace-steps')).toBeHidden();
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'chat-trace-collapsed.png') });

  await summary.click();
  await expect(summary).toHaveAttribute('aria-expanded', 'true');
  await expect(page.getByTestId('chat-trace-step')).toHaveCount(4);
  await expect(page.getByTestId('chat-turn-delegation-row')).toBeVisible();
  await expect(page.getByTestId('chat-turn-research-row')).toBeVisible();
  await expect(page.getByTestId('chat-turn-routing-row')).toHaveAttribute('data-resolved-from', 'catalog');
  await page.screenshot({ path: join(OUT_DIR, 'chat-trace-expanded.png') });
});

test('a fabricated claim is an inline interrupt and opens the trace', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await installChatTurn(
    page,
    reply(9102, [example('routing_decision'), example('tool_receipt'), example('receipt_claims')]),
  );
  await sendTurn(page, 'Verify the task claims');

  const claims = page.getByTestId('chat-turn-claims-row');
  await expect(claims).toHaveCount(1);
  await expect(claims).toHaveAttribute('data-flagged', 'true');
  await expect(page.getByTestId('chat-trace-summary')).toHaveAttribute('aria-expanded', 'true');
  await expect(page.getByTestId('chat-trace-steps')).toBeVisible();
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'chat-trace-interrupt.png') });
});

test('Verbose opens a clean trace and Quiet closes it, persisting the choice', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await installChatTurn(page, reply(9103, [example('routing_decision'), example('tool_receipt')]));
  await sendTurn(page, 'Check the repo status');

  await expect(page.getByTestId('chat-trace-steps')).toBeHidden();
  await page.getByRole('radio', { name: 'Verbose' }).click();
  await expect(page.getByTestId('chat-trace-steps')).toBeVisible();
  await page.getByRole('radio', { name: 'Quiet' }).click();
  await expect(page.getByTestId('chat-trace-steps')).toBeHidden();
  await expect
    .poll(() => page.evaluate(() => window.localStorage.getItem('gui.chat.verbosity.v1')))
    .toBe('"quiet"');
});
```

- [ ] **Step 2: Run**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/chat-turn-trace.spec.ts e2e/chat-trust-chips.spec.ts --project=chromium --reporter=line`
Expected: `6 passed`. Then `ls crates/vox-gui/ui/review-bundle/latest/chat-trace-*.png` lists the three screenshots. Exit 124 is a STOP.

- [ ] **Step 3: Prove the spec sees a collapse bug.** In `TurnTrace.tsx` change `hidden={!expanded}` to `hidden={false}`; run the Step 2 command for `e2e/chat-turn-trace.spec.ts` only into `target/turn-trace-t8-mutant.txt`; confirm `a clean turn shows one collapsed trace row that expands into its steps` FAILS on `toBeHidden`; restore; `git diff --stat` shows no tracked changes.

- [ ] **Step 4: Commit (Claude Code)** — Claude views the three screenshots before committing.

```bash
git add -- crates/vox-gui/ui/e2e/chat-turn-trace.spec.ts
git commit -m "test(chat-trace): Playwright captures the collapsed, expanded and interrupt trace"
```

---

### Task 9: Golden seam — one real agent turn, rendered from the same example

<!-- AMENDED: T9 (was T10) — first_diff helper replaced by assert_eq! on the two Values; pinned spec is PricingSource::OpenRouter; contract anchor updated (no engine_events). -->

**Files:**
- Modify: `contracts/gui/turn-event-kinds.v1.json` (add `golden_turn`)
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx`

**Interfaces:**
- Consumes: `try_run_agent_turn` and the Task 2b helpers `test_state`, `register_pinned`, `plain_body`, `point_openrouter_at`, `restore_openrouter`, `MODEL_ID`; the streaming fallback in `agent_loop::stream_final_answer` (an SSE request carries `Accept: text/event-stream`; an empty stream falls back to `llm_chat`); `TurnTrace` (Task 6).
- Produces: contract `golden_turn: { description, events: [routing_decision, tool_receipt] }`; Rust test `the_golden_turn_matches_the_contract`; vitest `renders the Rust-verified golden turn`.

Expected Rust output, derived from the code (the test checks this prediction; it must not be edited to match): the turn pins
`acme/widget-5.5` (priced `OpenRouter`) through the global override on `efficiency`, so the resolver's pref branch returns
it with no rationale and `SelectionSource::Global` → reason `Pinned by the global model override`; the model calls
`vox_git_status` once, which `turn_event_for_result` ignores and the receipt ledger records → one `tool_receipt`
(`fulfilled: true`, `verified: true`, as in `agent_loop::tests::tool_call_emits_a_verifiable_tool_receipt_event`).

- [ ] **Step 1: Write the failing Rust test.** In `message_turn_trace_tests.rs`, change `use wiremock::matchers::method;` to `use wiremock::matchers::{header, method};`, add `use serde_json::Value;` directly after `use std::sync::Arc;`, and append:

```rust
const CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/gui/turn-event-kinds.v1.json"
));

/// Receipt ids are server-minted UUIDv7s; the golden carries a fixed stand-in.
const GOLDEN_RECEIPT_ID: &str = "01920000-aaaa-7bbb-8ccc-000000000001";

fn tool_call_body() -> Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "vox_git_status", "arguments": "{}"},
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    })
}

/// The golden seam: what the real default chat path emits for one tool-calling turn equals the
/// contract's `golden_turn`, which the GUI renders in `TurnTrace.test.tsx`.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn the_golden_turn_matches_the_contract() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let server = MockServer::start().await;
    // Streaming attempts get an empty SSE stream (the tool-call signature), so they fall back to
    // plain requests and never consume the tool-call response below. Mounted first: wiremock
    // answers with the first matching mock.
    Mock::given(method("POST"))
        .and(header("accept", "text/event-stream"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("data: [DONE]\n\n", "text/event-stream"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_body()))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(plain_body("done, saw the tool result")))
        .mount(&server)
        .await;
    let prev = point_openrouter_at(&server.uri());
    let state = test_state();
    register_pinned(&state, MODEL_ID);

    let result = try_run_agent_turn(
        &state,
        "system prompt",
        "what's the git status?",
        "golden-session",
        None,
        false,
        None,
        None,
        None,
        None,
        Some("efficiency"),
        None,
    )
    .await;
    restore_openrouter(prev);

    let turn = result
        .expect("an OpenRouter pick runs the agent loop")
        .expect("the turn succeeds");
    let mut events = turn.events.clone();
    for ev in &mut events {
        if ev.get("receipt_id").is_some() {
            ev["receipt_id"] = Value::String(GOLDEN_RECEIPT_ID.to_string());
        }
    }
    let actual = Value::Array(events);
    let contract: Value = serde_json::from_str(CONTRACT).expect("contract is valid JSON");
    assert_eq!(
        &actual,
        &contract["golden_turn"]["events"],
        "the golden turn differs from what try_run_agent_turn emits (left: actual, right: contract)"
    );
}
```

- [ ] **Step 2: Write the failing vitest.** Append to `TurnTrace.test.tsx`:

```tsx
describe('golden turn (same example the Rust golden test checks)', () => {
  it('renders the Rust-verified golden turn: resolved model, one tool, one verified receipt', () => {
    const golden = CONTRACT.golden_turn;
    if (!golden) throw new Error('contract has no golden_turn');
    render(<TurnTrace events={golden.events} verbosity="normal" />);
    const summary = screen.getByTestId('chat-trace-summary');
    expect(summary).toHaveTextContent('acme/widget-5.5 · 1 tool · 1 receipt ✓');
    expect(summary).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getAllByTestId('chat-trace-step').map((li) => li.getAttribute('data-kind'))).toEqual([
      'routing_decision',
      'tool_receipt',
    ]);
  });
});
```

- [ ] **Step 3: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib turn_trace_tests > target/turn-trace-t9-red.txt 2>&1; timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/TurnTrace.test.tsx >> target/turn-trace-t9-red.txt 2>&1; tail -40 target/turn-trace-t9-red.txt`
Expected: the two Task 2b wiring tests pass; `the_golden_turn_matches_the_contract ... FAILED` (right side is `Null`: no `golden_turn` yet); the vitest golden case fails with `contract has no golden_turn`. Save the `left:` value from the panic: Step 4's golden must equal it.

- [ ] **Step 4: Add the golden to the contract.** In `contracts/gui/turn-event-kinds.v1.json` replace the last five lines

```json
        "reason": "Chosen by the model scorer as the best match for your request"
      }
    }
  ]
}
```

(the end of the `routing_decision` entry, the close of `kinds`, and the close of the file) with

```json
        "reason": "Chosen by the model scorer as the best match for your request"
      }
    }
  ],
  "golden_turn": {
    "description": "What try_run_agent_turn emits for one tool-calling turn: acme/widget-5.5 pinned by the global override, Efficient mode, one vox_git_status call. receipt_id is normalised by the Rust test.",
    "events": [
      {
        "kind": "routing_decision",
        "family": "acme/widget",
        "resolved_id": "acme/widget-5.5",
        "resolved_from": "catalog",
        "mode": "efficiency",
        "objective": "Most quality per dollar; no flagship while another model fits",
        "reason": "Pinned by the global model override"
      },
      {
        "kind": "tool_receipt",
        "tool": "vox_git_status",
        "receipt_id": "01920000-aaaa-7bbb-8ccc-000000000001",
        "fulfilled": true,
        "verified": true
      }
    ]
  }
}
```

If the `left:` value saved in Step 3 differs from these two events in any field other than `receipt_id`, STOP and paste both; do not edit the golden to match.

- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib -- turn_events turn_trace_tests 2>&1 | tail -15 && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat 2>&1 | tail -10`
Expected: all pass, including Task 1's contract tests (the new top-level key is ignored by them) and `turnEventContract.test.tsx`.

- [ ] **Step 6: Mutation proof.** In `message_turn_trace_tests.rs` change `const MODEL_ID: &str = "acme/widget-5.5";` to `const MODEL_ID: &str = "acme/widget-5.6";`; run `timeout 1500s cargo test -p vox-orchestrator-mcp --lib turn_trace_tests > target/turn-trace-t9-mutant.txt 2>&1`; confirm `the_golden_turn_matches_the_contract ... FAILED` with `acme/widget-5.6` on the left; restore; `git diff --stat` shows only this task's files, the contract growing by about 25 lines.

- [ ] **Step 7:** `rustfmt --edition 2024 crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs`; re-run Step 5.

- [ ] **Step 8: Commit (Claude Code)**

```bash
git add -- contracts/gui/turn-event-kinds.v1.json crates/vox-orchestrator-mcp/src/chat_tools/chat/message_turn_trace_tests.rs crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx
git commit -m "test(chat-trace): golden turn proves the engine's events are what the chat renders"
```

---

### Verification sweep — Owner: Claude

<!-- AMENDED: T6 — gui-honesty manifest/triage refresh added here (not an agent task). -->

- [ ] `timeout 1500s cargo clippy -p vox-orchestrator -p vox-orchestrator-mcp --all-targets -- -D warnings` and `timeout 1500s cargo clippy -p vox-gui --all-targets -- -D warnings` — clean for touched files.
- [ ] `timeout 600s pnpm --dir crates/vox-gui/ui exec vitest run` and `timeout 300s pnpm --dir crates/vox-gui/ui typecheck` — all green.
- [ ] Regenerate the GUI honesty manifest (it lists `ModelBadge.tsx`): `cd /Users/brbrainerd/dev/vox/crates/vox-gui/ui && node scripts/inventory.mjs` (writes `docs/agents/gui-honesty-manifest.json`), then hand-edit `docs/agents/gui-honesty-triage.md`: remove the four `ModelBadge.tsx` rows (lines ~52 and ~195–198 today) and add one row for `TurnTrace.tsx` (summary toggle button, local state, KEEP). Commit both with a message naming this plan.
- [ ] `timeout 1500s cargo run -q -p vox-cli -- ci pre-push` (fast tier); regenerate and commit any inventory it names.
- [ ] Add a row to `docs/src/architecture/where-things-live.md`: "Chat turn trace (turn events, routing decision)" → contract `contracts/gui/turn-event-kinds.v1.json`, producers `crates/vox-orchestrator-mcp/src/chat_tools/chat/{agent_loop,turn_events}.rs`, provenance `crates/vox-orchestrator/src/models/provenance.rs`, GUI `lib/turnEvents.ts` (model/mode label owner), `lib/turnTrace.ts`, `components/surfaces/Chat/TurnTrace.tsx`.
- [ ] View `review-bundle/latest/chat-trace-{collapsed,expanded,interrupt}.png`.

---

## Decisions (resolved 2026-09-28; the user delegated open decisions to Claude)

1. **`RoutingSummaryDto` location.** It lives in `crates/vox-gui/src/commands/models.rs` (Tauri command), not in
   `vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs` as the brief said; `dashboard_api.rs::get_routing_summary`
   builds untyped JSON for the HTTP dashboard and is left alone.
2. **Where `routing_decision` is built.** In `try_run_agent_turn` (`message.rs`), where the chat model is resolved:
   `run_agent_turn` only receives an already-mapped `LlmConfig`. The golden seam therefore drives `try_run_agent_turn`.
   Tests live in a `#[path]` child module (`message_turn_trace_tests.rs`) so neither the function nor
   `AgentTurnResult` is widened and `message.rs` (2146 lines) grows by three lines of declaration.
3. **`ranking` is not emitted.** `decide()` exposes `alternatives` as ids in candidate order with no scores, and the chat
   resolver discards the decision. Emitting scores would mean inventing them. The contract does not list the field.
4. **Label rule, owned by this plan** (`lib/turnEvents.ts`: `routingModelLabel`, `MODE_NAMES`, `modeLabel`):
   catalog → `<resolved_id>`; local → `<resolved_id> (local)`; bootstrap → `<family> (offline)`; a reply with no routing
   decision → its plain `modelId`, never with "(latest)". Mode names Free / Efficient / Balanced / Genius keyed by wire
   value. `2026-09-28-chat-surfaces-consolidation.md` imports these and must not define its own; its pre-flight is
   `rg -n "export function routingModelLabel|export const MODE_NAMES" crates/vox-gui/ui/src/lib/turnEvents.ts`
   (two lines). <!-- AMENDED: T4 — single label owner (cross-plan). -->
5. **`resolved_from` derives from `PricingSource`**, not `released_at`: local backends → `local`;
   `PricingSource::Bootstrap` → `bootstrap`; anything else (OpenRouter, AnthropicDirect, LiteLLM, UserConfig, Telemetry,
   Unknown) → `catalog`. Direct-provider specs are therefore never mislabelled bootstrap. <!-- AMENDED: T2 -->
6. **`reason` is a constant or a registry/contract value, never echoed input.** Every `SelectionSource` except
   `AutoRouted` maps to a constant; an auto-routed rationale is used unless it is one of the two `resolve.rs` lines that
   echo a requested id (`Fallback: requested …`, `Sticky VoxLocal: …`), which become constants. A test pins those two
   `resolve.rs` format strings so a rewording forces a review. <!-- AMENDED: T2 -->
7. **`mode`/`objective` only when the composer sent a mode.** With none, the resolver used its own default axes
   (cost-first unless a source policy applies), so the event makes no promise. A trigger-source policy override of a
   composer mode is still not reflected (Deferred). <!-- AMENDED: T2 -->
8. **Interrupts among live kinds:** only a flagged `receipt_claims`. An unverified receipt alone is a failed step. Normal
   opens the trace on a failed step or an interrupt (which keeps Phase 5's trust-chips spec passing unchanged). Interrupts
   render through `ChatTurnEventRow` with no extra `role="status"` (the transcript is already an `aria-live` log).
   <!-- AMENDED: T6 -->
9. **Steps stay in the DOM** under `hidden`, so `aria-controls` resolves and receipt rows remain countable.
10. **Model-influenced text is never rendered:** `research_milestone.query` (sentinel test + mutation proof) and
    requested/pinned model ids in `reason`.
11. **`ModelBadge` is deleted** (its only importer was `ChatTranscript`); replies without events keep their model id
    and latency in a plain summary line, and the "Assistant" header always renders. `PhaseChip` and `ChatAgentEventRow` are
    not revived: no PAV event reaches a chat turn today (see Deferred). <!-- AMENDED: T6 -->
12. **The verbosity control lives in `ChatTranscript`**, the hook's only consumer, so one hook instance drives the control
    and every trace (`useLocalStorage` does not sync between instances). `ChatSurface.tsx` (1128 lines, no header) is not
    touched.
13. **Fixture ids are fictional** (`acme/widget-5.5`), so contract, mocks and screenshots carry no versioned cloud id.
14. **Task 3 pairs a Rust DTO with its 3-line TS mirror**, an exception to "no mixed Rust/TS task": they are one seam and
    the TS side is checked by `typecheck`.
15. **The engine-events half is cut** (old Task 6, `engine_events`, the `AgentEventKind` round-trip tests, the engine
    validators and step descriptions). See Deferred. <!-- AMENDED: T6(old) -->

## Deferred (next plans)

- **Engine events in the trace** (was Task 6). Evidence, 2026-09-28: five of the eleven turn-relevant `AgentEventKind`s have
  no emitter anywhere (`rg -n "ToolTimedOut \{|CompactionTriggered \{|ContextTruncated \{|SemanticDriftDetected \{|LlmCallCompleted \{" crates`
  matches only `events.rs`); the other six (`InjectionDetected`, `ReplanTriggered`, `BudgetAlert`, `ScopeViolation`,
  `DoubtReported`, `PavPhaseChanged`) are emitted but carry no chat session id, and the synchronous chat turn has no
  `task_id`/`agentToTask` mapping, so none can reach a chat turn. Follow-up: (1) add emitters for the five; (2) add a
  `session_id` (the chat session) to the variants a chat turn can trigger, set by `run_agent_turn`'s callers; (3) then route
  them in `sessionChatStore.resolveSessionForEvent` and attach them in `chatCorrelation.ts` (the drafted design: known
  `session_id` only, never create a session; drop when no turn is in flight), with interrupts for injection, scope violation
  and `CostExceeded`, and `PhaseChip` revived for PAV steps. That plan must run after the surfaces plan, which edits
  `chatCorrelation.test.ts`.
- **`ranking` in `routing_decision`** once `decide()` returns per-candidate scores (the rail's "alternatives" disclosure in
  the surfaces plan needs the same data).
- **`routing_decision` on the other paths:** `call_llm_with_pref` fallback, `cognitive_profile` turns, and background tasks
  (`AiTaskProcessor`). Their replies show the plain `modelId` fallback today.
- **Effective clutch from the resolver** (Decision 7), and a typed rationale from `resolve.rs` so `routing_reason` stops
  matching two prefixes.
- **Per-turn cost in the summary:** not computed on the sync path (`message.rs` records `cost_usd: None`).
- **"Approval required" and "harness issue affecting this session" as trace interrupts** (the transcript's harness strip
  and the Needs-you card cover them today); **locks waited on** (Phase 5 05-05 `LockWaiting`) as trace steps.
- **Inspector drawer** reusing the research debugger's stepper for raw per-step payloads.
- **Deduplicate `installTrustOverrides` / `installChatTurn`** into `e2e/lib/`.
- **`ChatAgentEventRow`** stays orphaned; the visual-cleanup plan's orphan task deletes it.

## Execution Order

<!-- AMENDED: all — renumbered (1, 2a, 2b, 3–9); chatCorrelation.ts/sessionChatStore.ts no longer touched; surfaces-plan ordering added. -->

- **Strictly sequential: 1 → 2a → 2b → 3 → 4 → 5 → 6 → 7 → 8 → 9.** Shared files: the contract (T1, T2b, T9);
  `turn_events.rs` (T1 → T2a → T2b); `message_turn_trace_tests.rs` (T2b → T9); `turnEvents.ts` (T4 → T5, T6);
  `ChatTranscript.tsx` and its test (T6 → T7); `TurnTrace.tsx` / `TurnTrace.test.tsx` (T6 → T8 mutation step → T9). T3
  needs T2a's `provenance`. T4's contract test needs T2b's flip (no planned kinds). T8 needs T6 and T7. Drive 2a and 2b back
  to back (2a leaves a `dead_code` warning that 2b removes).
- **Other plans:** run after Phase 5 and after the model-routing and chat-lane plans (prerequisites above). This plan runs
  **before `2026-09-28-chat-surfaces-consolidation.md`**, which edits `chatCorrelation.test.ts` and
  `e2e/chat-trust-chips.spec.ts` and imports `routingModelLabel` / `MODE_NAMES` from this plan's `lib/turnEvents.ts`
  (Decision 4). The visual plan's status-colour guard will scan `Chat/*.tsx`; the files this plan adds use no raw status
  colours.
- **Pre-flight per task:** working tree clean for the task's files; HEAD on `main`; the Vite dev server on :1420 for T6/T8.
- **SDD ledger:**
  - R1 turn-event contract is one JSON file checked from Rust (real producers) and TS — ruling: settled
  - R2 `routing_decision` built in `try_run_agent_turn`, tests in a `#[path]` child module — ruling: settled
  - R3 `resolved_from` from backend + `PricingSource` — ruling: settled (amended)
  - R4 `ranking` omitted until scores exist — ruling: settled
  - R5 label rules and mode names owned by `lib/turnEvents.ts`; local label `(local)` — ruling: settled (amended, cross-plan)
  - R6 interrupt set = flagged `receipt_claims`; "Normal opens on failure or interrupt" — ruling: settled (amended)
  - R7 steps kept in the DOM with `hidden` — ruling: settled
  - R8 `ModelBadge` deleted; plain `modelId` fallback and "Assistant" header kept — ruling: settled (amended)
  - R9 verbosity control in `ChatTranscript` — ruling: settled
  - R10 `RoutingSummaryDto` is the vox-gui Tauri DTO — ruling: settled (brief path corrected)
  - R11 `reason` never echoes input; `mode`/`objective` only with a composer mode — ruling: settled (amended)
  - R12 engine-events half deferred with evidence — ruling: settled (amended)
  - R13 `first_diff` replaced by `assert_eq!` — ruling: settled (amended)
  - contract T1 → T2b → T9, `turn_events.rs` T1 → T2a → T2b, `ChatTranscript.*` T6 → T7, `TurnTrace.*` T6 → T8 → T9 — sequential — settled
