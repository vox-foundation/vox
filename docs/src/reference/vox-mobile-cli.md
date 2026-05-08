---
title: "vox mobile CLI Reference"
description: "Reference for the vox mobile plugin: doctor, build subcommands and the [build] / [mobile] manifest sections."
category: "reference"
status: "current"
training_eligible: true
training_rationale: "Stable CLI reference for the vox-mobile plugin."
---

# `vox mobile` CLI Reference

Cross-compile a Vox project for Android and/or iOS. Implemented by the `vox-mobile` plugin binary, discovered on `PATH` per the plugin model documented in [README.md](../../../README.md). The main `vox` binary delegates `vox mobile <args>` to `vox-mobile` (see [vox-cli/src/main.rs](../../../crates/vox-cli/src/main.rs) — the dispatch block adjacent to the existing `vox-mens` / `vox-schola` delegation).

## Manifest

Mobile builds are driven by `[build]` and `[mobile]` sections in `Vox.toml`:

```toml
[build]
target = "mobile"

[mobile]
platforms = ["android", "ios"]

[mobile.android]
min_sdk = 26
target_sdk = 35
abis = ["arm64-v8a", "armeabi-v7a", "x86_64"]
ndk_version = "27.0.11902837"

[mobile.ios]
min_version = "15.0"
archs = ["aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios"]
```

Schema lives in [`crates/vox-pm/src/manifest.rs`](../../../crates/vox-pm/src/manifest.rs) (`BuildSection`, `MobileSection`, `AndroidConfig`, `IosConfig`). Validation runs through `vox_pm::manifest::validate_mobile`.

## `vox mobile doctor`

Detects local toolchain prerequisites (cargo-ndk, ANDROID_NDK_HOME, rustup Android targets; on macOS also `xcodebuild` and rustup iOS targets) and prints a per-platform readiness table. See [How-To: `vox mobile doctor`](../how-to/vox-mobile-doctor.md) for interpreting the output.

Exits 0 if at least one platform is fully configured; exits 1 otherwise.

## `vox mobile build [--platform <platform>] [--release]`

Cross-compile for the specified platform(s).

- `--platform android` — runs `cargo-ndk` per ABI; outputs `target/mobile/android/<abi>/lib<crate>.so`.
- `--platform ios` — macOS only; runs `cargo build --target=<arch> --lib` per arch + `xcodebuild -create-xcframework`; outputs `target/mobile/ios/<crate>.xcframework` (where `<crate>` is the project's Cargo `[package].name`).
- `--platform all` (default) — runs every platform listed in `[mobile.platforms]`. Skips iOS with a warning on non-macOS. A platform-specific failure is logged but does not abort sibling platforms; the orchestrator only exits non-zero when *every attempted platform* fails.

The `--release` flag enables optimized builds (passes `--release` to cargo-ndk and to per-arch `cargo build`).

## Spec

This CLI implements Phase 1 of [Vox Mobile Plugin Spec (2026)](../architecture/vox-mobile-plugin-spec-2026.md). Phase 2+ adds the host-shell FFI contract, mobile Clavis sources, Codex `bundled-sqlcipher`, and the reminder runtime — none of which are exercised by Phase 1's CLI.
