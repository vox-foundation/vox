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
result: pass
evidence: "2026-09-26 live run. Live mic capture (cpal) and the Tauri window were NOT exercised. The real release cdylib from `cargo build -p vox-plugin-speech --release` was installed with `vox plugin install --path` into a scratch VOX_PLUGINS_DIR. The model was openai/whisper-tiny.en at refs/pr/15, already in the default HF cache, so nothing was downloaded. `cargo test -p vox-gui --bin vox-gui commands::mic::tests -- --nocapture` then ran the new test dictation_transcribes_through_installed_oratio_plugin (c2350c4c3). It takes the committed fixture corpus_v1/utt_11.wav (\"run speech canary sample 1\") through write_wav_16k_mono and transcribe_audio_file, the path stop_mic_capture_and_transcribe takes after capture. It runs with VOX_ORATIO_BACKEND=whisper and the startup speech_plugin_backend::register(). Output: `oratio plugin transcript: \"Run Speech Canary Sample 1\"`, `test result: ok. 5 passed; 0 failed; 1 ignored`. Note: vox-gui builds with stt-sherpa, so GUI \"auto\" mode transcribes with Sherpa in-process. The plugin is reached only when Whisper is selected or Sherpa fails to start. The plugin first failed to load (`version 0.1.0 does not match running core version 0.6.0`); that was fixed in 98fca8371. With no plugin installed, the test prints `skipping: oratio plugin not loadable` and passes."

### 3. `vox oratio serve` transcribes through the plugin
expected: `vox oratio serve` answers a transcription request using the oratio plugin's Whisper transcriber.
result: pass
evidence: "2026-09-26 live run. `vox oratio serve` forwards to vox-ml-cli, so I ran `target/debug/vox-ml-cli oratio serve --port <p>` (built with `-p vox-ml-cli --features oratio`) against the installed plugin in a scratch VOX_PLUGINS_DIR. I then sent multipart POSTs to /transcribe with curl, using utt_11 as raw f32. 16 kHz with sample_rate=16000 returned `{\"raw_text\":\"Run Speech Canary Sample 1\",...\"segments\":[{\"start_ms\":0,\"end_ms\":30000,...}]}` with http=200. 44.1 kHz with sample_rate=44100 and 16 kHz with no sample_rate field returned the same text. The log showed `plugin.loaded id=\"oratio\" version=\"0.6.0\"`, and the backend is `whisper (oratio plugin)`. Bugs found live and fixed in 98fca8371: (1) the plugin was unloadable because Plugin.toml said 0.1.0, and serve answered a bodyless http=500 without logging anything; serve now logs the cause at warn. (2) The plugin ignored sample_rate: 44.1 kHz came back as \"mmm\", and it now resamples to 16 kHz. (3) Segment times were reported as 300000-600000 ms; they are now 0-30000. The new resample unit test was mutation-checked: without the resample it fails with \"got 44100\". The automated form is `cargo test -p vox-ml-cli --features oratio --lib oratio_cmd::tests` (c2350c4c3), which printed `16000 Hz: \"Run Speech Canary Sample 1\"` and `44100 Hz: \"Run Speech Canary Sample 1\"`, `2 passed`. It skips when the plugin or model is absent."

## Summary

total: 3
passed: 3
issues: 0
pending: 0
skipped: 0

## Gaps

[none]
