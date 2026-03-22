//! Default `vox doctor` checks: optional test-health tools, then full toolchain audit.

use tokio::process::Command;

use super::common::{self, AuthRegistriesOnly, Check};
use super::provider_policy::{ProviderPolicyEngine, ProviderSupportLevel};

pub async fn run_checks(auto_heal: bool, test_health: bool, checks: &mut Vec<Check>) {
    if test_health {
        println!("Running test health analysis...");

        let test_compile = Command::new("cargo")
            .args(["test", "--workspace", "--no-run"])
            .output()
            .await;

        checks.push(match test_compile {
            Ok(o) if o.status.success() => Check {
                name: "Test Compilation".to_string(),
                pass: true,
                detail: "all tests compile successfully".to_string(),
            },
            Ok(o) => Check {
                name: "Test Compilation".to_string(),
                pass: false,
                detail: format!(
                    "compilation failed:\n{}",
                    String::from_utf8_lossy(&o.stderr)
                ),
            },
            Err(e) => Check {
                name: "Test Compilation".to_string(),
                pass: false,
                detail: format!("failed to invoke cargo: {}", e),
            },
        });

        let llvm_cov = Command::new("cargo")
            .arg("llvm-cov")
            .arg("--version")
            .output()
            .await;
        checks.push(match llvm_cov {
            Ok(o) if o.status.success() => Check {
                name: "cargo-llvm-cov".to_string(),
                pass: true,
                detail: "found".to_string(),
            },
            _ => Check {
                name: "cargo-llvm-cov".to_string(),
                pass: false,
                detail: "not found — suggested for coverage: cargo install cargo-llvm-cov"
                    .to_string(),
            },
        });

        let nextest = Command::new("cargo")
            .arg("nextest")
            .arg("--version")
            .output()
            .await;
        checks.push(match nextest {
            Ok(o) if o.status.success() => Check {
                name: "cargo-nextest".to_string(),
                pass: true,
                detail: "found".to_string(),
            },
            _ => Check {
                name: "cargo-nextest".to_string(),
                pass: false,
                detail: "not found — suggested for fast testing: cargo install cargo-nextest"
                    .to_string(),
            },
        });
        return;
    }

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

    let node = Command::new("node").arg("--version").output().await;
    checks.push(match node {
        Ok(o) if o.status.success() => Check {
            name: "Node.js".to_string(),
            pass: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
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
    } else {
        "not found — run: npm install -g pnpm".to_string()
    };

    let mut actual_pnpm_pass = pnpm_pass;
    if !pnpm_pass && auto_heal {
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
        name: "pnpm".to_string(),
        pass: actual_pnpm_pass,
        detail: pnpm_detail,
    });

    let git = Command::new("git").arg("--version").output().await;
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

    let wasi_target = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .await;
    let wasi_installed = wasi_target
        .as_ref()
        .map(|o| {
            o.status.success()
                && String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|l| l.trim() == "wasm32-wasip1")
        })
        .unwrap_or(false);

    let mut wasi_detail = if wasi_installed {
        "installed — `vox run --isolation wasm` is fast (warm cache)".to_string()
    } else {
        "not installed — first WASI run will take ~10s to install".to_string()
    };

    if !wasi_installed && auto_heal {
        println!("  [auto-heal] Installing wasm32-wasip1 target...");
        let ok = Command::new("rustup")
            .args(["target", "add", "wasm32-wasip1"])
            .status()
            .await
            .is_ok_and(|s| s.success());
        if ok {
            wasi_detail =
                "installed via auto-heal — `vox run --isolation wasm` is now fast".to_string();
        } else {
            wasi_detail = "auto-heal failed — run: rustup target add wasm32-wasip1".to_string();
        }
    }

    checks.push(Check {
        name: "WASI target (wasm32-wasip1)".to_string(),
        pass: wasi_installed || auto_heal,
        detail: wasi_detail,
    });

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

    let kube = Command::new("kubectl")
        .arg("version")
        .arg("--client")
        .output()
        .await;
    checks.push(match kube {
        Ok(o) if o.status.success() => Check {
            name: "kubectl (optional)".to_string(),
            pass: true,
            detail: "found — ready for Kubernetes deployments".to_string(),
        },
        _ => Check {
            name: "kubectl (optional)".to_string(),
            pass: true,
            detail: "not found — required for 'vox deploy --target k8s'".to_string(),
        },
    });

    let mut has_manifest = tokio::fs::try_exists("Vox.toml").await.unwrap_or(false);
    let mut manifest_detail = if has_manifest {
        "found in current directory".to_string()
    } else {
        "not found — run: vox init".to_string()
    };

    if !has_manifest && auto_heal {
        println!("  [auto-heal] Scaffolding Vox.toml via vox init...");
        let manifest = vox_pm::VoxManifest::scaffold("vox-app", "application");
        if let Ok(s) = manifest.to_toml_string() {
            if tokio::fs::write("Vox.toml", s).await.is_ok() {
                has_manifest = true;
                manifest_detail = "Vox.toml scaffolded via auto-heal".to_string();
            }
        }
    }

    checks.push(Check {
        name: "Vox.toml".to_string(),
        pass: has_manifest,
        detail: manifest_detail,
    });

    let lsp_binary_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("vox-lsp")));

    let mut lsp_bin = match lsp_binary_path.as_ref() {
        Some(p) => tokio::fs::try_exists(p).await.unwrap_or(false),
        None => false,
    };
    let mut lsp_detail = if lsp_bin {
        "found in PATH".to_string()
    } else {
        "not built — run: cargo build -p vox-lsp --release".to_string()
    };

    if !lsp_bin && auto_heal {
        println!("  [auto-heal] Building vox-lsp...");
        if Command::new("cargo")
            .args(["build", "-p", "vox-lsp", "--release"])
            .status()
            .await
            .is_ok_and(|s| s.success())
        {
            lsp_bin = true;
            lsp_detail = "built successfully via auto-heal".to_string();
        } else {
            lsp_detail = "auto-heal failed to build vox-lsp".to_string();
        }
    }

    checks.push(Check {
        name: "vox-lsp binary".to_string(),
        pass: lsp_bin,
        detail: lsp_detail,
    });

    let config_dir: Option<std::path::PathBuf> = common::user_home_dir().map(|h| h.join(".vox"));
    let config_path = config_dir.as_ref().map(|d| d.join("config.toml"));
    let mut has_config = match config_path.as_ref() {
        Some(p) => tokio::fs::try_exists(p).await.unwrap_or(false),
        None => false,
    };
    let mut config_detail = if has_config {
        "found in ~/.vox/config.toml".to_string()
    } else {
        "not found — run: vox login".to_string()
    };

    if !has_config
        && auto_heal
        && let Some(dir) = &config_dir
    {
        println!("  [auto-heal] Creating default Vox configuration...");
        let _ = tokio::fs::create_dir_all(dir).await;
        let default_config = "[registry]\nurl = \"https://raw.githubusercontent.com/brbrainerd/vox/main/registry\"\n";
        if tokio::fs::write(dir.join("config.toml"), default_config)
            .await
            .is_ok()
        {
            has_config = true;
            config_detail = "config.toml created via auto-heal".to_string();
        }
    }

    checks.push(Check {
        name: "Vox Config".to_string(),
        pass: has_config,
        detail: config_detail,
    });

    let google_key = common::resolved_google_key().await;
    checks.push(match &google_key {
        Some(k) if k.starts_with("AIza") => Check {
            name: "Google AI Studio Key".to_string(),
            pass: true,
            detail: format!(
                "configured (free Gemini models available) — {}",
                common::redact_key(k)
            ),
        },
        Some(k) => Check {
            name: "Google AI Studio Key".to_string(),
            pass: true,
            detail: format!("configured — {}", common::redact_key(k)),
        },
        None => Check {
            name: "Google AI Studio Key".to_string(),
            pass: false,
            detail: "not found — run: vox login --registry google YOUR_KEY\n                          get a free key at: https://aistudio.google.com/apikey".to_string(),
        },
    });

    let or_key = common::resolved_openrouter_key().await;
    checks.push(match &or_key {
        Some(k) => Check {
            name: "OpenRouter Key (optional)".to_string(),
            pass: true,
            detail: format!(
                "configured (free :free models + paid SOTA available) — {}",
                common::redact_key(k)
            ),
        },
        None => Check {
            name: "OpenRouter Key (optional)".to_string(),
            pass: true,
            detail: "not configured — get a free key at https://openrouter.ai/keys".to_string(),
        },
    });

    let engine = ProviderPolicyEngine::new();
    let auth_path = common::vox_dot_dir().join("auth.json");
    if tokio::fs::try_exists(&auth_path).await.unwrap_or(false) {
        if let Ok(content) = tokio::fs::read_to_string(&auth_path).await {
            if let Ok(config) = serde_json::from_str::<AuthRegistriesOnly>(&content) {
                for (reg, _) in config.registries {
                    let policy = engine.policy_for(&reg);
                    let (pass, detail) = match policy {
                        Some(p) => {
                            let status = format!(
                                "{:?} / Quota Truth: {:?}",
                                p.support_level, p.quota_truth_level
                            );
                            (
                                !matches!(
                                    p.support_level,
                                    ProviderSupportLevel::UnsupportedInitially
                                ),
                                status,
                            )
                        }
                        None => (
                            true,
                            "No explicit policy defined (Basic support)".to_string(),
                        ),
                    };
                    checks.push(Check {
                        name: format!("Provider Policy: {}", reg),
                        pass,
                        detail,
                    });
                }
            }
        }
    }

    use vox_config::InferenceProfile;

    let profile = vox_config::inference_profile_from_env();
    let ollama_probe_skipped = matches!(
        profile,
        InferenceProfile::MobileLitert
            | InferenceProfile::MobileCoreml
            | InferenceProfile::CloudOpenAiCompatible
    );
    let ollama_detail = if ollama_probe_skipped {
        format!(
            "TCP probe skipped for VOX_INFERENCE_PROFILE={profile:?} (not desktop/lan Ollama); see docs/src/architecture/mobile-edge-ai-ssot.md"
        )
    } else {
        let ollama_reachable = std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], 11434)),
            std::time::Duration::from_millis(300),
        )
        .is_ok();
        if ollama_reachable {
            "running on localhost:11434 (local inference available)".to_string()
        } else {
            "not running — install from https://ollama.com if you want local models (or set VOX_INFERENCE_PROFILE if this host should not use loopback Ollama)".to_string()
        }
    };
    checks.push(Check {
        name: "Ollama Local (optional)".to_string(),
        pass: true,
        detail: ollama_detail,
    });

    let current_exe = std::env::current_exe().unwrap_or_default();
    let exe_path_str = current_exe.to_string_lossy();
    let in_vox_repo = tokio::fs::try_exists("Cargo.toml").await.unwrap_or(false)
        && tokio::fs::try_exists("crates/vox-cli")
            .await
            .unwrap_or(false);
    let is_local_dev = exe_path_str.contains("target")
        && (exe_path_str.contains("debug") || exe_path_str.contains("release"));
    let is_installed = exe_path_str.contains(".vox") && exe_path_str.contains("bin");
    let binary_source_pass = !in_vox_repo || is_local_dev;
    let binary_source_detail = if is_local_dev {
        format!("{} (local dev build)", exe_path_str)
    } else if is_installed && in_vox_repo {
        format!(
            "{} — using installed binary while in repo; prefer: export PATH=\"$(./scripts/dev-path.sh):$PATH\" or cargo run -p vox-cli -- ...",
            exe_path_str
        )
    } else {
        format!("{}", exe_path_str)
    };
    checks.push(Check {
        name: "Vox binary source".to_string(),
        pass: binary_source_pass,
        detail: binary_source_detail,
    });

    let update_hint = if exe_path_str.starts_with("/usr/bin/") || exe_path_str.starts_with("/bin/")
    {
        "sudo apt update && sudo apt install --only-upgrade vox"
    } else if exe_path_str.contains("WinGet") || exe_path_str.contains("WindowsApps") {
        "winget upgrade vox"
    } else if exe_path_str.contains(".cargo") {
        "cargo install vox-cli"
    } else {
        "vox update (or redownload from GitHub Releases)"
    };

    checks.push(Check {
        name: "App Updates".to_string(),
        pass: true,
        detail: format!("to update natively, run: {}", update_hint),
    });

    let vox_dir = common::user_home_dir().map(|h| h.join(".vox"));
    let db_check = match vox_dir.as_ref() {
        Some(d) => {
            tokio::fs::create_dir_all(d).await.is_ok() && {
                let test_file = d.join(".doctor_write_test");
                let ok = tokio::fs::write(&test_file, b"ok").await.is_ok();
                let _ = tokio::fs::remove_file(&test_file).await;
                ok
            }
        }
        None => false,
    };
    checks.push(Check {
        name: "VoxDB directory".to_string(),
        pass: db_check,
        detail: if db_check {
            format!(
                "{} (writable)",
                vox_dir
                    .as_ref()
                    .map(|d| d.display().to_string())
                    .unwrap_or_default()
            )
        } else {
            "~/.vox/ not writable — check permissions".to_string()
        },
    });

    let mut reg_pass = false;
    let mut reg_detail = "not registered — run: vox setup".to_string();
    if let Ok(db) = vox_db::Codex::connect_default().await {
        let key = "project.vox-workspace.path".to_string();
        if let Ok(path) = db.store().get_object_metadata("vox-workspace", &key).await {
            reg_pass = true;
            reg_detail = format!("registered at {}", path);
        } else if let Ok(path) = db
            .store()
            .get_object_metadata("vox-workspace", "path")
            .await
        {
            reg_pass = true;
            reg_detail = format!("registered at {}", path);
        } else if let Ok(mut rows) = db
            .store()
            .conn
            .query(
                "SELECT value FROM user_preferences WHERE key = ?1",
                (key.clone(),),
            )
            .await
        {
            if let Ok(Some(row)) = rows.next().await {
                if let Ok(val) = row.get::<String>(0) {
                    reg_pass = true;
                    reg_detail = format!("registered at {}", val);
                }
            }
        }
    }

    checks.push(Check {
        name: "Workspace Registration".to_string(),
        pass: reg_pass,
        detail: reg_detail,
    });
}
