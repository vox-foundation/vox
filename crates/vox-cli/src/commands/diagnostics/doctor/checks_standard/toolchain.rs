//! Core toolchain checks: Rust, Node, pnpm, mesh, Docker, WASI, …

use tokio::process::Command;

use super::super::common::Check;

/// Whether Node.js / pnpm are optional for the given install tier, per
/// `contracts/distribution/profiles.v1.yaml`.
///
/// `minimal` (`vox-langtool`) is a language toolchain only — check, fmt, run,
/// build — with no frontend bundling, so Node/pnpm are not needed. `default`
/// and `full` both ship the GUI (Axis) and/or fullstack scaffolding, which
/// bundle a frontend with pnpm, so both are required there. An unknown tier
/// is treated as `default` (required) rather than silently passing everything.
fn node_is_optional_for(tier: &str) -> bool {
    tier == "minimal"
}

pub async fn run(auto_heal: bool, tier: &str, checks: &mut Vec<Check>) {
    let cargo = Command::new("cargo").arg("--version").output().await;
    checks.push(match cargo {
        Ok(o) if o.status.success() => Check {
            name: "Rust / Cargo".to_string(),
            pass: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
        },
        _ => Check {
            name: "Rust / Cargo".to_string(),
            pass: false,
            detail: "not found — install from https://rustup.rs".to_string(),
        },
    });

    let node_optional = node_is_optional_for(tier);
    let node = Command::new("node").arg("--version").output().await;
    checks.push(match node {
        Ok(o) if o.status.success() => Check {
            name: if node_optional {
                "Node.js (optional)".to_string()
            } else {
                "Node.js".to_string()
            },
            pass: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
        },
        _ if node_optional => Check {
            name: "Node.js (optional)".to_string(),
            pass: true,
            detail: "not installed — not needed for this tier (language toolchain only, no frontend bundling)".to_string(),
        },
        _ => Check {
            name: "Node.js".to_string(),
            pass: false,
            detail: "not found — install from https://nodejs.org".to_string(),
        },
    });

    let pnpm_exe = if cfg!(target_os = "windows") {
        "pnpm.cmd"
    } else {
        "pnpm"
    };
    let pnpm = Command::new(pnpm_exe).arg("--version").output().await;
    let pnpm_pass = matches!(&pnpm, Ok(o) if o.status.success());
    let mut pnpm_detail = if pnpm_pass {
        format!("v{}", String::from_utf8_lossy(&pnpm.unwrap().stdout).trim())
    } else if node_optional {
        "not installed — not needed for this tier (language toolchain only, no frontend bundling)"
            .to_string()
    } else {
        "not found — run: npm install -g pnpm".to_string()
    };

    let mut actual_pnpm_pass = pnpm_pass;
    if !pnpm_pass && auto_heal && !node_optional {
        println!("  [auto-heal] Installing pnpm...");
        let npm_exe = if cfg!(target_os = "windows") {
            "npm.cmd"
        } else {
            "npm"
        };
        if Command::new(npm_exe)
            .args(["install", "-g", "pnpm"])
            .status()
            .await
            .is_ok_and(|s| s.success())
        {
            actual_pnpm_pass = true;
            pnpm_detail = "installed successfully via auto-heal".to_string();
        } else {
            pnpm_detail = "auto-heal failed: could not run npm install pnpm".to_string();
        }
    }

    checks.push(Check {
        name: if node_optional {
            "pnpm (optional)".to_string()
        } else {
            "pnpm".to_string()
        },
        pass: actual_pnpm_pass || node_optional,
        detail: pnpm_detail,
    });

    let git = // vox-arch-check: allow git-exec
        Command::new("git").arg("--version").output().await;
    checks.push(match git {
        Ok(o) if o.status.success() => Check {
            name: "Git".to_string(),
            pass: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
        },
        _ => Check {
            name: "Git".to_string(),
            pass: false,
            detail: "not found — install from https://git-scm.com".to_string(),
        },
    });

    let mesh_mode = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshMode)
        .expose()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "lan".to_string());
    let mesh_token_set = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshToken)
        .expose()
        .is_some_and(|v| !v.trim().is_empty());
    let mesh_scope_set = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshScopeId)
        .expose()
        .is_some_and(|v| !v.trim().is_empty());
    checks.push(Check {
        name: "Populi mesh security defaults".to_string(),
        pass: mesh_token_set && mesh_scope_set,
        detail: format!(
            "mode={mesh_mode}, token_set={mesh_token_set}, scope_set={mesh_scope_set} (recommended: `vox populi up`)"
        ),
    });

    let tailscale_ok = Command::new("tailscale")
        .arg("version")
        .output()
        .await
        .is_ok_and(|o| o.status.success());
    let wireguard_ok = Command::new("wg")
        .arg("show")
        .output()
        .await
        .is_ok_and(|o| o.status.success());
    let tunnel_ok = Command::new("cloudflared")
        .arg("--version")
        .output()
        .await
        .is_ok_and(|o| o.status.success())
        || Command::new("ngrok")
            .arg("version")
            .output()
            .await
            .is_ok_and(|o| o.status.success());
    checks.push(Check {
        name: "Overlay networking (optional)".to_string(),
        pass: true,
        detail: format!(
            "tailscale={}, wireguard={}, tunnel={} (use `vox populi status` for deeper diagnostics)",
            if tailscale_ok { "ok" } else { "missing" },
            if wireguard_ok { "ok" } else { "missing" },
            if tunnel_ok { "ok" } else { "missing" }
        ),
    });

    let docker = Command::new("docker").arg("--version").output().await;
    checks.push(match docker {
        Ok(o) if o.status.success() => Check {
            name: "Docker".to_string(),
            pass: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
        },
        _ => Check {
            name: "Docker (optional)".to_string(),
            pass: true,
            detail: "not installed — install from https://docs.docker.com/get-docker/".to_string(),
        },
    });

    let podman = Command::new("podman").arg("--version").output().await;
    checks.push(match podman {
        Ok(o) if o.status.success() => Check {
            name: "Podman".to_string(),
            pass: true,
            detail: format!("{} (rootless)", String::from_utf8_lossy(&o.stdout).trim()),
        },
        _ => Check {
            name: "Podman (optional)".to_string(),
            pass: true,
            detail: "not installed — install from https://podman.io/getting-started/installation"
                .to_string(),
        },
    });

    checks.push(interpreter_default_for_script_shaped());
    checks.push(optional_binary_check(
        "Cargo (optional for scripts)",
        probe_version("cargo").await,
    ));
    checks.push(optional_binary_check(
        "rustc (optional for scripts)",
        probe_version("rustc").await,
    ));
    checks.push(wasm32_wasip1_not_required(
        probe_wasm32_wasip1_installed().await,
    ));
    checks.push(executor_accepts_caps(probe_run_help().await.as_deref()));

    let zig = Command::new("zig").arg("version").output().await;
    checks.push(match zig {
        Ok(o) if o.status.success() => Check {
            name: "Zig (optional)".to_string(),
            pass: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
        },
        _ => Check {
            name: "Zig (optional)".to_string(),
            pass: true,
            detail: "not found — suggested for advanced cross-compilation (https://ziglang.org)"
                .to_string(),
        },
    });

    #[cfg(target_os = "linux")]
    {
        let mold = Command::new("mold").arg("--version").output().await;
        checks.push(match mold {
            Ok(o) if o.status.success() => Check {
                name: "mold linker".to_string(),
                pass: true,
                detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
            },
            _ => Check {
                name: "mold linker".to_string(),
                pass: false,
                detail: "not found — recommended for fast Linux builds: sudo apt install mold"
                    .to_string(),
            },
        });

        let cranelift = Command::new("rustup")
            .args(["component", "list", "--installed"])
            .output()
            .await;
        let has_cranelift = matches!(&cranelift, Ok(o) if String::from_utf8_lossy(&o.stdout).contains("rustc-codegen-cranelift-preview"));
        checks.push(Check {
            name: "Cranelift codegen".to_string(),
            pass: has_cranelift,
            detail: if has_cranelift {
                "installed".to_string()
            } else {
                "not installed — recommended for fast dev builds: rustup component add rustc-codegen-cranelift-preview".to_string()
            },
        });
    }
}

/// Same predicate `vox run --mode auto` uses for script-shaped files.
fn interpreter_default_for_script_shaped() -> Check {
    let sample = "pub fn main() { print(1) }";
    let shaped = crate::commands::runtime::run::run::is_script_shaped(sample);
    Check::new(
        "Interpreter default (script-shaped)",
        shaped,
        if shaped {
            "script-shaped files (`fn main()`, no service surfaces) default to the interpreter; no cargo"
                .to_string()
        } else {
            "is_script_shaped rejected a canonical script-shaped sample — routing is broken"
                .to_string()
        },
    )
}

fn optional_binary_check(name: &str, version: Option<String>) -> Check {
    Check::pass(
        name,
        match version.as_deref() {
            Some(v) => format!("on PATH: {v} (not required for scripts)"),
            None => "not on PATH — not required for scripts".to_string(),
        },
    )
}

fn wasm32_wasip1_not_required(installed: Option<bool>) -> Check {
    Check::pass(
        "wasm32-wasip1 (not required)",
        match installed {
            Some(true) => "installed; unused — scripts run under the interpreter",
            Some(false) => "not installed — not required",
            None => "rustup not on PATH — target not required",
        },
    )
}

fn run_help_lists_caps(help: &str) -> bool {
    help.contains("--caps")
}

fn executor_accepts_caps(help: Option<&str>) -> Check {
    match help {
        Some(h) if run_help_lists_caps(h) => {
            Check::pass("Executor accepts --caps", "vox run --help lists --caps")
        }
        Some(_) => Check::fail(
            "Executor accepts --caps",
            "vox run --help does not list --caps",
        ),
        None => Check::fail("Executor accepts --caps", "could not run `vox run --help`"),
    }
}

async fn probe_version(bin: &str) -> Option<String> {
    let out = Command::new(bin).arg("--version").output().await.ok()?;
    out.status
        .success()
        .then(|| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
}

async fn probe_wasm32_wasip1_installed() -> Option<bool> {
    let out = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .await
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).contains("wasm32-wasip1"))
}

async fn probe_run_help() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let out = Command::new(&exe)
        .args(["run", "--help"])
        .output()
        .await
        .ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // clap prints help before enforcing the required `file` arg.
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tier_severity_tests {
    use super::node_is_optional_for;

    #[test]
    fn minimal_tier_makes_node_and_pnpm_optional() {
        assert!(node_is_optional_for("minimal"));
    }

    #[test]
    fn default_and_full_tiers_require_node_and_pnpm() {
        assert!(!node_is_optional_for("default"));
        assert!(!node_is_optional_for("full"));
    }

    #[test]
    fn unknown_tier_is_treated_as_default_never_crashes_never_passes_everything() {
        assert!(!node_is_optional_for(""));
        assert!(!node_is_optional_for("not-a-real-tier"));
    }
}

#[cfg(test)]
mod interpreter_isolation_probes {
    use super::{
        executor_accepts_caps, interpreter_default_for_script_shaped, optional_binary_check,
        run_help_lists_caps, wasm32_wasip1_not_required,
    };

    #[test]
    fn interpreter_default_probes_the_script_shaped_predicate() {
        let c = interpreter_default_for_script_shaped();
        assert!(c.pass, "{}", c.detail);
        assert_eq!(c.name, "Interpreter default (script-shaped)");
        assert!(c.detail.contains("interpreter"));
    }

    #[test]
    fn cargo_and_rustc_rows_pass_whether_or_not_the_binary_is_on_path() {
        let found =
            optional_binary_check("Cargo (optional for scripts)", Some("cargo 1.96.0".into()));
        let missing = optional_binary_check("Cargo (optional for scripts)", None);
        assert!(found.pass && missing.pass);
        assert!(found.detail.contains("on PATH"));
        assert!(missing.detail.contains("not on PATH"));
    }

    #[test]
    fn wasm32_wasip1_row_always_passes_and_reports_the_probe() {
        let yes = wasm32_wasip1_not_required(Some(true));
        let no = wasm32_wasip1_not_required(Some(false));
        let no_rustup = wasm32_wasip1_not_required(None);
        assert!(yes.pass && no.pass && no_rustup.pass);
        assert!(yes.detail.contains("installed"));
        assert!(no.detail.contains("not installed"));
        assert!(no_rustup.detail.contains("rustup not on PATH"));
    }

    #[test]
    fn executor_caps_row_fails_when_help_omits_the_flag() {
        assert!(run_help_lists_caps("    --caps <CAPS>\n"));
        assert!(!run_help_lists_caps("    --mode <MODE>\n"));
        let ok = executor_accepts_caps(Some("Usage: vox run --caps <CAPS>\n"));
        let missing = executor_accepts_caps(Some("Usage: vox run --mode <MODE>\n"));
        let no_help = executor_accepts_caps(None);
        assert!(ok.pass);
        assert!(!missing.pass);
        assert!(!no_help.pass);
    }
}
