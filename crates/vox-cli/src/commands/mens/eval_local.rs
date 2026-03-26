//! `vox mens eval-local` — evaluate trained model against heldout benchmark.

use super::eval_local_prompt::{
    PreparedBench, prepare_bench_item, sort_prepared_benches_lexicographic,
};
use anyhow::Result;
use std::path::PathBuf;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

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
        host: "127.0.0.1".to_string(),
        max_tokens,
        temperature,
        system_prompt: None,
    };

    #[cfg(feature = "gpu")]
    let mut engine = if model.exists() {
        vox_populi::mens::tensor::candle_inference_serve::InferenceEngine::load(
            &model,
            &vox_populi::mens::DeviceKind::Cuda,
        )
        .ok()
    } else {
        None
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut passed_k = 0usize;
    let mut passed_1 = 0usize;
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
                if let Some(ref mut eng) = engine {
                    let k = samples.max(1);
                    let mut per_sample = Vec::with_capacity(k);
                    for sample_idx in 0..k {
                        let seed = seed_base
                            .saturating_add((manifest_index as u64).saturating_mul(1_000))
                            .saturating_add(sample_idx as u64);
                        let seed_opt = if temperature > 0.0 { Some(seed) } else { None };
                        // InferenceEngine currently accepts `top_p` rather than seed.
                        // Keep seed in eval metadata for reproducibility bookkeeping.
                        match eng.generate(&prompt, max_tokens, temperature as f64, None) {
                            Ok(output) => {
                                let verify =
                                    verify_completion(&output, &bench, &file, &id, manifest_index);
                                per_sample.push(serde_json::json!({
                                    "sample_index": sample_idx,
                                    "seed": seed_opt,
                                    "pass": verify.pass,
                                    "checks": verify.checks,
                                    "completion_chars": output.len(),
                                }));
                            }
                            Err(e) => {
                                eprintln!(
                                    "  {} Inference failed for {} sample {}: {}",
                                    "⚠".yellow(),
                                    id,
                                    sample_idx,
                                    e
                                );
                                per_sample.push(serde_json::json!({
                                    "sample_index": sample_idx,
                                    "seed": seed_opt,
                                    "pass": false,
                                    "error": e.to_string(),
                                }));
                            }
                        }
                    }
                    let pass1 = per_sample
                        .first()
                        .and_then(|v| v.get("pass"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let passk = per_sample
                        .iter()
                        .any(|v| v.get("pass").and_then(|x| x.as_bool()).unwrap_or(false));
                    (pass1, passk, per_sample)
                } else {
                    (
                        false,
                        false,
                        vec![serde_json::json!({
                            "sample_index": 0,
                            "pass": false,
                            "error": "engine fail to load"
                        })],
                    )
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

        let entry = serde_json::json!({
            "manifest_index": manifest_index,
            "id": id,
            "file": file,
            "context_files": context_files,
            "category": category,
            "description": description,
            "pass_at_1": pass_at_1,
            "pass_at_k": pass_at_k,
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
    eprintln!();
    eprintln!("  {}", "─".repeat(54));
    eprintln!(
        "  Overall: pass@1 {}/{} ({:.0}%) | pass@{} {}/{} ({:.0}%)",
        passed_1,
        total,
        pass_rate_at_1 * 100.0,
        samples.max(1),
        passed_k,
        total,
        pass_rate_at_k * 100.0
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
    crate::benchmark_telemetry::record_opt_blocking(
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
    checks: serde_json::Value,
}

fn verify_completion(
    completion: &str,
    bench_root: &std::path::Path,
    file_hint: &str,
    sample_id: &str,
    manifest_index: usize,
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
        target: "vox_mens_eval_local",
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
    CompletionVerification {
        pass,
        checks: serde_json::json!({
            "non_empty": non_empty,
            "parse_ok": parse_ok,
            "typecheck_ok": typecheck_ok,
            "diag_errors": diag_errors,
            "parse_error": parse_error
        }),
    }
}
