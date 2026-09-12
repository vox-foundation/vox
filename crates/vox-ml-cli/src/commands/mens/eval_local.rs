//! `vox mens eval-local` — evaluate trained model against heldout benchmark.

use super::eval_local_prompt::{
    PreparedBench, prepare_bench_item, sort_prepared_benches_lexicographic,
};
use anyhow::{Context, Result};
use std::path::PathBuf;

use vox_bounded_fs::read_utf8_path_capped;
// `AstEvalReport::coverage_score()` is distinct-construct-*kind*-count / 8, not
// a body-substance metric: every task in `mens/data/heldout_bench/manifest.json`
// asks for exactly one top-level declaration, so a correct answer and an
// empty-body stub of it both declare the same single kind and score exactly
// 1/8 = 0.125 — the metric cannot tell them apart at any threshold. The old
// 0.20 floor didn't add stub protection, it just made this dimension reject
// every single-declaration answer, correct or not (verified empirically: the
// manifest's own hand-authored `answer` for `workflow_fetch_save` scores
// 0.125 and failed the gate). Floor it at the maximum a single declaration
// can reach instead; `placeholder_marker_hits` and `is_trivial_placeholder_output`
// (checked alongside this in `verify_completion`) are what actually catch a
// stubbed body — this dimension only rejects the *degenerate* case of zero
// declared constructs (parse failure or a fully empty module).
const ANTI_STUB_MIN_CONSTRUCT_RICHNESS: f64 = 0.125;

pub fn run_eval_local(
    model: Option<PathBuf>,
    base: Option<PathBuf>,
    bench: PathBuf,
    max_tokens: usize,
    temperature: f32,
    samples: usize,
    seed_base: u64,
    output: Option<PathBuf>,
) -> Result<()> {
    use owo_colors::OwoColorize;

    // `--base <dir>` points at a base-model snapshot with no adapter present
    // (the `InferenceEngine::load` base-only path) — the baseline side of a
    // pass@k/BFCL candidate-vs-base comparison. `--model` is the historical
    // (adapter or merged run directory) entry point. Exactly one is required;
    // both being set would silently prefer one over the other.
    let is_base_only = base.is_some();
    let model = match (model, base) {
        (Some(m), None) => m,
        (None, Some(b)) => b,
        (None, None) => anyhow::bail!("either --model or --base is required"),
        (Some(_), Some(_)) => anyhow::bail!("--model and --base are mutually exclusive"),
    };

    if !model.exists() {
        anyhow::bail!(
            "Model checkpoint not found at {}.\n  Run `vox mens train` first.",
            model.display()
        );
    }

    let manifest_path = bench.join("manifest.json");
    let manifest: serde_json::Value = if manifest_path.exists() {
        let content = read_utf8_path_capped(&manifest_path)?;
        serde_json::from_str(&content)?
    } else {
        anyhow::bail!(
            "Benchmark manifest not found at {}",
            manifest_path.display()
        );
    };

    eprintln!("{}", "╔══════════════════════════════════════════╗".cyan());
    eprintln!("{}", "║   Vox Mens — Local Eval Harness        ║".cyan());
    eprintln!("{}", "╚══════════════════════════════════════════╝".cyan());
    eprintln!("  Model:  {}", model.display());
    eprintln!("  Bench:  {}", bench.display());
    eprintln!("  Tokens: {} per sample | temp {}", max_tokens, temperature);
    eprintln!();

    let benchmarks = manifest["benchmarks"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let model_size = std::fs::metadata(&model).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "  Checkpoint: {:.1} MB | {} benchmark samples",
        model_size as f64 / 1_048_576.0,
        benchmarks.len()
    );
    eprintln!(
        "  {} Prompt order: lexicographic by full prompt (prefix-cache friendly)",
        "ℹ".cyan()
    );
    eprintln!();

    #[cfg(feature = "gpu")]
    let _serve_cfg = crate::commands::ai::serve::ServeConfig {
        model_path: model.clone(),
        port: 0,
        host: std::net::Ipv4Addr::LOCALHOST.to_string(),
        max_tokens,
        temperature,
        system_prompt: None,
    };

    // Inference runs through whichever `MlBackend` plugin matches this host's
    // capabilities (CUDA on an NVIDIA host, Metal on Apple Silicon), not a
    // hardcoded id — see vox_plugin_host::resolve_extension_point. Load the
    // model directory once (the handle carries the dir; `run_inference`
    // rebuilds the engine from disk per call), then dispatch one generation
    // per benchmark prompt below. `--model` must be the training run
    // directory containing the merged/adapter + manifest + tokenizer + config.
    #[cfg(feature = "gpu")]
    let engine = {
        let plugin_id = vox_plugin_host::resolve_extension_point(
            "MlBackend",
            crate::commands::schola::merge_qlora::ML_BACKEND_CANDIDATES,
            &vox_plugin_host::probe(),
        )
        .context("no ML backend plugin matches this host's capabilities")?;
        let loaded = vox_plugin_host::cached_code_plugin(plugin_id).with_context(|| {
            format!("{plugin_id} plugin not found — install vox-plugin-{plugin_id}")
        })?;
        let backend = loaded
            .plugin
            .as_ml_backend()
            .into_option()
            .ok_or_else(|| anyhow::anyhow!("{plugin_id} plugin does not provide MlBackend"))?;
        let handle = backend
            .load_model(model.to_string_lossy().as_ref().into())
            .into_result()
            .map_err(|e| anyhow::anyhow!("load_model({}): {e}", model.display()))?;
        eprintln!(
            "  {} Inference backend: {plugin_id} plugin loaded",
            "✓".green()
        );
        // Keep `loaded` alive alongside the handle for the duration of the eval.
        (loaded, backend, handle)
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut passed_k = 0usize;
    let mut passed_1 = 0usize;
    let mut semantic_passed_k = 0usize;
    let mut anti_stub_passed_k = 0usize;
    let mut placeholder_event_count = 0usize;
    let mut trivial_placeholder_event_count = 0usize;
    let mut construct_richness_sum = 0.0_f64;
    let mut category_stats: std::collections::HashMap<String, (usize, usize, usize)> =
        std::collections::HashMap::new();

    #[cfg(feature = "gpu")]
    let _eval_session: Option<String> = None;

    let mut prepared: Vec<PreparedBench> = benchmarks
        .iter()
        .enumerate()
        .map(|(i, item)| prepare_bench_item(&bench, i, item))
        .collect();
    sort_prepared_benches_lexicographic(&mut prepared);

    for w in prepared {
        let prompt = w.prompt;
        let id = w.id;
        let file = w.file;
        let category = w.category;
        let description = w.description;
        let context_files = w.context_files;
        let semantic_expected_contains = w.semantic_expected_contains;
        let manifest_index = w.manifest_index;

        let (pass_at_1, pass_at_k, samples_json): (bool, bool, Vec<serde_json::Value>) = if prompt
            .is_empty()
            || !model.exists()
        {
            (
                false,
                false,
                vec![serde_json::json!({
                    "sample_index": 0,
                    "pass": false,
                    "error": "no prompt or model"
                })],
            )
        } else {
            #[cfg(feature = "gpu")]
            {
                let (_loaded, backend, handle) = &engine;
                // Greedy decoding is deterministic, so one generation suffices for pass@k.
                let prompt_json = serde_json::json!({
                    "prompt": &prompt,
                    "max_tokens": max_tokens,
                    "temperature": temperature,
                })
                .to_string();
                match backend
                    .run_inference(&**handle, prompt_json.as_str().into())
                    .into_result()
                {
                    Ok(resp) => {
                        let completion = serde_json::from_str::<serde_json::Value>(resp.as_str())
                            .ok()
                            .and_then(|v| {
                                v.get("generated_text")
                                    .and_then(|x| x.as_str())
                                    .map(str::to_string)
                            })
                            .unwrap_or_default();
                        let v = verify_completion(
                            &completion,
                            &bench,
                            &file,
                            &id,
                            manifest_index,
                            &semantic_expected_contains,
                        );
                        let sample = serde_json::json!({
                            "sample_index": 0,
                            "pass": v.pass,
                            "semantic_pass": v.semantic_pass,
                            "anti_stub_pass": v.anti_stub_pass,
                            "tool_call_json_valid": looks_like_json_tool_call(&completion),
                            "tool_name_exists": tool_call_names_a_tool(&completion),
                            "tool_call_salvaged": tool_call_was_salvaged(&completion),
                            "checks": v.checks,
                            "completion_preview": completion.chars().take(240).collect::<String>(),
                        });
                        (v.pass, v.pass, vec![sample])
                    }
                    Err(e) => (
                        false,
                        false,
                        vec![serde_json::json!({
                            "sample_index": 0,
                            "pass": false,
                            "error": format!("run_inference: {e}"),
                        })],
                    ),
                }
            }
            #[cfg(not(feature = "gpu"))]
            {
                (
                    false,
                    false,
                    vec![serde_json::json!({
                        "sample_index": 0,
                        "pass": false,
                        "error": format!(
                            "CPU: GPU needed for inference — model {:.1}MB, prompt {} chars",
                            model_size as f64 / 1_048_576.0,
                            prompt.len()
                        )
                    })],
                )
            }
        };

        let semantic_pass_at_k = samples_json.iter().any(|v| {
            v.get("semantic_pass")
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
        });
        let entry = serde_json::json!({
            "manifest_index": manifest_index,
            "id": id,
            "file": file,
            "context_files": context_files,
            "category": category,
            "description": description,
            "pass_at_1": pass_at_1,
            "pass_at_k": pass_at_k,
            "semantic_pass_at_k": semantic_pass_at_k,
            "k": samples.max(1),
            "samples": samples_json,
        });
        results.push(entry);

        let (p1, pk, t) = category_stats.entry(category.clone()).or_insert((0, 0, 0));
        *t += 1;
        if pass_at_1 {
            *p1 += 1;
            passed_1 += 1;
        }
        if pass_at_k {
            *pk += 1;
            passed_k += 1;
        }
        if semantic_pass_at_k {
            semantic_passed_k += 1;
        }
        if samples_json.iter().any(|v| {
            v.get("anti_stub_pass")
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
        }) {
            anti_stub_passed_k += 1;
        }
        if samples_json.iter().any(|v| {
            v.get("checks")
                .and_then(|c| c.get("placeholder_marker_hits"))
                .and_then(|x| x.as_u64())
                .unwrap_or(0)
                > 0
        }) {
            placeholder_event_count += 1;
        }
        if samples_json.iter().any(|v| {
            v.get("checks")
                .and_then(|c| c.get("trivial_placeholder_output"))
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
        }) {
            trivial_placeholder_event_count += 1;
        }
        let construct_richness_best = samples_json
            .iter()
            .filter_map(|v| {
                v.get("checks")
                    .and_then(|c| c.get("construct_richness_score"))
                    .and_then(|x| x.as_f64())
            })
            .fold(0.0_f64, f64::max);
        construct_richness_sum += construct_richness_best;

        let icon = if pass_at_k {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };
        eprintln!(
            "  {} [{:8}] {} — {} (p@1={} p@k={})",
            icon, category, id, description, pass_at_1, pass_at_k
        );
    }

    results.sort_by(|a, b| {
        let ia = a
            .get("manifest_index")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        let ib = b
            .get("manifest_index")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        ia.cmp(&ib)
    });

    let total = benchmarks.len();
    let pass_rate_at_1 = if total > 0 {
        passed_1 as f64 / total as f64
    } else {
        0.0
    };
    let pass_rate_at_k = if total > 0 {
        passed_k as f64 / total as f64
    } else {
        0.0
    };
    let semantic_task_success = if total > 0 {
        semantic_passed_k as f64 / total as f64
    } else {
        0.0
    };
    let anti_stub_task_success = if total > 0 {
        anti_stub_passed_k as f64 / total as f64
    } else {
        0.0
    };
    let placeholder_event_rate = if total > 0 {
        placeholder_event_count as f64 / total as f64
    } else {
        0.0
    };
    let trivial_placeholder_event_rate = if total > 0 {
        trivial_placeholder_event_count as f64 / total as f64
    } else {
        0.0
    };
    let construct_richness_mean = if total > 0 {
        construct_richness_sum / total as f64
    } else {
        0.0
    };
    eprintln!();
    eprintln!("  {}", "─".repeat(54));
    eprintln!(
        "  Overall: pass@1 {}/{} ({:.0}%) | pass@{} {}/{} ({:.0}%) | semantic {:.0}%",
        passed_1,
        total,
        pass_rate_at_1 * 100.0,
        samples.max(1),
        passed_k,
        total,
        pass_rate_at_k * 100.0,
        semantic_task_success * 100.0
    );
    for (cat, (p1, pk, t)) in &category_stats {
        eprintln!("    {:12} p@1 {}/{} p@k {}/{}", cat, p1, t, pk, t);
    }
    eprintln!();

    let report = serde_json::json!({
        "model": model.to_string_lossy(),
        "base_only": is_base_only,
        "bench": bench.to_string_lossy(),
        "max_tokens": max_tokens,
        "temperature": temperature,
        "k": samples.max(1),
        "seed_base": seed_base,
        "total": total,
        "passed_at_1": passed_1,
        "passed_at_k": passed_k,
        "pass_rate_at_1": pass_rate_at_1,
        "pass_rate_at_k": pass_rate_at_k,
        "semantic_task_success": semantic_task_success,
        "anti_stub_task_success": anti_stub_task_success,
        "placeholder_event_rate": placeholder_event_rate,
        "trivial_placeholder_event_rate": trivial_placeholder_event_rate,
        "construct_richness_mean": construct_richness_mean,
        "category_stats": category_stats.iter().map(|(k, (p1, pk, t))| {
            serde_json::json!({
                "category": k,
                "passed_at_1": p1,
                "passed_at_k": pk,
                "total": t,
                "pass_rate_at_1": *p1 as f64 / (*t as f64).max(1.0),
                "pass_rate_at_k": *pk as f64 / (*t as f64).max(1.0)
            })
        }).collect::<Vec<_>>(),
        "results": results,
    });

    if let Some(out_path) = output {
        std::fs::write(&out_path, serde_json::to_string_pretty(&report)?)?;
        eprintln!("  Report saved: {}", out_path.display());
        if let Some(parent) = out_path.parent() {
            let passk_path = parent.join("benchmark_passatk.json");
            std::fs::write(
                &passk_path,
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "vox_mens_benchmark_passatk_v1",
                    "k": samples.max(1),
                    "seed_base": seed_base,
                    "total": total,
                    "pass_rate_at_1": pass_rate_at_1,
                    "pass_rate_at_k": pass_rate_at_k
                }))?,
            )?;
            eprintln!("  pass@k summary saved: {}", passk_path.display());

            // Give eval-gates-agents.yaml's tool_call_salvage_rate a producer:
            // merge (not clobber) it into eval_results.json. rust_compile_rate /
            // clippy_clean_rate / tool_call_valid_json_rate / tool_name_exists_rate
            // are deliberately NOT touched here — see aggregate_gate_producer_keys's
            // doc comment for why (they already have real producers via
            // `vox corpus eval`, and this path used to silently overwrite them
            // with a weaker proxy).
            let eval_results_path = parent.join("eval_results.json");
            let mut eval_results_obj = read_json_object_or_empty(&eval_results_path);
            eval_results_obj.extend(aggregate_gate_producer_keys(&results));
            std::fs::write(
                &eval_results_path,
                serde_json::to_string_pretty(&serde_json::Value::Object(eval_results_obj))?,
            )?;
            eprintln!(
                "  eval_results.json updated: {}",
                eval_results_path.display()
            );
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }

    #[cfg(feature = "gpu")]
    vox_cli_core::benchmark_telemetry::record_opt_blocking(
        "eval_local",
        Some(pass_rate_at_k),
        Some(serde_json::json!({
            "total": total,
            "passed_at_1": passed_1,
            "passed_at_k": passed_k,
            "model": model.to_string_lossy(),
            "bench": bench.to_string_lossy(),
        })),
    );

    Ok(())
}

struct CompletionVerification {
    pass: bool,
    semantic_pass: bool,
    anti_stub_pass: bool,
    checks: serde_json::Value,
}

fn placeholder_marker_hits(source: &str) -> usize {
    let lower = source.to_ascii_lowercase();
    [
        "todo",
        "tbd",
        "placeholder",
        "stub",
        "not implemented",
        "coming soon",
    ]
    .iter()
    .filter(|m| lower.contains(**m))
    .count()
}

fn is_trivial_placeholder_output(source: &str) -> bool {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return true;
    }
    let code_lines = trimmed
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .count();
    code_lines <= 1 || trimmed.eq_ignore_ascii_case("return")
}

/// Lightweight shape check — not a second verifier, just "does this completion
/// parse as JSON" — mirroring `placeholder_marker_hits`'s role as a cheap
/// format signal alongside the real `verify_completion` pass/fail.
fn looks_like_json_tool_call(source: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(source.trim()).is_ok()
}

/// Cheap shape check mirroring `agent_loop.rs`'s `is_tool_call_shape`: an
/// object with a `"name"` key whose value is a string. Deliberately does
/// *not* require `"arguments"` — the live route's `fenced_json_candidate`
/// doesn't either (see `extract_tool_call_json`'s doc comment).
fn is_tool_call_shape(v: &serde_json::Value) -> bool {
    v.as_object()
        .is_some_and(|o| o.get("name").is_some_and(serde_json::Value::is_string))
}

/// Task B2 (MENS end-to-end completion): extract a tool-call-shaped JSON
/// object (`{"name": ...}`) from `source`, via the same fenced-block /
/// bare-text brace-matching extraction `vox-orchestrator-mcp`'s
/// `agent_loop.rs` salvage policy (`salvage_tool_call_from_text`) performs on
/// a live turn — duplicated here (not a shared crate edge: see the
/// dependency-discipline defactor policy) because this benchmark harness
/// calls the raw inference engine directly (`run_inference`), never the
/// `/v1/chat/completions` HTTP route, so there is no live turn to observe the
/// salvage from.
///
/// Mirrors the live route's two-tier strictness *exactly*, because a looser
/// match here made `tool_call_salvage_rate` read more optimistic than what a
/// real turn would recover (found in review after this duplicate first
/// diverged from `agent_loop.rs`):
///   - a fenced ` ```json {...} ``` ` block only needs `"name"` to be a
///     string (`is_tool_call_shape`, no `"arguments"` requirement) — same as
///     `fenced_json_candidate`.
///   - bare (non-fenced) text — including a whole completion that happens to
///     be top-level JSON — requires `"name"` and `"arguments"` to appear as
///     an *adjacent* literal key pair before brace-matching is even
///     attempted, same as `bare_json_candidate`. A model emitting an extra
///     key between them, or `"arguments"` before `"name"`, is a shape the
///     live salvage step does NOT recover, so this harness must not credit
///     it either.
fn extract_tool_call_json(source: &str) -> Option<serde_json::Value> {
    static FENCE_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let fence_re = FENCE_RE
        .get_or_init(|| regex::Regex::new(r"(?s)```json\s*(\{.*?\})\s*```").expect("static regex"));
    for caps in fence_re.captures_iter(source) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&caps[1])
            && is_tool_call_shape(&v)
        {
            return Some(v);
        }
    }
    static NAME_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let name_re = NAME_RE.get_or_init(|| {
        regex::Regex::new(r#""name"\s*:\s*"[^"]+"\s*,\s*"arguments"\s*:"#).expect("static regex")
    });
    for m in name_re.find_iter(source) {
        let bytes = source.as_bytes();
        let mut depth = 0i32;
        let mut start = None;
        let mut i = m.start();
        loop {
            match bytes.get(i) {
                Some(b'}') => depth += 1,
                Some(b'{') => {
                    if depth == 0 {
                        start = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
        let Some(start) = start else { continue };
        let mut depth = 0i32;
        for (j, b) in bytes.iter().enumerate().skip(start) {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(slice) = source.get(start..=j)
                            && let Ok(v) = serde_json::from_str::<serde_json::Value>(slice)
                            && is_tool_call_shape(&v)
                        {
                            return Some(v);
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Task B2: cheap format signal — does `source` contain a tool-call-shaped
/// JSON object naming a (non-empty-string) tool at all, structured or
/// salvaged. Like `looks_like_json_tool_call`, this is not a verifier that
/// the name is a real registered tool: this harness has no tool catalog to
/// check names against (unlike the live `/v1/chat/completions` route, which
/// does — see `vox-ml-cli`'s `serve/handlers.rs`).
fn tool_call_names_a_tool(source: &str) -> bool {
    extract_tool_call_json(source)
        .and_then(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| !s.is_empty())
        })
        .unwrap_or(false)
}

/// Task B2: true when a tool-call-shaped JSON object was only recoverable
/// via prose/fenced-block extraction (`source.trim()` alone does not parse
/// as JSON) — the same "step 2" condition `agent_loop.rs`'s salvage policy
/// uses to tag `tool_call_salvaged: true` on a live turn.
fn tool_call_was_salvaged(source: &str) -> bool {
    if serde_json::from_str::<serde_json::Value>(source.trim()).is_ok() {
        return false;
    }
    extract_tool_call_json(source).is_some()
}

/// Aggregate the `eval_results.json` keys eval-local can *honestly* produce
/// from its own already-computed per-item results — the same
/// `verify_completion` pass / json-shape signal, not a second verifier.
///
/// Only `tool_call_salvage_rate` is emitted here. `rust_compile_rate` /
/// `clippy_clean_rate` / `tool_call_valid_json_rate` / `tool_name_exists_rate`
/// are deliberately **not** computed by this function: each already has a
/// real, corpus-derived producer wired into `vox corpus eval`
/// (`crates/vox-ml-cli/src/commands/corpus/stats.rs::run_eval`), which writes
/// into the same `run_dir/eval_results.json` this function's caller also
/// targets, as part of the mens pipeline's `Eval` stage — i.e. *before*
/// eval-local runs, per the natural pipeline order (train -> corpus-eval ->
/// eval-local).
///
/// - `rust_compile_rate`/`clippy_clean_rate`: `compute_rust_spoke_metrics`
///   (`vox-corpus/src/corpus/eval_rust_metrics.rs`) spawns actual `cargo
///   build`/`cargo clippy`.
/// - `tool_call_valid_json_rate`/`tool_name_exists_rate`:
///   `compute_agentic_spoke_metrics`
///   (`vox-corpus/src/corpus/eval_agentic_metrics.rs`) checks the training
///   corpus's own agent_trace/tool_trace rows against the real tool
///   registry (`vox_mcp_registry`), the same "written by the eval step"
///   producer `eval-gates-agents.yaml`'s header comment documents.
///
/// eval-local's own verifier never runs a Rust compiler, clippy, or the real
/// tool registry — its `pass_at_k`/`anti_stub_pass`/`tool_call_json_valid`/
/// `tool_name_exists` sample flags are all downstream of the same
/// benchmark-completion heuristics, not the ground-truth checks the real
/// producers use. A prior version of this function computed
/// `rust_compile_rate`/`clippy_clean_rate` from that proxy and — because it
/// ran *after* the real producer in the natural pipeline order — silently
/// overwrote the real compiler/linter signal with it (see Task A3 report,
/// "Part (a) fix"). Emitting `tool_call_valid_json_rate`/
/// `tool_name_exists_rate` here would reintroduce the identical collision
/// against `compute_agentic_spoke_metrics`'s output now that it is wired
/// into `run_eval`, so those two keys were removed from this function's
/// output (a mens-end-to-end-completion fast-follow) the same way the rust
/// keys never appear here.
///
/// `tool_call_salvage_rate` has no other producer, so it stays here.
///
/// A category with zero rows omits `tool_call_salvage_rate` entirely
/// (matches the existing "not applicable" semantics elsewhere in
/// `check_run.rs`).
fn aggregate_gate_producer_keys(
    results: &[serde_json::Value],
) -> serde_json::Map<String, serde_json::Value> {
    fn category_of(entry: &serde_json::Value) -> &str {
        entry.get("category").and_then(|c| c.as_str()).unwrap_or("")
    }
    fn any_sample_flag(entry: &serde_json::Value, flag: &str) -> bool {
        entry
            .get("samples")
            .and_then(|s| s.as_array())
            .is_some_and(|samples| {
                samples
                    .iter()
                    .any(|s| s.get(flag).and_then(|v| v.as_bool()).unwrap_or(false))
            })
    }

    let mut out = serde_json::Map::new();

    let agent_rows: Vec<&serde_json::Value> = results
        .iter()
        .filter(|e| matches!(category_of(e), "agent_trace" | "tool_trace"))
        .collect();
    if !agent_rows.is_empty() {
        let n = agent_rows.len() as f64;
        let salvaged = agent_rows
            .iter()
            .filter(|e| any_sample_flag(e, "tool_call_salvaged"))
            .count() as f64;
        out.insert(
            "tool_call_salvage_rate".to_string(),
            serde_json::json!(salvaged / n),
        );
    }

    out
}

/// Read `path` as a JSON object, or an empty object if absent/unparseable.
/// Used to merge eval-local's producer keys into `eval_results.json` without
/// clobbering keys another producer (e.g. `vox corpus eval`) already wrote
/// there — `vox_parse_rate`, `construct_coverage_pct`, `context_breakdown`.
fn read_json_object_or_empty(path: &std::path::Path) -> serde_json::Map<String, serde_json::Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn verify_completion(
    completion: &str,
    bench_root: &std::path::Path,
    file_hint: &str,
    sample_id: &str,
    manifest_index: usize,
    semantic_expected_contains: &[String],
) -> CompletionVerification {
    let non_empty = !completion.trim().is_empty();
    let mut parse_ok = false;
    let mut typecheck_ok = false;
    let parse_error: Option<String>;
    let mut diag_errors = 0usize;

    let candidate_path = if !file_hint.is_empty() {
        bench_root.join(file_hint)
    } else {
        bench_root.join(format!("eval_local_{manifest_index}_{sample_id}.vox"))
    };
    tracing::debug!(
        target: "vox_eval::mens_local",
        sample_id,
        manifest_index,
        file_hint = file_hint,
        payload_len = completion.len(),
        payload_preview = %completion.chars().take(200).collect::<String>(),
        "eval-local verifier payload"
    );
    let frontend = crate::pipeline::run_frontend_str(completion, &candidate_path, false);
    match frontend {
        Ok(res) => {
            parse_ok = true;
            diag_errors = res.error_count();
            typecheck_ok = !res.has_errors();
            parse_error = None;
        }
        Err(err) => {
            parse_error = Some(err.to_string());
        }
    }

    let pass = non_empty && parse_ok && typecheck_ok;
    let placeholder_hits = placeholder_marker_hits(completion);
    let trivial_placeholder = is_trivial_placeholder_output(completion);
    let construct_richness = vox_compiler::ast_eval(completion).coverage_score();
    let anti_stub_pass = placeholder_hits == 0
        && !trivial_placeholder
        && construct_richness >= ANTI_STUB_MIN_CONSTRUCT_RICHNESS;
    let semantic_pass = pass
        && semantic_expected_contains
            .iter()
            .all(|needle| completion.contains(needle));
    CompletionVerification {
        pass: pass && anti_stub_pass,
        semantic_pass,
        anti_stub_pass,
        checks: serde_json::json!({
            "non_empty": non_empty,
            "parse_ok": parse_ok,
            "typecheck_ok": typecheck_ok,
            "diag_errors": diag_errors,
            "parse_error": parse_error,
            "placeholder_marker_hits": placeholder_hits,
            "trivial_placeholder_output": trivial_placeholder,
            "construct_richness_score": construct_richness,
            "anti_stub_pass": anti_stub_pass,
            "semantic_expected_contains": semantic_expected_contains,
            "semantic_pass": semantic_pass
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `--base <base-model-dir>` is the new base-only load path (Task
    /// followup: base-only inference); `--model` is unchanged. Neither or
    /// both being set is a usage error caught before any inference backend
    /// is touched, not a guess at which one the caller meant.
    #[test]
    fn run_eval_local_requires_exactly_one_of_model_or_base() {
        let bench = PathBuf::from("mens/data/heldout_bench");

        let neither = run_eval_local(None, None, bench.clone(), 8, 0.0, 1, 0, None);
        let err = neither.expect_err("neither --model nor --base must be a usage error");
        assert!(
            err.to_string().contains("either --model or --base"),
            "{err}"
        );

        let both = run_eval_local(
            Some(PathBuf::from("/a")),
            Some(PathBuf::from("/b")),
            bench,
            8,
            0.0,
            1,
            0,
            None,
        );
        let err = both.expect_err("--model and --base together must be a usage error");
        assert!(err.to_string().contains("mutually exclusive"), "{err}");
    }

    fn entry(category: &str, pass_at_k: bool, anti_stub_pass: bool) -> serde_json::Value {
        serde_json::json!({
            "category": category,
            "pass_at_k": pass_at_k,
            "samples": [
                {"anti_stub_pass": anti_stub_pass, "tool_call_json_valid": false}
            ],
        })
    }

    #[test]
    fn aggregate_gate_producer_keys_never_emits_rust_keys() {
        // Fix for review finding #1 (Task A3 follow-up): eval-local's own
        // verifier never runs cargo build/clippy — its old rust_compile_rate
        // and clippy_clean_rate were both downstream of the same
        // anti_stub_pass heuristic (rust_compile_rate <= clippy_clean_rate by
        // construction, since `pass = pass && anti_stub_pass`), so it was a
        // proxy pretending to be a compiler signal. Even feeding it a
        // rust_authoring row that would previously have produced
        // rust_compile_rate=1.0 must emit neither key now.
        let results = vec![entry("rust_authoring", true, true)];
        let keys = aggregate_gate_producer_keys(&results);
        assert!(
            !keys.contains_key("rust_compile_rate"),
            "eval-local must never compute rust_compile_rate (no compiler runs here): {keys:?}"
        );
        assert!(
            !keys.contains_key("clippy_clean_rate"),
            "eval-local must never compute clippy_clean_rate (no clippy runs here): {keys:?}"
        );
    }

    #[test]
    fn aggregate_gate_producer_keys_never_emits_tool_call_valid_json_rate() {
        // mens-end-to-end-completion fast-follow: `compute_agentic_spoke_metrics`
        // (wired into `vox corpus eval`'s `run_eval`) is now the real,
        // corpus-derived producer for `tool_call_valid_json_rate`. eval-local
        // must never emit it, or it clobbers that real signal on merge (the
        // exact collision class already fixed for the rust keys below).
        let mut passing = entry("agent_trace", true, true);
        passing["samples"][0]["tool_call_json_valid"] = serde_json::json!(true);
        let failing = entry("tool_trace", false, false);
        let results = vec![passing, failing];
        let keys = aggregate_gate_producer_keys(&results);
        assert!(
            !keys.contains_key("tool_call_valid_json_rate"),
            "eval-local must defer to compute_agentic_spoke_metrics for tool_call_valid_json_rate: {keys:?}"
        );
    }

    #[test]
    fn looks_like_json_tool_call_accepts_json_rejects_prose() {
        assert!(looks_like_json_tool_call(
            r#"{"tool_name":"x","arguments":{}}"#
        ));
        assert!(!looks_like_json_tool_call("not json at all"));
    }

    #[test]
    fn tool_call_names_a_tool_accepts_clean_and_salvaged_shapes() {
        assert!(tool_call_names_a_tool(
            r#"{"name":"read_file","arguments":{}}"#
        ));
        assert!(tool_call_names_a_tool(
            "I'll use it. ```json\n{\"name\":\"read_file\",\"arguments\":{}}\n```"
        ));
        assert!(!tool_call_names_a_tool("just prose, no tool call here"));
        assert!(!tool_call_names_a_tool(r#"{"arguments":{}}"#));
    }

    #[test]
    fn extract_tool_call_json_rejects_extra_key_between_name_and_arguments() {
        // Fix for the salvage-regex divergence (Task followup): the live
        // `/v1/chat/completions` route's `bare_json_candidate`
        // (crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs)
        // requires `"name"` and `"arguments"` to appear as an adjacent
        // literal key pair before it will brace-match and salvage a bare
        // (non-fenced) tool call. A model emitting an extra key wedged in
        // between is a shape the live route does NOT recover — this harness
        // must not credit it as a salvage/tool-name-exists hit either, or
        // `tool_call_salvage_rate` reads more optimistic than what users
        // actually experience.
        let text = r#"{"name": "read_file", "notes": "some extra context", "arguments": {}}"#;
        assert!(
            extract_tool_call_json(text).is_none(),
            "extra key between name and arguments must not be salvaged"
        );
        assert!(!tool_call_names_a_tool(text));
    }

    #[test]
    fn extract_tool_call_json_rejects_arguments_before_name() {
        // Same divergence, other direction: agent_loop.rs's regex anchors on
        // `"name"` occurring before `"arguments"` — arguments-first ordering
        // does not match it, so the live route completes the turn as plain
        // text rather than salvaging.
        let text = r#"{"arguments": {}, "name": "read_file"}"#;
        assert!(
            extract_tool_call_json(text).is_none(),
            "arguments-before-name ordering must not be salvaged"
        );
        assert!(!tool_call_names_a_tool(text));
    }

    #[test]
    fn tool_call_was_salvaged_only_when_extraction_was_needed() {
        assert!(
            !tool_call_was_salvaged(r#"{"name":"read_file","arguments":{}}"#),
            "clean top-level JSON is not a salvage"
        );
        assert!(tool_call_was_salvaged(
            "I'll use it. ```json\n{\"name\":\"read_file\",\"arguments\":{}}\n```"
        ));
        assert!(!tool_call_was_salvaged("no tool call in this text at all"));
    }

    #[test]
    fn tool_name_exists_rate_never_emitted_but_salvage_rate_is() {
        // mens-end-to-end-completion fast-follow: same deferral as
        // tool_call_valid_json_rate above, for `tool_name_exists_rate` —
        // `compute_agentic_spoke_metrics` is now its real producer.
        // `tool_call_salvage_rate` has no other producer, so it still comes
        // from here.
        fn entry_with_flags(
            category: &str,
            name_exists: bool,
            salvaged: bool,
        ) -> serde_json::Value {
            serde_json::json!({
                "category": category,
                "pass_at_k": true,
                "samples": [
                    {"tool_call_json_valid": false, "tool_name_exists": name_exists, "tool_call_salvaged": salvaged}
                ],
            })
        }
        let results = vec![
            entry_with_flags("agent_trace", true, false),
            entry_with_flags("agent_trace", true, true),
            entry_with_flags("tool_trace", false, false),
            entry_with_flags("tool_trace", false, false),
        ];
        let keys = aggregate_gate_producer_keys(&results);
        assert!(
            !keys.contains_key("tool_name_exists_rate"),
            "eval-local must defer to compute_agentic_spoke_metrics for tool_name_exists_rate: {keys:?}"
        );
        assert_eq!(
            keys.get("tool_call_salvage_rate").and_then(|v| v.as_f64()),
            Some(0.25)
        );
    }

    #[test]
    fn eval_results_json_merges_without_clobbering_other_producers() {
        // Another producer (`vox corpus eval`) may have already written
        // vox_parse_rate / construct_coverage_pct into eval_results.json — our
        // write must not destroy those keys.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eval_results.json");
        std::fs::write(
            &path,
            r#"{"vox_parse_rate": 0.99, "construct_coverage_pct": 42.0}"#,
        )
        .unwrap();

        let mut merged = read_json_object_or_empty(&path);
        merged.extend(aggregate_gate_producer_keys(&[entry(
            "agent_trace",
            true,
            true,
        )]));
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&serde_json::Value::Object(merged)).unwrap(),
        )
        .unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["vox_parse_rate"], 0.99);
        assert_eq!(v["construct_coverage_pct"], 42.0);
    }

    #[test]
    fn eval_results_json_merge_leaves_real_rust_producer_values_untouched() {
        // Fix for review finding #2 (Task A3 follow-up): `vox corpus eval`
        // (compute_rust_spoke_metrics — real cargo build/clippy) may already
        // have written rust_compile_rate/clippy_clean_rate into
        // eval_results.json before eval-local runs (the natural pipeline
        // order is train -> corpus-eval -> eval-local). eval-local's merge
        // must never touch those two keys, no matter what its own results
        // contain — proven here by feeding it a rust_authoring row that
        // would previously (before the fix) have produced
        // rust_compile_rate=0.0, and confirming the real producer's 0.87/0.91
        // survive exactly.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eval_results.json");
        std::fs::write(
            &path,
            r#"{"rust_compile_rate": 0.87, "clippy_clean_rate": 0.91}"#,
        )
        .unwrap();

        let mut merged = read_json_object_or_empty(&path);
        merged.extend(aggregate_gate_producer_keys(&[entry(
            "rust_authoring",
            false,
            false,
        )]));
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&serde_json::Value::Object(merged)).unwrap(),
        )
        .unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            v["rust_compile_rate"], 0.87,
            "real compiler-backed rust_compile_rate must survive eval-local's merge untouched"
        );
        assert_eq!(
            v["clippy_clean_rate"], 0.91,
            "real clippy-backed clippy_clean_rate must survive eval-local's merge untouched"
        );
    }

    #[test]
    fn eval_results_json_merge_leaves_real_agentic_producer_values_untouched() {
        // mens-end-to-end-completion fast-follow, agentic-key analogue of the
        // rust test above: `vox corpus eval` (compute_agentic_spoke_metrics —
        // real tool-registry-backed check) may already have written
        // tool_call_valid_json_rate/tool_name_exists_rate into
        // eval_results.json before eval-local runs. eval-local's merge must
        // never touch those two keys, no matter what its own results
        // contain — proven here by feeding it an agent_trace row that would
        // previously (before this fix) have produced
        // tool_call_valid_json_rate=0.0/tool_name_exists_rate=0.0, and
        // confirming the real producer's 0.93/0.88 survive exactly.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eval_results.json");
        std::fs::write(
            &path,
            r#"{"tool_call_valid_json_rate": 0.93, "tool_name_exists_rate": 0.88}"#,
        )
        .unwrap();

        let mut merged = read_json_object_or_empty(&path);
        merged.extend(aggregate_gate_producer_keys(&[entry(
            "agent_trace",
            false,
            false,
        )]));
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&serde_json::Value::Object(merged)).unwrap(),
        )
        .unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            v["tool_call_valid_json_rate"], 0.93,
            "real corpus-backed tool_call_valid_json_rate must survive eval-local's merge untouched"
        );
        assert_eq!(
            v["tool_name_exists_rate"], 0.88,
            "real corpus-backed tool_name_exists_rate must survive eval-local's merge untouched"
        );
    }

    /// Fast-follow (Task -1A corpus-ceiling kill test): every task in
    /// `mens/data/heldout_bench/manifest.json` asks for exactly one
    /// top-level declaration, so `AstEvalReport::coverage_score()`
    /// (distinct construct kinds / 8) tops out at 1/8 = 0.125 for *any*
    /// correct single-declaration answer — below the old 0.20 anti-stub
    /// floor. This reproduces that with the manifest's own hand-authored,
    /// verified-correct answer for `workflow_fetch_save`.
    #[test]
    fn anti_stub_gate_accepts_correct_single_declaration_answer() {
        let answer = "workflow fetch_and_save(url: str) to str {\n    let data = http.get(url)\n    fs.write_file(\"output.txt\", data)\n    return data\n}";
        let dir = tempfile::tempdir().unwrap();
        let v = verify_completion(answer, dir.path(), "", "workflow_fetch_save", 0, &[]);

        let richness = v.checks["construct_richness_score"].as_f64().unwrap();
        assert!(
            (richness - 0.125).abs() < 1e-9,
            "a single top-level declaration can only reach 1/8 distinct \
             construct kinds; got {richness}"
        );
        assert!(
            v.anti_stub_pass,
            "a verified-correct single-declaration answer must pass the \
             anti-stub gate: {:?}",
            v.checks
        );
    }

    /// Companion to the test above: a genuinely stubbed body for the same
    /// task must still fail the anti-stub gate after the fix — the
    /// construct-richness floor was never what caught this class of stub
    /// (a stub's `fn`/`workflow` kind scores identically to a correct
    /// answer's); `placeholder_marker_hits` is, and must keep working.
    #[test]
    fn anti_stub_gate_still_rejects_placeholder_stub_for_same_task() {
        let stub = "workflow fetch_and_save(url: str) to str {\n    // TODO: implement\n}";
        let dir = tempfile::tempdir().unwrap();
        let v = verify_completion(stub, dir.path(), "", "workflow_fetch_save", 0, &[]);

        assert!(
            !v.anti_stub_pass,
            "a TODO-stubbed body must still fail the anti-stub gate: {:?}",
            v.checks
        );
    }
}
