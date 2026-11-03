//! The live diagram-authoring run (EVAL-01): the one explicitly invoked step that calls models.
//! Never part of default CI. Run it only as plan 91-05 describes:
//!
//! ```text
//! EVAL_DIAGRAM_FREEZE_COMMIT=<pre-registration commit> \
//!   cargo test -p vox-cli --test it -- --ignored --exact eval_diagram_live::live_run --nocapture
//! ```
//!
//! Optional: `EVAL_DIAGRAM_MAX_SPEND_USD` (default and maximum 25) and `EVAL_DIAGRAM_RUN_NAME`
//! (default the UTC date). Every precondition fails closed with a HUMAN ACTION REQUIRED message
//! before any model call; the run never fabricates an output.

use std::path::Path;

use vox_cli::commands::model::eval_diagram::live::{LiveConfig, run_live};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "live LLM run: spends up to the $25 cap; run only as 91-05 describes"]
async fn live_run() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let max_spend_usd = match std::env::var("EVAL_DIAGRAM_MAX_SPEND_USD") {
        Ok(v) => v.parse().unwrap_or_else(|e| {
            panic!("HUMAN ACTION REQUIRED: EVAL_DIAGRAM_MAX_SPEND_USD {v:?}: {e}")
        }),
        Err(_) => 25.0,
    };
    let cfg = LiveConfig {
        dataset_dir: repo_root
            .join("contracts")
            .join("eval")
            .join("diagram-authoring"),
        report_path: repo_root
            .join("docs")
            .join("src")
            .join("architecture")
            .join("voxdiagram-authoring-reliability-findings-2026.md"),
        run_name: std::env::var("EVAL_DIAGRAM_RUN_NAME")
            .unwrap_or_else(|_| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        freeze_commit: std::env::var("EVAL_DIAGRAM_FREEZE_COMMIT").unwrap_or_default(),
        prereg_subject_prefix: "docs(91-04): pre-register".into(),
        max_spend_usd,
        repo_root,
    };
    match run_live(&cfg).await {
        Ok(out) => {
            let attempts =
                std::fs::read_to_string(out.run_dir.join("attempts.jsonl")).unwrap_or_default();
            println!(
                "header (selection, models, probes): {}",
                attempts.lines().next().unwrap_or("")
            );
            println!("run directory: {}", out.run_dir.display());
            println!("calls: {}, spent: {} USD", out.calls, out.spent_usd);
            println!("stopped: {:?}, complete: {}", out.stopped, out.complete);
            for p in &out.problems {
                println!("problem: {p}");
            }
            assert!(
                out.complete,
                "HUMAN ACTION REQUIRED: the run is incomplete: {out:?}"
            );
        }
        Err(e) => panic!("{e:#}"),
    }
}
