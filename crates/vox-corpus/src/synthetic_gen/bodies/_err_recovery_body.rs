// ─── Error → Recovery pairs (Gap 6) ──────────────────────────────────────────

/// Training pairs for recognizing and recovering from common build, runtime,
/// and training errors. Teaches the model to diagnose root causes and emit
/// corrective actions rather than summarizing failure.
pub fn generate_error_recovery_pairs(
    out: &mut impl Write,
    _cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;

    let errors: &[(&str, &str, &str)] = &[
        (
            "error[E0502]: cannot borrow `state` as mutable because it is also borrowed as immutable",
            "borrow_checker_conflict",
            "Separate the immutable read and mutable write into distinct scopes. Move the immutable borrow to finish before the mutable borrow begins, or clone the data if both must coexist.",
        ),
        (
            "nvcc fatal: Cannot find compiler 'cl.exe' in PATH",
            "msvc_cuda_path",
            "You must build from within a Visual Studio Developer Command Prompt. Run 'x64 Native Tools Command Prompt for VS 2022' from the Start Menu, navigate to the repo root, and re-run 'cargo vox-cuda-release'. Do NOT use nested subshell calls like 'cmd /c vcvars64.bat && cargo ...'.",
        ),
        (
            "error: Legacy Arca schema chain detected. Run 'vox codex export-legacy', initialize a fresh Codex database, then 'vox codex import-legacy'",
            "db_schema_mismatch",
            "The VoxDb file has an incompatible legacy schema. Steps: (1) backup: rename vox.db to vox.db.bak, (2) run 'vox codex verify' to confirm fresh schema loads, (3) optionally restore data with 'vox codex import-legacy'.",
        ),
        (
            "error: no training rows after rating >= 3 and context filter Some(\"vox\")",
            "empty_training_corpus",
            "The training JSONL has no rows matching the 'vox' category filter. Either: (a) remove --context-filter to use all rows, (b) regenerate corpus with 'vox mens corpus generate', or (c) check that mix.yaml points to files with category='vox' rows.",
        ),
        (
            "CUDA out of memory. Tried to allocate 2.00 GiB",
            "cuda_oom",
            "Reduce memory pressure: (1) lower --seq-len (512→256), (2) reduce --rank (16→8), (3) raise --grad-accum (8→16) to keep effective batch size, (4) use --preset safe or 4080_safe. Set VOX_CANDLE_DEVICE=cpu to fall back to CPU training.",
        ),
        (
            "error: package ID specification `candle-kernels` did not match any packages",
            "cargo_workspace",
            "candle-kernels is a patched crate under patches/ but must be built via the workspace from the repo root (not from patches/candle-kernels-0.9.2/). Navigate to the repo root and run 'cargo build -p vox-cli --features gpu,mens-candle-cuda'.",
        ),
        (
            "thread 'main' panicked at 'Failed to connect to Codex, retrying (3/3)'",
            "db_connection_exhausted",
            "All DB connection retries failed. Check: (1) VOX_DB_URL and VOX_DB_TOKEN env vars are set for remote, or VOX_DB_PATH for local. (2) The db file exists and isn't locked by another process. (3) Run 'vox codex verify' to test connectivity.",
        ),
        (
            "warning: unused variable `result` [-W unused-variables]",
            "unused_variable",
            "Either use the variable or prefix it with '_' (e.g., '_result') to silence the warning. In Rust, unused variables in build scripts can cause CI failures if -D warnings is set.",
        ),
        (
            "error[E0499]: cannot borrow as mutable more than once at a time",
            "double_mutable_borrow",
            "Split the borrow: either restructure to avoid simultaneous mutable references, use interior mutability (RefCell/Mutex), or clone before the second borrow. In async Rust, hold locks for minimal scope and never across await points.",
        ),
        (
            "loss: NaN (training step 42)",
            "training_nan_loss",
            "NaN loss indicates numerical instability. Try: (1) lower learning rate (2e-4 → 5e-5), (2) enable gradient clipping if available, (3) check training data for malformed rows (very long sequences, encoding errors), (4) use --qlora-lm-head-only as escape hatch for deep proxy stacks.",
        ),
    ];

    let prompts = [
        "I see this error: {err}\nHow do I fix it?",
        "Build failed with: {err}\nWhat is the root cause and fix?",
        "Training crashed with: {err}\nWhat should I do?",
        "Error encountered: {err}\nProvide the corrective steps.",
    ];

    for (err_msg, category, solution) in errors {
        for tmpl in &prompts {
            let prompt = tmpl.replace("{err}", err_msg);
            let response = json!({
                "error_class": category,
                "solution": solution,
                "confidence": "high",
            });
            emit_line(out, &prompt, &response, category, "error_recovery")?;
            count += 1;
        }
    }
    Ok(count)
}
