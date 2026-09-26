# Phase 3: Extract Misplaced Crates to Plugin Architecture - Context

**Gathered:** 2026-09-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Move CORE-inappropriate functionality to the plugin architecture without losing it, and remove the last unconditional Candle dependency from CORE (L0-L3) crate manifests. Requirements: REQ-dead-crate-extract-to-plugin, REQ-dead-crate-misplaced.

Scout findings (2026-09-25, verify before relying on them):
- **vox-grammar-export** is still a CORE crate (1,522 lines) with six direct consumers: vox-compiler (`compact_prompt::emit_compact_llm_prompt`), vox-constrained-gen (`ebnf::emit_ebnf`, `grammar_ir`), vox-cli (`export`, `ebnf`, `versioning::verify_grammar_alignment`), vox-cli-ci (`ssot_markdown`), vox-orchestrator-mcp (`ebnf::emit_ebnf`), vox-populi. No `GrammarExportPlugin` ABI exists. This is the main real work.
- **vox-webhook** was already extracted as `crates/vox-plugin-webhook` (has `WebhookOrchestratorBridge`, `OrchestratorInboxItem`); vox-orchestrator has no dependency on it, but nothing loads or dispatches the plugin.
- **vox-ssg** was folded into `crates/vox-cli/src/utils/ssg/` (L4 app) by `9d385a60b` (2026-05-12). No longer a CORE crate.
- **Candle in CORE:** `vox-quantize` (L0) has an unconditional `candle-core` dependency; it is reached only through optional features (`vox-populi` `mens-candle-qlora`, `vox-ml-cli` `quantize`). `vox-populi` (L2) has Candle only behind optional features. So a default CORE build already pulls no Candle.
- **vox-oratio** is now `vox-speech` (L4). `vox-gui` still enables vox-speech's deprecated `stt-candle` feature; its stated blocker (audio-ingress) was deleted in Phase 1. `vox-plugin-speech` already ships the Candle STT backend via `as_speech_to_text`.

</domain>

<decisions>
## Implementation Decisions

### Grammar-export (SC#1)
- **D-01:** Split, not wholesale move. Keep a small CORE core (grammar IR plus the compact-prompt emitter that vox-compiler and vox-constrained-gen need unconditionally) and move the emitters/export/SSOT-markdown surfaces into a new `vox-plugin-grammar-export` implementing a new `GrammarExportPlugin` ABI, dispatched through `vox-plugin-host`. Research picks the exact split line from actual consumer use; vox-compiler must never depend on plugin availability. — **Reversibility:** costly — the ABI becomes a plugin contract and consumers move to host dispatch.
- **D-02:** When the plugin is absent at runtime (e.g. `vox grammar export`, orchestrator-mcp EBNF emit), fail with a clear, actionable error naming the plugin and how to install it. No silent fallback, no bundled duplicate.

### Webhook (SC#2)
- **D-03:** Close SC#2 by wiring real host dispatch: the orchestrator side loads `vox-plugin-webhook` through `vox-plugin-host` and routes webhook events into the inbox via `WebhookOrchestratorBridge` / `OrchestratorInboxItem`, with a test. No direct crate edge from vox-orchestrator to the plugin.
- **D-04:** Opt-in activation: load the plugin and start any listener only when a webhook config section is present. No config means no plugin load and no network listener (network-facing surface, safe default).

### SSG (SC#3)
- **D-05:** Accept the existing `vox-cli/src/utils/ssg` module as satisfying SC#3's intent (it is not a CORE crate). Record the evidence (deletion commit `9d385a60b`, current location, L4 layer) and amend SC#3 wording to "vox-ssg is not a CORE crate". No plugin extraction.

### Candle in CORE (SC#4)
- **D-06:** Interpretation: default CORE builds pull no Candle (already true), and no L0-L3 crate manifest names Candle as an unconditional dependency. Optional, feature-gated Candle deps in vox-populi stay.
- **D-07:** Keep `vox-quantize` in its CORE layer (no relayer, so no new crate-edge exception). Make its `candle-core` dependency optional behind a feature inside vox-quantize itself, and have its consumers (vox-populi `mens-candle-qlora`, vox-ml-cli `quantize`) enable that feature. User explicitly declined authorizing a vox-populi -> vox-quantize upward-edge exception.
- **D-08:** Complete the oratio extraction: repoint vox-gui's Candle STT path to `vox-plugin-speech` via the plugin host (sherpa stays a direct vox-speech feature), then delete vox-speech's `stt-candle` feature and its heavy deps.

### Carried forward from earlier phases
- Verify before assuming: prove already-done criteria with command output, do not rebuild (Phase 2 D-01).
- Fail closed on trust/network boundaries (Phase 2 D-02).
- Crate-edge `exceptions` entries are user-authorized only, per edge. Never regenerate `crate-edges.allow.v1.json` edges or `fan-in-snapshot.v1.json` to admit an edge. If any gate demands a new exception, STOP and ask with the exact gate output.
- Work directly on `main`, commit by explicit pathspec, never `git add -A` or `--no-verify`; the working tree is shared with other sessions' uncommitted work.
- Derived contracts (`crate-graph.v1.json`, `crate-build-map.v1.json`) are regenerated in the same commit as the manifest change that moves them. `crate-build-map.v1.json` is regenerated with `graph crate-map --no-refresh-graph --write-summary` after writing `graphify-out/crate_audit.json` from the committed map's `compile_s` values, then re-inserting the `measured_on` key (the generator drops it).

### Claude's Discretion
- Plan split and ordering across the four criteria (grammar-export is the largest; SSG is evidence-only).
- Exact ABI method set for `GrammarExportPlugin`, following the existing `vox-plugin-api` extension pattern.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Disposition source
- `docs/src/architecture/dead-crate-fate-plan-2026-05-08.md` — the PRD: EXTRACT vox-grammar-export / vox-webhook, REWRITE-AS-PLUGIN vox-ssg, finish vox-oratio extraction.
- `docs/src/architecture/dead-crate-deep-dive-2026-05-08.md` — per-crate analysis behind the PRD.

### Plugin architecture
- `docs/src/architecture/plugin-system-redesign-2026.md` and `plugin-system-redesign-sp1-plan-2026.md` / `plugin-system-redesign-sp2-plan-2026.md` — current plugin model.
- `docs/src/architecture/plugin-registration-resolution-2026.md` — how plugins are registered and resolved.
- `crates/vox-plugin-api/src/abi.rs` — ABI root; `as_speech_to_text` (line ~104) is the extension-accessor pattern to copy for `GrammarExportPlugin`.
- `crates/vox-plugin-api/src/extensions/speech_to_text.rs` — an extension trait to model the new one on.
- `crates/vox-plugin-host/src/lib.rs` — `load_code_plugin_by_id` (line ~136) / `load_code_plugin` (line ~148).
- `crates/vox-plugin-catalog/catalog.toml` — first-party plugin catalog (new plugin entry goes here; regenerates `docs/src/reference/plugin-catalog.generated.md`).

### Existing extractions
- `crates/vox-plugin-webhook/src/webhook/bridge.rs` — `WebhookOrchestratorBridge`, `OrchestratorInboxItem`.
- `crates/vox-plugin-speech/src/audio.rs` — Candle STT behind the plugin ABI.
- `crates/vox-speech/Cargo.toml` — deprecated `stt-candle` feature and its TODO.

### Architecture contracts
- `contracts/ci/crate-layers.v1.json` — layer assignments (L0-L3 = CORE).
- `contracts/ci/crate-edges.allow.v1.json` — edge ratchet (exceptions user-authorized only).
- `docs/src/architecture/where-things-live.md` — update rows for any moved concept in the same change.
- `AGENTS.md` §Dependency Discipline, §Cryptography Policy §3 (build-toolchain invariant: no cmake/nasm/Go/perl/libclang).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox-plugin-host::load_code_plugin*`: the dispatch path both the grammar-export plugin and the webhook plugin use.
- `vox-plugin-speech`: a working example of moving a Candle-heavy backend behind the plugin ABI.

### Established Patterns
- Optional feature-gated heavy deps (`dep:` features) — e.g. vox-populi `mens-candle-qlora`, vox-ml-cli `quantize`. D-07 applies the same pattern inside vox-quantize.
- Plugin crates (`vox-plugin-*`) version independently of the workspace.

### Integration Points
- vox-compiler / vox-constrained-gen: must keep compiling against the CORE grammar core only.
- vox-cli `grammar` commands and vox-orchestrator-mcp EBNF emit: switch to host dispatch with the D-02 error on absence.
- vox-orchestrator config: new opt-in webhook section (D-04).
- vox-gui dictation path: Candle STT via vox-plugin-speech (D-08).

</code_context>

<specifics>
## Specific Ideas

No specific UI or behavioral references beyond the decisions above.

</specifics>

<deferred>
## Deferred Ideas

- Relayering vox-quantize out of CORE (declined for now because it needs a user-authorized upward-edge exception).
- Moving vox-populi's optional inference/qlora code out of populi ("strict manifest" interpretation of SC#4).

</deferred>

---

*Phase: 03-extract-misplaced-crates-to-plugin-architecture*
*Context gathered: 2026-09-25*
