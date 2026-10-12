---
phase: 03-extract-misplaced-crates-to-plugin-architecture
plan: 05
subsystem: vox-speech / vox-gui / vox-ml-cli (Oratio STT)
status: complete
tags: [oratio, whisper, plugin-host, D-08, D-12, seam, mutation-proof]
requires:
  - "03-04 complete (d9275060c)"
provides:
  - "vox_speech::backend_dispatch::{ExternalWhisperTranscribe, register_whisper_transcriber, has_registered_whisper_transcriber, whisper_backend}"
  - "vox-gui and `vox oratio` register the oratio-plugin Whisper transcriber at startup"
affects:
  - "03-06 (deletes stt-candle once nothing reaches the in-process Candle backend)"
tech-stack:
  added: []
  patterns: ["host-registered fn-pointer seam instead of a new crate edge", "defactored ~15-line adapter", "derived build-map applied as generator before/after delta"]
key-files:
  created:
    - crates/vox-gui/src/commands/speech_plugin_backend.rs
  modified:
    - crates/vox-speech/src/backend_dispatch.rs
    - crates/vox-speech/src/serve.rs
    - crates/vox-speech/src/lib.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/src/commands/mod.rs
    - crates/vox-gui/Cargo.toml
    - contracts/ci/crate-graph.v1.json
    - contracts/ci/crate-build-map.v1.json
    - crates/vox-ml-cli/src/commands/oratio_cmd.rs
decisions:
  - "Whisper selection in vox-speech goes to a host-registered transcriber first. It falls back to in-process Candle only while stt-candle is compiled, and otherwise returns an actionable error. There is no vox-speech -> vox-plugin-host edge."
  - "serve now passes the posted sample_rate to the backend instead of ignoring it"
metrics:
  started: 2026-09-26T03:45:16
  completed: 2026-09-26T21:25:26
  duration: "~17.7h wall clock, mostly build/start stalls and a permission pause"
estimate:
  tokens: 95000
  tasks: 3
actuals:
  tokens: 9439
  tasks: 3
  commits: 4
plan_head_before: d9275060c55d44ed58dac3bfce2c9e3f8d7a947c
---

# Phase 3 Plan 05: Repoint Oratio's Whisper to vox-plugin-speech Summary

vox-speech's Whisper backend selection now goes through a single seam: a transcriber the host registers, using the oratio plugin's exact `SpeechToText::transcribe` contract. vox-gui (mic dictation) and `vox oratio` (serve, session, subtitle) register a plugin-host adapter at startup. As a result, every Whisper path reaches Candle through vox-plugin-speech, and vox-speech gains no crate edge. Sherpa stays in-process.

## Commits

| # | SHA | Subject | Paths |
|---|-----|---------|-------|
| T1 | `4f791903a` | refactor(03-05): route Oratio's Whisper selection through a host-registered transcriber | vox-speech `backend_dispatch.rs`, `lib.rs`, `serve.rs` (3) |
| T2 | `0724cda26` | feat(03-05): vox-gui dictation reaches Candle Whisper through vox-plugin-speech | the 6 planned paths |
| T3 | `66c29256c` | feat(03-05): vox oratio serve/session/subtitle reach Candle Whisper through the plugin | `oratio_cmd.rs` (1) |
| T3 lint | `bd60ce91f` | fix(03-05): clear clippy -D warnings in oratio_cmd.rs under --features oratio | `oratio_cmd.rs` (1) |

## Verification evidence

All cargo output was captured with `--color never`, and every log has 0 ESC bytes.

- **T1 RED** (`target/phase03-05-red.txt`, rc=101). First error: `error[E0422]: cannot find struct, variant or union type TimedSegment in this scope`, followed by `E0425 register_whisper_transcriber`.
- **T1 verify 1:** `default=0 serve=0 verify1=0`. 74 passed on the default build; all six named tests ran, plus `pcm_from_le_bytes_roundtrip`.
- **T1 verify 2:** `candle=0 sherpa=0 verify2=0`; toestub reported "No issues found".
- **T1 extra run (not in the plan):** `--features stt-candle,stt-sherpa --lib backend_dispatch` passed 6/6. That includes the Sherpa-fallback test, which only compiles with `stt-sherpa`.
- **T2 RED** (`target/phase03-05-t2-red.txt`, rc=101). The only error was `error[E0425]: cannot find function register in this scope`.
- **T2 verify 1:** `rc=0 verify1=0`. `register_installs_the_oratio_plugin_transcriber` and the three mic tests passed, and `mic.rs` is unchanged against HEAD.
- **T2 verify 2:** `t2verify2=0`, run on the staged index.
  - The graph equals the start graph plus `vox-plugin-host` in vox-gui's list.
  - `affected-crates --check` exited 0.
  - The staged build map equals HEAD with vox-plugin-host `fan_in` 7 -> 8, and `measured_on` is present.
  - vox-gui's Cargo.lock block is unchanged.
  - Staged numstats: crate-graph `1 0`, build-map `1 1`.
- **Build-map delta:** the generator's before/after diff (`target/phase03-05-bm-delta.txt`) is exactly one row: `{"crate":"vox-plugin-host","dependents":52,"blast_s":588.0,"fan_in":7}` -> `…"fan_in":8}`. The baseline rows equal HEAD's rows exactly.
- **T3 RED** (`target/phase03-05-t3-red.txt`, rc=101): `error[E0425]: cannot find function register_oratio_whisper_transcriber in this scope`.
- **Mutation proof:**
  - backend_dispatch.rs was clean against HEAD before the mutation.
  - The mutant was `registered_whisper_transcriber().filter(|_| false)` inside `whisper_backend()`. Presence check: `mutant-present=1`.
  - The mutant compiled and ran; the log's only `error` line is `error: test failed`.
  - Result: `registered_transcriber_serves_whisper_selection ... FAILED`. It panicked with the "registered no Whisper transcriber … vox plugin install oratio" error, which is the fallback the mutant forces.
  - After restoring, `git diff --quiet HEAD` passed (`restored-identical=0`), and the post-mutant run is `... ok`.
- **T3 verify 1:** `t3verify1=0`.
- **T3 verify 2 (clippy):** `speech=0 serve=0 mlcli=0 gui=101`, with `t3verify2=0` (0 diagnostics located in this plan's files).
  - `gui=101` is a dependency failure outside this plan: `crates/vox-search/src/crawler.rs:77:5` (`sort_by_key`) and `:111:17` (collapsible `if`). That file is at HEAD (`793e24c39`) and clean in the tree.
  - Because that dependency fails, vox-gui itself was never linted under `-D`. As an extra check, `cargo clippy -p vox-gui --all-targets` without `-D` passed (rc=0) with 0 warnings in this plan's vox-gui files. Its warnings are all in another session's dirty `commands/search_probe.rs`.
- **T3 verify 3 (commit audit):** `t3verify3=0`. The three planned commits hold exactly 3, 6 and 1 paths.
- **crate-edges:** `vox-gui -> vox-plugin-host` is in the baseline (`["vox-gui","vox-plugin-host"]`) and is not reported. The gate exits 1 with the same three violations 03-04 recorded. They come from vox-gui's HEAD manifest, not this plan, and the coordinator confirmed they are not a STOP:
  ```
  NEW EDGE not in baseline: vox-gui -> vox-db-types
  NEW EDGE not in baseline: vox-gui -> vox-research-shim
  NEW EDGE not in baseline: vox-research-shim -> vox-compiler
  Error: crate-edges: 3 violation(s)
  ```
  Nothing was edited: no exceptions entry, no `edges` change, no `--tighten`, no fan-in snapshot change.
- **Hooks:** fmt-fix and tdd-guard passed on all four commits. fmt-fix printed "formatted 48–50 file(s)" each time; that is its known side effect on other sessions' dirty `.rs` files. None of those files were staged or reverted.

## Deviations from Plan

1. **[Rule 3 - Blocking] serve.rs docs and chunk decoding.** `clippy --features serve` failed on two things already in HEAD's code: `missing_docs` for `pub mod serve` and for `run_serve_worker`. It also flagged `chunks_exact` with a constant size in the new code. I added a module doc and a function doc, and used `as_chunks::<4>()`. All of this is in T1.
2. **[Rule 1 - Plan's lint rule] Follow-up commit `bd60ce91f`.** `clippy -p vox-ml-cli --features oratio -D warnings` found nine diagnostics in `oratio_cmd.rs`'s existing code: seven collapsible_if, one explicit_counter_loop and one let_unit_value. The file only compiles under the non-default `oratio` feature. As the plan directs, I fixed them in a separate `fix(03-05)` commit limited to that file. The changes are mechanical and behaviour-preserving; the counter stays 1-based.
3. **Sherpa-fallback test renamed** to `create_backend_auto_falls_back_to_whisper_when_sherpa_init_fails`. It is gated `stt-sherpa` and asserts the fallback backend's name is `"whisper (oratio plugin)"`.
4. **Behaviour change in serve.** The posted `sample_rate` is now passed to the backend; before, it was ignored. The in-process Candle fallback requires 16 kHz, so a different rate now returns HTTP 500 instead of a silently mis-rated transcript. The oratio plugin ignores `sample_rate`, as it did before.
5. **Build-map field applied by the coordinator.** The permission classifier denied my edit of the HEAD-blob scratch copy. The user approved the change in chat, and the coordinator applied the one field (`fan_in` 7 -> 8) to the working tree. I verified that the file equals HEAD with only that field changed, then staged it with a plain `git add --`. No scratch blob was used.
6. **Sidecar autobuild.** `crates/vox-gui/build.rs` runs `cargo build -p vox-cli --release --bin vox-ml-cli`, but that binary lives in the `vox-ml-cli` package, so autobuild failed. I built the sidecar by hand (`cargo build -p vox-ml-cli --release --bin vox-ml-cli`, copied to `target/release/vox-ml-cli-aarch64-apple-darwin`). Nothing was committed for this; it is logged in deferred-items.md.
7. **crate_audit.json left as found.** It differs from HEAD's compile times only by the two crates without compile times, as 03-01 recorded. The baseline map rows equal HEAD's exactly.

## Environment notes

- Freshly linked binaries stalled before `main` for 25+ minutes machine-wide while `syspolicyd` was busy. This affected the vox-gui test binary, a control `vox-ml-cli --version`, and another session's nextest binaries. All eventually ran and passed.
- `lldb` and `ping` are blocked by the lean-ctx shell allowlist.
- `VOX_SKIP_FRESHNESS_CHECK` was not used. There was no `--no-verify`, `git add -A`, stash, checkout, restore or reset, and nothing was pushed. Cargo.lock was not staged. STATE.md and ROADMAP.md progress rows were not updated.

## Known Stubs

None.

## Threat Flags

None. No new network endpoints; `serve` keeps its 127.0.0.1 bind and multipart surface. The new abi_stable boundary is covered by T-03-19 through T-03-21:
- `parse_plugin_transcription` requires `text`, rejects malformed segments, and never panics.
- A registered transcriber always wins, with no silent fallback after a plugin error.

## Self-Check: PASSED

- `crates/vox-gui/src/commands/speech_plugin_backend.rs` exists.
- Commits `4f791903a`, `0724cda26`, `66c29256c` and `bd60ce91f` are present in `git log`.
