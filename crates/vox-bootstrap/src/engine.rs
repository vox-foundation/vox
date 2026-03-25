//! Bootstrap evaluation: probe host toolchain; optional `--apply` runs low-risk heals.

use std::io::{self, Write};
use std::process::Command;

use crate::report::{BootstrapItem, BootstrapReport};

/// CLI-driven options for probing (and optionally fixing) the host toolchain.
#[derive(Debug, Clone, Copy)]
pub struct BootstrapOptions {
    /// Include dev probes (`rustfmt`, `clippy`).
    pub dev: bool,
    /// Treat LLVM/Clang as required on Windows (Turso / aegis native builds).
    pub install_clang: bool,
    /// Run safe heals (`rustup component add`, etc.).
    pub apply: bool,
    /// Install the vox CLI via cargo after successful checks.
    pub install: bool,
}

fn platform_str() -> String {
    if cfg!(target_os = "windows") {
        "windows".into()
    } else if cfg!(target_os = "linux") {
        "linux".into()
    } else if cfg!(target_os = "macos") {
        "macos".into()
    } else {
        "other".into()
    }
}

fn run_cmd(bin: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if detail.is_empty() {
            format!("exit code {}", out.status)
        } else {
            detail
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Evaluate all probes. Mutates the system only when `opts.apply` runs component installs.
#[must_use]
pub fn evaluate(opts: BootstrapOptions) -> BootstrapReport {
    let mut items = Vec::new();

    let rustc_ok = run_cmd("rustc", &["--version"]);
    items.push(BootstrapItem {
        id: "rustc",
        description: "Rust compiler (`rustc --version`)",
        required: true,
        ok: rustc_ok.is_ok(),
        detail: rustc_ok.unwrap_or_else(|e| e),
        heal_command: Some("https://rustup.rs/ — then open a new shell".to_string()),
    });

    let cargo_ok = run_cmd("cargo", &["--version"]);
    items.push(BootstrapItem {
        id: "cargo",
        description: "Cargo (`cargo --version`)",
        required: true,
        ok: cargo_ok.is_ok(),
        detail: cargo_ok.unwrap_or_else(|e| e),
        heal_command: Some("rustup default stable (or reinstall from rustup.rs)".to_string()),
    });

    if opts.dev {
        let fmt_ok = run_cmd("rustfmt", &["--version"]);
        let fmt_heal = "rustup component add rustfmt";
        if opts.apply && fmt_ok.is_err() {
            let _ = Command::new("rustup")
                .args(["component", "add", "rustfmt"])
                .status();
        }
        let fmt_ok_after = run_cmd("rustfmt", &["--version"]);
        items.push(BootstrapItem {
            id: "rustfmt",
            description: "rustfmt (`rustfmt --version`)",
            required: false,
            ok: fmt_ok_after.is_ok(),
            detail: fmt_ok_after.unwrap_or_else(|e| e),
            heal_command: Some(fmt_heal.to_string()),
        });

        let clippy_out = Command::new("cargo").args(["clippy", "--version"]).output();
        let (ok_before, detail_before) = match &clippy_out {
            Ok(o) if o.status.success() => {
                (true, String::from_utf8_lossy(&o.stdout).trim().to_string())
            }
            Ok(o) => (false, String::from_utf8_lossy(&o.stderr).trim().to_string()),
            Err(e) => (false, e.to_string()),
        };
        if opts.apply && !ok_before {
            let _ = Command::new("rustup")
                .args(["component", "add", "clippy"])
                .status();
        }
        let clippy_after = Command::new("cargo").args(["clippy", "--version"]).output();
        let (ok, detail) = match clippy_after {
            Ok(o) if o.status.success() => {
                (true, String::from_utf8_lossy(&o.stdout).trim().to_string())
            }
            Ok(_) => (false, detail_before.clone()),
            Err(e) => (false, e.to_string()),
        };
        items.push(BootstrapItem {
            id: "clippy",
            description: "Clippy (`cargo clippy --version`)",
            required: false,
            ok,
            detail,
            heal_command: Some("rustup component add clippy".to_string()),
        });
    }

    if cfg!(target_os = "windows") {
        let clang =
            run_cmd("clang-cl", &["--version"]).or_else(|_| run_cmd("clang", &["--version"]));
        items.push(BootstrapItem {
            id: "turso_clang",
            description: "LLVM/Clang for Turso aegis (`clang-cl` or `clang` on PATH)",
            required: opts.install_clang,
            ok: clang.is_ok(),
            detail: clang.unwrap_or_else(|e| e),
            heal_command: Some(
                "winget install -e LLVM.LLVM — then add LLVM\\bin to PATH and restart the shell"
                    .to_string(),
            ),
        });
    }

    BootstrapReport {
        platform: platform_str(),
        items,
    }
}

/// Print a human-readable report; returns process exit code (`0` = all required probes passed).
pub fn run_and_print(opts: BootstrapOptions, w: &mut impl Write) -> io::Result<i32> {
    let report = evaluate(opts);
    writeln!(w, "Platform: {}", report.platform)?;
    for item in &report.items {
        let status = if item.ok { "OK" } else { "FAIL" };
        writeln!(w, "  [{status}] {} — {}", item.description, item.detail)?;
        if !item.ok
            && let Some(ref h) = item.heal_command
        {
            writeln!(w, "       hint: {h}")?;
        }
    }
    let ok = report.required_ok();
    if ok && opts.install {
        writeln!(w, "\nDependencies met. Installing vox-cli...")?;
        let status = Command::new("cargo")
            .args(["install", "--path", "crates/vox-cli"])
            .status()?;
        if !status.success() {
            writeln!(w, "Failed to install vox-cli")?;
            return Ok(1);
        }
        writeln!(w, "vox-cli installed successfully!")?;
    }
    Ok(if ok { 0 } else { 1 })
}
