# Vox Mobile — Phase 1: Cdylib Build Target Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable `vox build --target=mobile` to produce a cross-compiled `cdylib` for Android (`.so` per ABI) and iOS (XCFramework) via a new `vox-mobile` plugin binary, parallel to `vox-mens`/`vox-schola`.

**Architecture:** New workspace crate `crates/vox-mobile` ships its own `vox-mobile` binary that the main `vox` CLI discovers on `PATH`. It wraps `cargo-ndk` (Android) and `cargo build --target=<arch>-apple-ios` + `xcodebuild -create-xcframework` (iOS). The Vox compiler's codegen learns to emit `crate-type = ["cdylib", "staticlib"]` when the manifest target is `"mobile"`. The manifest crate (`vox-pm`) gains a `MobileSection`. No FFI surface yet — Phase 1 only proves the build pipeline produces loadable artifacts.

**Tech Stack:** Rust, clap (CLI), serde + toml (manifest), [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk) (Android cross-compile), `xcodebuild` (iOS XCFramework assembly), `vox-pm` (manifest schema), `vox-compiler` (codegen), `assert_cmd` + `tempfile` (integration tests).

**Spec:** [docs/src/architecture/vox-mobile-plugin-spec-2026.md](../../src/architecture/vox-mobile-plugin-spec-2026.md), Phase 1.

**Prerequisites for the engineer running this plan:**
- A Linux or macOS dev machine. Android tasks work on all three OSes; iOS tasks (Tasks 6, 7-iOS branch, 9-iOS) require macOS with Xcode CLT installed.
- `cargo-ndk` installed: `cargo install cargo-ndk`.
- Android NDK 27 installed (e.g. via Android Studio SDK Manager or `sdkmanager "ndk;27.0.11902837"`); `ANDROID_NDK_HOME` set.
- Rust Android targets installed: `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`.
- (macOS only) Rust iOS targets: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios`.

---

## File Structure

**New files:**
- `crates/vox-mobile/Cargo.toml` — crate manifest, declares `vox-mobile` binary
- `crates/vox-mobile/src/main.rs` — clap CLI dispatcher
- `crates/vox-mobile/src/lib.rs` — re-exports for testability
- `crates/vox-mobile/src/cli.rs` — clap argument structs
- `crates/vox-mobile/src/doctor.rs` — toolchain detection
- `crates/vox-mobile/src/build/mod.rs` — build orchestration
- `crates/vox-mobile/src/build/android.rs` — cargo-ndk invocation per ABI
- `crates/vox-mobile/src/build/ios.rs` — cargo + xcodebuild invocation per arch
- `crates/vox-mobile/src/manifest_resolve.rs` — read `[mobile]` section from `Vox.toml`
- `crates/vox-mobile/tests/cli_smoke.rs` — `--version`, `--help`, dispatch tests
- `crates/vox-mobile/tests/build_android.rs` — golden test: tiny `.vox` → loadable `.so`
- `crates/vox-mobile/tests/build_ios.rs` — golden test: tiny `.vox` → importable XCFramework (gated by `cfg(target_os = "macos")`)
- `crates/vox-mobile/tests/fixtures/hello_mobile/Vox.toml` — minimal mobile manifest fixture
- `crates/vox-mobile/tests/fixtures/hello_mobile/src/main.vox` — minimal Vox source
- `docs/src/reference/vox-mobile-cli.md` — CLI reference page
- `docs/src/how-to/vox-mobile-doctor.md` — troubleshooting guide

**Modified files:**
- `Cargo.toml` (workspace root) — add `crates/vox-mobile` to `members` (it's already covered by `crates/*`, but verify)
- `crates/vox-pm/src/manifest.rs` — add `MobileSection` and `BuildSection.target` field on `VoxManifest`
- `crates/vox-pm/src/lib.rs` — re-export `MobileSection`
- `crates/vox-compiler/src/codegen_ts/...` (or wherever Cargo.toml emission lives — verify in Task 4) — emit `crate-type = ["cdylib", "staticlib"]` when manifest target is `"mobile"`

**Test-only fixture files** (committed to repo for golden tests):
- `crates/vox-mobile/tests/fixtures/hello_mobile/expected/android.json` — expected ABIs and output paths
- `crates/vox-mobile/tests/fixtures/hello_mobile/expected/ios.json` — expected iOS archs and XCFramework structure

---

## Task 1: Bootstrap the `vox-mobile` crate

**Files:**
- Create: `crates/vox-mobile/Cargo.toml`
- Create: `crates/vox-mobile/src/main.rs`
- Create: `crates/vox-mobile/src/lib.rs`
- Create: `crates/vox-mobile/src/cli.rs`
- Create: `crates/vox-mobile/tests/cli_smoke.rs`

**Goal:** A buildable `vox-mobile` binary that responds to `--version` and `--help`. No real subcommands yet.

- [ ] **Step 1: Write the failing smoke test**

Create `crates/vox-mobile/tests/cli_smoke.rs`:

```rust
use assert_cmd::Command;

#[test]
fn version_flag_prints_semver() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::starts_with("vox-mobile "));
}

#[test]
fn help_lists_subcommands() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("doctor"))
        .stdout(predicates::str::contains("build"));
}
```

- [ ] **Step 2: Run test, expect compilation failure**

Run: `cargo test -p vox-mobile --tests cli_smoke`
Expected: FAIL — `error: package ID specification 'vox-mobile' did not match any packages`.

- [ ] **Step 3: Create the crate manifest**

Create `crates/vox-mobile/Cargo.toml`:

```toml
[package]
name = "vox-mobile"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Vox mobile build plugin: cross-compiles Vox apps for Android and iOS."
keywords = ["vox", "mobile", "android", "ios"]
categories = ["command-line-utilities"]
default-run = "vox-mobile"

[[bin]]
name = "vox-mobile"
path = "src/main.rs"

[dependencies]
anyhow = { workspace = true }
clap = { workspace = true, features = ["derive"] }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }
vox-pm = { workspace = true }
workspace-hack = { workspace = true }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 4: Create the CLI scaffold**

Create `crates/vox-mobile/src/cli.rs`:

```rust
//! Clap argument structures for the `vox-mobile` binary.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "vox-mobile",
    version,
    about = "Vox mobile build plugin: cross-compile for Android and iOS"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Inspect the local toolchain (cargo-ndk, NDK, rustup targets, Xcode CLT).
    Doctor,
    /// Cross-compile the current Vox project for a mobile platform.
    Build {
        /// Target platform: android, ios, or all (default).
        #[arg(long, default_value = "all")]
        platform: String,
        /// Build in release mode.
        #[arg(long)]
        release: bool,
    },
}
```

Create `crates/vox-mobile/src/lib.rs`:

```rust
//! `vox-mobile`: mobile build plugin for the Vox toolchain.

pub mod cli;
```

Create `crates/vox-mobile/src/main.rs`:

```rust
//! Entry point for the `vox-mobile` binary.

use anyhow::Result;
use clap::Parser;
use vox_mobile::cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => {
            println!("vox-mobile doctor: not yet implemented (Task 3)");
            Ok(())
        }
        Command::Build { platform, release } => {
            println!("vox-mobile build --platform={platform} --release={release}: not yet implemented (Task 5+)");
            Ok(())
        }
    }
}
```

- [ ] **Step 5: Run tests, verify they pass**

Run: `cargo test -p vox-mobile --tests cli_smoke`
Expected: PASS for both `version_flag_prints_semver` and `help_lists_subcommands`.

- [ ] **Step 6: Commit**

```bash
git add crates/vox-mobile/
git commit -m "feat(vox-mobile): bootstrap plugin crate skeleton with clap CLI"
```

---

## Task 2: Extend `VoxManifest` with `[build] target` and `[mobile]` section

**Files:**
- Modify: `crates/vox-pm/src/manifest.rs`
- Modify: `crates/vox-pm/src/lib.rs` (re-exports)
- Test: `crates/vox-pm/src/manifest_tests.rs` (or extend existing tests)

**Goal:** `Vox.toml` files with `[build] target = "mobile"` and a `[mobile]` block parse cleanly into typed Rust structs. Invalid configs are rejected with helpful errors.

- [ ] **Step 1: Write the failing test**

Create or extend `crates/vox-pm/src/manifest_tests.rs` (if there's an existing test file in `crates/vox-pm/tests/`, use that path):

```rust
use vox_pm::manifest::{MobileSection, VoxManifest};

#[test]
fn parses_minimal_mobile_manifest() {
    let toml_src = r#"
[package]
name = "hello-mobile"
kind = "application"

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
"#;

    let manifest: VoxManifest = toml::from_str(toml_src).expect("parse failed");
    let build = manifest.build.expect("missing [build]");
    assert_eq!(build.target.as_deref(), Some("mobile"));

    let mobile = manifest.mobile.expect("missing [mobile]");
    assert_eq!(mobile.platforms, vec!["android".to_string(), "ios".to_string()]);

    let android = mobile.android.expect("missing [mobile.android]");
    assert_eq!(android.min_sdk, Some(26));
    assert_eq!(android.target_sdk, Some(35));
    assert_eq!(android.abis, vec!["arm64-v8a", "armeabi-v7a", "x86_64"]);
    assert_eq!(android.ndk_version.as_deref(), Some("27.0.11902837"));

    let ios = mobile.ios.expect("missing [mobile.ios]");
    assert_eq!(ios.min_version.as_deref(), Some("15.0"));
    assert_eq!(ios.archs.len(), 3);
}

#[test]
fn rejects_unknown_platform() {
    let toml_src = r#"
[package]
name = "x"

[build]
target = "mobile"

[mobile]
platforms = ["windows-mobile"]
"#;
    let manifest: VoxManifest = toml::from_str(toml_src).unwrap();
    let result = vox_pm::manifest::validate_mobile(&manifest);
    assert!(result.is_err(), "unknown platform should be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("windows-mobile"), "error should name the offending platform; got: {msg}");
}
```

- [ ] **Step 2: Run test, expect failure**

Run: `cargo test -p vox-pm parses_minimal_mobile_manifest rejects_unknown_platform`
Expected: FAIL — `MobileSection` is undefined; `build` field doesn't exist on `VoxManifest`.

- [ ] **Step 3: Add `BuildSection`, `MobileSection`, and validator to `manifest.rs`**

Edit `crates/vox-pm/src/manifest.rs`. Add at the bottom of the file (after the existing structs):

```rust
/// `[build]` section: target output flavor.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildSection {
    /// "fullstack" | "server" | "client" | "mobile"; None = manifest default.
    #[serde(default)]
    pub target: Option<String>,
}

/// `[mobile]` section: mobile-specific build configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSection {
    /// Platforms to build for; subset of {"android", "ios"}.
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub android: Option<AndroidConfig>,
    #[serde(default)]
    pub ios: Option<IosConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidConfig {
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
    #[serde(default)]
    pub abis: Vec<String>,
    pub ndk_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IosConfig {
    pub min_version: Option<String>,
    #[serde(default)]
    pub archs: Vec<String>,
}

/// Validate `[mobile]` semantically (after parse).
/// Errors: unknown platform, empty `platforms` when target=mobile, mismatched `[mobile.android]` without "android" in `platforms`, etc.
pub fn validate_mobile(manifest: &VoxManifest) -> Result<(), anyhow::Error> {
    let Some(mobile) = manifest.mobile.as_ref() else {
        return Ok(());
    };
    const KNOWN: &[&str] = &["android", "ios"];
    for p in &mobile.platforms {
        if !KNOWN.contains(&p.as_str()) {
            anyhow::bail!(
                "[mobile.platforms] contains unknown platform '{}'. Known platforms: {}.",
                p,
                KNOWN.join(", ")
            );
        }
    }
    if mobile.platforms.is_empty()
        && manifest
            .build
            .as_ref()
            .and_then(|b| b.target.as_deref())
            == Some("mobile")
    {
        anyhow::bail!(
            "[build] target = \"mobile\" requires [mobile.platforms] to list at least one platform"
        );
    }
    Ok(())
}
```

Then add the new fields to `VoxManifest` (modify the existing struct definition at the top of the file):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxManifest {
    pub package: PackageSection,
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub features: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub bin: Vec<BinSpec>,
    #[serde(default)]
    pub workspace: Option<WorkspaceSection>,
    #[serde(default)]
    pub skills: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub orchestrator: Option<toml::Table>,
    #[serde(default)]
    pub deploy: Option<DeploySection>,
    /// `[build]` section — target output flavor.
    #[serde(default)]
    pub build: Option<BuildSection>,
    /// `[mobile]` section — mobile platform configuration.
    #[serde(default)]
    pub mobile: Option<MobileSection>,
}
```

- [ ] **Step 4: Update `lib.rs` re-exports**

Edit `crates/vox-pm/src/lib.rs` and add to the re-export list:

```rust
pub use manifest::{AndroidConfig, BuildSection, IosConfig, MobileSection, validate_mobile};
```

- [ ] **Step 5: Run tests, verify they pass**

Run: `cargo test -p vox-pm parses_minimal_mobile_manifest rejects_unknown_platform`
Expected: PASS for both.

- [ ] **Step 6: Sanity-check the rest of the workspace still builds**

Run: `cargo check --workspace`
Expected: clean. If anything depends on the exact field set of `VoxManifest`, it should still compile because the new fields default.

- [ ] **Step 7: Commit**

```bash
git add crates/vox-pm/
git commit -m "feat(vox-pm): add [build] and [mobile] sections to VoxManifest"
```

---

## Task 3: `vox mobile doctor` — toolchain detection

**Files:**
- Create: `crates/vox-mobile/src/doctor.rs`
- Modify: `crates/vox-mobile/src/lib.rs` (add `pub mod doctor;`)
- Modify: `crates/vox-mobile/src/main.rs` (call into doctor)
- Create: `crates/vox-mobile/tests/doctor.rs`

**Goal:** `vox mobile doctor` prints a structured table of what's installed and what's missing, with install hints. Exits 0 if all required tools are present for at least one platform; exits 1 otherwise.

- [ ] **Step 1: Write the failing test**

Create `crates/vox-mobile/tests/doctor.rs`:

```rust
use assert_cmd::Command;

#[test]
fn doctor_prints_check_table() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    let output = cmd.arg("doctor").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Doctor must always list every checked tool, present-or-missing.
    assert!(stdout.contains("cargo-ndk"), "doctor should check cargo-ndk; got: {stdout}");
    assert!(stdout.contains("ANDROID_NDK_HOME"), "doctor should check ANDROID_NDK_HOME; got: {stdout}");
    assert!(stdout.contains("aarch64-linux-android"), "doctor should check the rustup target; got: {stdout}");
    #[cfg(target_os = "macos")]
    {
        assert!(stdout.contains("xcodebuild"), "doctor should check xcodebuild on macOS; got: {stdout}");
    }
}

#[test]
fn doctor_succeeds_when_at_least_one_platform_is_complete() {
    // We can't reliably guarantee any platform is fully installed in CI,
    // so this test only asserts the binary runs without panicking.
    // Exit code semantics are exercised by the unit tests in doctor.rs.
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.arg("doctor").assert();
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cargo test -p vox-mobile --test doctor`
Expected: FAIL — `vox-mobile doctor` currently prints the placeholder string from Task 1.

- [ ] **Step 3: Implement the doctor module**

Create `crates/vox-mobile/src/doctor.rs`:

```rust
//! `vox-mobile doctor` — toolchain detection.

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::process::Command;

/// One toolchain check.
#[derive(Debug)]
pub struct Check {
    pub name: String,
    pub status: Status,
    pub install_hint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Present,
    Missing,
}

/// Run all checks for a platform-agnostic doctor pass.
pub fn run() -> Result<i32> {
    let android_checks = android_checks();
    let ios_checks = ios_checks();

    println!("vox-mobile doctor\n");
    println!("Android:");
    for c in &android_checks {
        print_check(c);
    }
    println!("\niOS:");
    for c in &ios_checks {
        print_check(c);
    }

    let android_ok = android_checks.iter().all(|c| c.status == Status::Present);
    let ios_ok = ios_checks.iter().all(|c| c.status == Status::Present);

    if !android_ok && !ios_ok {
        println!("\nNo platform is fully configured. Install at least one platform's prerequisites and re-run `vox mobile doctor`.");
        return Ok(1);
    }
    if android_ok {
        println!("\nAndroid: ready to build.");
    }
    if ios_ok {
        println!("iOS: ready to build.");
    }
    Ok(0)
}

fn android_checks() -> Vec<Check> {
    let mut checks = vec![
        check_executable("cargo-ndk", "cargo install cargo-ndk"),
        check_env("ANDROID_NDK_HOME", "Install Android NDK r27 via Android Studio SDK Manager and `export ANDROID_NDK_HOME=<path>`"),
    ];
    for target in ["aarch64-linux-android", "armv7-linux-androideabi", "x86_64-linux-android"] {
        checks.push(check_rustup_target(target));
    }
    checks
}

fn ios_checks() -> Vec<Check> {
    if cfg!(not(target_os = "macos")) {
        return vec![Check {
            name: "iOS toolchain".into(),
            status: Status::Missing,
            install_hint: "iOS builds require macOS with Xcode CLT".into(),
        }];
    }
    let mut checks = vec![
        check_executable("xcodebuild", "Install Xcode Command Line Tools: `xcode-select --install`"),
    ];
    for target in ["aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios"] {
        checks.push(check_rustup_target(target));
    }
    checks
}

fn check_executable(name: &str, hint: &str) -> Check {
    let status = if which::which(name).is_ok() { Status::Present } else { Status::Missing };
    Check { name: name.into(), status, install_hint: hint.into() }
}

fn check_env(var: &str, hint: &str) -> Check {
    let status = match env::var(var) {
        Ok(v) if !v.trim().is_empty() && PathBuf::from(&v).exists() => Status::Present,
        _ => Status::Missing,
    };
    Check { name: var.into(), status, install_hint: hint.into() }
}

fn check_rustup_target(target: &str) -> Check {
    let output = Command::new("rustup").args(["target", "list", "--installed"]).output();
    let status = match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.lines().any(|line| line.trim() == target) {
                Status::Present
            } else {
                Status::Missing
            }
        }
        _ => Status::Missing,
    };
    Check {
        name: format!("rustup target {target}"),
        status,
        install_hint: format!("rustup target add {target}"),
    }
}

fn print_check(c: &Check) {
    let mark = match c.status {
        Status::Present => "[OK]",
        Status::Missing => "[--]",
    };
    println!("  {} {}", mark, c.name);
    if c.status == Status::Missing {
        println!("       hint: {}", c.install_hint);
    }
}
```

- [ ] **Step 4: Add `which` dependency**

Edit `crates/vox-mobile/Cargo.toml` — add to `[dependencies]`:

```toml
which = "6"
```

- [ ] **Step 5: Wire doctor into the CLI**

Edit `crates/vox-mobile/src/lib.rs`:

```rust
pub mod cli;
pub mod doctor;
```

Edit `crates/vox-mobile/src/main.rs`:

```rust
use anyhow::Result;
use clap::Parser;
use vox_mobile::cli::{Cli, Command};
use vox_mobile::doctor;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => {
            let exit_code = doctor::run()?;
            std::process::exit(exit_code);
        }
        Command::Build { platform, release } => {
            println!("vox-mobile build --platform={platform} --release={release}: not yet implemented (Task 5+)");
            Ok(())
        }
    }
}
```

- [ ] **Step 6: Run tests, verify they pass**

Run: `cargo test -p vox-mobile --test doctor`
Expected: PASS.

- [ ] **Step 7: Manually verify on the dev machine**

Run: `cargo run -p vox-mobile -- doctor`
Expected: a printed table showing which Android and iOS prerequisites are present or missing on your machine.

- [ ] **Step 8: Commit**

```bash
git add crates/vox-mobile/
git commit -m "feat(vox-mobile): add doctor subcommand for toolchain detection"
```

---

## Task 4: Compiler emits `crate-type = ["cdylib", "staticlib"]` when target is mobile

**Files:**
- Locate first: search for the existing crate-type emission in `crates/vox-compiler/`. Run `grep -rn "crate-type\|crate_type" crates/vox-compiler/src/` to find the exact site.
- Modify: the file that writes the emitted Cargo.toml (likely under `crates/vox-compiler/src/codegen_ts/` or a `cargo_emit.rs` sibling).
- Test: `crates/vox-compiler/tests/codegen_mobile.rs` (new file).

**Goal:** When the source manifest has `[build] target = "mobile"`, the Vox compiler's emitted Cargo.toml carries `crate-type = ["cdylib", "staticlib"]` and includes `vox-runtime`, `vox-oratio` (with `stt-sherpa`), `vox-crypto`, `vox-db`, and (Android only) `jni` as dependencies.

- [ ] **Step 1: Locate the existing crate-type emission**

Run: `cd crates/vox-compiler && grep -rn "crate-type\|crate_type" src/ | head -10`
Note the file path and line number. Read 50 lines of context around the match. Identify the function and write its full path: `<file>:<function>` for use in Step 3.

- [ ] **Step 2: Write the failing test**

Create `crates/vox-compiler/tests/codegen_mobile.rs`:

```rust
//! Verifies that `target = "mobile"` lowering emits a cdylib+staticlib Cargo.toml
//! with the mobile dependency set.

use std::path::PathBuf;
use vox_compiler::codegen::cargo_toml_for_manifest;
use vox_pm::manifest::{BuildSection, MobileSection, PackageSection, VoxManifest};

#[test]
fn mobile_target_emits_cdylib_and_staticlib() {
    let manifest = VoxManifest {
        package: PackageSection {
            name: "hello-mobile".into(),
            version: "0.1.0".into(),
            kind: "application".into(),
            ..Default::default()
        },
        build: Some(BuildSection { target: Some("mobile".into()) }),
        mobile: Some(MobileSection {
            platforms: vec!["android".into(), "ios".into()],
            android: None,
            ios: None,
        }),
        ..Default::default()
    };

    let cargo_toml = cargo_toml_for_manifest(&manifest);
    assert!(
        cargo_toml.contains(r#"crate-type = ["cdylib", "staticlib"]"#),
        "expected crate-type cdylib+staticlib; got:\n{cargo_toml}"
    );
    assert!(cargo_toml.contains("vox-runtime"), "missing vox-runtime dep");
    assert!(cargo_toml.contains("vox-oratio"), "missing vox-oratio dep");
    assert!(cargo_toml.contains("stt-sherpa"), "vox-oratio should be enabled with stt-sherpa feature");
    assert!(cargo_toml.contains("vox-crypto"), "missing vox-crypto dep");
    assert!(cargo_toml.contains("vox-db"), "missing vox-db dep");
}

#[test]
fn server_target_does_not_emit_cdylib() {
    let manifest = VoxManifest {
        package: PackageSection {
            name: "hello-server".into(),
            version: "0.1.0".into(),
            kind: "application".into(),
            ..Default::default()
        },
        build: Some(BuildSection { target: Some("server".into()) }),
        ..Default::default()
    };

    let cargo_toml = cargo_toml_for_manifest(&manifest);
    assert!(
        !cargo_toml.contains("cdylib"),
        "server target should not emit cdylib; got:\n{cargo_toml}"
    );
}
```

(Add `Default` derives to `VoxManifest`, `PackageSection` if not already present — usually one-line attribute additions.)

- [ ] **Step 3: Run, expect failure**

Run: `cargo test -p vox-compiler --test codegen_mobile`
Expected: FAIL — `cargo_toml_for_manifest` is not exported (or not implemented for the new target).

- [ ] **Step 4: Implement the lowering**

Inside the file located in Step 1, modify the function that emits `[lib]` / `[[bin]]` and `crate-type` to branch on `manifest.build.as_ref().and_then(|b| b.target.as_deref())`:

```rust
// Inside the existing function that builds the Cargo.toml string:
let target = manifest.build.as_ref().and_then(|b| b.target.as_deref()).unwrap_or("fullstack");
let crate_type_line = match target {
    "mobile" => r#"crate-type = ["cdylib", "staticlib"]"#,
    _ => r#"crate-type = ["rlib"]"#,  // or whatever the existing default is — preserve it
};
// ... write `[lib]\n{crate_type_line}\n` into the emitted Cargo.toml.

// Then in the dependency block, append the mobile-specific deps if target == "mobile":
if target == "mobile" {
    cargo_toml.push_str(r#"
vox-runtime = { workspace = true }
vox-oratio = { workspace = true, features = ["stt-sherpa"] }
vox-crypto = { workspace = true }
vox-db = { workspace = true }
"#);
    // jni is Android-only; add a target-cfg dependency:
    cargo_toml.push_str(r#"
[target.'cfg(target_os = "android")'.dependencies]
jni = "0.21"
"#);
}
```

If `cargo_toml_for_manifest` doesn't exist yet as a public function, expose the existing internal builder under that name in `crates/vox-compiler/src/codegen/mod.rs` (or wherever codegen entry points live) so the test can reach it.

- [ ] **Step 5: Run tests, verify they pass**

Run: `cargo test -p vox-compiler --test codegen_mobile`
Expected: PASS for both tests.

- [ ] **Step 6: Verify no other compiler tests regressed**

Run: `cargo test -p vox-compiler`
Expected: clean. If any tests assume `crate-type = ["rlib"]` on a default build, they should still pass because the default branch is preserved.

- [ ] **Step 7: Commit**

```bash
git add crates/vox-compiler/
git commit -m "feat(vox-compiler): emit cdylib+staticlib + mobile deps when target=mobile"
```

---

## Task 5: `vox mobile build --platform=android` via cargo-ndk

**Files:**
- Create: `crates/vox-mobile/src/build/mod.rs`
- Create: `crates/vox-mobile/src/build/android.rs`
- Create: `crates/vox-mobile/src/manifest_resolve.rs`
- Modify: `crates/vox-mobile/src/lib.rs`
- Modify: `crates/vox-mobile/src/main.rs`
- Test: `crates/vox-mobile/tests/build_android.rs`
- Test fixtures: `crates/vox-mobile/tests/fixtures/hello_mobile/`

**Goal:** From a project with `[build] target = "mobile"`, `vox mobile build --platform=android` cross-compiles the cdylib for every ABI listed in `[mobile.android].abis` and lays out the artifacts under `target/mobile/android/<abi>/libvox_app.so`.

- [ ] **Step 1: Create the test fixture**

Create `crates/vox-mobile/tests/fixtures/hello_mobile/Vox.toml`:

```toml
[package]
name = "hello_mobile"
version = "0.1.0"
kind = "application"

[build]
target = "mobile"

[mobile]
platforms = ["android"]

[mobile.android]
min_sdk = 26
target_sdk = 35
abis = ["aarch64-linux-android"]   # one ABI for fast tests
ndk_version = "27.0.11902837"
```

Create `crates/vox-mobile/tests/fixtures/hello_mobile/src/main.vox`:

```
// vox:skip
fn main() {
    println("hello from mobile");
}
```

(If a real runnable Vox source isn't viable for the fixture yet — for instance because the `target = "mobile"` lowering paths in vox-compiler aren't fully wired — substitute the Cargo.toml the compiler *would* emit by hand-writing it under `crates/vox-mobile/tests/fixtures/hello_mobile_emitted/Cargo.toml` and have the test invoke cargo-ndk against that directly. The goal of Task 5 is to validate the cargo-ndk wrapper, not the compiler.)

- [ ] **Step 2: Write the failing integration test**

Create `crates/vox-mobile/tests/build_android.rs`:

```rust
use assert_cmd::Command;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hello_mobile")
}

#[test]
fn build_android_produces_so_per_abi() {
    // Skip the test if cargo-ndk is not installed (developer machines without it).
    if which::which("cargo-ndk").is_err() {
        eprintln!("skipping: cargo-ndk not installed");
        return;
    }
    if std::env::var("ANDROID_NDK_HOME").is_err() {
        eprintln!("skipping: ANDROID_NDK_HOME not set");
        return;
    }

    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.current_dir(fixture_dir())
        .arg("build")
        .arg("--platform=android")
        .arg("--release")
        .assert()
        .success();

    let so = fixture_dir().join("target/mobile/android/aarch64-linux-android/libhello_mobile.so");
    assert!(so.exists(), "expected {} to exist", so.display());
}
```

- [ ] **Step 3: Run, expect failure**

Run: `cargo test -p vox-mobile --test build_android`
Expected: FAIL — `vox mobile build` is unimplemented (Task 1 placeholder).

- [ ] **Step 4: Implement manifest resolution**

Create `crates/vox-mobile/src/manifest_resolve.rs`:

```rust
//! Reads `Vox.toml` from the current working directory and validates it for mobile builds.

use anyhow::{Context, Result, bail};
use std::path::Path;
use vox_pm::manifest::{VoxManifest, validate_mobile};

pub fn load(project_dir: &Path) -> Result<VoxManifest> {
    let manifest_path = project_dir.join("Vox.toml");
    let toml_src = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: VoxManifest = toml::from_str(&toml_src)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let target = manifest.build.as_ref().and_then(|b| b.target.as_deref());
    if target != Some("mobile") {
        bail!(
            "expected [build] target = \"mobile\" in {}; got {:?}",
            manifest_path.display(),
            target
        );
    }
    validate_mobile(&manifest).context("validating [mobile] section")?;
    Ok(manifest)
}
```

- [ ] **Step 5: Implement the Android build orchestrator**

Create `crates/vox-mobile/src/build/mod.rs`:

```rust
pub mod android;
```

Create `crates/vox-mobile/src/build/android.rs`:

```rust
//! Android build path: cargo-ndk per ABI.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;
use vox_pm::manifest::AndroidConfig;

pub fn build(project_dir: &Path, android: &AndroidConfig, release: bool) -> Result<()> {
    if android.abis.is_empty() {
        bail!("[mobile.android].abis is empty; nothing to build");
    }
    let out_root = project_dir.join("target/mobile/android");
    std::fs::create_dir_all(&out_root).with_context(|| format!("creating {}", out_root.display()))?;

    for abi in &android.abis {
        eprintln!("[vox-mobile] building Android {abi}");
        let mut cmd = Command::new("cargo-ndk");
        cmd.current_dir(project_dir)
            .arg("--target").arg(abi)
            .arg("--platform").arg(android.min_sdk.unwrap_or(26).to_string())
            .arg("--output-dir").arg(out_root.join(abi))
            .arg("--")
            .arg("build");
        if release {
            cmd.arg("--release");
        }
        let status = cmd.status().with_context(|| format!("invoking cargo-ndk for {abi}"))?;
        if !status.success() {
            bail!("cargo-ndk build failed for ABI {abi}: exit {}", status.code().unwrap_or(-1));
        }
    }
    eprintln!("[vox-mobile] Android build complete: {}", out_root.display());
    Ok(())
}
```

- [ ] **Step 6: Wire build into the CLI**

Edit `crates/vox-mobile/src/lib.rs`:

```rust
pub mod build;
pub mod cli;
pub mod doctor;
pub mod manifest_resolve;
```

Edit `crates/vox-mobile/src/main.rs`:

```rust
use anyhow::{Result, bail};
use clap::Parser;
use std::env;
use vox_mobile::build;
use vox_mobile::cli::{Cli, Command};
use vox_mobile::doctor;
use vox_mobile::manifest_resolve;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => {
            let exit_code = doctor::run()?;
            std::process::exit(exit_code);
        }
        Command::Build { platform, release } => {
            let project_dir = env::current_dir()?;
            let manifest = manifest_resolve::load(&project_dir)?;
            let mobile = manifest.mobile.as_ref().expect("validated by manifest_resolve");

            match platform.as_str() {
                "android" => {
                    let android = mobile.android.as_ref()
                        .ok_or_else(|| anyhow::anyhow!("missing [mobile.android] section"))?;
                    build::android::build(&project_dir, android, release)?;
                }
                "ios" => {
                    bail!("ios build not yet implemented (Task 6)");
                }
                "all" => {
                    bail!("--platform=all not yet implemented (Task 7)");
                }
                other => bail!("unknown platform '{other}'; use android, ios, or all"),
            }
            Ok(())
        }
    }
}
```

- [ ] **Step 7: Run integration test**

Run: `cargo test -p vox-mobile --test build_android`
Expected: PASS if cargo-ndk + NDK are installed; SKIPPED with the printed message otherwise.

- [ ] **Step 8: Commit**

```bash
git add crates/vox-mobile/
git commit -m "feat(vox-mobile): implement android build via cargo-ndk per ABI"
```

---

## Task 6: `vox mobile build --platform=ios` via cargo + xcodebuild

**Files:**
- Create: `crates/vox-mobile/src/build/ios.rs`
- Modify: `crates/vox-mobile/src/build/mod.rs`
- Modify: `crates/vox-mobile/src/main.rs`
- Test: `crates/vox-mobile/tests/build_ios.rs`

**Goal:** On macOS, `vox mobile build --platform=ios` cross-compiles the cdylib for each iOS arch and assembles them into a single `target/mobile/ios/VoxApp.xcframework` via `xcodebuild -create-xcframework`.

- [ ] **Step 1: Write the failing test**

Create `crates/vox-mobile/tests/build_ios.rs`:

```rust
use assert_cmd::Command;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hello_mobile")
}

#[cfg(target_os = "macos")]
#[test]
fn build_ios_produces_xcframework() {
    if which::which("xcodebuild").is_err() {
        eprintln!("skipping: xcodebuild not installed");
        return;
    }

    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.current_dir(fixture_dir())
        .arg("build")
        .arg("--platform=ios")
        .arg("--release")
        .assert()
        .success();

    let xcf = fixture_dir().join("target/mobile/ios/VoxApp.xcframework");
    assert!(xcf.exists(), "expected {} to exist", xcf.display());
    let info = xcf.join("Info.plist");
    assert!(info.exists(), "expected XCFramework Info.plist");
}

#[cfg(not(target_os = "macos"))]
#[test]
fn build_ios_fails_clearly_on_non_macos() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    let output = cmd.current_dir(fixture_dir())
        .arg("build")
        .arg("--platform=ios")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("macOS"), "expected macOS gate error; got: {stderr}");
}
```

Add `[mobile.ios]` to the fixture `Vox.toml`:

```toml
[mobile.ios]
min_version = "15.0"
archs = ["aarch64-apple-ios"]    # one arch for fast tests
```

And add `"ios"` to `[mobile].platforms`.

- [ ] **Step 2: Run test, expect failure**

Run: `cargo test -p vox-mobile --test build_ios`
Expected: on macOS, FAIL because `--platform=ios` currently bails. On other OSes, PASS for the `not(target_os = "macos")` branch — but only after Step 3 implements the gate cleanly.

- [ ] **Step 3: Implement the iOS build orchestrator**

Create `crates/vox-mobile/src/build/ios.rs`:

```rust
//! iOS build path: cargo build per arch + xcodebuild -create-xcframework.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;
use vox_pm::manifest::IosConfig;

pub fn build(project_dir: &Path, ios: &IosConfig, release: bool) -> Result<()> {
    if !cfg!(target_os = "macos") {
        bail!("iOS builds require macOS; current OS is {}", std::env::consts::OS);
    }
    if ios.archs.is_empty() {
        bail!("[mobile.ios].archs is empty; nothing to build");
    }

    let out_root = project_dir.join("target/mobile/ios");
    std::fs::create_dir_all(&out_root).with_context(|| format!("creating {}", out_root.display()))?;

    let mut slice_dirs: Vec<(PathBuf, String)> = Vec::new();
    for arch in &ios.archs {
        eprintln!("[vox-mobile] building iOS {arch}");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(project_dir)
            .arg("build")
            .arg("--target").arg(arch)
            .arg("--lib");
        if release {
            cmd.arg("--release");
        }
        let status = cmd.status().with_context(|| format!("invoking cargo build for {arch}"))?;
        if !status.success() {
            bail!("cargo build failed for arch {arch}: exit {}", status.code().unwrap_or(-1));
        }
        let profile = if release { "release" } else { "debug" };
        let staticlib = project_dir.join("target").join(arch).join(profile).join("libvox_app.a");
        if !staticlib.exists() {
            bail!("expected staticlib {} after cargo build; not found", staticlib.display());
        }
        slice_dirs.push((staticlib, arch.clone()));
    }

    // Assemble the XCFramework.
    let xcf_path = out_root.join("VoxApp.xcframework");
    if xcf_path.exists() {
        std::fs::remove_dir_all(&xcf_path).context("clearing previous XCFramework")?;
    }
    let mut cmd = Command::new("xcodebuild");
    cmd.arg("-create-xcframework");
    for (lib, _arch) in &slice_dirs {
        cmd.arg("-library").arg(lib);
    }
    cmd.arg("-output").arg(&xcf_path);
    let status = cmd.status().context("invoking xcodebuild -create-xcframework")?;
    if !status.success() {
        bail!("xcodebuild -create-xcframework failed: exit {}", status.code().unwrap_or(-1));
    }
    eprintln!("[vox-mobile] iOS build complete: {}", xcf_path.display());
    Ok(())
}
```

Update `crates/vox-mobile/src/build/mod.rs`:

```rust
pub mod android;
pub mod ios;
```

Update `crates/vox-mobile/src/main.rs` — replace the `"ios"` arm:

```rust
"ios" => {
    let ios = mobile.ios.as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing [mobile.ios] section"))?;
    build::ios::build(&project_dir, ios, release)?;
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test -p vox-mobile --test build_ios`
Expected: PASS on both macOS (full test) and non-macOS (clean error gate).

- [ ] **Step 5: Commit**

```bash
git add crates/vox-mobile/
git commit -m "feat(vox-mobile): implement ios build via cargo + xcodebuild XCFramework"
```

---

## Task 7: `vox mobile build --platform=all` orchestration

**Files:**
- Modify: `crates/vox-mobile/src/main.rs`
- Test: `crates/vox-mobile/tests/build_all.rs`

**Goal:** `--platform=all` (the default) builds whichever platforms are listed in `[mobile.platforms]`. On non-macOS, iOS is skipped with a warning, not a hard error.

- [ ] **Step 1: Write the failing test**

Create `crates/vox-mobile/tests/build_all.rs`:

```rust
use assert_cmd::Command;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hello_mobile")
}

#[test]
fn build_all_runs_all_listed_platforms() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    let output = cmd.current_dir(fixture_dir())
        .arg("build")
        .arg("--platform=all")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The binary must print a per-platform line for each requested platform.
    assert!(stderr.contains("Android") || stderr.contains("android"), "should report Android attempt; got: {stderr}");
    #[cfg(not(target_os = "macos"))]
    {
        assert!(stderr.contains("skipping iOS"), "iOS should be skipped on non-macOS; got: {stderr}");
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cargo test -p vox-mobile --test build_all`
Expected: FAIL (`--platform=all` currently bails).

- [ ] **Step 3: Implement the orchestration**

Replace the `"all"` arm in `crates/vox-mobile/src/main.rs`:

```rust
"all" => {
    for platform in &mobile.platforms {
        match platform.as_str() {
            "android" => {
                if let Some(android) = mobile.android.as_ref() {
                    eprintln!("[vox-mobile] === Android ===");
                    if let Err(e) = build::android::build(&project_dir, android, release) {
                        eprintln!("[vox-mobile] Android build failed: {e:#}");
                    }
                }
            }
            "ios" => {
                if !cfg!(target_os = "macos") {
                    eprintln!("[vox-mobile] skipping iOS: requires macOS");
                    continue;
                }
                if let Some(ios) = mobile.ios.as_ref() {
                    eprintln!("[vox-mobile] === iOS ===");
                    if let Err(e) = build::ios::build(&project_dir, ios, release) {
                        eprintln!("[vox-mobile] iOS build failed: {e:#}");
                    }
                }
            }
            _ => unreachable!("validated by validate_mobile"),
        }
    }
}
```

- [ ] **Step 4: Run, verify**

Run: `cargo test -p vox-mobile --test build_all`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-mobile/
git commit -m "feat(vox-mobile): orchestrate --platform=all across listed platforms"
```

---

## Task 8: Plugin discovery integration test

**Files:**
- Create: `crates/vox-mobile/tests/plugin_dispatch.rs`

**Goal:** Verify the main `vox` binary discovers `vox-mobile` on `PATH` and dispatches `vox mobile <args>` to it.

- [ ] **Step 1: Write the test**

Create `crates/vox-mobile/tests/plugin_dispatch.rs`:

```rust
use assert_cmd::Command;
use std::env;

#[test]
fn vox_dispatches_mobile_subcommand_to_plugin() {
    // Build vox-mobile so it's available, then invoke `vox mobile --help`
    // with a PATH that includes the vox-mobile binary.
    let bin_dir = env!("CARGO_TARGET_DIR")
        .parse::<std::path::PathBuf>()
        .ok()
        .or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .find_map(|a| {
                    let candidate = a.join("target/debug");
                    candidate.exists().then(|| candidate)
                })
        })
        .expect("could not locate target/debug");

    let path = env::var("PATH").unwrap_or_default();
    let new_path = format!("{}{}{}", bin_dir.display(), if cfg!(windows) { ';' } else { ':' }, path);

    let mut cmd = Command::cargo_bin("vox").unwrap();
    cmd.env("PATH", new_path)
        .arg("mobile")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("doctor"));
}
```

- [ ] **Step 2: Run**

Run: `cargo build -p vox-mobile && cargo test -p vox-mobile --test plugin_dispatch`
Expected: PASS. If vox's plugin dispatch is broken or differs from the README claim, the failure surfaces here.

- [ ] **Step 3: Commit**

```bash
git add crates/vox-mobile/tests/plugin_dispatch.rs
git commit -m "test(vox-mobile): verify plugin discovery dispatches vox mobile to vox-mobile"
```

---

## Task 9: Documentation

**Files:**
- Create: `docs/src/reference/vox-mobile-cli.md`
- Create: `docs/src/how-to/vox-mobile-doctor.md`

**Goal:** Two short docs: a CLI reference for `vox mobile *` subcommands, and a troubleshooting page for the doctor's findings.

- [ ] **Step 1: Write the CLI reference**

Create `docs/src/reference/vox-mobile-cli.md`:

```markdown
---
title: "vox mobile CLI Reference"
description: "Reference for the vox mobile plugin: doctor, build subcommands and the [build] / [mobile] manifest sections."
category: "reference"
status: "current"
training_eligible: true
training_rationale: "Stable CLI reference for the vox-mobile plugin."
---

# `vox mobile` CLI Reference

Cross-compile a Vox project for Android and/or iOS. Implemented by the `vox-mobile` plugin binary, discovered on `PATH` per the plugin model documented in [README.md](../../../README.md).

## Manifest

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

## `vox mobile doctor`

Detects local toolchain prerequisites and prints a per-platform readiness table. See [How-To: `vox mobile doctor`](../how-to/vox-mobile-doctor.md) for interpreting the output.

Exits 0 if at least one platform is fully configured; exits 1 otherwise.

## `vox mobile build [--platform <platform>] [--release]`

Cross-compile for the specified platform.

- `--platform android` — runs cargo-ndk per ABI; outputs `target/mobile/android/<abi>/lib<crate>.so`.
- `--platform ios` — macOS only; runs cargo per arch + `xcodebuild -create-xcframework`; outputs `target/mobile/ios/VoxApp.xcframework`.
- `--platform all` (default) — runs every platform listed in `[mobile.platforms]`. Skips iOS with a warning on non-macOS.

## Spec

This CLI implements Phase 1 of [Vox Mobile Plugin Spec (2026)](../architecture/vox-mobile-plugin-spec-2026.md).
```

- [ ] **Step 2: Write the doctor troubleshooting page**

Create `docs/src/how-to/vox-mobile-doctor.md`:

```markdown
---
title: "How to interpret vox mobile doctor output"
description: "Troubleshooting guide for the vox mobile doctor toolchain checks."
category: "how-to"
status: "current"
training_eligible: true
training_rationale: "Stable troubleshooting reference for the vox-mobile plugin."
---

# How to interpret `vox mobile doctor`

`vox mobile doctor` prints one row per checked tool. `[OK]` means present; `[--]` means missing and prints an `install hint`.

## Android prerequisites

| Check | Install hint |
|---|---|
| `cargo-ndk` | `cargo install cargo-ndk` |
| `ANDROID_NDK_HOME` | Install Android NDK r27 via Android Studio SDK Manager and `export ANDROID_NDK_HOME=<path>`. |
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

## Exit codes

- **0** — at least one platform is fully configured. `vox mobile build` for that platform should succeed.
- **1** — no platform is fully configured. Install at least one platform's prerequisites and re-run.

## Spec

This subcommand implements part of Phase 1 of [Vox Mobile Plugin Spec (2026)](../architecture/vox-mobile-plugin-spec-2026.md).
```

- [ ] **Step 3: Regenerate doc indexes**

Run: `cargo run -p vox-doc-pipeline`
Expected: `SUMMARY.md` and `architecture-index.md` (if those new docs trigger inclusion) updated.

- [ ] **Step 4: Verify CI doc-pipeline check passes**

Run: `cargo run -p vox-doc-pipeline -- --check`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add docs/
git commit -m "docs(vox-mobile): add CLI reference and doctor troubleshooting"
```

---

## Task 10: Final integration check

- [ ] **Step 1: Run the full workspace test suite**

Run: `cargo test --workspace`
Expected: all tests pass (or skip gracefully when toolchain is unavailable).

- [ ] **Step 2: Run clippy across the new crate**

Run: `cargo clippy -p vox-mobile -- -D warnings`
Expected: clean.

- [ ] **Step 3: Verify the README plugin table is accurate**

Open `README.md`. Confirm `vox-mobile` is listed in the plugins table alongside `vox-mens` and `vox-schola`. If not, add it:

```markdown
| `vox-mobile` | `vox mobile`, `vox mobile doctor`, `vox mobile build` | Cross-compiles Vox apps for Android (cargo-ndk) and iOS (XCFramework). Ships its own toolchain checks. |
```

- [ ] **Step 4: Commit any README change**

```bash
git add README.md
git commit -m "docs(README): list vox-mobile in plugin table"
```

- [ ] **Step 5: Verify doc pipeline check is still green**

Run: `cargo run -p vox-doc-pipeline -- --check`
Expected: clean.

---

## Self-Review

**Spec coverage:** All Phase 1 deliverables from [vox-mobile-plugin-spec-2026.md §Phase 1](../../src/architecture/vox-mobile-plugin-spec-2026.md#phase-1--cdylib-build-target-for-android-and-ios) are covered: manifest schema (Task 2), cdylib lowering (Task 4), Android cargo-ndk (Task 5), iOS xcodebuild XCFramework (Task 6), `vox mobile build` orchestration (Tasks 5–7), `vox mobile doctor` (Task 3), golden test fixtures (Tasks 5, 6), reference docs (Task 9). Plugin-discovery integration (Task 8) is a sanity check on the existing `vox` PATH-dispatch mechanism.

**Placeholder scan:** Task 4 contains a "locate first" investigative step (Step 1) because the exact crate-type emission site needs to be confirmed in the codebase. This is acceptable — the task body is concrete; only the file path is to-be-discovered. Task 5 Step 1 has a fallback note for the case where the compiler's `target = "mobile"` lowering isn't yet wired enough to emit a runnable crate; this is realistic given the test-first ordering and gives the implementer a clear escape hatch.

**Type consistency:** `MobileSection.android` is `Option<AndroidConfig>` in Task 2 and dereferenced as such in Tasks 5/7. `BuildSection.target` is `Option<String>` and matched as `Some("mobile")` consistently. `cargo_toml_for_manifest` is named identically in Task 4 test and implementation references.

**Out-of-scope clarifications:** This plan does NOT implement: host-shell FFI contract (Phase 2), Clavis mobile sources (Phase 3), Codex `bundled-sqlcipher` (Phase 4), reminder runtime (Phase 5), `vox mobile init/run/sign/package` (Phase 6). Those are separate plans.

---

## Phases 2–6 — outline only

Following plans (one per Phase from the spec) would mirror this structure. Sketch for the engineer drafting them next:

- **Phase 2 — Host-shell contract + bindgen:** define `contracts/mobile/host-shell.v1.yaml`, write a YAML→Rust/Kotlin/Swift codegen in `crates/vox-mobile/src/bindgen/`, add golden tests for re-emit stability. ~10 tasks.
- **Phase 3 — Mobile Clavis sources:** add `crates/vox-clavis/src/sources/{android_keystore.rs,ios_keychain.rs,passphrase_argon2id.rs}`, add `vox.mobile.db_key` to spec.rs, extend `vox clavis doctor`. ~6 tasks.
- **Phase 4 — Codex `bundled-sqlcipher`:** add feature flag to `vox-db`, add `with_encryption_key` connection builder, add manifest `[storage] encryption` parsing, add `vox db rekey` and `vox db {export,restore}-encrypted` subcommands. ~8 tasks.
- **Phase 5 — Reminder runtime:** add `vox-stdlib::reminder` module, change-notification hook in Codex (verify exists), RRULE next-occurrence helper, configurable table/predicate/handler from `[mobile.reminders]`. ~7 tasks.
- **Phase 6 — `vox mobile init/run/sign/package` + docs + templates:** scaffold generator, debug keystore wrapper, apksigner integration, F-Droid metadata generator, three docs (init tutorial, encryption how-to, full CLI reference). ~10 tasks.
