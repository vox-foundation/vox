//! `vox run` script-mode execution engine.
//!
//! Compiles a `.vox` file with a top-level `fn main()` to a Rust binary (or
//! WASI module) and executes it. Results are cached by content hash in
//! `~/.vox/script-cache/<hash>/`. All script builds share a single
//! `~/.vox/script-target/` so `vox-runtime` and its transitive dependencies
//! are only compiled once.

use anyhow::Result;

use crate::commands::runtime::run::backend::{NativeBackend, RunBackend, WasiBackend};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

// ── Error taxonomy (P0) ───────────────────────────────────────────────────────

/// Configuration for script execution.
#[derive(Debug, Clone)]
pub struct ScriptOpts {
    /// Enable platform-native sandbox (Landlock/JobObjects).
    pub sandbox: bool,
    /// Allow script to resolve and call MCP tools.
    pub allow_mcp: bool,
    /// Force fresh compilation, bypassing content-hash cache.
    pub no_cache: bool,
    /// Explicit isolation tier string (e.g. `"wasm"`, `"container"`).
    /// When `Some("wasm")` the script is compiled to WASI and run via Wasmtime.
    pub isolation: Option<String>,
    /// Trust classification string (e.g. `"trusted_dev"`, `"untrusted"`).
    /// When set, governs the default isolation tier if `isolation` is `None`.
    pub trust_class: Option<String>,
    /// P1.3: Preopened directories for WASI: (host_path, guest_path, mode)
    #[cfg(feature = "script-execution")]
    pub wasi_dirs: Vec<(PathBuf, String, crate::wasi_dir_mode::WasiDirMode)>,
}

impl ScriptOpts {
    /// Returns `true` when the WASI execution lane should be used.
    ///
    /// WASI is active when:
    /// - `--isolation wasm` / `--isolation wasi` is explicit, OR
    /// - `--trust-class untrusted` is set and no explicit isolation overrides it
    pub fn use_wasi(&self) -> bool {
        if let Some(iso) = self.isolation.as_deref() {
            return matches!(iso.to_lowercase().as_str(), "wasm" | "wasi" | "wasmtime");
        }
        // Default derived from trust class
        matches!(
            self.trust_class
                .as_deref()
                .unwrap_or("trusted_dev")
                .to_lowercase()
                .as_str(),
            "untrusted"
        )
    }

    /// Resolve the effective isolation tier name for display.
    pub fn effective_isolation(&self) -> &str {
        if let Some(iso) = self.isolation.as_deref() {
            return iso;
        }
        match self
            .trust_class
            .as_deref()
            .unwrap_or("trusted_dev")
            .to_lowercase()
            .as_str()
        {
            "untrusted" => "wasm",
            "semi_trusted" | "semi-trusted" => "container",
            _ => "permissive",
        }
    }

    /// P2: Select the appropriate backend for this execution.
    pub fn backend(&self) -> Box<dyn RunBackend> {
        if self.use_wasi() {
            Box::new(WasiBackend)
        } else {
            Box::new(NativeBackend)
        }
    }
}

/// Print the execution plan for `vox run --explain` without executing.
///
/// When `as_json` is `true`, emits machine-readable JSON instead of human text
/// (useful for IDE/tooling integration).
pub fn print_execution_plan(
    file: &Path,
    isolation: Option<&str>,
    trust_class: Option<&str>,
    sandbox: bool,
    as_json: bool,
) {
    let tc = trust_class.unwrap_or("trusted_dev");
    let opts = ScriptOpts {
        sandbox,
        allow_mcp: false,
        no_cache: false,
        isolation: isolation.map(str::to_string),
        trust_class: trust_class.map(str::to_string),
        #[cfg(feature = "script-execution")]
        wasi_dirs: Vec::new(),
    };
    let tier = opts.effective_isolation();
    let artifact = if opts.use_wasi() {
        "wasi_component"
    } else {
        "native_dev"
    };
    let backend = if opts.use_wasi() {
        "Wasmtime WASI P1"
    } else {
        "Native binary (cargo)"
    };

    let cache_dir = vox_config::paths::script_cache_dir(opts.use_wasi()).join("<source-hash>");

    let isolation_src = if isolation.is_some() {
        "explicit --isolation flag"
    } else if trust_class.is_some() {
        "derived from --trust-class"
    } else if sandbox {
        "derived from --sandbox"
    } else {
        "default for trust class"
    };

    let security = {
        use crate::isolation::IsolationPolicy;
        tier.parse::<IsolationPolicy>()
            .map(|p: IsolationPolicy| p.security_statement().to_string())
            .unwrap_or_else(|_| "Unknown tier".to_string())
    };

    if as_json {
        // Machine-readable output for IDE/tooling consumption (P3)
        println!("{{");
        println!(
            "  \"file\": \"{}\",",
            file.display().to_string().replace('\\', "/")
        );
        println!("  \"trust_class\": \"{tc}\",");
        println!("  \"isolation\": \"{tier}\",");
        println!("  \"isolation_source\": \"{isolation_src}\",");
        println!("  \"artifact\": \"{artifact}\",");
        println!("  \"backend\": \"{backend}\",");
        println!(
            "  \"cache_dir\": \"{}\",",
            cache_dir.display().to_string().replace('\\', "/")
        );
        println!("  \"security\": \"{security}\"");
        println!("}}");
    } else {
        println!();
        println!("Execution plan for: {}", file.display());
        println!("  TrustClass:   {tc}");
        println!("  Isolation:    {tier} ({isolation_src})");
        println!("  Artifact:     {artifact}");
        println!("  Backend:      {backend}");
        println!("  CacheDir:     {}/", cache_dir.display());
        println!();
        println!("  Security:     {security}");
        println!();
    }
}

/// Compile and execute a `.vox` source file as a script.
///
/// Uses content-hash caching to avoid redundant recompiles. Dispatches
/// to [`NativeBackend`] or [`WasiBackend`] depending on `opts`.
pub async fn run(file: &Path, args: &[String], opts: &ScriptOpts) -> Result<()> {
    // P3.2: Perform a light GC of old script entries only on cache-miss paths.
    // We track whether we hit the cache to skip GC on warm runs.
    let mut is_cache_hit = false;

    let result: crate::pipeline::FrontendResult =
        crate::pipeline::run_frontend(file, false).await?;

    if !result.module.has_entrypoint() {
        anyhow::bail!(
            "No `fn main()` found in {}. Script files must contain a top-level main function.",
            file.display()
        );
    }

    if result.has_errors() {
        crate::pipeline::print_diagnostics(&result, file, false);
        anyhow::bail!("Type checking failed");
    }

    // Mens registry publish runs once at the start of `commands::run::run` (all `vox run` modes).

    let hir = &result.hir;
    let source = &result.source;

    let backend = opts.backend();

    // Compute content-hash cache key using SipHasher for cross-version stability.
    // DefaultHasher is not guaranteed stable across Rust versions; SipHasher is.
    let hash = {
        let mut hasher = twox_hash::XxHash64::default();
        // Seed with a fixed version so we can bump it when codegen / build layout changes.
        "vox-cache-v3".hash(&mut hasher);
        source.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };

    let cache_dir = vox_config::paths::script_cache_dir(opts.use_wasi()).join(&hash);
    let ws = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let lane = if opts.use_wasi() {
        crate::build_lock::BuildLane::ScriptWasi
    } else {
        crate::build_lock::BuildLane::ScriptNative
    };
    let shared_target = crate::build_lock::resolve_target_dir(
        lane,
        &ws.display().to_string(),
        crate::build_lock::lane_isolation(),
    );

    let stamp_path = cache_dir.join(".compiled");

    let artifact_path = if !opts.no_cache && stamp_path.exists() {
        // Cache hit — skip GC, skip compile
        is_cache_hit = true;
        let binary_name = if backend.cache_label().contains("wasi") {
            "vox-script.wasm"
        } else if cfg!(target_os = "windows") {
            "vox-script.exe"
        } else {
            "vox-script"
        };
        cache_dir.join(binary_name)
    } else {
        // Cache miss or force recompile
        std::fs::create_dir_all(&cache_dir)?;
        let path = backend.compile(hir, &cache_dir, &shared_target, opts)?;
        std::fs::write(&stamp_path, &hash).ok();
        path
    };

    // GC only on fresh compiles, not warm cache hits (avoids slow I/O on every run)
    if !is_cache_hit {
        let max_entries = std::env::var("VOX_SCRIPT_CACHE_MAX_ENTRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100usize);
        let max_mb = std::env::var("VOX_SCRIPT_CACHE_MAX_SIZE_MB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500u64);
        let _ = crate::fs_utils::gc_script_cache(max_entries, max_mb);
    }

    // Execute via backend
    let status = backend.execute(&artifact_path, args, opts)?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

/// Evaluate a Vox expression inline — wraps it in a synthetic `fn main`.
pub async fn eval_inline(expr: &str, sandbox: bool) -> Result<()> {
    let synthetic_source = format!("fn main():\n    print(str({}))\n", expr);

    // Convention: stable path for the inline eval scratch file, shared across
    // repeated `vox eval` invocations. Do NOT replace with tempfile::tempdir()
    // — the compiler subprocess needs to find this file by a predictable path.
    let tmp_dir = std::env::temp_dir().join("vox-eval");
    fs::create_dir_all(&tmp_dir)?;
    let tmp_file = tmp_dir.join("eval_script.vox");
    fs::write(&tmp_file, &synthetic_source)?;

    let opts = ScriptOpts {
        sandbox,
        allow_mcp: false,
        no_cache: false,
        isolation: None,
        trust_class: None,
        #[cfg(feature = "script-execution")]
        wasi_dirs: Vec::new(),
    };

    run(&tmp_file, &[], &opts).await
}
