---
phase: "3"
slug: "extract-misplaced-crates-to-plugin-architecture"
status: planned
nyquist_compliant: true
wave_0_complete: false
created: "2026-09-25"
updated: "2026-09-26"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Decisions in force: 03-CONTEXT.md D-03..D-14 (D-01/D-02 superseded by D-11: no grammar-export plugin).
> Plans: 03-01 .. 03-06, one per wave (sequential: shared git index, shared Cargo.lock / crate-graph / build-map, shared ROADMAP).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` / cargo-nextest |
| **Config file** | `.config/nextest.toml` |
| **Quick run command** | `cargo test -p <touched-crate> --lib` |
| **Full suite command** | `cargo run -q -p vox-cli -- ci pre-push --complete` (may be red on HEAD for pre-existing reasons; judge only this phase's delta) |
| **Estimated runtime** | minutes per crate; the build broker queue often adds 5-15+ minutes; the first vox-gui build also autobuilds its release sidecars |

---

## Sampling Rate

- **After every task commit:** the task's own `<verify>` blocks (targeted `cargo test` / `cargo check`), plus the exact tdd-guard command on the staged `.rs` files; clippy on the touched crates in each plan's audit task
- **After every plan wave:** `cargo run -q -p vox-cli -- ci crate-edges` (pass signal: no NEW/UPWARD line naming a crate this phase touched; HEAD already carries 3 unrelated violations); `cargo run -q -p vox-cli -- ci plugin-surface-sync` after the ABI change (03-03); `affected-crates --check` after any manifest change
- **Before `/gsd-verify-work`:** phase tests green; no new crate-edge line; `affected-crates --check` green; build-map changes limited to the planned deltas (HEAD's `crate-build-map-parity` drift of ~60 rows + 2 missing crates is pre-existing and out of scope)
- **Max feedback latency:** bounded by the build broker; a queued build is never a pass

---

## Per-Task Verification Map

| Task ID | Requirement / SC | Behavior | Test Type | Automated Command (abridged — full block in the plan) | File Exists |
|---------|------------------|----------|-----------|-------------------------------------------------------|-------------|
| 03-01-T1 | REQ-dead-crate-extract-to-plugin, SC#1 (D-11) | automaton.rs and vox-populi's unused edge removed; consumers green; crate-graph -1 element, build-map vox-grammar-export fan_in 6->5, Cargo.lock -1 line | unit + gate | `cargo test -p vox-grammar-export -p vox-constrained-gen`; `cargo check -p vox-compiler -p vox-populi`; `ci affected-crates --check`; `ci crate-edges` (no line naming vox-populi/vox-grammar-export) | ✅ existing + NEW in-file lib.rs tests |
| 03-01-T2 | SC#1/SC#3 (D-11, D-05) | SC wording amended; vox-ssg evidence; where-things-live row | evidence + doc lint | `git show --stat 9d385a60b`; `jq '.layers \| has("vox-ssg")'`; `vox-doc-pipeline --lint-only --paths architecture/where-things-live.md` | ✅ |
| 03-02-T1 | REQ-dead-crate-misplaced, SC#4 (D-06/D-07/D-09) | vox-quantize Candle optional (`engine`); consumers opt in; CORE Candle scan empty below L4 | build + scan | `cargo tree -p vox-quantize -e normal --depth 1` (no candle); `cargo test -p vox-quantize --features engine`; `cargo check -p vox-populi --features mens-candle-qlora`; `cargo check -p vox-ml-cli --features quantize`; `cargo metadata --no-deps` + crate-layers jq scan | ✅ scan defined in plan |
| 03-02-T2 | SC#4 | scan mutation-proven; clippy both feature sets; SC#4 wording; D-09 follow-up recorded | mutation + lint | scan re-run with `optional` removed must list `vox-quantize L0`; `cargo clippy -p vox-quantize [--features engine]` | ✅ |
| 03-03-T1 | REQ-dead-crate-extract-to-plugin, SC#2 (D-10, ABI) | `WebhookInbox` poll extension; ABI 13 lockstep; plugin drains its router's events through the erased trait object | unit + gate | `cargo test -p vox-plugin-webhook -p vox-plugin-api`; `ci plugin-surface-sync`; `ci generate-plugin-catalog-docs --check` | ❌ W0 -> created in T1 (tests first) |
| 03-03-T2 | SC#2 (D-14, D-04) | init starts nothing; tokenless start refused; plugin-owned runtime; bind errors surfaced; stop works | unit | `cargo test -p vox-plugin-webhook` (7 named tests) + region grep on `init` | ❌ W0 -> created in T2 (tests first) |
| 03-03-T3 | SC#2 | token guard mutation-proven; clippy; commit audit | mutation + lint | blank-token acceptance must fail `start_listening_refuses_without_ingress_token` | ✅ |
| 03-04-T1 | SC#2 (D-03/D-10/D-13) | synthetic webhook JSON -> hopper item with `IntakeSource::Webhook`; bounded fields; payload never copied | unit (no network) | `cargo test -p vox-orchestrator-mcp --lib webhook_intake` (4 named tests) | ❌ W0 -> created in T1 |
| 03-04-T2 | SC#2 (D-04/D-14 host side) | no section = no token read, no load; blank token = no load; plan JSON; both constructors spawn | unit | `cargo test -p vox-orchestrator --lib config::webhook_intake`; `cargo test -p vox-orchestrator-mcp --lib webhook_intake` | ❌ W0 -> created in T2 |
| 03-04-T3 | SC#2 | both opt-in guards mutation-proven; clippy; crate-edges; SC#2/REQUIREMENTS/where-things-live | mutation + lint + gate | mutants must fail `no_config_section_means_no_plan_and_no_token_read` / `missing_or_blank_token_fails_closed` | ✅ |
| 03-05-T1 | REQ-dead-crate-misplaced, SC#4 (D-08/D-12) | Whisper selection via host-registered transcriber; plugin JSON contract pinned; serve uses it | unit | `cargo test -p vox-speech --lib`; `cargo test -p vox-speech --features serve --lib serve`; `cargo check -p vox-speech --features stt-candle` / `stt-sherpa` | ❌ W0 -> created in T1 |
| 03-05-T2 | SC#4 (D-08) | vox-gui registers the oratio-plugin transcriber; mic.rs untouched and its tests green; crate-graph +1, build-map fan_in +1 | unit + gate | `cargo test -p vox-gui --bin vox-gui -- commands::speech_plugin_backend commands::mic`; `ci affected-crates --check` | ✅ mic.rs has 6 tests (confirmed) + NEW test |
| 03-05-T3 | SC#4 (D-12) | vox-ml-cli registers it for `vox oratio`; seam mutation-proven; clippy | unit + mutation | `cargo test -p vox-ml-cli --features oratio --lib commands::oratio_cmd`; mutant must fail `registered_transcriber_serves_whisper_selection` | ❌ W0 -> created in T3 |
| 03-06-T1 | SC#4 (D-08/D-12) | vox-speech has no Candle feature/deps/modules; decoding under `audio-decode`; every consumer builds; lock limited to vox-speech block | build matrix + manifest check | `cargo metadata` jq (0 candle deps); 7-command matrix (vox-speech default/audio-decode/serve/stt-sherpa, vox-ml-cli oratio, vox-orchestrator-mcp oratio-rerank, vox-gui) | ❌ W0 -> audio_io/srt tests created in T1 |
| 03-06-T2 | SC#4 | final CORE scan; vox-speech check mutation-proven; docs; clippy | scan + mutation + lint | 03-02 scan verbatim; mutant (optional candle-core re-added) must flip the check to false | ✅ |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every Wave 0 item is created by the first (test-first) step of the task that needs it; no separate Wave 0 plan.

- [ ] Synthetic-JSON test harness for the webhook poll boundary (vox-orchestrator-mcp) — 03-04-T1
- [ ] vox-plugin-webhook tests for no auto-spawn and token-required listener (D-14) — 03-03-T2
- [ ] A Candle-in-CORE manifest scan that reads layers from `contracts/ci/crate-layers.v1.json` — 03-02-T1 (verify block 2; RED before the change)
- [x] Confirm `#[cfg(test)]` coverage for `crates/vox-gui/src/commands/mic.rs` — confirmed at planning time (six tests incl. one ignored hardware smoke); 03-05 does not edit mic.rs code
- [ ] In-file tests tdd-guard requires on staged files — grammar-export lib.rs (03-01-T1), webhook router.rs (03-03-T2), vox-speech serve.rs (03-05-T1), vox-ml-cli oratio_cmd.rs (03-05-T3), audio_io.rs + srt.rs (03-06-T1)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live GUI dictation still transcribes via the plugin | SC#4 (D-08) | Needs a microphone and an installed speech plugin | `vox plugin install oratio` (or `--path crates/vox-plugin-speech --yes`), run vox-gui, set the STT backend to `whisper` in Settings, dictate a sentence, confirm text appears |
| Webhook end to end through the real dylib over HTTP | SC#2 (D-03/D-04) | Needs the built/installed webhook plugin, a token and an HTTP client | Install `vox-plugin-webhook`, set `VOX_WEBHOOK_INGRESS_TOKEN`, add `[orchestrator.webhook]` with `bind_addr = "127.0.0.1:9080"` to Vox.toml, start the MCP server, `POST /webhooks/github` with `Authorization: Bearer <token>` and an `X-GitHub-Event: push` payload, confirm a Webhook item appears in the hopper inbox; repeat without the header and confirm 401 |
| `vox oratio serve` through the plugin | SC#4 (D-12) | Needs the installed speech plugin and a model | Build vox-ml-cli with `--features oratio`, run `vox oratio serve --port 0`, POST multipart PCM to `/transcribe`, confirm JSON text |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency bounded
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending (plan-checker)
