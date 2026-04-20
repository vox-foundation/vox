//! Candle device selection and fresh-run artifact cleanup.

use std::path::Path;

use anyhow::Result;
use candle_core::Device;

use crate::mens::tensor::device::{DeviceKind, probe_gpu};
use crate::mens::tensor::train_log;

pub(super) fn select_candle_device(
    kind: DeviceKind,
    allow_cpu_fallback: bool,
) -> Result<(Device, String)> {
    let device_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxCandleDevice);
    if device_resolved
        .expose()
        .is_some_and(|v| v.trim().to_lowercase() == "cpu")
    {
        return Ok((Device::Cpu, "cpu(forced)".into()));
    }

    let (device, label) = match kind {
        DeviceKind::Cpu => (Device::Cpu, "cpu".into()),
        DeviceKind::Cuda => (Device::new_cuda(0)?, "cuda:0".into()),
        DeviceKind::Metal => (Device::new_metal(0)?, "metal:0".into()),
        DeviceKind::Best => {
            let g = probe_gpu();
            #[cfg(feature = "mens-candle-qlora-cuda")]
            {
                if g.vendor.as_str() == "apple" {
                    let d = match Device::new_metal(0) {
                        Ok(device) => device,
                        Err(err) => {
                            if !allow_cpu_fallback {
                                anyhow::bail!("Metal unavailable and CPU fallback disabled: {err}");
                            }
                            train_log::warn("Metal unavailable — falling back to CPU");
                            Device::Cpu
                        }
                    };
                    let lbl = if matches!(d, Device::Cpu) {
                        "cpu(fallback)"
                    } else {
                        "metal:0"
                    };
                    (d, lbl.into())
                } else {
                    // WMIC / env probes often report `unknown`; prefer a real CUDA init when this
                    // binary is CUDA-enabled.
                    match Device::new_cuda(0) {
                        Ok(device) => (device, "cuda:0".into()),
                        Err(err) => {
                            if !allow_cpu_fallback {
                                anyhow::bail!("CUDA unavailable and CPU fallback disabled: {err}");
                            }
                            train_log::warn(&format!(
                                "CUDA unavailable ({err}) — falling back to CPU (GPU vendor probe='{}')",
                                g.vendor
                            ));
                            (Device::Cpu, "cpu(fallback)".into())
                        }
                    }
                }
            }
            #[cfg(not(feature = "mens-candle-qlora-cuda"))]
            {
                match g.vendor.as_str() {
                    "nvidia" => {
                        let has_nvcc = std::process::Command::new("nvcc")
                            .arg("--version")
                            .output()
                            .is_ok();
                        if !has_nvcc {
                            train_log::warn(
                                "NVIDIA GPU detected, but Vox was built without CUDA support.",
                            );
                            if allow_cpu_fallback {
                                train_log::warn(
                                    "Falling back to CPU because CUDA Toolkit (nvcc) is missing.",
                                );
                                (Device::Cpu, "cpu(no-cuda-build)".into())
                            } else {
                                anyhow::bail!(
                                    "NVIDIA GPU detected, but CUDA Toolkit is missing. Please install it, or bypass via `--allow-cpu-fallback`."
                                );
                            }
                        } else {
                            train_log::warn(
                                "NVIDIA GPU detected and CUDA Toolkit (nvcc) is present.",
                            );
                            train_log::warn(
                                "JIT-orchestrating recompilation of vox-cli with CUDA features...",
                            );

                            let mut cmd = std::process::Command::new("cargo");
                            cmd.args([
                                "run",
                                "--release",
                                "-p",
                                "vox-cli",
                                "--features",
                                "gpu,mens-candle-cuda",
                                "--",
                            ]);
                            let args: Vec<String> = std::env::args().skip(1).collect();
                            cmd.args(args);

                            let mut child = cmd.spawn().expect("failed to spawn JIT recompilation");
                            let status = child.wait().expect("failed to wait on JIT recompilation");
                            std::process::exit(status.code().unwrap_or(1));
                        }
                    }
                    "apple" => {
                        let d = match Device::new_metal(0) {
                            Ok(device) => device,
                            Err(err) => {
                                if !allow_cpu_fallback {
                                    anyhow::bail!(
                                        "Metal unavailable and CPU fallback disabled: {err}"
                                    );
                                }
                                train_log::warn("Metal unavailable — falling back to CPU");
                                Device::Cpu
                            }
                        };
                        let lbl = if matches!(d, Device::Cpu) {
                            "cpu(fallback)"
                        } else {
                            "metal:0"
                        };
                        (d, lbl.into())
                    }
                    v => {
                        train_log::warn(&format!(
                            "Unknown GPU vendor '{v}' — falling back to CPU (non-CUDA build)"
                        ));
                        (Device::Cpu, "cpu(fallback)".into())
                    }
                }
            }
        }
    };
    Ok((device, label))
}

/// Remove prior QLoRA checkpoints in `out` so a `--force-restart` run starts clean on disk.
pub(super) fn purge_fresh_start_artifacts(out: &Path) -> std::io::Result<()> {
    let Ok(rd) = std::fs::read_dir(out) else {
        return Ok(());
    };
    for ent in rd.flatten() {
        let name = ent.file_name();
        let s = name.to_string_lossy();
        let drop = s == crate::mens::tensor::checkpoint_state::CHECKPOINT_FILENAME
            || s == "checkpoint_state.json.tmp"
            || s == "candle_qlora_adapter.safetensors"
            || (s.starts_with("pause_step_") && s.ends_with(".safetensors"))
            || (s.starts_with("checkpoint_step_") && s.ends_with(".safetensors"))
            || (s.starts_with("checkpoint_epoch_") && s.ends_with(".safetensors"));
        if drop {
            let _ = std::fs::remove_file(ent.path());
        }
    }
    Ok(())
}
