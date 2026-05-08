---
title: "How to interpret vox mobile doctor output"
description: "Troubleshooting guide for the vox mobile doctor toolchain checks."
category: "how-to"
status: "current"
training_eligible: true
training_rationale: "Stable troubleshooting reference for the vox-mobile plugin."
---

# How to interpret `vox mobile doctor`

`vox mobile doctor` prints one row per checked tool. `[OK]` means the tool is present and usable; `[--]` means missing and prints an `install hint:` line below.

## Android prerequisites

| Check | Install hint |
|---|---|
| `cargo-ndk` | `cargo install cargo-ndk` |
| `ANDROID_NDK_HOME` | Install Android NDK r27 via Android Studio SDK Manager and `export ANDROID_NDK_HOME=<path>`. The check verifies both that the variable is set and that the path exists; if it points at a stale or moved NDK, the hint will quote the bad value. |
| `rustup target aarch64-linux-android` | `rustup target add aarch64-linux-android` |
| `rustup target armv7-linux-androideabi` | `rustup target add armv7-linux-androideabi` |
| `rustup target x86_64-linux-android` | `rustup target add x86_64-linux-android` |

## iOS prerequisites (macOS only)

| Check | Install hint |
|---|---|
| `xcodebuild` | `xcode-select --install` |
| `rustup target aarch64-apple-ios` | `rustup target add aarch64-apple-ios` |
| `rustup target aarch64-apple-ios-sim` | `rustup target add aarch64-apple-ios-sim` |
| `rustup target x86_64-apple-ios` | `rustup target add x86_64-apple-ios` |

On non-macOS hosts, `vox mobile doctor` prints a single placeholder row "iOS toolchain — iOS builds require macOS with Xcode CLT" instead of the table above.

## Exit codes

- **0** — at least one platform is fully configured. `vox mobile build --platform=<that-platform>` for that platform should succeed.
- **1** — no platform is fully configured. Install at least one platform's prerequisites and re-run.

## Spec

This subcommand implements part of Phase 1 of [Vox Mobile Plugin Spec (2026)](../architecture/vox-mobile-plugin-spec-2026.md).
