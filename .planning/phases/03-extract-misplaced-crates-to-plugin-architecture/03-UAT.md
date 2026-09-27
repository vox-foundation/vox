---
status: complete
phase: 03-extract-misplaced-crates-to-plugin-architecture
source: [03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md, 03-04-SUMMARY.md, 03-05-SUMMARY.md, 03-06-SUMMARY.md]
started: 2026-09-26T22:10:00-07:00
updated: 2026-09-26T22:10:00-07:00
---

## Current Test

[testing complete]

## Tests

### 1. Live webhook reaches the hopper through the real plugin library
expected: With an `[orchestrator.webhook]` section and `VOX_WEBHOOK_INGRESS_TOKEN` set, an HTTP POST carrying `Authorization: Bearer <token>` to the plugin's listener becomes a hopper intake item with source `webhook`; the same request without the header gets 401.
result: pass
evidence: "2026-09-26 live run: `cargo build -p vox-plugin-webhook`, then `cargo test -p vox-orchestrator-mcp --test webhook_plugin_e2e -- --nocapture` (f77a59adc). The test installs the real cdylib in the `vox plugin install` layout under a temp VOX_PLUGINS_DIR and sets VOX_WEBHOOK_INGRESS_TOKEN. It then runs ServerState::new_full with `[webhook] bind_addr = \"127.0.0.1:<free port>\"` and sends two POSTs to /webhooks/github. The POST without a header got 401; the POST with the bearer token got 202. Output: `hopper item Webhook: webhook git_push: github/push (delivery wh_0981cac8)`, `test result: ok. 1 passed`. The first run failed at load with `plugin 'webhook' version 0.1.0 does not match running core version 0.6.0`: Plugin.toml had not been bumped to the workspace version, so production could never load the plugin. Fixed in df57ca69c (version 0.6.0, plus an in-crate drift test that was red at 0.1.0). Mutation check: forcing the router's bearer check open made the e2e test fail with 202 != 401; the router was restored byte-identical."

### 2. Live GUI dictation transcribes through the oratio plugin
expected: With vox-plugin-speech installed, dictating in vox-gui produces transcribed text via the registered plugin transcriber.
result: skipped
reason: "Deferred follow-up: user chose on 2026-09-26 to close Phase 3 and run the live checks later (needs a microphone and an installed speech plugin)."

### 3. `vox oratio serve` transcribes through the plugin
expected: `vox oratio serve` answers a transcription request using the oratio plugin's Whisper transcriber.
result: skipped
reason: "Deferred follow-up: user chose on 2026-09-26 to close Phase 3 and run the live checks later (needs an installed speech plugin)."

## Summary

total: 3
passed: 1
issues: 0
pending: 0
skipped: 2

## Gaps

[none]

## Deferred Follow-Ups

- test: 2
  idea: "Run live GUI dictation through the oratio plugin."
  deferred_at: 2026-09-26
- test: 3
  idea: "Run `vox oratio serve` through the plugin."
  deferred_at: 2026-09-26
