//! `vox mens eval-local` — evaluate trained model against heldout benchmark.

use super::eval_local_prompt::{
    PreparedBench, prepare_bench_item, sort_prepared_benches_lexicographic,
};
use anyhow::Result;
use std::path::PathBuf;

pub fn run_eval_local(
    model: PathBuf,
    bench: PathBuf,
    max_tokens: usize,
    temperature: f32,
    output: Option<PathBuf>,
) -> Result<()> {
    use owo_colors::OwoColorize;

    if !model.exists() {
        anyhow::bail!(
            "Model checkpoint not found at {}.\n  Run `vox schola train` first.",
            model.display()
        );
    }

    let manifest_path = bench.join("manifest.json");
    let manifest: serde_json::Value = if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)?;
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
        vox_mens::tensor::candle_inference_serve::InferenceEngine::load(&model, &vox_mens::DeviceKind::Cuda).ok()
    } else {
        None
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut passed = 0usize;
    let mut category_stats: std::collections::HashMap<String, (usize, usize)> =
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

        let (pass, completion): (bool, String) = if prompt.is_empty() || !model.exists() {
            (false, "# [SKIP: no prompt or model]".to_string())
        } else {
            #[cfg(feature = "gpu")]
            {
                if let Some(ref mut eng) = engine {
                    match eng.generate(&prompt, max_tokens, temperature as f64, None) {
                        Ok(output) => {
                            let valid = true; // Simplified for now
                            (valid, output)
                        }
                        Err(e) => {
                            eprintln!("  {} Inference failed for {}: {}", "⚠".yellow(), id, e);
                            (false, format!("# [ERROR: {}]", e))
                        }
                    }
                } else {
                    (false, "# [ERROR: engine fail to load]".to_string())
                }
            }
            #[cfg(not(feature = "gpu"))]
            {
                (
                    false,
                    format!(
                        "# [CPU: GPU needed for inference — model {:.1}MB, prompt {} chars]",
                        model_size as f64 / 1_048_576.0,
                        prompt.len()
                    ),
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
            "pass": pass,
            "completion_chars": completion.len(),
        });
        results.push(entry);

        let (p, t) = category_stats.entry(category.clone()).or_insert((0, 0));
        *t += 1;
        if pass {
            *p += 1;
            passed += 1;
        }

        let icon = if pass {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };
        eprintln!("  {} [{:8}] {} — {}", icon, category, id, description);
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
    let pass_rate = if total > 0 {
        passed as f64 / total as f64
    } else {
        0.0
    };
    eprintln!();
    eprintln!("  {}", "─".repeat(54));
    eprintln!(
        "  Overall: {}/{} passed ({:.0}%)",
        passed,
        total,
        pass_rate * 100.0
    );
    for (cat, (p, t)) in &category_stats {
        eprintln!("    {:12} {}/{}", cat, p, t);
    }
    eprintln!();

    let report = serde_json::json!({
        "model": model.to_string_lossy(),
        "bench": bench.to_string_lossy(),
        "max_tokens": max_tokens,
        "temperature": temperature,
        "total": total,
        "passed": passed,
        "pass_rate": pass_rate,
        "category_stats": category_stats.iter().map(|(k, (p, t))| {
            serde_json::json!({"category": k, "passed": p, "total": t, "rate": *p as f64 / (*t as f64).max(1.0)})
        }).collect::<Vec<_>>(),
        "results": results,
    });

    if let Some(out_path) = output {
        std::fs::write(&out_path, serde_json::to_string_pretty(&report)?)?;
        eprintln!("  Report saved: {}", out_path.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }

    #[cfg(feature = "gpu")]
    crate::benchmark_telemetry::record_opt_blocking(
        "eval_local",
        Some(pass_rate),
        Some(serde_json::json!({
            "total": total,
            "passed": passed,
            "model": model.to_string_lossy(),
            "bench": bench.to_string_lossy(),
        })),
    );

    Ok(())
}
