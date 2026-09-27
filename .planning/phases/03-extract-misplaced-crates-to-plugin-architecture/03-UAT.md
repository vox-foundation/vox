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
result: skipped
reason: "Deferred follow-up: user chose on 2026-09-26 to close Phase 3 and run the live checks later. The router-level 401/200 bearer behaviour is covered by an automated, mutation-checked test (e5c269ada); the dylib-load -> poll -> submit path is not."

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
passed: 0
issues: 0
pending: 0
skipped: 3

## Gaps

[none]

## Deferred Follow-Ups

- test: 1
  idea: "Run the live webhook -> hopper check against the real plugin library."
  deferred_at: 2026-09-26
- test: 2
  idea: "Run live GUI dictation through the oratio plugin."
  deferred_at: 2026-09-26
- test: 3
  idea: "Run `vox oratio serve` through the plugin."
  deferred_at: 2026-09-26
