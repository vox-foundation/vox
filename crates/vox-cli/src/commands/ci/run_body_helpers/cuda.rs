use anyhow::{Result, anyhow};
use std::process::Command;

use crate::commands::ci::{cargo_bin, nvcc_available, repo_root};

fn cuda_feature_command_specs() -> [[&'static str; 5]; 2] {
    [
        ["check", "-p", "vox-plugin-speech", "--features", "cuda"],
        [
            "check",
            "-p",
            "vox-ml-cli",
            "--features",
            "gpu,mens-candle-cuda",
        ],
    ]
}

pub(crate) fn run_cuda_features() -> Result<()> {
    if std::env::var("SKIP_CUDA_FEATURE_CHECK").unwrap_or_default() == "1" {
        println!("CUDA feature checks skipped (SKIP_CUDA_FEATURE_CHECK=1)");
        return Ok(());
    }
    let nvcc_ok = nvcc_available();
    if !nvcc_ok {
        println!(
            "CUDA feature checks skipped (nvcc not found — use PATH or CUDA_PATH/CUDA_HOME to toolkit root)"
        );
        return Ok(());
    }
    let root = repo_root();
    let cargo = cargo_bin();
    for spec in cuda_feature_command_specs() {
        let status = Command::new(&cargo)
            .current_dir(&root)
            .args(spec)
            .status()?;
        if !status.success() {
            return Err(anyhow!("cargo {} failed", spec.join(" ")));
        }
    }
    println!("CUDA feature checks OK (vox-plugin-speech, vox-ml-cli)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::cuda_feature_command_specs;

    #[test]
    fn cuda_feature_command_specs_target_current_owners() {
        assert_eq!(
            cuda_feature_command_specs(),
            [
                ["check", "-p", "vox-plugin-speech", "--features", "cuda",],
                [
                    "check",
                    "-p",
                    "vox-ml-cli",
                    "--features",
                    "gpu,mens-candle-cuda",
                ],
            ]
        );
    }
}
