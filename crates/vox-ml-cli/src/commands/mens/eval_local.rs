//! `vox mens eval-local` — evaluate trained model against heldout benchmark.

use super::eval_local_prompt::{
    PreparedBench, prepare_bench_item, sort_prepared_benches_lexicographic,
};
use super::metrics::verify_completion;
use anyhow::{Context, Result};
use std::path::PathBuf;

use vox_bounded_fs::read_utf8_path_capped;

pub fn run_eval_local(
    model: PathBuf,
    bench: PathBuf,
    max_tokens: usize,
    temperature: f32,
    samples: usize,
    seed_base: u64,
    output: Option<PathBuf>,
) -> Result<()> {
    use owo_colors::OwoColorize;

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
                            "pass_compile": v.pass_compile,
                            "pass_ast": v.pass_ast,
                            "pass_exec": v.pass_exec,
                            "semantic_pass": v.semantic_pass,
                            "anti_stub_pass": v.anti_stub_pass,
                            "distinct_4_ratio": v.distinct_4_ratio,
                            "constraint_adherence": v.constraint_adherence,
                            "symbol_binding_accuracy": v.symbol_binding_accuracy,
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
