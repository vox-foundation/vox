//! Wasmtime engine construction and WASM module execution.
//!
//! `WasmHost` is the single engine handle shared across all Vox WASM call sites.
//! Engine creation is cheap (amortized); reuse across executions saves JIT warmup.

use std::path::Path;

use anyhow::Result;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::p1::WasiP1Ctx;

use crate::{WasmExecOpts, WasmRunOutcome};

/// Wasmtime engine host — single-source-of-truth WASM executor.
///
/// Construct once with [`WasmHost::new`] (or [`WasmHost::with_fuel`]) and reuse
/// across multiple [`execute`][WasmHost::execute] calls.
pub struct WasmHost {
    engine: Engine,
    /// If `Some`, fuel is enabled in the engine config and set per-store.
    fuel: Option<u64>,
}

impl WasmHost {
    /// Create a default `WasmHost` without fuel limits.
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        Ok(Self { engine, fuel: None })
    }

    /// Create a `WasmHost` with per-execution fuel limit.
    ///
    /// When fuel is set, every Wasm instruction costs 1 unit.
    /// Executions that exhaust fuel return an [`anyhow::Error`] wrapping
    /// Wasmtime's out-of-fuel trap.
    pub fn with_fuel(fuel: u64) -> Result<Self> {
        let mut cfg = Config::new();
        cfg.consume_fuel(true);
        let engine = Engine::new(&cfg)?;
        Ok(Self {
            engine,
            fuel: Some(fuel),
        })
    }

    /// Execute a compiled `.wasm` module and return the outcome.
    ///
    /// Stdout and stderr are captured via in-memory pipes so they do not
    /// race with the calling process's stdio.  After execution the bytes are
    /// available in [`WasmRunOutcome::stdout`] / [`WasmRunOutcome::stderr`].
    ///
    /// Exit codes are mapped:
    /// - `_start()` returns `Ok(())` → exit code 0
    /// - guest calls `proc_exit(N)` → exit code N (via [`wasmtime_wasi::I32Exit`])
    /// - any other trap → propagated as [`anyhow::Error`]
    pub fn execute(&self, module_path: &Path, opts: &WasmExecOpts) -> Result<WasmRunOutcome> {
        let started = std::time::Instant::now();

        let stdout_pipe = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(64 * 1024);
        let stderr_pipe = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(64 * 1024);

        let module = Module::from_file(&self.engine, module_path)?;

        let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |t| t)?;
        let pre = linker.instantiate_pre(&module)?;

        let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
        builder
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone());

        // Argv: argv[0] = module stem or "wasm-module", rest = caller args.
        let mut argv = vec![
            module_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("wasm-module")
                .to_string(),
        ];
        argv.extend(opts.args.iter().cloned());
        builder.args(&argv);

        // Environment variables.
        for (k, v) in &opts.env {
            builder.env(k, v);
        }

        // Stdin injection (optional).
        if let Some(ref stdin_bytes) = opts.stdin {
            let bytes: bytes::Bytes = stdin_bytes.clone().into();
            let pipe = wasmtime_wasi::p2::pipe::MemoryInputPipe::new(bytes);
            builder.stdin(pipe);
        }

        // Preopened directories.
        for preopen in &opts.preopens {
            let (dp, fp) = preopen.wasi_perms();
            builder.preopened_dir(&preopen.host, &preopen.guest, dp, fp)?;
        }

        let wasi_ctx = builder.build_p1();
        let mut store = Store::new(&self.engine, wasi_ctx);

        // Apply fuel limit when configured.
        if let Some(fuel) = self.fuel.or(opts.fuel_override) {
            store.set_fuel(fuel)?;
        }

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

        let wall_ms = started.elapsed().as_millis() as u64;
        let stdout = stdout_pipe.contents().to_vec();
        let stderr = stderr_pipe.contents().to_vec();

        Ok(WasmRunOutcome {
            exit_code,
            stdout,
            stderr,
            wall_ms,
        })
    }
}

impl Default for WasmHost {
    fn default() -> Self {
        Self::new().expect("Failed to create default WasmHost")
    }
}
