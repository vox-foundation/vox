use super::{parse_cargo_error, RunBackend, ScriptOpts};
use super::WasiDirMode;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
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

        let output = vox_compiler::codegen_rust::generate_script_with_target(
            hir,
            "vox-script",
            crate::fs_utils::resolve_vox_runtime_path().as_deref(),
            vox_compiler::codegen_rust::ScriptTarget::Wasi,
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

