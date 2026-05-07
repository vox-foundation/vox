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
/// Returns 0 if at least one platform is fully configured, 1 otherwise.
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
        check_env(
            "ANDROID_NDK_HOME",
            "Install Android NDK r27 via Android Studio SDK Manager and `export ANDROID_NDK_HOME=<path>`",
        ),
    ];
    for target in [
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "x86_64-linux-android",
    ] {
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
    let mut checks = vec![check_executable(
        "xcodebuild",
        "Install Xcode Command Line Tools: `xcode-select --install`",
    )];
    for target in [
        "aarch64-apple-ios",
        "aarch64-apple-ios-sim",
        "x86_64-apple-ios",
    ] {
        checks.push(check_rustup_target(target));
    }
    checks
}

fn check_executable(name: &str, hint: &str) -> Check {
    let status = if which::which(name).is_ok() {
        Status::Present
    } else {
        Status::Missing
    };
    Check {
        name: name.into(),
        status,
        install_hint: hint.into(),
    }
}

fn check_env(var: &str, hint: &str) -> Check {
    let status = match env::var(var) {
        Ok(v) if !v.trim().is_empty() && PathBuf::from(&v).exists() => Status::Present,
        _ => Status::Missing,
    };
    Check {
        name: var.into(),
        status,
        install_hint: hint.into(),
    }
}

fn check_rustup_target(target: &str) -> Check {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();
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
