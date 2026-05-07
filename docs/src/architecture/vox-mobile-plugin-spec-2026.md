---
title: "Vox Mobile Plugin Spec (2026)"
description: "Promotes the existing mobile-pwa template into a first-class vox-mobile plugin binary, adds cdylib cross-compile, host-shell FFI contract, mobile Clavis sources via the existing SecureStore variant, Codex bundled-sqlcipher, and a table-based reminder runtime — the platform additions needed to build a native-feel offline mobile app in Vox. Driving use case is the vox-mental-tracker app."
category: "architecture"
status: "roadmap"
training_eligible: true
training_rationale: "Strategic plan; canonical reference for the vox-mobile plugin, cdylib build target, host-shell contract, and the small set of Clavis/Codex/stdlib additions that mobile apps depend on."
---

# Vox Mobile Plugin Spec (2026)

## Premise

Vox today supports mobile only via the [`mobile-pwa` template](../../../crates/vox-cli/src/templates/mobile_pwa.rs) — a CLI-side scaffolder that produces a Capacitor + Vite + Workbox project. The template is useful but limited: it is a one-shot file emission with no ongoing build/sign/run lifecycle, no on-device Vox runtime, no encryption-at-rest story, no reliable reminder primitive, and no FFI bridge to the platforms it targets.

This spec **promotes that template into a real plugin binary** (`vox-mobile`) parallel to [`vox-mens` and `vox-schola`](../../../README.md), and adds the small, focused set of platform primitives that any voice-first or offline-first mobile Vox app needs.

The **driving use case is `apps/vox-mental-tracker`** — an offline-first, append-only health-event logbook (see its app-tree docs at [`apps/vox-mental-tracker/docs/architecture/data-model-ssot.md`](../../../apps/vox-mental-tracker/docs/architecture/data-model-ssot.md)). The app already exists in working form using Capacitor + Vite + a custom Capacitor plugin (`vox-sherpa-transcribe`), and the gaps it has — true on-device sherpa-onnx STT instead of `SpeechRecognizer`, encryption-at-rest, reliable scheduled reminders, an iOS parity path — are exactly the gaps this spec fills at the Vox-platform layer. Per [`apps/vox-mental-tracker/AGENTS.md`](../../../apps/vox-mental-tracker/AGENTS.md), app-internal product details (data model, clinical export specifics, user-facing privacy copy) stay app-tree; this doc is platform-generic and serves any mobile Vox app.

Every addition in this spec is justified by a concrete need from vox-mental-tracker, with the test that each addition is also useful for any other mobile Vox app that follows. The plugin is mobile-first and **platform-agnostic between iOS and Android** from day one. Desktop and web are explicitly out of scope (they are served by `--target=server` and `--target=fullstack` per the [external frontend interop plan](external-frontend-interop-plan-2026.md)).

## Non-goals

- **Not removing the `mobile-pwa` template.** It is retained as the simplest "easy mode" path for Vox apps that don't need on-device inference, encryption, or alarms. `vox-mobile` adds capability; it does not subtract.
- **Not replacing Capacitor as the UI layer.** The `component` keyword still lowers to React/TSX; Capacitor still hosts the WebView. Native UI emission (Jetpack Compose, SwiftUI) remains out of scope. The differentiator added here is what runs *underneath* the WebView: a Vox-compiled cdylib with the full Vox runtime, oratio, db, and stdlib.
- **Not introducing new bare keywords or bare-keyword UI primitives.** Per [AGENTS.md §Grammar Unification](../../../AGENTS.md), behavior expressible as a decorator or a stdlib call goes there. This spec adds zero new bare keywords and zero new decorators (encryption is a manifest setting; reminders are an `@table` row).
- **Not replacing Clavis or Codex.** Encryption metadata flows through Clavis (existing). The encrypted store is Codex (existing) with a new feature flag.
- **Not a generalized native FFI.** The host-shell contract is mobile-specific and frozen at v1; out-of-band FFI for other purposes is not in scope.

## Decisions baked into this plan

- **`vox-mobile` is a plugin binary**, discovered on `PATH` per the existing model documented in [README.md §Optional plugins](../../../README.md). It ships as `vox-mobile-<version>-<target>.tar.gz` and registers the `vox mobile *` subcommand surface.
- **Default on-device STT is sherpa-onnx**, not Candle Whisper, and not Android `SpeechRecognizer`. Rationale: the [VoicePing 2026 offline ASR benchmark](https://voiceping.net/en/blog/research-offline-speech-transcription-benchmark/) measured sherpa-onnx as ~51× faster than whisper.cpp on Android for the same Whisper-Tiny model; Android `SpeechRecognizer` (which the existing `vox-sherpa-transcribe` Capacitor plugin currently uses as a placeholder) depends on a Google-installed offline pack that is not guaranteed and not auditable, so it doesn't satisfy "fully offline" in the audited sense. [vox-oratio](../../../crates/vox-oratio/Cargo.toml) already supports the sherpa backend behind the `stt-sherpa` feature; this plugin's cdylib build pulls it in for mobile targets, and `vox-sherpa-transcribe`'s Android implementation should swap from `SpeechRecognizer` to JNI-into-the-cdylib once Phase 2 lands.
- **Encryption-at-rest is a deployment concern, not a language concern.** No new `@vault` decorator and no `@table(encrypted: true)` argument. Instead: Clavis resolves the database encryption key from the existing [`SecretSource::SecureStore`](../../../crates/vox-clavis/src/types.rs) variant (Android Keystore / iOS Keychain underneath, abstracted) or an `Argon2id`-derived passphrase fallback; Codex opens the SQLite connection with `PRAGMA key`; and the manifest carries one `[storage] encryption = { source = "clavis:..." }` line. App `@table` declarations are unchanged.
- **Reminders are data, not declarations.** No re-introduction of `@scheduled` ([currently reserved with diagnostic E028](../../../crates/vox-compiler/src/pipeline.rs)). Apps declare a `@table type Reminder { ... }` (or in vox-mental-tracker's case, configure the runtime to watch `HealthEventLog` rows where `event_kind = "scheduled_reminder"` and `event_at` is in the future) and the mobile plugin's reminder runtime reconciles rows to host-platform alarms via the host-shell FFI. This keeps the grammar minimal and makes user-managed reminders fall out of the existing CRUD machinery for free.
- **The host-shell contract is versioned and frozen.** App authors never touch the Kotlin/Swift shell after `vox mobile init` generates it. Surface changes are spec-versioned (`host-shell.v1`, `v2`, …) like other contracts under [`contracts/`](../../../contracts/).
- **App-tree boundaries are respected.** Per `apps/<app>/AGENTS.md` conventions (see [`apps/vox-mental-tracker/AGENTS.md`](../../../apps/vox-mental-tracker/AGENTS.md)), app-owned product docs, contracts (`apps/<app>/contracts/`), and Capacitor plugins (`apps/<app>/plugins/`) stay in the app tree. The `vox-mobile` plugin provides infrastructure those apps consume; it does not pull app-specific schemas, payloads, or UI into the platform.

---

## Phase 1 — Cdylib build target for Android *and* iOS

**Goal:** Make `vox build --target=mobile` produce a cross-compiled cdylib for both Android (`.so` per ABI) and iOS (universal `.dylib`/XCFramework), containing the Vox runtime, oratio, db, and the application's compiled `.vox` source. No FFI surface yet — this phase verifies that the build produces loadable artifacts on both platforms.

**Scope:**

1. New manifest target `target = "mobile"` with a `[mobile]` block:
   ```toml
   [build]
   target = "mobile"

   [mobile]
   platforms = ["android", "ios"]
   android.min_sdk = 26
   android.target_sdk = 35
   android.abis = ["arm64-v8a", "armeabi-v7a", "x86_64"]
   android.ndk_version = "27.0.11902837"   # pinned; vox mobile doctor verifies
   ios.min_version = "15.0"
   ios.archs = ["aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios"]
   ```
2. Build pipeline:
   - The Vox compiler emits a Rust crate as today, but with `crate-type = ["cdylib", "staticlib"]` (cdylib for Android JNI, staticlib for iOS bridging).
   - The crate's dependency set is extended to include `vox-runtime`, `vox-oratio` (with `stt-sherpa`), `vox-crypto`, `vox-db` (with `bundled-sqlcipher` once Phase 4 lands), and the `jni` crate (Android only, gated by `cfg(target_os = "android")`).
   - **Android:** [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk) drives the cross-compile per ABI. Output layout: `target/mobile/android/<abi>/libvox_app.so`, ready to drop into Android `jniLibs/<abi>/`.
   - **iOS:** `cargo build --target=aarch64-apple-ios[…]` per arch, then [`xcodebuild -create-xcframework`](https://developer.apple.com/documentation/xcode/creating-a-multi-platform-binary-framework-bundle) to assemble a single `VoxApp.xcframework` containing device + simulator slices. Output layout: `target/mobile/ios/VoxApp.xcframework`.
   - The plugin verifies the toolchain via `vox mobile doctor` (Android: cargo-ndk + pinned NDK; iOS: Xcode CLT + targets installed via rustup).
3. `vox mobile build --platform=android|ios|all [--release]` orchestrates the above. Re-runs are content-hash cached per the existing Vox build cache. `--platform=all` is the default and runs both pipelines (Android on Linux/macOS/Windows; iOS only on macOS — doctor surfaces a clear error otherwise).
4. **No platform-specific code in app `.vox` sources.** Platform branching lives only in the host shells and in the cdylib's FFI entry points (Phase 2). App authors write one `.vox` source tree.

**Deliverables:** `target = "mobile"` manifest schema, `vox mobile build` subcommand with both platforms, `vox mobile doctor`, golden test fixtures for both Android (one `.so` per ABI loads) and iOS (XCFramework imports cleanly into a Swift package).

**Risks:**
- **NDK / linker pain (Android).** Mitigation: pin NDK version in the manifest; fail loudly if the installed NDK does not match; document tested combinations in [docs/src/reference/cli.md](../reference/cli.md).
- **iOS code-signing pain.** Mitigation: Phase 1 builds unsigned XCFrameworks (signing is a Phase 6 concern); developer-mode Xcode imports work without provisioning profiles for local testing.
- **macOS-only iOS builds.** Documented constraint; `vox mobile build --platform=android` on Linux/Windows continues to work.
- **Artifact size.** sherpa-onnx alone is ~54 MB; with bundled Whisper-tiny.en model assets, expect a 100–150 MB final binary floor on each platform. Document in doctor output.

---

## Phase 2 — Host-shell contract and bindgen

**Goal:** Define the v1 FFI contract between the cdylib and the Kotlin/Swift host shell, then auto-generate the binding code. App authors should never need to touch the host shell after `vox mobile init`.

**Scope:**

1. **Contract file** at [`contracts/mobile/host-shell.v1.yaml`](../../../contracts/mobile/host-shell.v1.yaml). Versioned and frozen — additive changes only within `v1`; breaking changes bump to `v2`.

2. **Surface — host → cdylib (called from Kotlin/Swift):**

   | Function | Purpose |
   |---|---|
   | `vox_mobile_init(config_dir, vault_key_handle) -> Result<()>` | Boot the runtime; resolve Clavis key; open Codex with `PRAGMA key`. |
   | `vox_mobile_invoke(endpoint_path, json_args) -> Result<json>` | Call any Vox `@endpoint` from the host. The Vox-emitted Axum router is reused; transport is in-process function call instead of HTTP. |
   | `vox_mobile_record_pcm(pcm_data, sample_rate_hz) -> Result<entry_id>` | Hand a captured audio buffer to oratio for transcription, classify, persist. |
   | `vox_mobile_reminder_fired(reminder_id) -> Result<()>` | Notify the runtime that the host alarm scheduler fired an alarm. The runtime invokes the app's `on_reminder_fired` handler. |
   | `vox_mobile_shutdown() -> Result<()>` | Flush, close DB, release model memory. |

3. **Surface — cdylib → host (function pointers registered at init):**

   | Callback | Purpose |
   |---|---|
   | `request_alarm(reminder_id, fire_at_unix_ms, title, body)` | Ask the host to schedule a wake-up via `AlarmManager.setAlarmClock` (Android) or `UNUserNotificationCenter` (iOS). |
   | `cancel_alarm(reminder_id)` | Remove a previously requested alarm. |
   | `show_notification(title, body, deeplink_path)` | Post a user-visible notification. |
   | `request_share(file_path, mime_type)` | Open the platform share sheet for an export file. |
   | `request_battery_whitelist_prompt()` | First-launch only; deeplink to `Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`. |

4. **Wire format:** all complex args are JSON serialized per the [Wire Format v1 SSOT](wire-format-v1-ssot.md). Primitives (ints, byte buffers, strings) cross the boundary natively via the [`jni` crate](https://github.com/jni-rs/jni-rs) types on Android.

5. **Bindgen:** `vox mobile bindgen` reads the contract and emits **Android and iOS bindings together**:
   - **Rust-side (Android):** `#[no_mangle] pub extern "C" fn Java_com_vox_app_<MethodName>(...)` stubs that delegate into the Vox runtime. Naming follows the [Android JNI convention](https://source.android.com/docs/setup/build/rust/building-rust-modules/android-rust-patterns).
   - **Rust-side (iOS):** `#[no_mangle] pub extern "C" fn vox_mobile_<method>(...)` C-ABI stubs called via Swift's `@_silgen_name`.
   - **Kotlin-side:** a `VoxNative` object with `external fun` declarations matching the surface, plus a `companion object { init { System.loadLibrary("vox_app") } }`.
   - **Swift-side:** a `VoxNative.swift` file with `@_silgen_name`-decorated externs and Swift-friendly wrapper methods, plus a `module.modulemap` for the XCFramework. Both bindings are generated from the same contract YAML, in the same `vox mobile bindgen` run.

6. Re-running `vox mobile bindgen` produces byte-identical output for the same contract version. Golden tests pin this. Hand-edits to generated files are discouraged; a `// vox:user-edit` zone is available per the same convention as emitted React components in the [external frontend interop plan](external-frontend-interop-plan-2026.md).

**Deliverables:** contract file, bindgen subcommand, generated Kotlin/Rust stubs in the `vox mobile init` template, golden tests for re-emit stability.

**Risks:**
- **Drift between Vox `@endpoint` set and the FFI surface.** Mitigation: `vox_mobile_invoke` is the single generic dispatch — any new `@endpoint` is automatically reachable from the host without bindgen changes. The named functions (`record_pcm`, `reminder_fired`) are the small frozen set that need real native types.

---

## Phase 3 — Mobile Clavis sources

**Goal:** Add platform-native secret sources for mobile, so the database encryption key can be wrapped by hardware-backed keystores.

**Scope:**

The post-2026-Q2 Clavis architecture restructured the spec layer: secrets are declared as `SecretId` enum variants in [`crates/vox-clavis/src/spec/ids.rs`](../../../crates/vox-clavis/src/spec/ids.rs), backed by `&'static [SecretSpec]` arrays in [`crates/vox-clavis/src/spec/registry/`](../../../crates/vox-clavis/src/spec/registry/), with the existing [`SecretSource`](../../../crates/vox-clavis/src/types.rs) enum's `SecureStore` variant abstracting the platform secure store. This phase plugs the mobile case into that existing structure rather than inventing parallel surfaces.

1. **New `SecretId` variant** in [`crates/vox-clavis/src/spec/ids.rs`](../../../crates/vox-clavis/src/spec/ids.rs):

   ```rust
   pub enum SecretId {
       // ... existing variants ...
       VoxMobileDbKey,
   }
   ```

2. **New registry file** `crates/vox-clavis/src/spec/registry/mobile.rs` (mirroring the shape of `platform.rs`, `llm.rs`, etc.):

   ```rust
   use crate::policy::SecretPolicy;
   use crate::spec::ids::SecretId;
   use crate::spec::types::*;

   pub const SPECS_MOBILE: &[SecretSpec] = &[
       SecretSpec {
           id: SecretId::VoxMobileDbKey,
           canonical_env: "VOX_MOBILE_DB_KEY",         // dev/test override only; never in prod APK
           aliases: &[],
           deprecated_aliases: &[],
           backend_key: Some("vox.mobile.db_key.v1"),  // SecureStore key alias
           auth_registry: None,
           policy: SecretPolicy::required_or_kdf_fallback(),
           remediation: "Generated on first launch. Wrapped by Android Keystore (StrongBox when available) or iOS Keychain. Falls back to user-passphrase + Argon2id when the platform store is unavailable.",
           scope_description: "vox-mobile: at-rest database encryption key",
       },
   ];
   ```

   Register the slice from [`spec/registry/mod.rs`](../../../crates/vox-clavis/src/spec/registry/) so `all_specs()` includes it.

3. **`SecureStore` source backing on mobile.** The cdylib's `SecureStore` resolver (a small new module under [`crates/vox-clavis/src/sources/`](../../../crates/vox-clavis/src/sources/) — `secure_store.rs`) calls back into the host shell to perform the actual wrap/unwrap, using the host-shell contract callbacks `keystore_wrap` / `keystore_unwrap` introduced in Phase 2. On Android the host shell talks to `AndroidKeyStore`; on iOS, Keychain Services. The cdylib never sees the unwrapping key material — only wrapped blobs in transit and the resolved plaintext key in memory only for the duration of the SQLite connection open.

4. **Argon2id passphrase fallback.** When the platform secure store is unavailable (older devices, unprovisioned StrongBox, user opted out), `SecretPolicy::required_or_kdf_fallback()` triggers the fallback path: a small new source `crates/vox-clavis/src/sources/argon2id_passphrase.rs` derives the key from a user-entered passphrase + a per-install salt at `<config_dir>/.vox/db_key.salt`. KDF parameters (`m_cost`, `t_cost`, `p_cost`) are pinned in code; bumps require a key-version increment (`vox.mobile.db_key.v2`) and a one-time re-encryption migration.

5. **Resolution surface unchanged.** Consumers call `vox_clavis::resolve_secret(SecretId::VoxMobileDbKey)` exactly as for any other secret. Codex's Phase-4 connection builder (next phase) feeds the resolved key into `PRAGMA key`.

6. **Host-shell contract callbacks** (defined in Phase 2 spec, used here):
   - `keystore_wrap(plaintext_key) -> Result<wrapped_blob>`
   - `keystore_unwrap(wrapped_blob) -> Result<plaintext_key>`
   - The plaintext key buffer is zeroed (`zeroize` crate) immediately after `PRAGMA key` returns.

7. **`vox clavis doctor` extension** — add per-platform availability check that exercises a round-trip wrap/unwrap and reports the actual backing (StrongBox / TEE / software-only on Android; Secure Enclave / software on iOS) plus the passphrase-fallback presence.

**Deliverables:** `SecretId::VoxMobileDbKey` enum addition, `spec/registry/mobile.rs` and `mod.rs` registration, `sources/secure_store.rs` with host-bridge callback integration, `sources/argon2id_passphrase.rs`, `SecretPolicy::required_or_kdf_fallback()` policy variant, host-shell callback additions, doctor check, security-property tests (round-trip wrap/unwrap fidelity, KDF parameter pinning, zeroize verification).

**Risks:**
- **Vendor differences in StrongBox availability.** Mitigation: doctor reports the actual backing on each device; passphrase fallback is always present and selectable.
- **Argon2id parameter choice.** Mitigation: pin `m_cost = 64 MiB`, `t_cost = 3`, `p_cost = 1` in the spec entry as a starting point per OWASP 2024 guidance; bump via a numbered key version (`v2`) plus a one-time re-encryption when hardware moves.
- **`SecureStore` resolver coupling to host callbacks.** The cdylib's `SecureStore` source is unusable in non-mobile builds. Mitigation: `cfg(feature = "mobile-secure-store")` gates the Clavis additions; desktop Vox installs are unaffected.

---

## Phase 4 — Codex `bundled-sqlcipher` and `[storage] encryption`

**Goal:** Make Codex able to open an encrypted SQLite database when a Clavis-resolved key is present.

**Scope:**

1. New cargo feature on `vox-db`: `bundled-sqlcipher`, which sets `libsqlite3-sys` to `features = ["bundled-sqlcipher-vendored-openssl"]` per the [rusqlite docs](https://docs.rs/crate/rusqlite/latest). This compiles SQLCipher from source, statically linked, with vendored OpenSSL — no system-library dependency on Android.
2. Connection builder extension:
   ```rust
   CodexConnection::open(path)
       .with_encryption_key(SecretRef::new("vox.mobile.db_key"))
       .open()?
   ```
   Internally: at connect time, resolve the key via Clavis; immediately issue `PRAGMA key = '<hex>'`; verify by running a no-op query that would fail on a wrong key; zero the in-memory key buffer.
3. Manifest schema addition:
   ```toml
   [storage]
   encryption = { source = "clavis:vox.mobile.db_key", required = true }
   ```
   When `required = true`, Codex refuses to open the DB without a resolvable key. When omitted, Codex opens unencrypted (current behavior, unchanged).
4. Migration handling: PRAGMA key fires before any SQL, so existing migrations work transparently. A **one-time conversion** subcommand `vox db rekey --from-plain --to-clavis=vox.mobile.db_key` covers the upgrade path for existing unencrypted databases.
5. Backup/restore: a `vox db export-encrypted --out=<path>` produces a portable `.vox-vault` file (the SQLCipher-encrypted DB itself, plus a `manifest.json` containing the wrapped key under a backup-specific KEK). The complementary `vox db restore-encrypted` reverses it. Apps with deterministic clinical-export contracts (such as vox-mental-tracker's [`csv-columns.v1.yaml`](../../../apps/vox-mental-tracker/contracts/export/csv-columns.v1.yaml)) layer their own export pipelines on top — `.vox-vault` is the full-fidelity raw backup; deterministic CSV/JSON bundles are app-defined.

**Deliverables:** feature flag, connection builder extension, manifest schema update, rekey/export/restore subcommands, golden tests for round-trip integrity, docs at [docs/src/reference/data-storage.md](../reference/data-storage.md).

**Risks:**
- **Build size impact.** SQLCipher + vendored OpenSSL adds ~3–5 MB to the cdylib. Document in `vox mobile doctor` output.
- **PRAGMA key timing.** A wrong key produces an opaque "file is not a database" error. Mitigation: connection builder runs a sentinel SELECT after PRAGMA key and surfaces a clear `Codex::EncryptionKeyMismatch` error.

---

## Phase 5 — Reminder runtime (`vox-stdlib::reminder`)

**Goal:** Apps declare reminders as `@table` rows; a small runtime reconciles those rows to host-platform alarms.

**Scope:**

1. **Convention, not new grammar.** Default convention — apps declare:
   ```vox
   // vox:skip
   @table type Reminder {
       id:           ulid,
       fire_at_utc:  datetime,
       recurrence:   Option<str>,   // RFC 5545 RRULE; None = one-shot
       title:        str,
       body:         str,
       payload_json: Option<str>,
       active:       bool,
   }
   ```
   The table name `Reminder` is the default convention the runtime watches. Apps with a different shape register their own table, predicate, and handler in `Vox.toml`. For example, vox-mental-tracker's append-only `HealthEventLog` uses a `scheduled_reminder` event_kind with the future fire instant in `event_at`:
   ```toml
   [mobile.reminders]
   table            = "HealthEventLog"
   active_predicate = "event_kind = 'scheduled_reminder' AND correction_of = '' AND CAST(event_at AS INTEGER) > recorded_at_monotonic"
   fire_at_column   = "event_at"
   handler          = "on_reminder_fired"
   ```
   This integrates cleanly with the app's existing append-only model: a reminder is just an event whose `event_at` is in the future; firing it inserts a child event (e.g. `reminder_fired`); cancelling it inserts a `correction_of` row referencing the original `event_id`. No new tables, no new schema migrations.

   Required columns on the configured table: a primary key (any sortable type — `ulid`, `str`, `int`), a `datetime`-or-equivalent "fire-at" column (string ISO-8601 or integer ms), and whatever fields the predicate filters on. Everything else (title, body, payload, recurrence) is app-defined and surfaces to the handler as the row type.
2. **Runtime behavior:**
   - At `vox_mobile_init`, the runtime queries all `active = true` reminders with `fire_at_utc > now()` within a configurable horizon (default 30 days), and calls `request_alarm` for each through the host-shell callback.
   - On `INSERT` / `UPDATE` to the reminders table (intercepted via Codex's existing change-notification hook), the runtime computes the delta and issues `request_alarm` / `cancel_alarm` callbacks immediately. **No polling loop.** The host scheduler is the source of truth for "time elapsed."
   - On `vox_mobile_reminder_fired(reminder_id)`, the runtime loads the row, calls the user-defined `on_reminder_fired(r: Reminder)` function, then computes the next occurrence (if `recurrence` is set), updates `fire_at_utc`, and re-issues `request_alarm`.
3. **Boot rescheduling:** the host shell's `BOOT_COMPLETED` receiver calls `vox_mobile_init` which reconciles all alarms. Idempotent.
4. **Doze posture (Android):** `request_alarm` lowers to [`AlarmManager.setAlarmClock()`](https://developer.android.com/reference/android/app/AlarmManager#setAlarmClock) by default — Doze-exempt, fires on time even in deep idle, costs only that the system shows a status-bar icon for the next user-visible alarm. For non-user-facing reminders (e.g., "audio retention sweep"), the host uses `setExactAndAllowWhileIdle`. On Android 17+, the new callback-based variant is preferred per the [release notes](https://developer.android.com/about/versions/17/release-notes).
5. **Battery-whitelist prompt:** triggered once on first reminder creation via the `request_battery_whitelist_prompt` callback.
6. **Stdlib API surface (in `.vox`):**
   ```vox
   // vox:skip
   import std.reminder

   fn schedule_morning_check_in() {
       reminder.create(Reminder {
           fire_at_utc: today_at_local("09:00"),
           recurrence: Some("FREQ=DAILY;BYHOUR=9"),
           title: "Morning check-in",
           body: "How did you sleep?",
           ..
       });
   }

   fn on_reminder_fired(r: Reminder) {
       notify(r.title, r.body);
       // Recurrence next-occurrence handled by runtime; no app code needed.
   }
   ```

**Deliverables:** stdlib `reminder` module, change-notification hook in Codex (if not already present), RRULE next-occurrence helper (rrule crate), unit tests for the reconciler, integration test simulating fire→handler→reschedule round trip.

**Risks:**
- **One handler limitation.** A single `on_reminder_fired` is the only dispatch point. Apps that need multiple handler types use the `payload_json` field to discriminate (vox-mental-tracker's existing event-payload schemas under `apps/vox-mental-tracker/contracts/event-payloads/` are the canonical pattern). This avoids hard-wiring a registry into the platform.
- **Clock skew.** Reconciler uses `now()` from the host clock. Documented; matches every other Android scheduler.

---

## Phase 6 — `vox mobile init` template + docs + distribution

**Goal:** Make starting a new mobile Vox app a single command. Make distributing it documented and friction-light.

**Scope:**

1. `vox mobile init <name> --platforms=android[,ios]`:
   - Scaffolds repo with `Vox.toml [build] target = "mobile"`, an `src/` tree with a starter `@endpoint` and `@table`, a `components/` tree (Capacitor WebView UI), and a generated `shell-android/` (Kotlin) and optionally `shell-ios/` (Swift). The shells are ~300 LOC each, generated from the host-shell contract via bindgen.
   - Includes a starter `Vox.toml [storage] encryption` block pointing at `clavis:vox.mobile.db_key`, with the spec entry pre-populated.
   - Includes `<uses-permission android:name="android.permission.INTERNET" tools:node="remove"/>` in the manifest by default — the trust-signal "no internet permission" posture from the input-doc analysis. Apps that need internet opt back in via the manifest.
2. `vox mobile run --platform=android --device=<adb-id>`:
   - Builds, signs (debug keystore), installs, launches with `adb`. Equivalent to `npx cap run android` for the existing template, but covers the full Vox cdylib pipeline.
3. `vox mobile sign --keystore=<path> --keystore-properties=<path>`:
   - Wraps `apksigner` with the pinned signing config. Recommends a 10 000-day keystore for sideload distribution.
4. `vox mobile package --release`:
   - Produces a signed APK plus an F-Droid metadata bundle (`metadata/<package>.yml`) suitable for self-hosted F-Droid repos and IzzyOnDroid submission. F-Droid official-repo reproducible builds are a follow-on goal, not a Phase 6 deliverable.
5. **Docs:**
   - New tutorial under [`docs/src/tutorials/`](../tutorials/): "Build a mobile Vox app from scratch" — covers init through sideload.
   - New how-to under [`docs/src/how-to/`](../how-to/): "Encrypt your mobile Vox app's database" — covers the Clavis + Codex setup.
   - Reference page under [`docs/src/reference/`](../reference/): full `vox mobile *` CLI surface, host-shell contract spec.
6. **Telemetry:** `vox mobile *` subcommands emit `vox.script.mobile.*` events per [AGENTS.md §VoxScript-First Glue Code](../../../AGENTS.md). On-device runtime telemetry is **opt-in only** and disabled by default for mobile builds — privacy posture is the headline feature.

**Deliverables:** init/run/sign/package subcommands, scaffolded template, three docs (tutorial, how-to, reference), telemetry events, doctor checks for the full toolchain.

---

## Cross-cutting concerns

- **Versioning.** The host-shell contract is `host-shell.v1`. Breaking changes bump to `v2` with a parallel-emit grace period — apps' generated shells specify which version they were built against.
- **Documentation governance.** Docs go through [docs/src/contributors/documentation-governance.md](../contributors/documentation-governance.md); architecture index and SUMMARY are regenerated by `vox-doc-pipeline`, never hand-edited.
- **Security defaults must fail closed.** No internet permission by default; encryption `required = true` by default in the `vox mobile init` manifest; no telemetry opt-ins by default; share-sheet only (no auto-send).
- **Migration support.** `vox migrate mobile-pwa-to-mobile-plugin` rewrites a `mobile-pwa`-template project into the new plugin shape so existing users are not stranded.
- **Prior-art alignment.** This spec aligns with the [external frontend interop plan](external-frontend-interop-plan-2026.md): mobile is a third target alongside `server` and `fullstack`, sharing the same `@endpoint` semantics, wire format, and stdlib. A mobile app's endpoints can be re-targeted to `--target=server` to give the user a self-hosted sync server later, with no schema or business-logic changes.

## Sequencing and dependencies

```
Phase 1 (cdylib + cargo-ndk)
   │
   ├─► Phase 2 (host-shell contract + bindgen)
   │     │
   │     └─► Phase 5 (reminder runtime)
   │
   └─► Phase 4 (Codex bundled-sqlcipher) ──► Phase 3 (Clavis mobile sources)
                                                │
                                                └─► Phase 6 (init/run/sign/package + docs)
```

- Phase 1 unblocks everything; nothing else can land first.
- Phase 4 has no Phase-1 dependency and **could land first** as a desktop/server feature (encrypted local-first apps benefit independently).
- Phase 3 depends on Phase 4 (it produces the key Codex consumes) and on Phase 2 callbacks (it needs the keystore wrap/unwrap host bridge).
- Phase 5 depends on Phase 2 callbacks (it needs `request_alarm`).
- Phase 6 is the last user-visible phase and depends on everything.

## Effort estimate

- Phase 1: ~2 weeks (cargo-ndk plumbing + cdylib lowering)
- Phase 2: ~2–3 weeks (contract design + bindgen + golden tests)
- Phase 3: ~1.5 weeks (Clavis sources + spec entries)
- Phase 4: ~1 week (feature flag + connection extension; rusqlite already does most of it)
- Phase 5: ~1.5 weeks (reconciler + RRULE + tests)
- Phase 6: ~2 weeks (template + tutorials + signing + packaging)

**Total: ~10–11 weeks** of focused work for one engineer to deliver all six phases of the platform plugin. Apps consuming the plugin (e.g. vox-mental-tracker, currently at ~55% completion against its own [handoff plan](../../../apps/vox-mental-tracker/docs/README.md)) can begin migrating off Capacitor-Vite-only flows as each phase lands — Phase 1 alone unblocks running real Vox-emitted business logic on-device, even before encryption (Phase 4) or the reminder runtime (Phase 5) lands.

## What this plan does *not* yet decide

- **iOS provisioning workflow details.** Phase 1 builds unsigned XCFrameworks suitable for local development; an `ios-distribution-spec-2026.md` sub-spec will cover provisioning profiles, App Store Connect, and TestFlight automation.
- **Native UI alternative.** Capacitor stays as the UI layer for v1 on both platforms. A future spec could explore Slint or Compose-Multiplatform as a parallel UI emission target, but that work is independent of this plan.
- **Desktop / web target expansion.** Out of scope: those continue to be served by `--target=server` and `--target=fullstack` per the [external frontend interop plan](external-frontend-interop-plan-2026.md). A `vox-desktop` plugin (Tauri-style) could later mirror this spec's shape if demand emerges.
- **F-Droid official-repo reproducible builds.** Possible, but Rust toolchain reproducibility is a known cost. Phase 6 ships sideload + IzzyOnDroid; reproducible builds are a follow-on.
- **vox-cli/templates/mobile_pwa.rs deprecation timeline.** The template is retained for Phase 6 launch; deprecation timing depends on adoption telemetry.
- **Specific telemetry event schema** for the `vox.script.mobile.*` family — defined in a Phase 6 sub-spec.

## Open questions for review

- Does the `vox mobile invoke(endpoint_path, json_args)` generic dispatcher cover every reasonable host→app interaction, or are there cases where the host needs a typed Kotlin/Swift call site for a specific endpoint? If the latter is common, Phase 2 grows a typed-binding emitter.
- Should `passphrase-argon2id` be the default Clavis source on the first run (asking the user to set a passphrase) or should the keystore source be the default with passphrase as recovery? Current spec leans keystore-first; security review may push the other way.
- Should the host-shell contract include a `request_haptic` callback for record-button feedback, or is that a UI-layer (Capacitor plugin) concern? Current spec defers to Capacitor.
