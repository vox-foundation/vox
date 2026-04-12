//! Legacy `vox train` post-eval gate (mens-dei + gpu).

#[cfg(all(feature = "mens-dei", feature = "gpu"))]
use std::path::Path;

/// Default minimum Vox parse rate for legacy `vox train` post-training eval (`VOX_EVAL_MIN_PARSE_RATE`).
#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) const LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_PARSE_RATE: f64 = 0.80;
/// Default minimum construct coverage **fraction** (0.0–1.0) for legacy `vox train` (`VOX_EVAL_MIN_COVERAGE`).
#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) const LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_COVERAGE: f64 = 0.60;

/// After legacy local/native training, evaluate `train.jsonl` quality and optionally enforce `VOX_EVAL_STRICT`.
///
/// Writes `eval_results.json` with `construct_coverage_pct` on the **0–100** scale so
/// [`super::check_run::check_run`] / `vox mens eval-gate` agree with `eval_local` policy thresholds.
#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) fn run_legacy_train_post_eval_gate(
    data_dir: &Path,
    output_dir: Option<&Path>,
) -> anyhow::Result<()> {
    use owo_colors::OwoColorize;

    let train_jsonl = data_dir.join("train.jsonl");
    if !train_jsonl.exists() {
        return Ok(());
    }

    let min_parse_rate = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxEvalMinParseRate)
        .expose()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_PARSE_RATE);
    let min_coverage = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxEvalMinCoverage)
        .expose()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_COVERAGE);

    let eval_output = output_dir
        .unwrap_or(data_dir)
        .to_path_buf()
        .join("eval_results.json");

    println!("{}", "\n── Post-Training Eval Gate ──".bold());

    let eval_result = crate::commands::corpus::eval_metrics(&train_jsonl);

    let (parse_rate, coverage_frac) = match eval_result {
        Ok(m) => (m.parse_rate, m.coverage_pct),
        Err(e) => {
            eprintln!("{} Eval gate error: {}", "⚠".yellow(), e);
            return Ok(()); // Non-fatal — don't block training on eval errors
        }
    };

    let parse_ok = parse_rate >= min_parse_rate;
    let coverage_ok = coverage_frac >= min_coverage;
    let coverage_pct_display = coverage_frac * 100.0;

    println!(
        "  Vox parse rate:      {:.1}%  (threshold: {:.0}%) {}",
        parse_rate * 100.0,
        min_parse_rate * 100.0,
        if parse_ok {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        }
    );
    println!(
        "  Construct coverage:  {:.1}%  (threshold: {:.0}%) {}",
        coverage_pct_display,
        min_coverage * 100.0,
        if coverage_ok {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        }
    );

    let gate_result = serde_json::json!({
        "vox_parse_rate": parse_rate,
        "construct_coverage_pct": coverage_pct_display,
        "min_parse_rate": min_parse_rate,
        "min_coverage": min_coverage,
        "gate_passed": parse_ok && coverage_ok,
        "timestamp": "unknown",
    });

    std::fs::write(&eval_output, serde_json::to_string_pretty(&gate_result)?).ok(); // Write best-effort

    if !parse_ok || !coverage_ok {
        eprintln!(
            "{}",
            "\n⚠ Eval gate FAILED — training data quality below thresholds."
                .red()
                .bold()
        );
        eprintln!("  Review eval_results.json and regenerate corpus before promoting this model.");
        let marker = eval_output
            .parent()
            .unwrap_or(data_dir)
            .join("eval_gate_failed.json");
        std::fs::write(&marker, serde_json::to_string_pretty(&gate_result)?).ok();
        let strict = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxEvalStrict)
            .expose()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        if strict {
            anyhow::bail!(
                "Eval gate FAILED (VOX_EVAL_STRICT=1). Parse rate: {:.1}%, Coverage: {:.1}%",
                parse_rate * 100.0,
                coverage_pct_display
            );
        }
    } else {
        println!(
            "{}",
            "✓ Eval gate PASSED — training data meets quality thresholds."
                .green()
                .bold()
        );
        let marker = eval_output
            .parent()
            .unwrap_or(data_dir)
            .join("eval_gate_failed.json");
        std::fs::remove_file(&marker).ok();
    }

    Ok(())
}
