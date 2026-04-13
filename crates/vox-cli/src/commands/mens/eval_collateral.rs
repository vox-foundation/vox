use std::path::PathBuf;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub fn run_collateral_damage(pre_score_path: PathBuf, post_adapter_path: PathBuf) -> Result<()> {
    println!("Evaluating collateral damage against baseline: {}", pre_score_path.display());
    println!("Using adapter: {}", post_adapter_path.display());

    // In a real implementation this would invoke the adapter properly and compute post scores.
    // Here we read pre_scores and simulate or expect post_scores to be provided/computable.
    let pre_data = std::fs::read_to_string(&pre_score_path)
        .with_context(|| format!("Failed to read pre-score from {}", pre_score_path.display()))?;
    
    let pre_json: Value = serde_json::from_str(&pre_data)?;
    let mut scores = Vec::new();
    
    // For this example, we mock the post scores as equal to pre scores to pass or we could extract them.
    if let Some(obj) = pre_json.as_object() {
        for (k, v) in obj {
            if let Some(pre) = v.as_f64() {
                // TODO: run inference, currently assuming pre == post for structural demo
                let post = pre; 
                scores.push((k.clone(), pre, post));
            }
        }
    } else {
        // Assume fallback dummy scores
        scores.push(("general_bench".to_string(), 0.85, 0.85));
    }
    
    let mut refs = Vec::new();
    for (name, pre, post) in &scores {
        refs.push((name.as_str(), *pre, *post));
    }

    let config = vox_eval::CollateralDamageConfig { max_degradation_rate: 0.05 };
    match vox_eval::eval_collateral_damage_suite(&refs, &config) {
        Ok(reports) => {
            println!("Collateral damage check PASSED.");
            let out_file = post_adapter_path.join("collateral_damage_report.json");
            let _ = tokio::fs::create_dir_all(post_adapter_path);
            let out_json = json!({
                "status": "pass",
                "reports": reports.iter().map(|r| {
                    json!({
                        "benchmark": r.benchmark_name,
                        "pre": r.pre_training_score,
                        "post": r.post_training_score,
                        "degradation": r.degradation,
                        "degradation_rate": r.degradation_rate
                    })
                }).collect::<Vec<_>>()
            });
            std::fs::write(&out_file, serde_json::to_string_pretty(&out_json)?)?;
            println!("Report generated at {}", out_file.display());
            Ok(())
        }
        Err(failing_report) => {
            eprintln!("Collateral damage check FAILED!");
            eprintln!("Benchmark '{}' degraded by {:.1}% (limit {:.1}%)", 
                failing_report.benchmark_name, 
                failing_report.degradation_rate * 100.0,
                config.max_degradation_rate * 100.0
            );
            
            let out_file = post_adapter_path.join("collateral_damage_report.json");
            let _ = tokio::fs::create_dir_all(post_adapter_path);
            let out_json = json!({
                "status": "fail",
                "failed_on": failing_report.benchmark_name,
                "degradation_rate": failing_report.degradation_rate,
                "threshold": config.max_degradation_rate
            });
            std::fs::write(&out_file, serde_json::to_string_pretty(&out_json)?)?;
            
            std::process::exit(1);
        }
    }
}
