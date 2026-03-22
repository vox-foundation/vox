//! Unified backend adapter for script execution (P2).

use anyhow::Result;

use crate::commands::runtime::run::script::ScriptOpts;
pub use crate::wasi_dir_mode::WasiDirMode;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Parse raw cargo stderr into an actionable suggestion.
/// Returns `(summary, suggestion)` where suggestion may be empty.
pub fn parse_cargo_error(stderr: &str, target_wasi: bool) -> (String, String) {
    // Use Option to distinguish "explicitly silent" (compile_error! guardrail fired,
    // message is already in the output) from "no pattern matched" (→ WASI generic fallback).
    let matched: Option<String> = if stderr.contains("target 'wasm32-wasip1' not found")
        || stderr.contains("unknown target triple")
    {
        Some("Run: rustup target add wasm32-wasip1".to_string())
    } else if stderr.contains("error[E0433]") || stderr.contains("error[E0432]") {
        Some("Hint: Check imports — a dependency or crate name may be wrong.".to_string())
    } else if stderr.contains("error[E0308]") {
        Some("Hint: Type mismatch — check function return types and argument types.".to_string())
    } else if stderr.contains("compile_error!") {
        // WASI guardrail already printed its own message inline — stay silent.
        Some(String::new())
    } else if stderr.contains("Blocking waiting for file lock") {
        Some(
            "Hint: Another `vox run` is compiling. Wait or use --no-cache to force fresh."
                .to_string(),
        )
    } else {
        None // no specific pattern — fall through to WASI generic if applicable
    };

    let suggestion = matched.unwrap_or_else(|| {
        if target_wasi {
            "Hint: WASI scripts cannot use actors, workflows, async main, HTTP, or MCP tools."
                .to_string()
        } else {
            String::new()
        }
    });

    let summary = stderr
        .lines()
        .find(|l| l.trim_start().starts_with("error"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| {
            if target_wasi {
                "WASI compilation failed"
            } else {
                "Compilation failed"
            }
            .to_string()
        });

    (summary, suggestion)
}

/// Interface for script execution backends (Native, WASI).
pub trait RunBackend {
    /// Label for the cache directory (e.g. "script-cache", "script-cache-wasi").
    fn cache_label(&self) -> &str;

    /// Compile HIR to the target artifact and return the binary/wasm path.
    fn compile(
        &self,
        hir: &vox_hir::HirModule,
        cache_dir: &Path,
        shared_target: &Path,
        opts: &ScriptOpts,
    ) -> Result<PathBuf>;

    /// Execute the compiled artifact.
    fn execute(&self, artifact: &Path, args: &[String], opts: &ScriptOpts) -> Result<ExitStatus>;
}

/// Backend for running native binaries via cargo.
pub struct NativeBackend;

impl RunBackend for NativeBackend {
    fn cache_label(&self) -> &str {
        "script-cache"
    }

    fn compile(
        &self,
        hir: &vox_hir::HirModule,
        cache_dir: &Path,
        shared_target: &Path,
        opts: &ScriptOpts,
    ) -> Result<PathBuf> {
        // Native scripts share the same generated crate name (`vox-script`). A single workspace-level
        // `CARGO_TARGET_DIR` would let parallel compiles clobber `vox-script.exe` before copy.
        let _ = shared_target;
        let script_target_dir = cache_dir.join("target");

        let binary_name = if cfg!(target_os = "windows") {
            "vox-script.exe"
        } else {
            "vox-script"
        };
        let per_entry_binary = cache_dir.join(binary_name);

        let output = vox_codegen_rust::generate_script(
            hir,
            "vox-script",
            crate::fs_utils::resolve_vox_runtime_path().as_deref(),
        )
        .map_err(|e| anyhow::anyhow!("Rust code generation failed: {e}"))?;

        output.write_to_dir(cache_dir)?;

        let use_release = opts.trust_class.as_deref() != Some("trusted_dev")
            && std::env::var("VOX_SCRIPT_RELEASE").is_ok();

        let profile_args: &[&str] = if use_release {
            &["build", "--release"]
        } else {
            &["build", "--profile", "script-dev"]
        };

        // Per-entry `script-dev` profile only under this cache dir. Do not write under
        // `script-cache/.cargo/` — that path is shared by all hashes and parallel `vox run`
        // invocations would race on the same files (Windows: flaky "path not found").
        let cargo_config_dir = cache_dir.join(".cargo");
        if !cargo_config_dir.exists() {
            let _ = std::fs::create_dir_all(&cargo_config_dir);
        }
        let config_content = r#"[profile.script-dev]
        inherits = "dev"
        opt-level = 1
        codegen_units = 256
        incremental = true
        debug = false
        overflow-checks = false
        "#;
        for filename in &["config", "config.toml"] {
            let _ = std::fs::write(cargo_config_dir.join(filename), config_content);
        }
        let args: Vec<String> = profile_args[1..].iter().map(|s| (*s).to_string()).collect();
        let req = crate::build_service::CargoRequest::build(
            cache_dir.to_path_buf(),
            Some(script_target_dir.clone()),
            args,
        );
        let build_out = crate::build_service::run_cargo(&req)?;

        if !build_out.status.success() {
            let stderr = String::from_utf8_lossy(&build_out.stderr).to_string();
            let (summary, suggestion) = parse_cargo_error(&stderr, false);
            return Err(anyhow::anyhow!(
                "Compilation failed: {}\n{}\n---\n{}",
                summary,
                suggestion,
                stderr
            ));
        }

        let profile_dir = if use_release { "release" } else { "script-dev" };
        let binary_path = script_target_dir.join(profile_dir).join(binary_name);
        std::fs::copy(&binary_path, &per_entry_binary)?;

        Ok(per_entry_binary)
    }

    fn execute(&self, artifact: &Path, args: &[String], opts: &ScriptOpts) -> Result<ExitStatus> {
        let mut cmd = Command::new(artifact);
        cmd.args(args);
        if opts.sandbox {
            cmd.env("VOX_SANDBOX", "1");
        }
        if !opts.allow_mcp {
            cmd.env("VOX_NO_MCP", "1");
        }

        // Propagate the resolved cargo binary path so run_cmd("cargo", ...) inside the
        // script can find cargo even when the script binary is not launched via `cargo run`.
        // Precedence: VOX_CARGO_BIN > CARGO env var > which-resolved "cargo".
        let cargo_bin = std::env::var("VOX_CARGO_BIN")
            .or_else(|_| std::env::var("CARGO"))
            .unwrap_or_else(|_| {
                which::which("cargo")
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "cargo".to_string())
            });
        cmd.env("CARGO", &cargo_bin);
        cmd.env("VOX_CARGO_BIN", &cargo_bin);

        // P4: Redact sensitive env vars
        if opts.trust_class.as_deref() != Some("trusted_dev") {
            for (key, _) in std::env::vars() {
                let ku = key.to_uppercase();
                if ku.ends_with("_KEY")
                    || ku.ends_with("_SECRET")
                    || ku.ends_with("_TOKEN")
                    || ku.ends_with("_PASSWORD")
                {
                    cmd.env_remove(&key);
                }
            }
        }

        // P4: Native sandbox enforcement (Landlock on Linux, Job Objects on Windows)
        if opts.sandbox {
            super::sandbox::enforce_sandbox(&mut cmd, opts)?;
        }

        // On Windows with --sandbox, spawn then assign Job Object immediately
        #[cfg(target_os = "windows")]
        if opts.sandbox {
            let mut child = cmd.spawn()?;
            super::sandbox::post_spawn_sandbox(&child)?;
            let status = child.wait()?;
            return Ok(status);
        }

        Ok(cmd.status()?)
    }
}

/// Backend for running WASI modules via Wasmtime.
pub struct WasiBackend;

impl RunBackend for WasiBackend {
    fn cache_label(&self) -> &str {
        "script-cache-wasi"
    }

    fn compile(
        &self,
        hir: &vox_hir::HirModule,
        cache_dir: &Path,
        shared_target: &Path,
        _opts: &ScriptOpts,
    ) -> Result<PathBuf> {
        let per_entry_wasm = cache_dir.join("vox-script.wasm");

        let output = vox_codegen_rust::generate_script_with_target(
            hir,
            "vox-script",
            crate::fs_utils::resolve_vox_runtime_path().as_deref(),
            vox_codegen_rust::ScriptTarget::Wasi,
        )
        .map_err(|e| anyhow::anyhow!("WASI codegen failed: {e}"))?;

        output.write_to_dir(cache_dir)?;

        // Only install WASI target if not already present — avoids ~500ms overhead on warm builds.
        let wasi_installed = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("wasm32-wasip1"))
            .unwrap_or(false);
        if !wasi_installed {
            let _ = Command::new("rustup")
                .args(["target", "add", "wasm32-wasip1"])
                .status();
        }

        let cargo_lock = shared_target.join(".cargo-lock");
        if cargo_lock.exists() {
            eprintln!("Waiting for another vox run to finish compiling (WASI)...");
        }

        let req = crate::build_service::CargoRequest::build(
            cache_dir.to_path_buf(),
            Some(shared_target.to_path_buf()),
            vec![
                "--release".to_string(),
                "--target".to_string(),
                "wasm32-wasip1".to_string(),
            ],
        );
        let build_out = crate::build_service::run_cargo(&req)?;

        if !build_out.status.success() {
            let stderr = String::from_utf8_lossy(&build_out.stderr).to_string();
            let (summary, suggestion) = parse_cargo_error(&stderr, true);
            return Err(anyhow::anyhow!(
                "Compilation failed: {}\n{}\n---\n{}",
                summary,
                suggestion,
                stderr
            ));
        }

        let shared_wasm = shared_target
            .join("wasm32-wasip1")
            .join("release")
            .join("vox-script.wasm");
        std::fs::copy(&shared_wasm, &per_entry_wasm)?;

        Ok(per_entry_wasm)
    }

    fn execute(&self, artifact: &Path, args: &[String], _opts: &ScriptOpts) -> Result<ExitStatus> {
        #[cfg(feature = "script-execution")]
        {
            // Inline Wasmtime execution (`--features script-execution`)
            let stdout_pipe = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(64 * 1024);
            let stderr_pipe = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(64 * 1024);

            let engine = wasmtime::Engine::default();
            let module = wasmtime::Module::from_file(&engine, artifact)?;

            let mut linker: wasmtime::Linker<wasmtime_wasi::p1::WasiP1Ctx> =
                wasmtime::Linker::new(&engine);
            wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |t| t)?;
            let pre = linker.instantiate_pre(&module)?;

            let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
            builder
                .stdout(stdout_pipe.clone())
                .stderr(stderr_pipe.clone());

            // Pass host args to guest (P2: properly populates argv)
            let mut guest_args = vec!["vox-script".to_string()];
            guest_args.extend(args.iter().cloned());
            builder.args(&guest_args);

            for (host, guest, mode) in &_opts.wasi_dirs {
                let (dp, fp) = match mode {
                    WasiDirMode::ReadOnly => (
                        wasmtime_wasi::DirPerms::READ,
                        wasmtime_wasi::FilePerms::READ,
                    ),
                    WasiDirMode::ReadWrite => (
                        wasmtime_wasi::DirPerms::all(),
                        wasmtime_wasi::FilePerms::all(),
                    ),
                };
                builder.preopened_dir(host, guest, dp, fp)?;
            }

            let wasi_ctx = builder.build_p1();
            let mut store = wasmtime::Store::new(&engine, wasi_ctx);
            let instance = pre.instantiate(&mut store)?;
            let func = instance.get_typed_func::<(), ()>(&mut store, "_start")?;

            let exit_code = match func.call(&mut store, ()) {
                Ok(()) => 0,
                Err(e) => {
                    if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                        exit.0
                    } else {
                        return Err(e.into());
                    }
                }
            };

            let stdout = String::from_utf8_lossy(stdout_pipe.contents().as_ref()).to_string();
            let stderr = String::from_utf8_lossy(stderr_pipe.contents().as_ref()).to_string();
            print!("{}", stdout);
            eprint!("{}", stderr);

            // Correctly map exit_code to ExitStatus
            #[cfg(target_family = "unix")]
            {
                use std::os::unix::process::ExitStatusExt;
                Ok(ExitStatus::from_raw(exit_code << 8))
            }
            #[cfg(target_family = "windows")]
            {
                use std::os::windows::process::ExitStatusExt;
                Ok(ExitStatus::from_raw(exit_code as u32))
            }
        }

        #[cfg(not(feature = "script-execution"))]
        {
            let mut cmd = Command::new("wasmtime");
            cmd.arg(artifact).args(args);
            Ok(cmd.status()?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_cargo_error;

    // ── helpers ────────────────────────────────────────────────────────────────

    /// Build a minimal rustc "error[Exxxx]" block as cargo emits it.
    fn ec(code: &str, msg: &str, detail: &str) -> String {
        format!(
            "error[{code}]: {msg}\n  --> src/main.rs:3:5\n   |\n3  |     {detail}\n   |     ^^^^\n"
        )
    }

    fn cargo_build_failed_plain(msg: &str) -> String {
        format!("error: {msg}\n\nerror: could not compile `vox-script` due to previous error\n")
    }

    // ── summary extraction ─────────────────────────────────────────────────────

    #[test]
    fn summary_extracts_first_error_line_native() {
        let stderr = ec("E0308", "mismatched types", "42");
        let (summary, _) = parse_cargo_error(&stderr, false);
        assert!(summary.starts_with("error[E0308]"), "got: {summary}");
    }

    #[test]
    fn summary_extracts_first_error_line_wasi() {
        let stderr = ec("E0433", "failed to resolve: use of undeclared crate", "foo");
        let (summary, _) = parse_cargo_error(&stderr, true);
        assert!(summary.starts_with("error[E0433]"), "got: {summary}");
    }

    #[test]
    fn summary_fallback_native_when_no_error_line() {
        let (summary, _) = parse_cargo_error(
            "Compiling vox-script v0.1.0\nwarning: unused import\n",
            false,
        );
        assert_eq!(summary, "Compilation failed");
    }

    #[test]
    fn summary_fallback_wasi_when_no_error_line() {
        let (summary, _) = parse_cargo_error(
            "Compiling vox-script v0.1.0\nwarning: unused import\n",
            true,
        );
        assert_eq!(summary, "WASI compilation failed");
    }

    // ── suggestion branches ────────────────────────────────────────────────────

    #[test]
    fn suggestion_wasm_target_not_found() {
        let stderr = "error[E0463]: can't find crate for `std`\n  = note: the `wasm32-wasip1` target may not be installed\nerror: target 'wasm32-wasip1' not found\n";
        let (_, suggestion) = parse_cargo_error(stderr, true);
        assert!(
            suggestion.contains("rustup target add wasm32-wasip1"),
            "got: {suggestion}"
        );
    }

    #[test]
    fn suggestion_unknown_target_triple() {
        let stderr = "error: unknown target triple `wasm32-wasip1`\n";
        let (_, suggestion) = parse_cargo_error(stderr, false);
        assert!(
            suggestion.contains("rustup target add wasm32-wasip1"),
            "got: {suggestion}"
        );
    }

    #[test]
    fn suggestion_import_error_e0433() {
        let stderr = ec(
            "E0433",
            "failed to resolve: use of undeclared crate or module `serde`",
            "serde",
        );
        let (_, suggestion) = parse_cargo_error(&stderr, false);
        assert!(
            suggestion.contains("dependency or crate name"),
            "got: {suggestion}"
        );
    }

    #[test]
    fn suggestion_import_error_e0432() {
        let stderr = ec("E0432", "unresolved import `tokio::runtime`", "tokio");
        let (_, suggestion) = parse_cargo_error(&stderr, false);
        assert!(
            suggestion.contains("dependency or crate name"),
            "got: {suggestion}"
        );
    }

    #[test]
    fn suggestion_type_mismatch_e0308() {
        let stderr = ec(
            "E0308",
            "mismatched types: expected `i32`, found `&str`",
            "\"hello\"",
        );
        let (_, suggestion) = parse_cargo_error(&stderr, false);
        assert!(suggestion.contains("Type mismatch"), "got: {suggestion}");
    }

    #[test]
    fn suggestion_compile_error_macro_is_empty() {
        let stderr = "error: compile_error!(\"actors are not supported in WASI scripts\")\n --> src/main.rs:2:1\n";
        let (_, suggestion) = parse_cargo_error(stderr, true);
        assert!(suggestion.is_empty(), "got: {suggestion}");
    }

    #[test]
    fn suggestion_cargo_file_lock() {
        let stderr = "Blocking waiting for file lock on build directory\n";
        let (_, suggestion) = parse_cargo_error(stderr, false);
        assert!(
            suggestion.to_lowercase().contains("wait") || suggestion.contains("no-cache"),
            "got: {suggestion}"
        );
    }

    #[test]
    fn suggestion_wasi_generic_fallback_when_no_other_match() {
        let stderr = cargo_build_failed_plain("aborting due to previous error");
        let (_, suggestion) = parse_cargo_error(&stderr, true);
        assert!(
            suggestion.contains("WASI scripts cannot use actors"),
            "got: {suggestion}"
        );
    }

    #[test]
    fn suggestion_empty_for_plain_native_error() {
        let stderr = cargo_build_failed_plain("aborting due to previous error");
        let (_, suggestion) = parse_cargo_error(&stderr, false);
        assert!(suggestion.is_empty(), "got: {suggestion}");
    }

    // ── priority: most-specific rule wins ─────────────────────────────────────

    #[test]
    fn target_not_found_takes_priority_over_wasi_generic() {
        let stderr = "error: target 'wasm32-wasip1' not found\nerror: could not compile\n";
        let (_, suggestion) = parse_cargo_error(stderr, true);
        assert!(
            suggestion.contains("rustup target add"),
            "got: {suggestion}"
        );
        assert!(
            !suggestion.contains("actors"),
            "WASI generic should NOT fire: {suggestion}"
        );
    }

    #[test]
    fn compile_error_macro_takes_priority_over_wasi_generic() {
        let stderr = "error: compile_error!(\"async fn main is not supported in WASI scripts\")\n --> src/main.rs:1:1\n";
        let (_, suggestion) = parse_cargo_error(stderr, true);
        assert!(suggestion.is_empty(), "got: {suggestion}");
    }

    // ── real-world cargo stderr sample ────────────────────────────────────────

    #[test]
    fn wasm_target_missing_sample() {
        let stderr = r#"error: target 'wasm32-wasip1' not found in channel
  |
  = help: run `rustup target add wasm32-wasip1`
"#;
        let (_, suggestion) = parse_cargo_error(stderr, true);
        assert!(suggestion.contains("rustup target add wasm32-wasip1"));
    }

    #[test]
    fn wasi_generic_fallback_sample() {
        let stderr = "error: some weird wasm error\nerror: could not compile `vox-script`";
        let (summary, suggestion) = parse_cargo_error(stderr, true);
        assert_eq!(summary, "error: some weird wasm error");
        assert!(suggestion.contains("WASI scripts cannot use actors"));
    }

    #[test]
    fn file_lock_blocking_sample() {
        let stderr = "    Blocking waiting for file lock on build directory";
        let (_, suggestion) = parse_cargo_error(stderr, false);
        assert!(suggestion.contains("Another `vox run` is compiling"));
    }

    #[test]
    fn compile_error_guardrail_sample() {
        let stderr = r#"error: custom error from compile_error!
  --> src/main.rs:2:1
   |
2  | compile_error!("Actors are not supported in WASI");
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
"#;
        let (_, suggestion) = parse_cargo_error(stderr, true);
        assert!(suggestion.is_empty());
    }

    #[test]
    fn real_world_e0308_sample() {
        let stderr = r#"error[E0308]: mismatched types
  --> src/main.rs:5:5
   |
5  |     42.0
   |     ^^^^ expected `i32`, found `f64`

For more information about this error, try `rustc --explain E0308`.
error: could not compile `vox-script` (bin "vox-script") due to 1 previous error
"#;
        let (summary, suggestion) = parse_cargo_error(stderr, false);
        assert!(summary.starts_with("error[E0308]"), "summary: {summary}");
        assert!(
            suggestion.contains("Type mismatch"),
            "suggestion: {suggestion}"
        );
    }

    #[test]
    fn real_world_e0433_sample() {
        let stderr = r#"error[E0433]: failed to resolve: use of undeclared crate or module `uuid`
  --> src/main.rs:1:5
   |
1  |     uuid::Uuid::new_v4()
   |     ^^^^ use of undeclared crate or module `uuid`

For more information about this error, try `rustc --explain E0433`.
error: could not compile `vox-script` due to 1 previous error
"#;
        let (summary, suggestion) = parse_cargo_error(stderr, false);
        assert!(summary.starts_with("error[E0433]"), "summary: {summary}");
        assert!(
            suggestion.contains("dependency or crate name"),
            "suggestion: {suggestion}"
        );
    }
}
