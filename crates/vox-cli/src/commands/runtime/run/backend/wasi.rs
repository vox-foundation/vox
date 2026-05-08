use super::WasiDirMode;
use super::{RunBackend, ScriptOpts, parse_cargo_error};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use vox_wasm_engine::{Preopen, PreopenMode, WasmExecOpts, WasmHost, WasmRunOutcome};

/// Backend for running WASI modules via Wasmtime.
pub struct WasiBackend;

impl RunBackend for WasiBackend {
    fn cache_label(&self) -> &str {
        "script-cache-wasi"
    }

    fn compile(
        &self,
        hir: &vox_compiler::hir::HirModule,
        cache_dir: &Path,
        shared_target: &Path,
        _opts: &ScriptOpts,
    ) -> Result<PathBuf> {
        let per_entry_wasm = cache_dir.join("vox-script.wasm");

        let output = vox_codegen::codegen_rust::generate_script_with_target(
            hir,
            "vox-script",
            crate::fs_utils::resolve_vox_runtime_path().as_deref(),
            vox_codegen::codegen_rust::ScriptTarget::Wasi,
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

    fn execute(&self, artifact: &Path, args: &[String], opts: &ScriptOpts) -> Result<ExitStatus> {
        // Delegate to the vox-wasm-engine SSOT for all Wasmtime engine + WASI wiring.
        let host = WasmHost::new()?;

        let preopens = opts
            .wasi_dirs
            .iter()
            .map(|(host_path, guest, mode)| Preopen {
                host: host_path.clone(),
                guest: guest.clone(),
                mode: match mode {
                    WasiDirMode::ReadOnly => PreopenMode::ReadOnly,
                    WasiDirMode::ReadWrite => PreopenMode::ReadWrite,
                },
            })
            .collect();

        let exec_opts = WasmExecOpts {
            args: args.to_vec(),
            preopens,
            fuel_override: None,
            stdin: None,
            env: Vec::new(),
        };

        let outcome: WasmRunOutcome = host.execute(artifact, &exec_opts)?;

        // Print captured output to real stdio.
        print!("{}", outcome.stdout_str());
        eprint!("{}", outcome.stderr_str());

        // Map exit_code to ExitStatus.
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::process::ExitStatusExt;
            Ok(ExitStatus::from_raw(outcome.exit_code << 8))
        }
        #[cfg(target_family = "windows")]
        {
            use std::os::windows::process::ExitStatusExt;
            Ok(ExitStatus::from_raw(outcome.exit_code as u32))
        }
    }
}
