//! Dogfood Populi pipeline: corpus extract → validate → pairs → eval → optional native train.
//!
//! Replaces the orchestration previously embedded in `scripts/run_populi_pipeline.ps1`.

use anyhow::Result;
use std::path::PathBuf;

/// Run the same stages as `scripts/run_populi_pipeline.ps1` (thin PowerShell delegate).
pub async fn run(
    data_dir: PathBuf,
    output_dir: PathBuf,
    skip_train: bool,
    strict_gate: bool,
    device: Option<String>,
) -> Result<()> {
    let validated = PathBuf::from("populi/data/validated.jsonl");

    tracing::info!(
        data_dir = %data_dir.display(),
        output_dir = %output_dir.display(),
        skip_train,
        strict_gate,
        "populi pipeline: start"
    );

    if let Some(p) = validated.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&output_dir)?;

    crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Extract {
        dir: PathBuf::from("examples"),
        output: validated.clone(),
    })
    .await?;

    crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Extract {
        dir: PathBuf::from("docs"),
        output: validated.clone(),
    })
    .await?;

    crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Validate {
        input: validated.clone(),
        output: Some(validated.clone()),
        no_recheck: true,
    })
    .await?;

    let train_jsonl = data_dir.join("train.jsonl");
    crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Pairs {
        input: validated.clone(),
        output: train_jsonl.clone(),
        docs: Some(PathBuf::from("docs/src")),
    })
    .await?;

    let eval_out = output_dir.join("eval_results.json");
    crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Eval {
        input: train_jsonl.clone(),
        output: eval_out,
        print_summary: false,
    })
    .await?;

    if skip_train {
        tracing::info!("populi pipeline: skip train (--skip-train)");
        return Ok(());
    }

    #[cfg(feature = "gpu")]
    {
        let device = device.unwrap_or_else(|| "best".into());
        // SAFETY: CLI process; no concurrent `getenv` readers rely on these during this block.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_BENCHMARK", "1");
            if strict_gate {
                std::env::set_var("VOX_EVAL_STRICT", "1");
                std::env::set_var("VOX_BENCHMARK_MIN_PASS_RATE", "0.80");
            } else {
                std::env::set_var("VOX_EVAL_STRICT", "0");
                std::env::set_var("VOX_BENCHMARK_MIN_PASS_RATE", "0.0");
            }
        }

        // Call `train::run_train` directly so this async fn does not recurse through `populi::run`
        // (which would make the future infinitely large).
        crate::commands::populi::train::run_train(
            crate::commands::populi::PopuliTrainBackendCli::Lora.into(),
            None,
            device,
            data_dir,
            output_dir,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            42,
            None,
            None,
            vox_populi::TrainingDeploymentTarget::Workstation,
            "normal".into(),
            None,
            None,
            None,
            crate::commands::populi::PopuliTokenizerCli::Vox.into(),
            false,
            false,
            None,
            false,
            None,
            1,
        )?;
        Ok(())
    }

    #[cfg(not(feature = "gpu"))]
    {
        let _ = device;
        anyhow::bail!(
            "populi pipeline: native train was requested but this `vox` binary was built without the `gpu` feature; pass `--skip-train` or rebuild with `--features gpu`"
        );
    }
}
