//! `vox ci cuda-release-build` — mirrors `cargo vox-cuda-release` / `scripts/populi/cursor_background_cuda_build.ps1`.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::commands::ci::cargo_bin;

fn cuda_release_build_command_spec() -> [&'static str; 8] {
    [
        "build",
        "-p",
        "vox-ml-cli",
        "--bin",
        "vox-ml-cli",
        "--release",
        "--features",
        "gpu,mens-candle-cuda",
    ]
}

/// `cargo build -p vox-ml-cli --bin vox-ml-cli --release --features gpu,mens-candle-cuda`, tee to `log_dir/cuda_build_<UTC>.log`.
pub(crate) fn run_cuda_release_build(root: &Path, log_dir: PathBuf) -> Result<()> {
    fs::create_dir_all(&log_dir).with_context(|| format!("mkdir {}", log_dir.display()))?;
    let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let log_path = log_dir.join(format!("cuda_build_{stamp}.log"));

    eprintln!(
        ">> vox-ml-cli CUDA release build (log: {}); needs nvcc + toolchain per docs",
        log_path.display()
    );

    let cargo = cargo_bin();
    let mut cmd = Command::new(&cargo);
    cmd.current_dir(root)
        .args(cuda_release_build_command_spec());
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .context("spawn cargo CUDA release build for vox-ml-cli")?;
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");

    let log = Arc::new(Mutex::new(
        fs::File::create(&log_path).with_context(|| format!("create {}", log_path.display()))?,
    ));

    let h_out = {
        let log = Arc::clone(&log);
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                println!("{line}");
                if let Ok(mut w) = log.lock() {
                    let _ = writeln!(w, "{line}");
                    let _ = w.flush();
                }
            }
        })
    };
    let h_err = {
        let log = Arc::clone(&log);
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                eprintln!("{line}");
                if let Ok(mut w) = log.lock() {
                    let _ = writeln!(w, "{line}");
                    let _ = w.flush();
                }
            }
        })
    };

    let st = child.wait().context("wait cargo")?;
    let _ = h_out.join();
    let _ = h_err.join();
    if !st.success() {
        return Err(anyhow!(
            "vox-ml-cli CUDA release build failed ({st}); see {}",
            log_path.display()
        ));
    }
    println!(
        "vox-ml-cli CUDA release build OK — log: {}",
        log_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::cuda_release_build_command_spec;

    #[test]
    fn cuda_release_build_command_spec_targets_ml_cli() {
        assert_eq!(
            cuda_release_build_command_spec(),
            [
                "build",
                "-p",
                "vox-ml-cli",
                "--bin",
                "vox-ml-cli",
                "--release",
                "--features",
                "gpu,mens-candle-cuda",
            ]
        );
    }
}
