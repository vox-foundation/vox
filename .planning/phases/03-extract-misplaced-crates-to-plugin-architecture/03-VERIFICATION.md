---
phase: 03-extract-misplaced-crates-to-plugin-architecture
verified: 2026-09-26T00:00:00Z
status: human_needed
score: 6/8 must-haves verified
covered_files:

  - ".planning/REQUIREMENTS.md"
  - ".planning/ROADMAP.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-01-PLAN.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-01-SUMMARY.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-02-PLAN.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-02-SUMMARY.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-03-PLAN.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-03-SUMMARY.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-04-PLAN.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-04-SUMMARY.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-05-PLAN.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-05-SUMMARY.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-06-PLAN.md"
  - ".planning/phases/03-extract-misplaced-crates-to-plugin-architecture/03-06-SUMMARY.md"
  - "contracts/ci/crate-graph.v1.json"
  - "contracts/plugin/extension-points.v1.yaml"
  - "crates/vox-grammar-export/src/lib.rs"
  - "crates/vox-gui/src/commands/speech_plugin_backend.rs"
  - "crates/vox-gui/src/main.rs"
  - "crates/vox-ml-cli/Cargo.toml"
  - "crates/vox-ml-cli/src/commands/oratio_cmd.rs"
  - "crates/vox-orchestrator-mcp/src/server_state.rs"
  - "crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs"
  - "crates/vox-orchestrator/src/config/orchestrator_fields.rs"
  - "crates/vox-orchestrator/src/config/webhook_intake.rs"
  - "crates/vox-plugin-api/src/abi.rs"
  - "crates/vox-plugin-api/src/extensions/webhook_inbox.rs"
  - "crates/vox-plugin-api/src/lib.rs"
  - "crates/vox-plugin-webhook/src/lib.rs"
  - "crates/vox-plugin-webhook/src/webhook/router.rs"
  - "crates/vox-populi/Cargo.toml"
  - "crates/vox-quantize/Cargo.toml"
  - "crates/vox-quantize/src/lib.rs"
  - "crates/vox-speech/Cargo.toml"
  - "crates/vox-speech/src/backend_dispatch.rs"
  - "crates/vox-speech/src/lib.rs"

covered_digest: "v1:sha256:62b2f3179dcd53f20da6dfbd0de6a4170b4177bc08bcf270fd10e33ec81939db"
behavior_unverified: 2
overrides_applied: 0
behavior_unverified_items:

  - truth: "SC#2 end to end: a webhook delivered over HTTP to the real vox-plugin-webhook dylib reaches the orchestrator hopper as IntakeSource::Webhook"
    test: "Install vox-plugin-webhook, set VOX_WEBHOOK_INGRESS_TOKEN, add [orchestrator.webhook] bind_addr = \"127.0.0.1:9080\" to Vox.toml, start the MCP server, POST /webhooks/github with Authorization: Bearer <token> and X-GitHub-Event: push; then repeat without the header"
    expected: "A Webhook-sourced item appears in the hopper inbox; the header-less request gets 401 and produces no item"
    why_human: "run_poller/start_listener (dlopen via cached_code_plugin, start_listening, poll loop) and the router's bearer middleware are exercised by no test; unit tests cover the plan/gating, the JSON->hopper submission and the in-process inbox separately"
  - truth: "SC#4 functionality kept: Whisper transcription in vox-gui dictation and `vox oratio serve` actually runs through vox-plugin-speech"
    test: "vox plugin install oratio; run vox-gui with STT backend = whisper and dictate; build vox-ml-cli --features oratio, run `vox oratio serve --port 0`, POST multipart PCM to /transcribe"
    expected: "Dictated text appears; serve returns JSON text"
    why_human: "The seam is tested with a fake transcriber and the registration is tested, but the real abi_stable SpeechToText call into the plugin dylib needs an installed plugin, a model and (for GUI) a microphone"
human_verification:

  - test: "Webhook end to end through the real dylib over HTTP (03-VALIDATION.md manual row 2)"
    expected: "Bearer-authenticated POST lands as a Webhook hopper item; missing header -> 401, no item"
    why_human: "Needs the built/installed plugin, a token and an HTTP client; no automated test covers the dlopen + poll loop"
  - test: "Live GUI dictation via the oratio plugin (03-VALIDATION.md manual row 1)"
    expected: "With STT backend = whisper, dictated sentence is transcribed"
    why_human: "Needs a microphone and an installed speech plugin"
  - test: "`vox oratio serve` through the plugin (03-VALIDATION.md manual row 3)"
    expected: "POST /transcribe returns JSON text"
    why_human: "Needs the installed speech plugin and a Whisper model"
  - test: "Decide whether bearer-only ingress is acceptable for provider webhooks"
    expected: "Either accept that GitHub/Slack/Discord deliveries need a relay that injects Authorization: Bearer, or schedule per-source HMAC (WebhookHandler::with_secret) as an alternative auth path"
    why_human: "Design decision: the host-started listener uses WebhookHandler::new() (no HMAC secret) and requires a bearer header that GitHub/Slack/Discord cannot send natively"
---

# Phase 3: Extract Misplaced Crates to Plugin Architecture — Verification Report

**Phase Goal:** Crates that don't belong in the CORE compile graph move to the plugin architecture without losing functionality, and CORE loses its last direct Candle dependency bleed (SC text as amended by D-05, D-09..D-14).
**Verified:** 2026-09-26
**Status:** human_needed
**Re-verification:** No (initial verification)

All evidence below was read from committed objects (`git show HEAD:<path>`) or produced by commands run in this session. Phase-owned paths were clean in the working tree. Other sessions had uncommitted edits in `crates/vox-orchestrator-mcp/src/{lib,dispatch,input_schemas}.rs`, `Cargo.lock` (+19 lines) and elsewhere, so cargo runs compiled those edits too. None of them touch the files verified here.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC#1 (D-11): no grammar-export plugin or ABI exists; the automaton module is gone; vox-populi has no vox-grammar-export dependency; the SC text cites crate-audit D-4/D-18 | ✓ VERIFIED | `git ls-tree HEAD crates/` has no `*grammar*` plugin dir. `git grep -i grammar` over `crates/vox-plugin-api` and `extension-points.v1.yaml` finds nothing. `crates/vox-grammar-export/src/` has no `automaton.rs`, and `lib.rs` has no `automaton` reference. The vox-populi manifest, its Cargo.lock block and its crate-graph entry have no grammar-export. ROADMAP:79 cites "D-4/D-18 … commit 0a8d1518c". crate-audit doc lines 140/266/440 confirm D-4/D-18. `cargo test -p vox-grammar-export`: 6+18 passed. |
| 2 | SC#2 plugin side (D-10/D-14): WebhookInbox extension exists at ABI 13, every tracked code Plugin.toml is at 13, `init()` spawns nothing, and the listener refuses to start without a token | ✓ VERIFIED | `webhook_inbox.rs` defines `poll_events(max) -> RResult<RVec<RString>,RBoxError>`. `VOX_PLUGIN_ABI_VERSION = 13`, MIN_SUPPORTED 12. All 11 code manifests are at 13 (the bad-abi fixture is 1 on purpose; skill manifests have no abi key). `init()` only returns `new_plugin()`. `start_listener` returns `.ok_or("…refusing to start the listener without an ingress token")`. `cargo test -p vox-plugin-webhook`: 52 passed, including `constructing_the_plugin_starts_no_listener`, `start_listening_refuses_without_ingress_token` and the `poll_events_*` tests. `ci plugin-surface-sync`: OK (13 extension points). |
| 3 | SC#2 host side (D-03/D-04/D-13): an opt-in `[orchestrator.webhook]`-gated poller in vox-orchestrator-mcp resolves the token only through `vox_secrets` `SecretId::WebhookIngressToken`, submits `IntakeSource::Webhook`, and adds no crate edge to vox-plugin-webhook | ✓ VERIFIED | `server_state/webhook_intake.rs` resolves the token only via `vox_secrets::resolve_secret(SecretId::WebhookIngressToken)`, with 0 `std::env` reads. It calls `cached_code_plugin("webhook")`, then `start_listening`, then `as_webhook_inbox().poll_events`, then `hopper.submit(…, IntakeSource::Webhook, …)`. The poller is spawned from both ServerState constructors (lines 335, 409) with `orchestrator_config.webhook.as_ref()` and the orchestrator's live `hopper()`. `[orchestrator]` is parsed through `impl_load.rs:35`. `WebhookIngressToken` is present in `vox-secrets` `spec/ids.rs:87`. The defactor marker sits above `route_kind`. No manifest other than the plugin's own names vox-plugin-webhook, and no crate-graph entry has it as a dependency. Tests: `cargo test -p vox-orchestrator-mcp --lib webhook` 8/8 passed (no-config means no token read, blank token fails closed, submission lands as Webhook, payload not leaked); `cargo test -p vox-orchestrator --lib webhook_intake` 3/3 passed. |
| 4 | SC#2 end to end: an HTTP delivery to the real dylib reaches the hopper | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | `run_poller`/`start_listener` (dlopen, start, poll loop) and the router's bearer middleware are wired but no test exercises them. See Human Verification 1. |
| 5 | SC#3 (D-05): crates/vox-ssg is absent, crate-layers has no vox-ssg key, and utils/ssg lives in vox-cli (L4) | ✓ VERIFIED | `git ls-tree -d HEAD crates/vox-ssg` is empty. `grep -c vox-ssg crate-layers.v1.json` = 0. `crates/vox-cli/src/utils/ssg/mod.rs` exists, and `layers["vox-cli"]` = 4. where-things-live:427 records `9d385a60b`. |
| 6 | SC#4 CORE half (D-06/D-07/D-09): no L0-L3 crate has an unconditional Candle dependency; vox-quantize's candle-core is optional behind `engine` | ✓ VERIFIED | Re-ran the 03-02 scan (`cargo metadata --no-deps` joined with crate-layers.v1.json) and it printed only `vox-plugin-mens-candle-{core,cuda,metal} L4`, core_count=0. The output is non-empty, so the filter is live. vox-quantize has `candle-core optional`, `default = []`, `engine = ["dep:candle-core"]`, and cuda/metal imply engine. Extra transitive check: the only L0-L3 edges to a Candle-carrying crate are `vox-populi -> vox-quantize` (optional, features=[engine]) and `vox-orchestrator -> vox-speech` (optional; vox-speech has no candle). `cargo check -p vox-quantize` rc=0. |
| 7 | SC#4 oratio half (D-08/D-12): vox-speech has no stt-candle/cuda feature and no Candle deps; vox-gui and vox-ml-cli register the oratio-plugin Whisper transcriber | ✓ VERIFIED | vox-speech features are `[audio-decode, cloud, compiler-rerank, default, serve, stt-sherpa]`, with no candle/tokenizers deps. The four Candle modules and the melfilters are gone from `src/backends/`. No workspace manifest enables `vox-speech/stt-candle`; the only `stt-candle` left is in vox-plugin-speech itself. vox-gui `main.rs:24` calls `commands::speech_plugin_backend::register()` → `register_whisper_transcriber(...)` over `cached_code_plugin("oratio")` (plugin id `oratio` in vox-plugin-speech's Plugin.toml). vox-ml-cli `oratio_cmd::run` calls `register_oratio_whisper_transcriber()` first. `cargo check -p vox-speech` rc=0; `cargo test -p vox-speech --lib backend_dispatch` 6/6 passed (registered transcriber wins; unregistered is an actionable error). |
| 8 | SC#4 functionality kept: live Whisper through the plugin in the GUI and in `oratio serve` | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | The adapter is wired and the seam is tested with a fake. The real plugin call needs an installed plugin, a model and a mic. See Human Verification 2 and 3. |

**Score:** 6/8 truths verified (2 present, behavior-unverified). No truth FAILED.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-plugin-api/src/extensions/webhook_inbox.rs` | ✓ VERIFIED | Trait plus revision const, with the `as_webhook_inbox` accessor last in `VoxPlugin` (abi.rs:129), default RNone tested |
| `crates/vox-plugin-webhook/src/lib.rs` | ✓ VERIFIED | Inert init, fail-closed start, plugin-owned runtime, stop/shutdown abort |
| `crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs` | ✓ VERIFIED | Wired at 2 call sites, tested |
| `crates/vox-orchestrator/src/config/webhook_intake.rs` | ✓ VERIFIED | `Option<WebhookIntakeConfig>` field on OrchestratorConfig, default None |
| `crates/vox-quantize/Cargo.toml` (engine feature) | ✓ VERIFIED | Consumers opt in: vox-populi `features=["engine"]`, vox-ml-cli the same |
| `crates/vox-gui/src/commands/speech_plugin_backend.rs` | ✓ VERIFIED | Called from main |
| `crates/vox-grammar-export/src/automaton.rs` | ✓ DELETED | as required |
| `crates/vox-speech/src/backends/candle_*.rs`, `multilingual.rs`, `logit_processors.rs`, melfilters | ✓ DELETED | as required |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| server_state.rs (both ctors) | webhook_intake::spawn_webhook_intake_poller | `orchestrator_config.webhook.as_ref()`, `orchestrator.hopper()` | WIRED |
| webhook_intake.rs | vox-plugin-webhook | `vox_plugin_host::cached_code_plugin("webhook")`, ABI only (no crate edge) | WIRED (runtime link not exercised by a test) |
| webhook_intake.rs | vox_secrets | `resolve_secret(SecretId::WebhookIngressToken)` | WIRED |
| vox-gui main / vox-ml-cli oratio run | vox_speech::backend_dispatch | `register_whisper_transcriber` | WIRED |
| vox-speech whisper_backend | host-registered transcriber | fn-pointer seam, no plugin-host edge | WIRED (tested) |

### Behavioral Spot-Checks / Gates (run this session, `--color never`)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Webhook plugin behavior | `cargo test -p vox-plugin-webhook` | 52 passed | ✓ PASS |
| Host poller gating/submission | `cargo test -p vox-orchestrator-mcp --lib webhook` | 8 passed | ✓ PASS |
| Config section parse | `cargo test -p vox-orchestrator --lib webhook_intake` | 3 passed | ✓ PASS |
| Whisper seam | `cargo test -p vox-speech --lib backend_dispatch` | 6 passed | ✓ PASS |
| Grammar-export after automaton removal | `cargo test -p vox-grammar-export` | 6 + 18 passed | ✓ PASS |
| vox-speech builds | `cargo check -p vox-speech` | rc=0 | ✓ PASS |
| vox-quantize default builds | `cargo check -p vox-quantize` | rc=0 | ✓ PASS |
| ABI/manifest lockstep | `vox ci plugin-surface-sync` | OK (13 extension points) | ✓ PASS |
| Build-map parity | `vox ci crate-build-map-parity` | committed summary matches crate-graph | ✓ PASS |
| Crate graph fresh | `vox ci affected-crates --check` | rc=0 | ✓ PASS |
| CORE Candle scan | 03-02 jq scan | only 3 L4 crates; core_count=0 | ✓ PASS |
| crate-edges | `vox ci crate-edges` | rc=1: exactly the 3 known pre-existing violations (vox-gui→vox-db-types, vox-gui→vox-research-shim, vox-research-shim→vox-compiler) plus a stale-baseline warning for vox-populi→vox-grammar-export | ✓ (out of scope, known) |

I did no mutation testing, because the brief forbade editing source files. Mutation proofs are claimed in 03-02, 03-03, 03-04, 03-05 and 03-06, but not re-verified here.

### Probe Execution

Step 7c: SKIPPED. No `scripts/*/tests/probe-*.sh` is declared by the phase plans.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| REQ-dead-crate-extract-to-plugin | 03-01, 03-03, 03-04 | ✓ SATISFIED (automated); E2E pending human | Truths 1–4. The amended acceptance text (D-11, D-10/D-13) matches the code. |
| REQ-dead-crate-misplaced | 03-01, 03-02, 03-05, 03-06 | ✓ SATISFIED (automated); live STT pending human | Truths 5–8 |

No orphaned requirements. The REQUIREMENTS.md traceability rows and checkboxes still read Pending/`[ ]`; updating them belongs to the orchestrator.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| crates/vox-quantize/src/lib.rs | 24 | `TODO(SP-1 Task 2+)` | ℹ️ Info | Pre-existing (2cd7c904c4, 2026-05-31), not a debt marker |
| crates/vox-plugin-webhook/src/lib.rs | 19-21 | Stale comment ("Suppress dead-code lint until the bridge is wired") plus crate-wide `allow(dead_code, unused_imports)` | ℹ️ Info | Hides the now-uncalled `bind_addr_from_env` and in-plugin bridge (03-03 follow-up) |
| crates/vox-plugin-webhook/src/webhook/router.rs | ~104 | Bearer compared with `==` (not constant-time) | ⚠️ Warning | Timing side channel on the ingress token; low practical risk |
| .vox/audit/2026-05-11-oratio-full-runtime/plugins/oratio/0.1.0/Plugin.toml | — | `abi-version = 12` | ℹ️ Info | Tracked audit snapshot outside plugin-surface-sync scope; still ≥ MIN_SUPPORTED 12 |

No TBD, FIXME or XXX markers in phase-modified files.

### Human Verification Required

#### 1. Webhook end to end through the real dylib over HTTP

**Test:** Install vox-plugin-webhook and set `VOX_WEBHOOK_INGRESS_TOKEN`. Add `[orchestrator.webhook]` with `bind_addr = "127.0.0.1:9080"` to Vox.toml and start the MCP server. `POST /webhooks/github` with `Authorization: Bearer <token>` and `X-GitHub-Event: push`, then repeat without the header.
**Expected:** A Webhook-sourced hopper item appears. The request without the header gets 401 and creates no item.
**Why human:** No test covers the dlopen, start_listening, poll loop or bearer middleware.

#### 2. Live GUI dictation via the oratio plugin

**Test:** `vox plugin install oratio`, then set the vox-gui STT backend to whisper and dictate.
**Expected:** The text appears.
**Why human:** Needs a mic, the installed plugin and a model.

#### 3. `vox oratio serve` through the plugin

**Test:** Build vox-ml-cli with `--features oratio`, run `vox oratio serve --port 0`, and POST multipart PCM to `/transcribe`.
**Expected:** JSON text comes back.
**Why human:** Needs the installed plugin and a model.

#### 4. Decision: bearer-only ingress for provider webhooks

The host-started listener builds `WebhookHandler::new()` (no HMAC secret) and requires `Authorization: Bearer`. GitHub, Slack and Discord cannot send a custom Authorization header, so their deliveries reach the orchestrator only through a relay that adds it. This satisfies SC#2 as written ("token-authenticated listener") and D-14. Decide whether that is acceptable or whether per-source HMAC should become an alternative auth path.

### Residual Follow-ups (not gaps)

- **D-09 (recorded):** `vox-plugin-mens-candle-core` is L3 in `docs/src/architecture/layers.toml:215` but L4 in `crate-layers.v1.json`, and it has unconditional candle deps. It is recorded in the ROADMAP Phase 3 scope note, the CONTEXT `<deferred>` section and the 03-02 SUMMARY Follow-ups, but not in a backlog or todo outside the phase. The two layer files also disagree on other crates: layers.toml has vox-speech at L3 (crate-layers L4), vox-quantize at L2 (L0) and vox-populi at L3 (L2). Under layers.toml's L0-L3, vox-plugin-mens-candle-core would be the only SC#4 violation. Worth a backlog item.
- The crate-edges baseline still lists the stale edge vox-populi→vox-grammar-export. Run `vox ci crate-edges --tighten` once the 3 unrelated violations clear.
- `crates/vox-plugin-sdk/README.md:38` sample manifest still says `abi-version = 12`.
- deferred-items.md: the vox-populi hub.rs E0063 was since fixed by `90f408a45`. The vox-gui build.rs sidecar autobuild (wrong package for vox-ml-cli) is still open.
- Known pre-existing failures were confirmed and excluded: crate-edges (3 violations), vox-search crawler.rs clippy, the sidecar autobuild, and secret-env-guard (another session's research.rs). Fan-in-budget was not run.

### Gaps Summary

No gaps. Every structural and contract claim in SC#1–SC#4 holds on the committed tree, and the relevant tests and gates pass. The status is human_needed because two runtime flows have no automated test: webhook HTTP → dylib → hopper, and live Whisper through the oratio plugin. These are the manual rows in 03-VALIDATION.md. There is also one design decision to make about bearer-only ingress for provider webhooks.

---

_Verified: 2026-09-26_
_Verifier: Claude (gsd-verifier)_

## Human Decisions (2026-09-26, recorded by the orchestrator)

- **Close with pending UAT.** The user chose to mark Phase 3 complete and run the three live checks later. They stay
  open as UAT items, not as verified truths:
  1. An HTTP webhook sent to the real plugin library reaches the hopper. (The router-level 401/200 bearer behaviour
     is now covered by `bearer_check_rejects_missing_or_wrong_token_and_admits_the_right_one`, added in
     `e5c269ada` and mutation-checked. The dylib-load -> poll -> submit path is still untested.)
  2. Live GUI dictation transcribes through the oratio plugin.
  3. `vox oratio serve` transcribes through the plugin.
- **Webhook auth: bearer-only accepted.** GitHub/Slack/Discord webhooks need a relay that adds
  `Authorization: Bearer`. Per-source HMAC verification is a recorded follow-up, not in scope.
- **Follow-ups closed after verification:** constant-time bearer compare and the stale dead-code comment
  (`e5c269ada`); vox-plugin-sdk README ABI sample and the vox-gui sidecar package (`4840ff1b7`).
