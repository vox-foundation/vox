# Vox Mobile — Phase 2: Host-Shell FFI Contract + Bindgen (Skeleton Plan)

> **Status:** Skeleton plan. Spec source: [vox-mobile-plugin-spec-2026.md §Phase 2](../../src/architecture/vox-mobile-plugin-spec-2026.md). Flesh out into a full TDD plan via `superpowers:writing-plans` before execution; the skeleton below identifies tasks, files, and acceptance criteria.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to expand this skeleton into a full per-task plan with code-level steps, then `superpowers:subagent-driven-development` to execute it. Do NOT attempt to execute the skeleton directly — task detail is intentionally light.

**Goal:** Define the v1 FFI contract between the cdylib (built by Phase 1) and the Kotlin/Swift host shell, then auto-generate the binding code on both sides from a single YAML source. Once Phase 2 lands, the Android host of `apps/vox-mental-tracker/plugins/vox-sherpa-transcribe` can swap its `SpeechRecognizer` placeholder for a JNI call into the cdylib's `vox-oratio` sherpa-onnx backend, and the iOS stub gains a real implementation through the same cross-platform path.

**Architecture:**
- New contract file at `contracts/mobile/host-shell.v1.yaml` defining the host↔cdylib surface (initially: 5 host→cdylib functions + 5 cdylib→host callbacks).
- New bindgen module under `crates/vox-mobile/src/bindgen/` with three emitters: Rust JNI (Android), Rust C-ABI (iOS), Kotlin (`VoxNative.kt`), Swift (`VoxNative.swift` + `module.modulemap`).
- New `vox mobile bindgen` subcommand reads the contract YAML and emits all four target files.
- Re-emit stability is golden-tested.

**Tech Stack:** `serde_yaml` for contract parsing, `prettyplease` or hand-rolled string emit for Rust output, plain string templates for Kotlin/Swift.

---

## File structure (target)

**New:**
- `contracts/mobile/host-shell.v1.yaml` — frozen contract source.
- `crates/vox-mobile/src/bindgen/mod.rs` — bindgen orchestrator.
- `crates/vox-mobile/src/bindgen/contract.rs` — YAML schema types + parser.
- `crates/vox-mobile/src/bindgen/emit_rust_android.rs` — JNI stubs.
- `crates/vox-mobile/src/bindgen/emit_rust_ios.rs` — C-ABI stubs.
- `crates/vox-mobile/src/bindgen/emit_kotlin.rs` — Kotlin `VoxNative` object.
- `crates/vox-mobile/src/bindgen/emit_swift.rs` — Swift bridging.
- `crates/vox-mobile/tests/bindgen_golden.rs` — re-emit stability tests.
- `crates/vox-mobile/tests/fixtures/bindgen/host-shell.v1.yaml` — minimal contract fixture.
- `crates/vox-mobile/tests/fixtures/bindgen/expected_*.{rs,kt,swift}` — golden outputs.
- `docs/src/reference/vox-mobile-host-shell-contract.md` — contract reference.

**Modified:**
- `crates/vox-mobile/src/cli.rs` — add `Command::Bindgen { contract: PathBuf, out_dir: PathBuf }`.
- `crates/vox-mobile/src/lib.rs` — `pub mod bindgen;`.
- `crates/vox-mobile/src/main.rs` — wire `Command::Bindgen` into `bindgen::run`.
- `crates/vox-mobile/Cargo.toml` — add `serde_yaml`, `prettyplease` deps.

---

## Surface — host-shell.v1 contract

### Host → cdylib

| Function | Signature | Purpose |
|---|---|---|
| `vox_mobile_init` | `(config_dir: str, vault_key_handle: u64) -> Result<()>` | Boot runtime; resolve Clavis key (Phase 3); open Codex with `PRAGMA key` (Phase 4). |
| `vox_mobile_invoke` | `(endpoint_path: str, json_args: str) -> Result<json>` | Generic dispatch — call any Vox `@endpoint` from the host. Reuses the Vox-emitted Axum router; transport is in-process function call instead of HTTP. |
| `vox_mobile_record_pcm` | `(pcm_data: bytes, sample_rate_hz: u32) -> Result<entry_id>` | Hand a captured PCM buffer to oratio for transcription, classify, persist. |
| `vox_mobile_reminder_fired` | `(reminder_id: str) -> Result<()>` | Notify the runtime that the host alarm scheduler fired. Phase 5 owns dispatch. |
| `vox_mobile_shutdown` | `() -> Result<()>` | Flush, close DB, release model memory. |

### Cdylib → host (function-pointer callbacks registered at init)

| Callback | Purpose |
|---|---|
| `request_alarm(reminder_id, fire_at_unix_ms, title, body)` | Ask host to schedule via `AlarmManager.setAlarmClock` (Android) / `UNUserNotificationCenter` (iOS). |
| `cancel_alarm(reminder_id)` | Remove a previously requested alarm. |
| `show_notification(title, body, deeplink_path)` | Post a user-visible notification. |
| `request_share(file_path, mime_type)` | Open the platform share sheet for an export file. |
| `request_battery_whitelist_prompt()` | First-launch only; deeplink to `Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`. |

Wire format: complex args are JSON serialized per the [Wire Format v1 SSOT](../../src/architecture/wire-format-v1-ssot.md). Primitives cross natively via the `jni` crate types on Android; via `@_silgen_name`-decorated C-ABI on iOS.

---

## Tasks (skeleton — flesh out with writing-plans before execution)

### Task 1: Contract YAML parser + types

- Define `crates/vox-mobile/src/bindgen/contract.rs` with serde-deserializable types: `Contract`, `Function`, `Param`, `ReturnType`, `Direction { HostToCdylib, CdylibToHost }`.
- Add `serde_yaml` dep.
- Unit-test parses a small inline contract; rejects unknown directions; rejects duplicate function names.

### Task 2: Author the v1 contract YAML

- Write `contracts/mobile/host-shell.v1.yaml` with the 10 functions/callbacks from the surface table above.
- Lock the contract: any change requires a v2 file (parallel-emit grace period).
- Add a brief `contracts/mobile/README.md` documenting the versioning rule.

### Task 3: Rust Android (JNI) emitter

- `crates/vox-mobile/src/bindgen/emit_rust_android.rs` produces `#[no_mangle] pub extern "C" fn Java_com_vox_app_<MethodName>(env: JNIEnv, ...) -> ...` stubs that delegate into a thin internal `vox_mobile_runtime::dispatch::*` layer (the runtime layer is Phase 2-runtime, can stub-out for now).
- Naming: [Android JNI convention](https://source.android.com/docs/setup/build/rust/building-rust-modules/android-rust-patterns).
- Golden test: re-emit produces byte-identical Rust source.

### Task 4: Rust iOS (C-ABI) emitter

- `crates/vox-mobile/src/bindgen/emit_rust_ios.rs` produces `#[no_mangle] pub extern "C" fn vox_mobile_<method>(...)` C-ABI stubs.
- Type bridge: bytes via `(ptr: *const u8, len: usize)` pairs; strings via `*const c_char`.
- Golden test as above.

### Task 5: Kotlin emitter

- `crates/vox-mobile/src/bindgen/emit_kotlin.rs` produces a `VoxNative` Kotlin object with `external fun` declarations matching the Android JNI surface, plus a `companion object { init { System.loadLibrary("vox_app") } }`.
- Output is one `VoxNative.kt` file the Phase-6 init template drops into the generated Android project.
- Golden test.

### Task 6: Swift emitter

- `crates/vox-mobile/src/bindgen/emit_swift.rs` produces:
  - `VoxNative.swift` — `@_silgen_name`-decorated externs + Swift-friendly wrapper methods.
  - `module.modulemap` — for the XCFramework consumers.
- Golden test.

### Task 7: `vox mobile bindgen` CLI

- Add `Command::Bindgen { contract: PathBuf, out_dir: PathBuf }` to `crates/vox-mobile/src/cli.rs`.
- Wire in `crates/vox-mobile/src/main.rs` to call `bindgen::run(&contract, &out_dir)`, which orchestrates the four emitters into the right subdirectories of `out_dir`.
- Integration test: run on the v1 contract, assert all four expected files appear with the right content.

### Task 8: Re-emit stability golden suite

- `crates/vox-mobile/tests/bindgen_golden.rs` — for each emitter, parse a fixture contract, emit, and assert byte-equal to a checked-in golden.
- Update protocol: when contract changes, regenerate goldens via `cargo test -p vox-mobile bindgen_golden -- --include-ignored` (or similar) and commit the diff.

### Task 9: Documentation

- `docs/src/reference/vox-mobile-host-shell-contract.md` — full contract surface with semantics for each function.
- Update `docs/src/reference/vox-mobile-cli.md` to document `vox mobile bindgen`.
- Regen doc-pipeline.

### Task 10: Wire `apps/vox-mental-tracker/plugins/vox-sherpa-transcribe` to call into the cdylib

- This is the **acceptance test** that Phase 2 actually unblocks the app. NOT in scope for Phase 2 platform work — but the plan should call it out as the verification step the app team will run after Phase 2 lands.
- Specifically: the app's Android Kotlin can replace its `SpeechRecognizer` placeholder with `VoxNative.recordPcm(pcmBytes, sampleRateHz)` once Phase 2's bindgen has produced `VoxNative.kt`. The iOS stub gains the same surface via Swift.

---

## Dependencies

- Phase 1 (cdylib build target) must be merged. — **PR #68**
- Wire format SSOT (already exists at `docs/src/architecture/wire-format-v1-ssot.md`).
- No dependency on Phases 3–5 (encryption, Clavis, reminder runtime); the surface includes their entry points but Phase 2 ships with stub-out implementations on the cdylib side.

## What this plan does *not* yet decide

- Specific JSON-vs-CBOR-vs-MessagePack for `vox_mobile_invoke`'s payloads (start with JSON for human-debuggability; CBOR migration would be a Phase 2.x).
- Whether `vox_mobile_record_pcm` accepts encoded audio (Opus, AAC) or only PCM (start with PCM; encoded formats are a Phase 2.x).
- Allocation strategy for byte buffers crossing the FFI boundary (likely `Box<[u8]>` with `vox_mobile_free` callback for symmetry; flesh out in writing-plans pass).
- Whether the Kotlin emitter targets coroutines (`suspend fun`) or callbacks for async ops (start with callbacks; coroutines are a polish layer).

---

## Effort estimate

- Skeleton flesh-out via writing-plans: ~1 day.
- Tasks 1–9 execution via subagent-driven-development: ~2 weeks of focused work for one engineer.
- Task 10 (app integration) is app-team work and runs in parallel.

Phases 3–6 outlines remain in the Phase 1 plan's appendix.
