//! Device selection, GPU probing (best-effort), and VRAM heuristics for Mens training.

/// CLI / env device intent for Burn (wgpu) and Candle qlora backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// Prefer host CPU / software paths.
    Cpu,
    /// Let the stack pick (wgpu default adapter; Candle CUDA → Metal → CPU when available).
    Best,
    /// Prefer NVIDIA CUDA when the Candle build supports it.
    Cuda,
    /// Prefer Apple Metal when the Candle build supports it (macOS).
    Metal,
}

/// Best-effort local GPU description (VRAM may be **0** when OS APIs are unavailable).
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// Marketing or driver-reported model name.
    pub model_name: String,
    /// Dedicated video memory in megabytes (0 = unknown / integrated / CPU-only).
    pub vram_mb: u64,
    /// Coarse vendor bucket: `nvidia`, `amd`, `intel`, `apple`, `cpu`, or `unknown`.
    pub vendor: String,
}

/// High-level training capacity hint derived from available VRAM (heuristic).
#[derive(Debug, Clone)]
pub struct TrainProfile {
    pub label: &'static str,
    pub max_seq_len: usize,
    pub suggested_batch: usize,
}

/// Parse `--device` / dispatch strings into a [`DeviceKind`].
pub fn normalize_device(raw: &str) -> Result<DeviceKind, String> {
    match raw.trim().to_lowercase().as_str() {
        "" | "cpu" => Ok(DeviceKind::Cpu),
        "best" | "auto" | "default" | "vulkan" | "dx12" | "d3d12" => Ok(DeviceKind::Best),
        "cuda" | "gpu" | "nvidia" => Ok(DeviceKind::Cuda),
        "metal" | "mps" => Ok(DeviceKind::Metal),
        other => Err(format!(
            "unknown device '{other}': expected cpu, best, cuda, metal, vulkan, or dx12"
        )),
    }
}

/// Apply process-level hints before Burn / Candle init (currently conservative / mostly no-op).
pub fn apply_backend_env(kind: DeviceKind) {
    tracing::debug!(?kind, "apply_backend_env");
}

/// Infer vendor string from a free-form GPU model name.
pub fn detect_gpu_vendor() -> String {
    let g = probe_gpu();
    g.vendor
}

fn vendor_from_model(model: &str) -> String {
    let m = model.to_lowercase();
    if m.contains("nvidia")
        || m.contains("geforce")
        || m.contains("rtx")
        || m.contains("gtx")
        || m.contains("quadro")
        || m.contains("tesla")
        || m.contains("titan")
        || m.contains("a100")
        || m.contains("a6000")
        || m.contains("h100")
        || m.contains("l40")
        || m.contains(" l4")
        || m.contains("hopper")
        || m.contains(" 4080")
        || m.contains(" 4090")
        || m.contains(" 4070")
        || m.contains(" 4060")
        || m.contains(" 3090")
        || m.contains(" 3080")
    {
        return "nvidia".into();
    }
    if m.contains("amd") || m.contains("radeon") {
        return "amd".into();
    }
    if m.contains("intel") && (m.contains("arc") || m.contains("iris") || m.contains("uhd")) {
        return "intel".into();
    }
    if m.contains("apple") || m.contains("m1") || m.contains("m2") || m.contains("m3") {
        return "apple".into();
    }
    "unknown".into()
}

/// Probe GPU metadata. Override with **`VOX_GPU_MODEL`** + **`VOX_GPU_VRAM_MB`** for CI or headless hosts.
#[must_use]
pub fn probe_gpu() -> GpuInfo {
    if let (Some(model), Some(vram_s)) = (
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxGpuModel).expose(),
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxGpuVramMb).expose(),
    ) && let Ok(vram_mb) = vram_s.parse::<u64>()
    {
        let vendor = vendor_from_model(model);
        return GpuInfo {
            model_name: model.to_string(),
            vram_mb,
            vendor,
        };
    }

    #[cfg(target_os = "windows")]
    {
        // Try nvidia-smi first on Windows for accurate VRAM (wmic caps at 4GB).
        if let Some(g) = probe_gpu_nvidia_smi_win_fallback() {
            return g;
        }
        if let Some((name, mb)) = probe_gpu_wmic() && mb > 0 {
            let vendor = vendor_from_model(&name);
            return GpuInfo {
                model_name: name,
                vram_mb: mb,
                vendor,
            };
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(g) = probe_gpu_linux() {
            return g;
        }
    }

    GpuInfo {
        model_name: "unknown".into(),
        vram_mb: 0,
        vendor: "cpu".into(),
    }
}

#[cfg(target_os = "linux")]
fn probe_gpu_linux() -> Option<GpuInfo> {
    probe_gpu_nvidia_smi_linux().or_else(probe_gpu_lspci_linux)
}

#[cfg(target_os = "linux")]
fn probe_gpu_nvidia_smi_linux() -> Option<GpuInfo> {
    use std::process::Command;
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let binding = String::from_utf8_lossy(&out.stdout);
    let line = binding.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split(',').map(|s| s.trim());
    let name = parts.next()?.to_string();
    let mb_part = parts.next().unwrap_or("0");
    let vram_mb: u64 = mb_part
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let vendor = vendor_from_model(&name);
    Some(GpuInfo {
        model_name: name,
        vram_mb,
        vendor,
    })
}

#[cfg(target_os = "linux")]
fn probe_gpu_lspci_linux() -> Option<GpuInfo> {
    use std::process::Command;
    let out = Command::new("lspci").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let lower = line.to_lowercase();
        if !lower.contains("vga") && !lower.contains("3d") && !lower.contains("display") {
            continue;
        }
        let name = line.split(':').nth(2).unwrap_or(line).trim().to_string();
        if name.len() < 3 {
            continue;
        }
        let vendor = vendor_from_model(&name);
        return Some(GpuInfo {
            model_name: name,
            vram_mb: 0,
            vendor,
        });
    }
    None
}

#[cfg(target_os = "windows")]
fn probe_gpu_wmic() -> Option<(String, u64)> {
    use std::process::Command;
    let out = Command::new("wmic")
        .args([
            "path",
            "win32_VideoController",
            "get",
            "Name,AdapterRAM",
            "/format:csv",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut best: Option<(String, u64)> = None;
    for line in text.lines().skip(1) {
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() < 3 {
            continue;
        }
        let ram_raw = parts[1].to_string();
        let ram = ram_raw.parse::<u64>().unwrap_or(0);
        let name = parts[2].to_string();
        if name.is_empty() {
            continue;
        }
        let mb = if ram > 0 { ram / (1024 * 1024) } else { 0 };
        best = Some(match best {
            None => (name, mb),
            Some((ref _n0, m0)) if mb > m0 => (name, mb),
            Some(b) => b,
        });
    }
    best
}

#[cfg(target_os = "windows")]
fn probe_gpu_nvidia_smi_win_fallback() -> Option<GpuInfo> {
    use std::process::Command;
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let binding = String::from_utf8_lossy(&out.stdout);
    let line = binding.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split(',').map(|s| s.trim());
    let name = parts.next()?.to_string();
    let mb_part = parts.next().unwrap_or("0");
    let vram_mb: u64 = mb_part
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    // If nvidia-smi failed to give us a real number too, skip
    if vram_mb == 0 {
        return None;
    }
    let vendor = vendor_from_model(&name);
    Some(GpuInfo {
        model_name: name,
        vram_mb,
        vendor,
    })
}

/// Very rough **megabyte** estimate for transformer-style training (activations + weights, FP32-ish).
#[must_use]
pub fn estimate_training_vram_mb(
    d_model: usize,
    n_heads: usize,
    n_layers: usize,
    vocab: usize,
    batch: usize,
    seq_len: usize,
) -> u64 {
    let _ = n_heads;
    let d = d_model as u64;
    let l = n_layers.max(1) as u64;
    let s = seq_len.max(1) as u64;
    let b = batch.max(1) as u64;
    let v = vocab.max(1) as u64;
    let embed = v.saturating_mul(d).saturating_mul(4);
    let block = d
        .saturating_mul(d)
        .saturating_mul(12)
        .saturating_mul(4)
        .saturating_mul(l);
    let acts = b
        .saturating_mul(s)
        .saturating_mul(d)
        .saturating_mul(l)
        .saturating_mul(16);
    let bytes = embed.saturating_add(block).saturating_add(acts);
    (bytes / (1024 * 1024)).max(128)
}

/// Rough VRAM heuristic for **Candle QLoRA** (qlora-rs NF4 frozen bases).
///
/// [`estimate_training_vram_mb`] assumes FP32-ish transformer blocks; NF4 weights are much smaller.
/// Activations, mmap embeddings, and trainable LoRA still dominate on long sequences — this uses a
/// conservative **~35%** scale of the FP heuristic (floor 256 MB). Use for warnings only.
#[must_use]
pub fn estimate_training_vram_mb_qlora(
    d_model: usize,
    n_heads: usize,
    n_layers: usize,
    vocab: usize,
    batch: usize,
    seq_len: usize,
) -> u64 {
    let fp_mb = estimate_training_vram_mb(d_model, n_heads, n_layers, vocab, batch, seq_len);
    fp_mb.saturating_mul(35).saturating_div(100).max(256)
}

/// Log GPU summary via [`tracing`] (`target = "vox_mens_gpu"`).
pub fn print_gpu_summary() {
    let g = probe_gpu();
    tracing::info!(
        target: "vox_mens_gpu",
        model = %g.model_name,
        vendor = %g.vendor,
        vram_mb = g.vram_mb,
        "[mens-gpu] {} | vendor={} | vram_mb={}",
        g.model_name,
        g.vendor,
        g.vram_mb,
    );
}

/// Same as [`print_gpu_summary`] with a caller prefix in the message.
pub fn print_gpu_summary_for(prefix: &str) {
    let g = probe_gpu();
    tracing::info!(
        target: "vox_mens_gpu",
        prefix,
        model = %g.model_name,
        vendor = %g.vendor,
        vram_mb = g.vram_mb,
        "{prefix} {} | vendor={} | vram_mb={}",
        g.model_name,
        g.vendor,
        g.vram_mb,
    );
}

/// VRAM currently in use — not available without driver APIs; returns **`None`**.
#[must_use]
pub fn sample_vram_used_mb() -> Option<u64> {
    let _ = std::hint::black_box(());
    None
}

/// User-facing hint when OOM / allocation fails.
#[must_use]
pub fn oom_guidance() -> &'static str {
    "Try `--preset safe`, lower `--seq-len` / `--batch-size`, or `--device cpu`. See docs/src/architecture/mens-training-ssot.md."
}

/// Map available VRAM to a coarse profile.
#[must_use]
pub fn recommend_config(vram_mb: u64) -> TrainProfile {
    if vram_mb >= 24_000 {
        TrainProfile {
            label: "high",
            max_seq_len: 1024,
            suggested_batch: 4,
        }
    } else if vram_mb >= 8_000 {
        TrainProfile {
            label: "mid",
            max_seq_len: 512,
            suggested_batch: 2,
        }
    } else {
        TrainProfile {
            label: "low",
            max_seq_len: 256,
            suggested_batch: 1,
        }
    }
}

/// Preset-name helper for docs and older call sites.
#[must_use]
pub fn recommend_config_for_profile(preset_name: &str) -> TrainProfile {
    match preset_name {
        "tiny" | "safe" | "4080_safe" => recommend_config(4_000),
        "a100" => recommend_config(40_000),
        _ => recommend_config(16_000),
    }
}

#[cfg(feature = "mens-gpu")]
#[must_use]
pub fn make_wgpu_device() -> burn::backend::wgpu::WgpuDevice {
    burn::backend::wgpu::WgpuDevice::default()
}

#[cfg(test)]
mod vram_estimate_tests {
    use super::*;

    #[test]
    fn qlora_heuristic_below_fp_estimate() {
        let fp = estimate_training_vram_mb(768, 12, 12, 32_000, 1, 512);
        let ql = estimate_training_vram_mb_qlora(768, 12, 12, 32_000, 1, 512);
        assert!(ql < fp, "ql={ql} fp={fp}");
        assert!(ql >= 256);
    }
}
