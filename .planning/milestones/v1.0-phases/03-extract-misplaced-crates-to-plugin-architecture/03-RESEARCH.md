# Phase 3: Extract Misplaced Crates to Plugin Architecture - Research

**Researched:** 2026-09-25
**Domain:** Rust workspace architecture — plugin ABI extraction (abi_stable dylib boundary), Cargo feature-gating of heavy ML deps
**Confidence:** MEDIUM — grammar-export split line, SSG evidence, and Candle-in-vox-quantize are HIGH (read source directly); webhook host↔plugin event dispatch mechanics are LOW/an open architecture question (no existing precedent in this codebase); a second, previously-undocumented Candle-in-CORE crate was found and is HIGH confidence but expands D-06/D-07's stated scope.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Grammar-export (SC#1)**
- **D-01:** Split, not wholesale move. Keep a small CORE core (grammar IR plus the compact-prompt emitter that vox-compiler and vox-constrained-gen need unconditionally) and move the emitters/export/SSOT-markdown surfaces into a new `vox-plugin-grammar-export` implementing a new `GrammarExportPlugin` ABI, dispatched through `vox-plugin-host`. Research picks the exact split line from actual consumer use; vox-compiler must never depend on plugin availability. — **Reversibility:** costly — the ABI becomes a plugin contract and consumers move to host dispatch.
- **D-02:** When the plugin is absent at runtime (e.g. `vox grammar export`, orchestrator-mcp EBNF emit), fail with a clear, actionable error naming the plugin and how to install it. No silent fallback, no bundled duplicate.

**Webhook (SC#2)**
- **D-03:** Close SC#2 by wiring real host dispatch: the orchestrator side loads `vox-plugin-webhook` through `vox-plugin-host` and routes webhook events into the inbox via `WebhookOrchestratorBridge` / `OrchestratorInboxItem`, with a test. No direct crate edge from vox-orchestrator to the plugin.
- **D-04:** Opt-in activation: load the plugin and start any listener only when a webhook config section is present. No config means no plugin load and no network listener (network-facing surface, safe default).

**SSG (SC#3)**
- **D-05:** Accept the existing `vox-cli/src/utils/ssg` module as satisfying SC#3's intent (it is not a CORE crate). Record the evidence (deletion commit `9d385a60b`, current location, L4 layer) and amend SC#3 wording to "vox-ssg is not a CORE crate". No plugin extraction.

**Candle in CORE (SC#4)**
- **D-06:** Interpretation: default CORE builds pull no Candle (already true), and no L0-L3 crate manifest names Candle as an unconditional dependency. Optional, feature-gated Candle deps in vox-populi stay.
- **D-07:** Keep `vox-quantize` in its CORE layer (no relayer, so no new crate-edge exception). Make its `candle-core` dependency optional behind a feature inside vox-quantize itself, and have its consumers (vox-populi `mens-candle-qlora`, vox-ml-cli `quantize`) enable that feature. User explicitly declined authorizing a vox-populi -> vox-quantize upward-edge exception.
- **D-08:** Complete the oratio extraction: repoint vox-gui's Candle STT path to `vox-plugin-speech` via the plugin host (sherpa stays a direct vox-speech feature), then delete vox-speech's `stt-candle` feature and its heavy deps.

**Carried forward from earlier phases**
- Verify before assuming: prove already-done criteria with command output, do not rebuild (Phase 2 D-01).
- Fail closed on trust/network boundaries (Phase 2 D-02).
- Crate-edge `exceptions` entries are user-authorized only, per edge. Never regenerate `crate-edges.allow.v1.json` edges or `fan-in-snapshot.v1.json` to admit an edge. If any gate demands a new exception, STOP and ask with the exact gate output.
- Work directly on `main`, commit by explicit pathspec, never `git add -A` or `--no-verify`; the working tree is shared with other sessions' uncommitted work.
- Derived contracts (`crate-graph.v1.json`, `crate-build-map.v1.json`) are regenerated in the same commit as the manifest change that moves them. `crate-build-map.v1.json` is regenerated with `graph crate-map --no-refresh-graph --write-summary` after writing `graphify-out/crate_audit.json` from the committed map's `compile_s` values, then re-inserting the `measured_on` key (the generator drops it).

### Claude's Discretion
- Plan split and ordering across the four criteria (grammar-export is the largest; SSG is evidence-only).
- Exact ABI method set for `GrammarExportPlugin`, following the existing `vox-plugin-api` extension pattern.

### Deferred Ideas (OUT OF SCOPE)
- Relayering vox-quantize out of CORE (declined for now because it needs a user-authorized upward-edge exception).
- Moving vox-populi's optional inference/qlora code out of populi ("strict manifest" interpretation of SC#4).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-dead-crate-extract-to-plugin | EXTRACT vox-grammar-export → `vox-plugin-grammar-export` (`GrammarExportPlugin` ABI, dispatched from vox-constrained-gen when available); EXTRACT vox-webhook → `vox-plugin-webhook` (already done as a crate; dispatch-via-plugin call still missing) | §Grammar-export split, §Plugin ABI mechanics, §Webhook dispatch |
| REQ-dead-crate-misplaced | REWRITE-AS-PLUGIN vox-ssg (already satisfied — evidence only); complete vox-oratio extraction (removes last direct Candle bleed) | §SSG evidence, §stt-candle retirement |
</phase_requirements>

## Summary

Two of the four sub-criteria (SC#2 webhook, SC#3 SSG) are already substantially or fully done at the crate/artifact level — the remaining work is orchestrator-side wiring (webhook) or pure evidence recording (SSG). SC#1 (grammar-export) is the real engineering effort and the scout's "six direct consumers" undercounts reality: **`vox-constrained-gen` calls `vox_grammar_export::ebnf::emit_ebnf()` directly and unconditionally** (not just `grammar_ir` types) via two public, always-compiled constructors (`EarleySampler::from_vox_grammar`, `PdaSampler::from_vox_grammar`), so the CORE-remaining "core" of vox-grammar-export must include the **EBNF emitter itself**, not just IR types and the compact-prompt emitter as D-01's context implied. `versioning.rs` also depends only on `ebnf::emit_ebnf()` and should stay CORE. Everything format-specific (`gbnf`, `json_schema`, `lark`, `ssot_markdown`, `x_grammar_2`, `automaton`, and the `export()` dispatcher) moves to the new plugin.

For SC#4, `vox-quantize`'s unconditional `candle-core` dependency is real and D-07's fix (optional feature, cfg-gated module tree, consumers opt in) is sound. Research also found a **layer-classification discrepancy** for `vox-plugin-mens-candle-core` (unconditional `candle-core`/`candle-nn`, both non-optional, verified): the human-authored `docs/src/architecture/layers.toml` pins it to **layer 3** with an explicit comment ("depends on vox-tensor/vox-db (L3), so must sit at L3 too" — i.e. CORE), but the CI-enforced `contracts/ci/crate-layers.v1.json` — the file CONTEXT.md's own canonical_refs cites as *the* layer-assignment source ("L0-L3 = CORE") — lists it as **layer 4** (not CORE) `[VERIFIED: docs/src/architecture/layers.toml:215 vs contracts/ci/crate-layers.v1.json:76]`. Which file governs SC4's "CORE (L0-L3)" gate determines whether this is a violation at all: by the CI contract this crate is out of scope for SC4; by the design doc it is in scope and unaddressed by D-06/D-07 (which only name vox-quantize). Either way, the two files disagree about a load-bearing classification and that disagreement should be resolved (or at minimum acknowledged) before this phase closes SC4 — see Open Questions.

The webhook side has real infrastructure already: `vox-plugin-webhook` is fully registered in `catalog.toml` and has a `Plugin.toml` (bundled in `vox-server`/`vox-dev` distributions), implements `VoxPlugin` + `as_http_listener()`, and internally already has `WebhookOrchestratorBridge`/`OrchestratorInboxItem`/`WebhookEventSink` — but these types live in a **private** (`mod webhook;`, not `pub mod webhook;`) module, and the plugin loader (`vox-plugin-host::Loader::load`) **hardcodes `DefaultVoxHost::new()`** with no way for a caller to inject a custom host or receive a callback. There is currently no ABI-stable path for the plugin to push events back into the orchestrator process. This is a genuine architecture gap, not a wiring oversight — see §Webhook dispatch for three concrete resolution options.

For D-08, `vox-gui` already has a **working, tested precedent** for exactly this pattern: `crates/vox-gui/src/commands/oratio.rs` already dispatches through `vox_plugin_host::cached_code_plugin("oratio")` → `.as_speech_to_text()` → `.transcribe_path()`. The remaining gap is `crates/vox-gui/src/commands/mic.rs`, which still calls `vox_speech::transcribe_path_detailed` (in-process Candle) directly for the live mic-button path — a 517-line, carefully-commented file with config-resolution-timing subtleties that must be preserved.

**Primary recommendation:** Sequence work as (1) SSG evidence-only doc update, (2) Candle/vox-quantize feature-gate (isolated, low-risk, no crate-edge changes), (3) surface the vox-plugin-mens-candle-core finding to the user as a checkpoint before deciding whether it's in scope, (4) webhook orchestrator wiring (needs an architecture decision on the host↔plugin callback, recommend the "poll_events + JSON boundary + orchestrator re-derives routing" option below), (5) grammar-export split (largest, do last, 2+ plans), (6) stt-candle retirement (repoint mic.rs to the oratio.rs pattern, then delete the feature).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Grammar IR + canonical EBNF emission | API/Backend (CORE, L1) | — | vox-compiler (L2) and vox-constrained-gen (L2) call it unconditionally at runtime; must never depend on plugin availability |
| Grammar format export (GBNF/JSON-Schema/Lark/SSOT-md/XGrammar-2) | Plugin (L4, `vox-plugin-grammar-export`) | API/Backend dispatch (`vox-plugin-host`, L3) | CLI/CI-only surfaces, not needed by the compiler/runtime hot path |
| Webhook HTTP ingress | Plugin (L4, `vox-plugin-webhook`) | Orchestrator intake (L3, `hopper`) | network listener is opt-in infra; classification/queueing belongs to the orchestrator's existing `HopperIntake` |
| Candle quantization engine | API/Backend (CORE, L0, `vox-quantize`) | — | stays CORE per D-07; only the *dependency* becomes optional, not the layer |
| Candle QLoRA training/inference | Disputed — `layers.toml` says CORE (L3); `crate-layers.v1.json` says Plugin (L4) for `vox-plugin-mens-candle-core` | Plugin (L4, cuda/metal plugins, undisputed) | flagged gap: unconditional Candle dep either way; whether it's a CORE violation depends on which layer file is authoritative (see Open Question 1) |
| Desktop dictation (mic capture → transcript) | Browser/Client (Tauri command, `vox-gui`, L4) | Plugin (`vox-plugin-speech` via `vox-plugin-host`, L4) | capture stays in-process (cpal); transcription dispatches to the plugin, matching the already-working `oratio.rs` command |
| Static site generation | CLI/App (L4, `vox-cli/src/utils/ssg`) | — | already correctly placed; evidence-only for this phase |

## Standard Stack

No new external dependencies. This phase is an internal architecture move using tooling the workspace already has:

| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|---------------|
| `abi_stable` | workspace-pinned (already used by every `vox-plugin-*` crate) | stable-ABI dylib trait objects (`#[sabi_trait]`, `StableAbi`) | existing plugin ABI mechanism; do not hand-roll FFI |
| `vox-plugin-host` (in-tree) | current | plugin discovery/loading (`Loader::load`, `load_code_plugin_by_id`, `cached_code_plugin`) | the one dispatch path all plugin consumers use |
| `vox-plugin-api` (in-tree) | current, `VOX_PLUGIN_ABI_VERSION = 12` | ABI root (`VoxPlugin`, `VoxHost`) + extension traits | adding `GrammarExportPlugin` is additive (bump `VOX_PLUGIN_ABI_VERSION` only, not `MIN_SUPPORTED`) |

No package-legitimacy audit applies — no external packages are being added.

## Package Legitimacy Audit

Not applicable. This phase adds zero external (crates.io) dependencies; it only restructures in-tree crates and Cargo feature flags.

## Grammar-Export Split (D-01) — Verified Consumer Map

**The scout's "six consumers" list in CONTEXT.md is incomplete.** Full `vox_grammar_export::` call-site audit `[VERIFIED: crates/**/*.rs, ripgrep this session]`:

| Consumer | Layer | Uses | Split destination |
|----------|-------|------|--------------------|
| `vox-compiler` (`src/llm_prompt.rs:9`) | L2 (CORE) | `compact_prompt::emit_compact_llm_prompt()` | **CORE core** |
| `vox-constrained-gen` (`src/earley.rs:13,196,268`, `src/pda.rs:14,242`) | L2 (CORE) | `grammar_ir::{Grammar, Symbol}` **and** `ebnf::emit_ebnf()` — via public, unconditional `EarleySampler::from_vox_grammar()` / `PdaSampler::from_vox_grammar()` | **CORE core** — `ebnf` module must stay, this was missed by the scout's phrasing |
| `vox-orchestrator-mcp` (`src/grammar_tools.rs:11`) | L4 | `ebnf::emit_ebnf()` only | CORE core (no plugin dispatch needed — same function CORE keeps) |
| `vox-cli` (`src/commands/grammar.rs:5,38`) | L4 | `export()`, `GrammarExportConfig`, `GrammarFormat` — the full multi-format dispatcher, i.e. the actual `vox grammar export` command | **Plugin** — this is D-02's named example of "plugin absent" error handling |
| `vox-cli/src/commands/ci/run_body_helpers/grammar.rs:12,61,98` | L4 | `ebnf::emit_ebnf()` (CORE), `export` + all format variants (plugin), `versioning::verify_grammar_alignment()` (CORE — see below) | **split** — `run_grammar_export_check` (a CI gate, "Wave 1 Task 45") calls `export()` across `[Ebnf, Gbnf, Lark, JsonSchema]` and must be rewritten to dispatch through the plugin |
| `vox-cli-ci` (`src/grammar_ssot_parity.rs:4`) | L3 (CORE by layer, but consumes a plugin-only surface) | `ssot_markdown` | **Plugin** |
| `vox-populi` (`Cargo.toml:107`) | L2 (CORE) | **dependency declared, zero actual usage found** `[VERIFIED: grep crates/vox-populi/ found no vox_grammar_export:: reference]` | Drop the dependency entirely — free cleanup, not part of the acceptance criteria but zero-risk |
| `vox-plugin-mens-candle-cuda` / `-metal` (`src/inference.rs:3`, comment only) | L4 (plugin) | Comment string only ("Grammar-constrained generation … is not pulled …"), no real import | No change needed |

**Split line (research recommendation, confirms/refines D-01):**
- **CORE core** (stays as a shrunk `vox-grammar-export`, or could be renamed — Claude's discretion per CONTEXT.md): `grammar_ir.rs` (253 lines), `ebnf.rs` (210 lines), `compact_prompt.rs` (232 lines), `versioning.rs` (38 lines, depends only on `ebnf::emit_ebnf`), plus the top-level types in `lib.rs` needed by both sides (`GrammarFormat`, `GrammarExportConfig`, `GrammarExportResult`, `grammar_version_matches_compiler`).
- **`vox-plugin-grammar-export` (new L4 plugin crate):** `gbnf.rs` (33 lines — currently just returns a deprecation error, keep as-is), `json_schema.rs` (306 lines), `lark.rs` (188 lines), `ssot_markdown.rs` (46 lines), `x_grammar_2.rs` (20 lines), `automaton.rs` (65 lines — a `JsonBraceDepthTracker` helper used by json_schema, no grammar_ir/ebnf dependency `[VERIFIED: crates/vox-grammar-export/src/automaton.rs:1-15]`), and the `export()` dispatch function (calls into the CORE crate for `ebnf`/`grammar_ir`, which is a normal downward L4→L1 dependency, no exception needed).

**Pre-existing risk found in the CI gate that must move:** `run_grammar_export_check` (`crates/vox-cli/src/commands/ci/run_body_helpers/grammar.rs:60-114`) calls `export(&config)?` for `GrammarFormat::Gbnf`, and `lib.rs`'s `export()` match arm for `Gbnf` unconditionally `return Err(...)` (CVE-2026-2069 deprecation). The `?` on line 77 would propagate that `Err` and fail the whole gate. **Verify at planning/execution time whether this gate is currently green** (`cargo run -q -p vox-cli -- ci grammar-export-check`) before assuming it passes today — if it's already red, that's out of scope for this phase's delta; if the split's rewrite accidentally fixes or breaks it, note which.

## Plugin ABI Mechanics (verified)

- **Root ABI:** `crates/vox-plugin-api/src/abi.rs` — `VoxPluginRoot` (raw C-repr struct with `abi_version`, `manifest_json`, `init`), `#[sabi_trait] trait VoxPlugin` with `id()`/`shutdown()` plus 12 optional `as_X() -> ROption<X_TO<..>>` accessors, each defaulting to `RNone`. Adding `GrammarExportPlugin` means: (1) new file `crates/vox-plugin-api/src/extensions/grammar_export.rs` with `#[sabi_trait] pub trait GrammarExportPlugin` (methods mirroring `export()`'s format dispatch, e.g. `fn export(&self, format: RStr<'_>, config_json: RStr<'_>) -> RResult<RString, RBoxError>`), (2) a new `as_grammar_export()` accessor on `VoxPlugin` in `abi.rs` defaulting to `RNone`, (3) register the module in `extensions/mod.rs`. This is purely additive — per `crates/vox-plugin-api/src/lib.rs:13-27`, additive ABI changes bump `VOX_PLUGIN_ABI_VERSION` (currently 12) only, not `VOX_PLUGIN_ABI_MIN_SUPPORTED` — old plugin binaries stay loadable.
- **Loading:** `vox-plugin-host::Loader::load(plugin_id, version, dylib_path)` (`crates/vox-plugin-host/src/loader.rs:32-70`) is the single load path: `VoxPluginRootRef::load_from_file`, ABI-compat check (`abi_compatible`), **hardcodes `let host = DefaultVoxHost::new();`** (line 59), calls `init(host_to)`. `load_code_plugin_by_id`/`load_code_plugin` (`crates/vox-plugin-host/src/lib.rs:136-230`) wrap discovery + `Loader::load` and produce an **actionable, already-implemented "not installed" error** (`format_install_hint`, line ~156) — this is the exact mechanism D-02 wants; no new error-formatting code is needed, just route the grammar CLI/CI call sites through this instead of a direct `vox_grammar_export::export()` call. `vox_plugin_host::cached_code_plugin(id)` (used by `oratio.rs`) exists as a memoized wrapper — reuse it for the new grammar plugin too.
- **Catalog/manifest:** `crates/vox-plugin-catalog/catalog.toml` `[[plugin]]` entries are free-text (`extension-points: Vec<String>` in `vox-plugin-types::plugin_manifest.rs:94`, no closed enum) — a new `id = "grammar-export"` entry with `extension-points = ["GrammarExportPlugin"]` is all that's needed; regenerates `docs/src/reference/plugin-catalog.generated.md` via the existing generator (`vox ci generate-plugin-catalog-docs` per AGENTS.md). Each plugin crate additionally needs its own `Plugin.toml` (see `crates/vox-plugin-webhook/Plugin.toml` / `crates/vox-plugin-speech/Plugin.toml` for the exact shape: `[plugin]` id/name/version/description/license, `[plugin.host] min-vox-version`, `[plugin.payload] kind = "code"`, `abi-version = 12`, `[plugin.payload.provides] extension-points = [...]`, `[plugin.payload.artifacts]` per-triple dylib filenames).
- **`vox ci plugin-abi-parity`** (`crates/vox-cli-ci/src/plugin_abi_parity.rs`) walks every `Plugin.toml` under `crates/` with a code/composite payload and **actually loads it** via `Loader::load`, skipping only test fixtures, `noop-bad-*` ids, and triples with no declared artifact. The new plugin crate must actually build as a `cdylib` and be discoverable for this gate to pass — plan for a build step before running it.
- **FFI-safe types:** all cross-boundary data is `abi_stable::std_types` (`RString`, `RStr`, `RResult`, `ROption`, `RSlice`) — strings and byte slices are fine (grammar text output is a `String`/`RString`), matching the existing `SpeechToText::transcribe_path` pattern (`RStr` in, `RResult<RString, RBoxError>` out, JSON-encoded structured payloads as strings).

## Webhook Dispatch (D-03/D-04) — verified state + open design question

**Already done:**
- `vox-plugin-webhook` (`crates/vox-plugin-webhook/Cargo.toml`) is `crate-type = ["cdylib", "rlib"]`, fully registered in `catalog.toml` (`id = "webhook"`, `extension-points = ["HttpListener"]`, `bundled-in = ["vox-server", "vox-dev"]`) and has its own `Plugin.toml` — **no catalog/distribution work needed**.
- `crates/vox-plugin-webhook/src/lib.rs` implements `VoxPlugin` (`id`, `shutdown`) and `as_http_listener()` returning a working `HttpListener` (`start_listening(config_json)` / `stop_listening()`), plus a legacy `init()`-time auto-spawn using `VOX_WEBHOOK_ADDR`/`VOX_WEBHOOK_INGRESS_TOKEN` env vars.
- `crates/vox-plugin-webhook/src/webhook/bridge.rs` already has `OrchestratorInboxItem` (with `InboxItemKind` classification: GitHub push→`GitPush`, PR→`PullRequest`, Discord/Slack→`ChannelMessage`, else→`ExternalEvent`; GitLab routes but logs a deprecation warning) and `WebhookOrchestratorBridge` (`new`/`spawn`/`run`, drains a `broadcast::Receiver<WebhookEvent>` into an `Arc<dyn WebhookEventSink>`), with solid existing unit tests.
- `vox-orchestrator` already has the landing spot: `crates/vox-orchestrator/src/hopper/types.rs:24-34` defines `IntakeSource::Webhook` `[VERIFIED: crates/vox-orchestrator/src/hopper/types.rs:22-34, quoted]`:
  ```rust
  pub enum IntakeSource {
      Developer,
      Agent,
      Webhook,
      Mesh { node_id: String },
  }
  ```
  and `HopperIntake::submit(intent: String, affinity_hints: Vec<String>, priority_hint: PriorityHint, source: IntakeSource, session_id: Option<String>) -> IntakeItem` (`crates/vox-orchestrator/src/hopper/store.rs:59-66`) is the plain-Rust, already-async, already-tested "inbox" entry point (`sqlite_store.rs`/`store.rs` both implement `HopperIntake`; `store.rs:644` already has a `IntakeSource::Webhook` test call). **All of the orchestrator-side plumbing to accept a webhook-originated task already exists — the only missing piece is getting a decoded webhook event from the plugin process into a `submit()` call.**

**The genuine gap — no host↔plugin event-push mechanism exists today:**
- `webhook::bridge`/`webhook::handler`/`webhook::sink` are inside a **private** module (`crates/vox-plugin-webhook/src/lib.rs:28`: `mod webhook;`, not `pub mod webhook;`) `[VERIFIED: crates/vox-plugin-webhook/src/lib.rs:28]` — as written today, nothing outside the plugin crate can reference `WebhookOrchestratorBridge`/`OrchestratorInboxItem` at all, publicly or privately, without either making the module public or relocating the types.
- Even if made `pub`, **D-03 explicitly forbids a direct crate edge from vox-orchestrator to the plugin** (and `vox-orchestrator` is L3/CORE while `vox-plugin-webhook` is L4 — a direct dependency would be an upward edge requiring a user-authorized `crate-edges` exception, which this phase's carried-forward rule says to STOP and ask about, not add unilaterally).
- `Loader::load` (`crates/vox-plugin-host/src/loader.rs:59`) **hardcodes `DefaultVoxHost::new()`** — there is no parameter or override point for a caller-supplied host today.
- `VoxHost` (`crates/vox-plugin-api/src/host.rs:29-34`) exposes only `data_dir()`, `log()`, `telemetry_event()` — **no callback for a plugin to push arbitrary events back to its host.** `DefaultVoxHost` (`crates/vox-plugin-host/src/host_impl.rs`) is a single generic implementation shared by every plugin type; it is not webhook-specific.
- The `HttpListener` extension (`crates/vox-plugin-api/src/extensions/http_listener.rs`) only starts/stops a listener via a JSON config string — no sink parameter.

Concretely, **as the ABI stands today there is no way for `vox-orchestrator` to receive webhook events from `vox-plugin-webhook` without adding new plumbing.** Three options, in order of recommendation:

1. **(Recommended) New "poll" extension + JSON boundary, re-derive routing in-orchestrator.** Add a small extension (either a new `#[sabi_trait]` trait, e.g. `WebhookInbox` with `fn poll_events(&self) -> RVec<RString>` JSON-serialized events, or extend `HttpListener` with a drain method) that the orchestrator calls on a tokio interval after loading the plugin via `load_code_plugin_by_id("webhook")`. The orchestrator deserializes each JSON event and re-implements the ~15-line `OrchestratorInboxItem::from_webhook` routing match itself (source+event_type → kind), then calls `HopperIntake::submit(..., IntakeSource::Webhook, ...)`. This satisfies "no direct crate edge" (orchestrator only depends on `vox-plugin-api`+`vox-plugin-host`, both CORE), requires **no change to `VoxHost` or `Loader`**, and the duplicated routing logic is <50 lines — explicitly sanctioned by AGENTS.md's Dependency Discipline rule 3 (defactor policy) with a `// vox:defactored-from vox-plugin-webhook <date>` comment. Test: unit-test the JSON→`IntakeItem` mapping directly (no network — feed synthetic JSON payloads), plus keep the plugin's own existing bridge tests as-is (internal to the plugin, still exercise the classification logic once).
2. **Extend `Loader::load` to accept a caller-supplied `VoxHost`, and extend `VoxHost` with a generic event-submission callback.** Add `Loader::load_with_host(..., host: VoxHost_TO<'static, RBox<()>>)`, keep `load()` as a thin wrapper over `DefaultVoxHost` (zero behavior change for the 5 existing call sites of `load_code_plugin`/`load_code_plugin_by_id`). Add one ABI-stable method to `VoxHost` (e.g. `submit_intake_event(kind: RStr, payload_json: RStr) -> RResult<(), RBoxError>`) implemented by an orchestrator-specific host wrapping `DefaultVoxHost`. More invasive (touches the shared `VoxHost` trait used by every plugin), but keeps the push model live (webhook dispatched as soon as it arrives, no polling latency).
3. **Relocate `WebhookEvent`/`WebhookEventSink`/`WebhookOrchestratorBridge`/`OrchestratorInboxItem` into `vox-plugin-api` (or a new tiny `-types` crate) so both sides depend downward on it.** Most literal match to D-03's wording (both sides use the *same* named types), consistent with AGENTS.md's "-types/-core crate" pattern — but does not solve the underlying ABI-boundary problem of getting a live `tokio::sync::broadcast::Receiver<WebhookEvent>` from inside the dylib into the host process; still needs one of options 1/2 for the actual data path. Treat this as a *possible refinement* of option 1/2's naming, not a replacement.

**D-04 (opt-in) integration point:** `crates/vox-orchestrator/src/config/` (`impl_env.rs`, `defaults.rs`, `impl_validate.rs`) is the existing config module family; a new optional `[webhook]` (or similar) section following the same `Option<T>` presence-gates-load pattern already used elsewhere in this config tree is the natural fit — load the plugin (via whichever option above) only when that section is `Some`.

**No network-touching test needed:** all of the routing/classification logic (`InboxItemKind` mapping) and the JSON↔`IntakeItem` boundary can be tested with synthetic in-memory payloads — this is exactly how the plugin's own `bridge.rs` tests already work (`CollectingSink`, `FailingSink` — no real HTTP).

## SSG Evidence (D-05)

Confirmed `[VERIFIED: git show --stat 9d385a60b]`: commit `9d385a60b` ("chore: Consolidate workspace crates and align dependencies", 2026-05-12) shows `crates/vox-ssg/Cargo.toml | 14 -` (deleted) and a rename `.../src/lib.rs => vox-cli/src/utils/ssg/mod.rs`. Today: `crates/vox-ssg/` does not exist; `crates/vox-cli/src/utils/ssg/mod.rs` exists; `contracts/ci/crate-layers.v1.json` has no `vox-ssg` entry (grep exit 1). `vox-cli` is layer 4. **D-05's acceptance is fully satisfied by existing state — this task is pure documentation.**

**Stale doc found:** `docs/src/architecture/where-things-live.md:427`, in a "### Misc — Planned crate" table, still lists `vox-ssg | Static site generator for Vox docs surface.` as if unimplemented. Per AGENTS.md's own rule ("update rows for any moved concept in the same change"), this row should be corrected or removed (compare the existing `vox-dashboard` row's "**Retired**... Use X instead" pattern for the phrasing to copy) as part of this phase's SC#3 wording amendment.

## Candle in CORE (D-06/D-07/D-08)

### vox-quantize (in scope, locked)

`crates/vox-quantize/Cargo.toml:9`: `candle-core = { workspace = true }` — **unconditional**, `optional` not set. Layer 0 (deepest CORE tier) `[VERIFIED: contracts/ci/crate-layers.v1.json:93, "vox-quantize": 0]`. Candle usage is pervasive: 8 of 9 source files reference candle (`policy.rs` 15, `write.rs` 16, `recombine.rs` 37, `read.rs` 24, `verify.rs` 7, `engine.rs` 8, `device.rs` 3, `error.rs` 2; only `lib.rs` has 0). Consumers: `vox-populi` (`mens-candle-qlora` feature gates `dep:vox-quantize`) and `vox-ml-cli` (`quantize` feature gates `dep:vox-quantize`) — both already `optional = true` on their *own* dependency edge to vox-quantize; the manifest-level violation is purely inside vox-quantize's own `Cargo.toml`.

**D-07 fix (straightforward):** add `candle-core = { workspace = true, optional = true }`, add a feature (name it e.g. `engine` or `quantize`) that does `["dep:candle-core"]`, cfg-gate essentially the whole module tree (`mod policy; mod device; mod write; mod recombine; mod read; mod verify; mod engine;` — everything except perhaps `error.rs`'s non-candle variants) behind that feature, and have `vox-populi`'s `mens-candle-qlora` and `vox-ml-cli`'s `quantize` feature blocks add `features = ["<name>"]` to their `vox-quantize = { workspace = true, optional = true, ... }` lines. No crate-edge changes, no relayer, matches the `cuda`/`metal` sub-feature pattern already present in the same file.

### vox-plugin-mens-candle-core — layer-classification discrepancy `[VERIFIED — new finding, not in CONTEXT.md]`

`crates/vox-plugin-mens-candle-core/Cargo.toml:21-22`: `candle-core = { workspace = true }` and `candle-nn = { workspace = true }` — **both unconditional**, confirmed via a workspace-wide scan (`grep -E '^candle-(core|nn|transformers) *=' crates/*/Cargo.toml` filtered for lines without `optional`) which found exactly five hits: `vox-ml-cli`, `vox-plugin-mens-candle-core`, `vox-plugin-mens-candle-cuda`, `vox-plugin-mens-candle-metal`, `vox-quantize`.

Its **layer classification disagrees between the two files this project treats as authoritative**:
- `docs/src/architecture/layers.toml:215`: `vox-plugin-mens-candle-core = { layer = 3, ... }` — comment: *"device-agnostic Candle QLoRA code shared by the two L4 plugins and vox-populi (model_card/manifest); depends on vox-tensor/vox-db (L3), so must sit at L3 too"* (i.e., CORE).
- `contracts/ci/crate-layers.v1.json:76`: `"vox-plugin-mens-candle-core": 4` (i.e., **not** CORE) — this is the file CONTEXT.md's own canonical_refs names as *the* CORE/L0-L3 definition for this phase.

Every other crate in that 5-hit scan is verified L4 in `crate-layers.v1.json` (`vox-ml-cli`: 4, `vox-plugin-mens-candle-cuda`: 4, `vox-plugin-mens-candle-metal`: 4) except `vox-quantize` (L0, the one D-07 already covers). So **by the CI-enforced contract, vox-quantize is the only L0-L3 crate with an unconditional Candle dependency** and D-06/D-07 fully close SC4 as literally written. **By the design doc (`layers.toml`), a second CORE crate remains unaddressed.** Its consumers (`vox-plugin-mens-candle-cuda`/`-metal`) depend on it as a plain, non-optional path dependency either way.

For context only (not needed to resolve the scope question, but relevant if the discrepancy is resolved toward "CORE"): Candle usage inside `vox-plugin-mens-candle-core` is **partial, not pervasive** — of 37 source files, several are 0% candle (`hf_keymap.rs` 399 lines/0, `checkpoint_state.rs` 236 lines/0, `oom.rs` 149 lines/0, `adapter_schema_v3.rs` 92 lines/0), several are candle-light (`qlora_preflight.rs` 720 lines/1 ref), and a few are candle-heavy (`manifest.rs` 374 lines/42 refs, `merge.rs` 584 lines/10 refs). Feature-gating this crate would be a materially larger and more invasive refactor than vox-quantize's near-total gate, since most of its logic doesn't touch candle at all.

Flagging this discrepancy rather than resolving it unilaterally — see Open Questions; this needs a decision (which file governs, and whether the gap is in scope) before planning commits to a task list.

### vox-oratio / stt-candle (D-08)

`crates/vox-speech` and `crates/vox-gui` are **layer 4**, not CORE (L0-L3) `[VERIFIED: contracts/ci/crate-layers.v1.json:111, "vox-speech": 4; vox-gui: 4]` — REQUIREMENTS.md's phrase "removes the last direct Candle dependency bleed from CORE" is imprecise by the phase's own CORE definition; treat D-08 as an independent app-tier cleanup (binary-size/compile-time hygiene for the shipped GUI binary), separate from the SC4 CORE-manifest check.

`crates/vox-speech/Cargo.toml:26-36`: `stt-candle` feature gates `dep:candle-core`, `dep:candle-nn`, `dep:candle-transformers`, `dep:hf-hub`, `dep:tokenizers`, `dep:symphonia`, `dep:rubato`, `dep:rand09`, `dep:byteorder` — already fully optional at the vox-speech level, with an explicit `# TODO(extraction-unit4-followup): drop stt-candle after audio-ingress rewire/retirement.` comment. `crates/vox-gui/Cargo.toml` requests `vox-speech = { ..., features = ["stt-candle", "stt-sherpa", "compiler-rerank"] }` unconditionally — this is the actual "bleed" (a shipped GUI binary statically linking Candle).

**A working precedent already exists in-tree:** `crates/vox-gui/src/commands/oratio.rs` (146 lines, `#[cfg(feature = "oratio")]`) already dispatches through the plugin: `vox_plugin_host::cached_code_plugin("oratio")` → `.plugin.as_speech_to_text()` → `.transcribe_path(path, config_json)` → parses the returned JSON's `"text"` field → `vox_speech::refine_raw_text(...)`. `.txt`/`.md` fixtures still resolve via `vox_speech::transcribe_path_detailed` directly (deterministic, no model, explicitly kept for fixture-based unit tests).

**The gap:** `crates/vox-gui/src/commands/mic.rs` (517 lines, NOT feature-gated — always compiled) still calls `vox_speech::transcribe_path_detailed(path, &ctx, None)` directly for the "live mic-button path" (`transcribe_audio_file`, lines 32-60), with careful, load-bearing comments about *not* using the process-cached `resolved_runtime_config()` (must re-resolve `OratioRuntimeConfig` fresh per call so a Settings toggle takes effect without restart) and about surfacing the full anyhow error chain (`{e:#}`) for actionable failures (missing model / bad tensor shape). D-08's work is: repoint `mic.rs`'s `transcribe_audio_file` to the same plugin-dispatch shape as `oratio.rs`'s `imp::transcribe_path_to_dto` (likely factoring the duplicated plugin-call logic into one shared helper both commands call, rather than copy-pasting it a second time), preserving the fresh-resolve and full-error-chain behavior, then delete `stt-candle` from `vox-speech/Cargo.toml` (and its now-dead `#[cfg(feature = "stt-candle")]` code in `backend_dispatch.rs`, `backends/mod.rs` (which gates `mod candle_engine; pub mod candle_whisper;`), `traits.rs`, `subtitle/srt.rs`, `lib.rs`), and drop `"stt-candle"` from `vox-gui/Cargo.toml`'s feature list (keep `"stt-sherpa"`, `"compiler-rerank"` per D-08's explicit carve-out).

All the `#[cfg(not(feature = "stt-candle"))]` fallback branches in vox-speech already produce clear "requires stt-candle feature" error messages — deleting the feature means these become the *only* code path (no `#[cfg]` needed at all once the feature and its `dep:` lines are gone), which simplifies rather than complicates the crate.

## Architecture Patterns

### System Architecture Diagram

```
                     ┌─────────────────────────────┐
                     │   vox-cli / vox-orchestrator │
                     │   -mcp (L4 / L3 callers)     │
                     └───────────┬─────────────────┘
                                 │ load_code_plugin_by_id("grammar-export")
                                 │ / cached_code_plugin("webhook")
                                 ▼
                     ┌─────────────────────────────┐
                     │   vox-plugin-host (L3)       │
                     │   Loader::load               │
                     │   → DefaultVoxHost (hardcoded)│
                     └───────────┬─────────────────┘
                    dlopen (same-process, in-addr-space)
                                 ▼
        ┌────────────────────────────────────────────────┐
        │  vox-plugin-grammar-export (L4, new)             │      vox-plugin-webhook (L4, exists)
        │  as_grammar_export() → export(format, cfg_json)  │      as_http_listener() → start_listening
        │     ↓ calls back into                            │         │ internally: WebhookState
        │  vox-grammar-export (L1, shrunk CORE)             │         │  → broadcast::Sender<WebhookEvent>
        │  ebnf::emit_ebnf() / grammar_ir / compact_prompt  │         │  → [GAP: no ABI path out] ───┐
        └───────────────┬────────────────────────────────┘          └───────────────────────────────┤
                        │ ▲ unconditional, no plugin dependency                                        │
                        │ │                                                                            ▼
              vox-compiler│vox-constrained-gen (L2, CORE)                              vox-orchestrator (L3)
              from_vox_grammar() — never plugin-gated                                  HopperIntake::submit(
                                                                                          IntakeSource::Webhook)
```

### Recommended Project Structure

```
crates/
├── vox-grammar-export/         # shrunk: grammar_ir.rs, ebnf.rs, compact_prompt.rs, versioning.rs, lib.rs (types only)
├── vox-plugin-grammar-export/  # new: gbnf.rs, json_schema.rs, lark.rs, ssot_markdown.rs, x_grammar_2.rs, automaton.rs, plugin entry (lib.rs, Plugin.toml)
├── vox-plugin-webhook/         # existing: add pub(crate) poll/host-callback wiring per chosen option
└── vox-orchestrator/
    └── src/config/             # new optional [webhook] config section (D-04)
```

### Pattern 1: Plugin-absence actionable error (D-02)
**What:** Route CLI/CI grammar-export call sites through `vox_plugin_host::load_code_plugin_by_id("grammar-export")` (or `cached_code_plugin`) instead of calling `vox_grammar_export::export()` directly.
**When to use:** Any call site currently using `export`, `json_schema`, `lark`, `ssot_markdown`, `x_grammar_2`, or `gbnf` from the old crate.
**Example:**
```rust
// Source: existing precedent, crates/vox-gui/src/commands/oratio.rs:45-51
let plugin = vox_plugin_host::cached_code_plugin("grammar-export")
    .map_err(|e| format!("grammar-export plugin load: {e}"))?;
let gx = plugin
    .plugin
    .as_grammar_export()  // new accessor, to be added
    .into_option()
    .ok_or_else(|| "grammar-export plugin missing GrammarExportPlugin accessor".to_string())?;
```
The `LoadError::InitFailed` message already produced by `load_code_plugin` when the plugin isn't installed (`crates/vox-plugin-host/src/lib.rs:154-159`) includes `format_install_hint` — this already satisfies D-02's "clear, actionable error naming the plugin and how to install it" with zero new error-formatting code.

### Anti-Patterns to Avoid
- **Don't gate `ebnf::emit_ebnf()` behind the plugin.** `vox-constrained-gen`'s `from_vox_grammar()` is a public, always-compiled, unconditional call path (used for constrained sampling, not a CLI convenience) — making it depend on plugin presence would violate D-01's explicit "vox-compiler must never depend on plugin availability" (and by the same logic, neither must vox-constrained-gen).
- **Don't literally move `WebhookOrchestratorBridge`/`OrchestratorInboxItem` into vox-orchestrator's Cargo.toml as a dependency on vox-plugin-webhook.** That's the direct crate edge D-03 explicitly forbids, and would also be an upward layer violation (L3→L4) requiring an unauthorized crate-edges exception.
- **Don't apply vox-quantize's "gate the whole module tree" pattern verbatim to `vox-plugin-mens-candle-core`.** Its candle usage is much more partial (see file-by-file counts above); a wholesale feature gate would break candle-free code (hf_keymap, checkpoint_state, etc.) for no reason.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Plugin-absent error message | A new "plugin not found" formatter | `vox_plugin_host::load_code_plugin_by_id`'s existing `format_install_hint` path | Already produces exactly the D-02-required message; reinventing it risks drifting from the one other plugin-absence UX in the codebase |
| Plugin dispatch caching | A new `OnceLock`/cache wrapper | `vox_plugin_host::cached_code_plugin` | Already exists, already used by `oratio.rs` |
| Webhook event classification | A second copy of source/event_type routing logic | Re-derive from the existing `OrchestratorInboxItem::from_webhook` match arms (small enough to duplicate under the AGENTS.md defactor policy, but copy the *logic*, don't reinvent categories) | Keeps GitHub/Discord/Slack/GitLab-deprecated routing consistent between the plugin's internal tests and the orchestrator's real dispatch |

**Key insight:** almost everything this phase needs (loader, caching, actionable errors, catalog entries, an extension-trait pattern, and even a full working GUI→plugin dispatch example) already exists in this codebase. The engineering risk is not building infrastructure, it's correctly drawing boundaries (CORE-core split line; host↔plugin event boundary) without breaking already-passing consumers.

## Common Pitfalls

### Pitfall 1: Treating vox-constrained-gen as a "thin consumer" like the others
**What goes wrong:** Moving `ebnf::emit_ebnf()` to the plugin (following the surface-level "6 consumers, move exports to plugin" framing) breaks `EarleySampler`/`PdaSampler` construction at CORE-tier, making constrained generation depend on plugin availability.
**Why it happens:** CONTEXT.md's scout note groups vox-constrained-gen's usage as "ebnf::emit_ebnf, grammar_ir" without flagging that `emit_ebnf` here is an unconditional, always-compiled call, not an optional/CLI-only one.
**How to avoid:** Keep `ebnf.rs` in the CORE-remaining crate; verified above.
**Warning signs:** any `cargo check -p vox-constrained-gen` failure citing a missing `vox_grammar_export::ebnf` path.

### Pitfall 2: Assuming D-06/D-07 fully close SC4
**What goes wrong:** Declaring SC4 done after fixing only vox-quantize without noticing `vox-plugin-mens-candle-core` has an unconditional Candle dependency AND is classified inconsistently as CORE (`layers.toml`, layer 3) vs. not-CORE (`crate-layers.v1.json`, layer 4) between the project's two layer-tracking files.
**Why it happens:** The scout's Candle audit only looked at crates literally named in the PRD/deep-dive docs; a manifest-level Candle scan cross-referenced against a *single* layer file can give a false "all clear" if that file happens to be the more permissive one.
**How to avoid:** Re-run the manifest-level Candle audit (`grep -E '^candle-(core|nn|transformers) *=' crates/*/Cargo.toml` filtered for lines without `optional`) against **both** `contracts/ci/crate-layers.v1.json` and `docs/src/architecture/layers.toml` before declaring SC4 satisfied, and flag any crate where the two disagree.
**Warning signs:** any grep for `candle` across all `Cargo.toml` files under `crates/` that is cross-checked against only one of the two layer files.

### Pitfall 3: Building the webhook wiring before deciding the host↔plugin data-path
**What goes wrong:** Starting to write the orchestrator-side dispatch code before picking one of the three options in §Webhook dispatch, then discovering mid-implementation that `Loader::load` can't be reached without touching `VoxHost` (a shared trait every other plugin also uses).
**Why it happens:** D-03's wording ("routes webhook events... via WebhookOrchestratorBridge/OrchestratorInboxItem") reads like the types just need importing, obscuring that they're private and unreachable without new ABI plumbing.
**How to avoid:** Pick the data-path option first (recommend option 1 — no VoxHost/Loader changes) as a planning-time decision, then write tasks against it.
**Warning signs:** any task description that says "import `WebhookOrchestratorBridge` into vox-orchestrator" without first addressing the module's `mod webhook;` (private) visibility and the missing host callback.

## Runtime State Inventory

Not applicable — this phase is a source-code/manifest restructuring (crate split, Cargo feature gates, plugin registration), not a rename/rebrand/data-migration. No stored data, live service config, OS-registered state, secrets, or build artifacts reference the old crate names in a way that requires migration beyond the already-covered Cargo/contracts files.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Build broker (cargo queue) | Every build/test/gate in this phase | ✓ (per Phase 2 environment_facts) | — | None — queued builds take 5-15+ min; prefer reading code over building per this phase's environment facts |
| `cargo run -p vox-cli -- ci <gate>` | crate-edges, arch-check, plugin-abi-parity, grammar-export-check verdicts | ✓ | — | Installed `vox`/`target/debug/vox` refuse as stale — always rebuild fresh |
| macOS Metal / Linux CUDA toolchains | Building `vox-plugin-mens-candle-cuda`/`-metal` with `cuda`/`metal` features to verify the Candle-gating change compiles | Unverified this session (macOS host — Metal only, no CUDA) | — | CI already builds CPU-only by design ("CI builds without these (no GPU on runners)" per vox-quantize's own Cargo.toml comment) — plan verification against CPU-only default features |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` / `cargo nextest` (`.config/nextest.toml` present) |
| Config file | `.config/nextest.toml` |
| Quick run command | `cargo test -p <crate> --lib` (or targeted `cargo nextest run -p <crate>`) |
| Full suite command | `cargo run -q -p vox-cli -- ci pre-push --full` (per AGENTS.md Local CI Gate Tiers) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-dead-crate-extract-to-plugin (grammar) | `vox-constrained-gen`/`vox-compiler` still build and pass with the shrunk CORE crate | unit/integration | `cargo test -p vox-constrained-gen -p vox-compiler` | ✅ existing tests in `earley.rs`/`pda.rs` exercise `from_vox_grammar()` |
| REQ-dead-crate-extract-to-plugin (grammar) | Plugin dispatch returns correct grammar text for each format | integration | new test in `crates/vox-plugin-grammar-export/tests/` mirroring `crates/vox-grammar-export/tests/export_test.rs` | ❌ Wave 0 — new plugin crate needs its own test harness copied/adapted from the existing `export_test.rs` |
| REQ-dead-crate-extract-to-plugin (grammar) | Absent-plugin error is actionable | unit | test that `load_code_plugin_by_id("grammar-export")` on an empty install root returns an `InitFailed` containing the install hint | ❌ Wave 0 |
| REQ-dead-crate-extract-to-plugin (webhook) | Webhook event → `IntakeItem` with `IntakeSource::Webhook` lands in hopper, no network | integration | new test feeding synthetic JSON through the chosen boundary → `HopperIntake::submit` | ❌ Wave 0 — depends on which option (§Webhook dispatch) is chosen |
| REQ-dead-crate-extract-to-plugin (webhook) | No config section → no plugin load, no listener | unit | assert `resolve_webhook_config()`-equivalent returns `None` and load is skipped | ❌ Wave 0 |
| REQ-dead-crate-misplaced (SSG) | n/a — evidence only | — | `git show --stat 9d385a60b`, `find crates -iname vox-ssg` | ✅ already run this session |
| REQ-dead-crate-misplaced (oratio) | `mic.rs`'s live path dispatches through the plugin, preserving fresh-config-resolve + full-error-chain behavior | unit | extend `crates/vox-gui/src/commands/mic.rs`'s existing (implied) test coverage / mirror `oratio.rs`'s `txt_fixture_maps_to_dto_text` test | ⚠️ verify existing test coverage in mic.rs at planning time — this session did not confirm a `#[cfg(test)]` block exists there |
| SC4 (Candle) | `vox-quantize` builds with candle optional/off by default, consumers opt in | build | `cargo check -p vox-quantize --no-default-features`, `cargo check -p vox-populi --features mens-candle-qlora`, `cargo check -p vox-ml-cli --features quantize` | ❌ Wave 0 — no existing "candle off" build target |

### Sampling Rate
- **Per task commit:** `cargo test -p <touched-crate> --lib`
- **Per wave merge:** `cargo run -q -p vox-cli -- ci pre-push --complete` (includes clippy, per the perennial-bug-pattern note in AGENTS.md)
- **Phase gate:** full suite green, plus `cargo run -q -p vox-cli -- ci crate-edges`, `arch-check`, `plugin-abi-parity` (delta-only — see §Gates below for pre-existing red state)

### Wave 0 Gaps
- [ ] `crates/vox-plugin-grammar-export/tests/` — new plugin crate has no tests yet; adapt from `crates/vox-grammar-export/tests/export_test.rs`
- [ ] A synthetic-JSON test harness for the webhook host↔plugin boundary (shape depends on the chosen option)
- [ ] A "candle disabled" build check for `vox-quantize` (none exists today — the crate currently can't be built without candle at all)
- [ ] Confirm `crates/vox-gui/src/commands/mic.rs` has an existing `#[cfg(test)]` module before assuming test coverage carries over unchanged

## Security Domain

`security_enforcement` is not set in `.planning/config.json` (file does not exist in this project) — treat as enabled per the default.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | no auth surface touched |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | Webhook payloads already go through `vox-plugin-webhook`'s existing HMAC signature verification (`ed25519-dalek`, `sha2`/`sha3` deps in its Cargo.toml) before reaching the orchestrator — this phase's dispatch wiring must not bypass that validation; the JSON boundary in §Webhook dispatch option 1 carries only *already-validated* events |
| V6 Cryptography | n/a | webhook signing/HMAC already goes through the plugin's existing code (out of scope to touch); no new crypto introduced by this phase — per AGENTS.md Cryptography Policy, transport/webhook crypto stays where it is, do not route it through `vox-crypto` |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Webhook plugin loaded/listener started even with no config (unwanted network exposure) | Elevated exposure / Denial of Service surface | D-04's opt-in gate — verify with a unit test that absent config → no load, not just "assume the code path is never hit" |
| Grammar-export plugin absence silently falling back to a stale bundled copy | Tampering (stale grammar drifting from compiler) | D-02's fail-closed, no-fallback requirement — verify the CORE crate does NOT retain a duplicate copy of any format emitter "just in case" |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Option 1 (poll_events extension + JSON boundary + re-derived routing) is the best-fit webhook dispatch mechanism | Webhook Dispatch | If the team prefers push-based delivery (lower latency), option 2 (extend `VoxHost`) may be worth the larger blast radius instead — this is a design recommendation, not a verified fact |
| A2 | `vox-plugin-mens-candle-core`'s Candle usage is partial enough that a full-crate feature gate would be wasteful, and a more surgical per-module gate is achievable | Candle in CORE | Not fully audited module-by-module for cross-references; the "0 candle refs" files might still transitively depend on candle-touched sibling modules within the same crate, requiring the gate to be broader than file-level counts suggest |
| A3 | Renaming vs. keeping `vox-grammar-export`'s crate name for the CORE-remaining half is Claude's discretion and either choice satisfies D-01 | Grammar-Export Split | If downstream tooling (docs generators, IDE tooling) hardcodes the crate name `vox-grammar-export`, a rename could require updates this research didn't enumerate |

## Open Questions

1. **Which layer file governs SC4, and does its answer include `vox-plugin-mens-candle-core`'s unconditional Candle dependency?**
   - What we know: `vox-plugin-mens-candle-core` has verified unconditional `candle-core`/`candle-nn`. `contracts/ci/crate-layers.v1.json` (the file CONTEXT.md's canonical_refs names as the L0-L3/CORE definition) says it is layer 4 (not CORE — SC4 already satisfied by D-07 alone). `docs/src/architecture/layers.toml` (the human-authored design doc, also cited by AGENTS.md as enforced by `vox-arch-check`) says layer 3 (CORE — SC4 would still be open).
   - What's unclear: whether this is a known, accepted drift between the two files (i.e., `crate-layers.v1.json` is the operative ratchet and `layers.toml`'s "3" is simply stale) or a real bug where one of the two needs correcting; and whether the user was aware of this crate at all at discuss-phase (it wasn't mentioned in CONTEXT.md's scout findings).
   - Recommendation: surface this to the user before finalizing the plan — either as a `checkpoint:human-verify` early in Wave 0, or by re-running `/gsd-discuss-phase` addendum. Do not silently declare SC4 satisfied on the strength of one file while the other disagrees, and do not silently expand locked-decision scope (D-06/D-07) to cover it either.

2. **Which webhook host↔plugin data-path option should the plan commit to?**
   - What we know: three viable shapes exist (see §Webhook Dispatch), with different invasiveness/latency tradeoffs.
   - What's unclear: whether polling latency (option 1) is acceptable for the orchestrator's webhook-triggered task intake, or whether push semantics (option 2) matter for this use case.
   - Recommendation: option 1 (poll + JSON + re-derived routing) as the default plan unless the user has a stated latency requirement — it's the lowest-risk, smallest-blast-radius choice and touches no shared ABI surface (`VoxHost`) used by every other plugin.

3. **Is `run_grammar_export_check` (the CI gate calling `export()` across 4 formats including the always-erroring `Gbnf`) currently green on HEAD?**
   - What we know: the code as written would propagate `Gbnf`'s deprecation `Err` through the `?` operator.
   - What's unclear: whether this gate is actually wired to run in CI, or is legacy/unreachable code; this session did not trace the CI dispatch table to confirm.
   - Recommendation: run `cargo run -q -p vox-cli -- ci grammar-export-check` at planning/execution time before writing tasks that assume its current pass/fail state.

## Sources

### Primary (HIGH confidence — read directly this session)
- `crates/vox-grammar-export/src/{lib.rs,ebnf.rs,grammar_ir.rs,versioning.rs,automaton.rs}` — split-line verification
- `crates/vox-constrained-gen/src/{earley.rs,pda.rs}`, `crates/vox-compiler/src/llm_prompt.rs` — CORE consumer verification
- `crates/vox-plugin-api/src/{abi.rs,host.rs,extensions/*.rs}`, `crates/vox-plugin-host/src/{lib.rs,loader.rs,host_impl.rs}` — ABI mechanics
- `crates/vox-plugin-webhook/src/{lib.rs,webhook/bridge.rs,webhook/sink.rs}`, `Cargo.toml`, `Plugin.toml` — webhook plugin state
- `crates/vox-orchestrator/src/hopper/{types.rs,store.rs}` — intake/inbox mechanism
- `crates/vox-gui/src/commands/{oratio.rs,mic.rs}` — stt-candle precedent + gap
- `crates/vox-quantize/Cargo.toml`, `crates/vox-plugin-mens-candle-core/Cargo.toml`, `docs/src/architecture/layers.toml` — Candle-in-CORE audit
- `contracts/ci/crate-layers.v1.json`, `contracts/ci/crate-edges.allow.v1.json` — layer/edge verification
- `git show --stat 9d385a60b` — SSG move evidence
- `docs/src/architecture/where-things-live.md` — stale doc row found

### Secondary (MEDIUM confidence)
- `docs/src/architecture/plugin-system-redesign-2026.md` — webhook mentioned in bundled-plugin lists, no bridge-pattern precedent found in this doc

### Tertiary (LOW confidence)
- None — no web search was needed; this phase is entirely internal-codebase research.

## Metadata

**Confidence breakdown:**
- Standard stack / Grammar-export split: HIGH — read every consumer call site directly
- Webhook host↔plugin mechanics: LOW/design-open — verified the *absence* of a mechanism, but the resolution is a recommendation, not a discovered fact
- Candle audit: HIGH for the manifest scan itself (`grep -E '^candle-(core|nn|transformers) *=' crates/*/Cargo.toml` filtered for `optional`, returned exactly 5 hits workspace-wide: vox-ml-cli, vox-plugin-mens-candle-core, vox-plugin-mens-candle-cuda, vox-plugin-mens-candle-metal, vox-quantize); MEDIUM for the CORE/not-CORE determination on `vox-plugin-mens-candle-core` specifically, since the two layer files disagree (see Open Question 1) — every other hit is unambiguously L4 per `crate-layers.v1.json`
- Pitfalls: HIGH — each is a verified code-level finding, not speculation

**Research date:** 2026-09-25
**Valid until:** 30 days (stable internal-architecture research; re-verify if `layers.toml`, `crate-edges.allow.v1.json`, or the plugin ABI version change in the interim)
