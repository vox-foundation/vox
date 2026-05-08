//! Wasmtime-based WASI sandbox runtime for skill execution.
//!
//! Implements [`SkillRuntime`] using wasmtime with WASI preview-1/preview-2
//! capability-bound sandboxing. This is the default runtime for pure-compute skills
//! (no subprocess, no GPU, no native libs).
//!
//! # Capabilities
//! - Pure-compute: ✅ (text transforms, JSON, regex, parsing, classification)
//! - HTTP outbound: ✅ (via wasi-http — future wiring)
//! - File IO: ✅ (via WASI preopens — restricted to declared directories)
//! - Subprocess exec: ❌ (not in WASI — use runtime-container)
//! - GPU access: ❌ (not addressable in WASM — use runtime-container)
//! - Threads: ⚠️ (wasi-threads exists but immature)
//!
//! # Status: SCAFFOLD
//! The engine construction and module loading work. Full WASI preopen plumbing,
//! fuel-based timeouts, and wasi-http wiring are TODO items tracked in
//! `docs/src/architecture/vox-container-vs-wasm-2026-05-08.md` Phase 4.

use anyhow::Result;
use std::path::PathBuf;
use vox_skill_runtime::{BuildOpts, RunOpts, RunOutcome, SkillRuntime};
use wasmtime::{Config, Engine, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

/// Wasmtime-based WASI sandbox runtime.
///
/// Provides the fastest cold-start (~µs) and smallest footprint (~5MB embedded)
/// of all `SkillRuntime` implementations. No external daemon required.
pub struct WasmRuntime {
    engine: Engine,
}

impl WasmRuntime {
    /// Create a new WasmRuntime with a fuel-enabled engine.
    pub fn new() -> Result<Self> {
        let mut cfg = Config::new();
        // Fuel enables bounded execution / timeout enforcement.
        cfg.consume_fuel(true);
        let engine = Engine::new(&cfg)?;
        Ok(Self { engine })
    }
}

impl SkillRuntime for WasmRuntime {
    fn name(&self) -> &str {
        "wasm"
    }

    fn available(&self) -> bool {
        // Wasmtime is an in-process embedding — always available.
        true
    }

    fn build(&self, opts: &BuildOpts) -> Result<()> {
        // For WASM: no image build step. Validate the artifact is a .wasm.
        // Optionally precompile with Module::serialize for faster subsequent loads.
        let artifact = opts
            .artifact_path
            .as_ref()
            .cloned()
            .unwrap_or_else(|| opts.context_dir.join("skill.wasm"));

        if !artifact.exists() {
            tracing::warn!(
                target: "wasm-runtime",
                path = ?artifact,
                "WASM artifact not found at expected path; \
                 skill must be pre-compiled to .wasm before execution"
            );
            // Not a hard error at build time — the artifact may not exist yet.
            return Ok(());
        }

        // Validate it's a valid WASM module.
        let _module = Module::from_file(&self.engine, &artifact).map_err(|e| {
            anyhow::anyhow!("WASM artifact {:?} is not a valid WASM module: {e}", artifact)
        })?;

        tracing::info!(
            target: "wasm-runtime",
            path = ?artifact,
            "WASM artifact validated successfully"
        );
        Ok(())
    }

    fn run(&self, opts: &RunOpts) -> Result<RunOutcome> {
        let artifact = &opts.artifact_path;

        if !artifact.exists() {
            anyhow::bail!(
                "WASM artifact not found: {:?}. \
                 Compile the skill to wasm32-wasip2 first.",
                artifact
            );
        }

        tracing::info!(
            target: "wasm-runtime",
            path = ?artifact,
            "Loading WASM module"
        );

        let module = Module::from_file(&self.engine, artifact)?;

        // Build a WASI context with capability-bound preopens.
        // TODO: map opts.volumes into WasiCtxBuilder preopened_dir entries.
        // TODO: map opts.env into WasiCtxBuilder envs.
        // TODO: wire wasi-http for outbound HTTP (wasi-http preview-2).
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio() // Prototype: capture in future iteration
            .build_p1();     // WASI preview-1 for broad compat

        let mut store = Store::new(&self.engine, wasi);

        // Apply fuel limit for bounded execution.
        let fuel = opts.cpu_limit_fuel.unwrap_or(1_000_000_000);
        store.set_fuel(fuel)?;

        // TODO: full WASI linker + function instantiation + entry-point call.
        // The linker setup requires wasmtime_wasi::add_to_linker which needs
        // the full component model or preview-1 adapter depending on target.
        //
        // For now, return a "not yet fully wired" error.
        // The wiring is tracked in docs/src/architecture/vox-container-vs-wasm-2026-05-08.md.
        let _ = module; // suppress unused warning; module is loaded and validated above

        tracing::warn!(
            target: "wasm-runtime",
            "WasmRuntime::run: WASI linker + entry-point wiring is TODO (scaffold). \
             Module loaded and validated; execution plumbing deferred to next batch."
        );

        // Return scaffold outcome to avoid panicking.
        // TODO: replace with actual execution result once linker is wired.
        anyhow::bail!(
            "WasmRuntime::run: WASI execution not yet fully wired (scaffold). \
             See docs/src/architecture/vox-container-vs-wasm-2026-05-08.md for next steps. \
             Artifact: {:?}",
            artifact
        )
    }
}
