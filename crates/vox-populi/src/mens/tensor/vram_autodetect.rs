//! VRAM auto-detection and training preset selection.
//!
//! Uses the HardwareRegistry SSOT to identify available video memory.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VramInfo {
    pub total_gb: f32,
    pub used_gb: f32,
    pub free_gb: f32,
}

fn parse_nvidia_smi_output(stdout: &str) -> Option<VramInfo> {
    let first_line = stdout.lines().next()?.trim();
    let parts: Vec<&str> = first_line.split(',').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return None;
    }
    let total_mib: f32 = parts[0].parse().ok()?;
    let used_mib: f32 = parts[1].parse().ok()?;
    let free_mib: f32 = parts[2].parse().ok()?;
    Some(VramInfo {
        total_gb: total_mib / 1024.0,
        used_gb: used_mib / 1024.0,
        free_gb: free_mib / 1024.0,
    })
}

/// Query total, used, and free VRAM info from `nvidia-smi`.
pub fn query_nvidia_smi_vram() -> Option<VramInfo> {
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.total,memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_nvidia_smi_output(&String::from_utf8_lossy(&out.stdout))
}

/// Query available GPU VRAM info.
pub fn get_system_vram_info() -> Option<VramInfo> {
    // Priority 1: env override
    if let Some(v) = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxVramOverrideGb).expose()
        && let Ok(gb) = v.parse::<f32>()
        && gb > 0.0
    {
        return Some(VramInfo {
            total_gb: gb,
            used_gb: 0.0,
            free_gb: gb,
        });
    }

    // Priority 2: nvidia-smi query
    if let Some(info) = query_nvidia_smi_vram() {
        return Some(info);
    }

    // Priority 3: hardware SSOT — covers Apple Silicon via `probe_metal()`'s
    // native Metal `recommendedMaxWorkingSetSize` accessor (see
    // `hardware::macos_metal::probe_metal`). The old shell-based
    // `vm_stat`/`sysctl` heuristic that used to live in this module was
    // deleted in favor of that measured value; see `probe_metal`'s own doc
    // comment for why that's the more trustworthy source.
    let hardware = futures::executor::block_on(crate::mens::hardware::probe());
    if hardware.vram_mb > 0 {
        let gb = hardware.vram_mb as f32 / 1024.0;
        return Some(VramInfo {
            total_gb: gb,
            used_gb: 0.0,
            free_gb: gb,
        });
    }

    None
}

/// Query available GPU VRAM in GiB.
pub fn get_system_vram_gb() -> Option<f32> {
    get_system_vram_info().map(|i| i.total_gb)
}

/// Which accelerator the training run will actually use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceleratorKind {
    Cuda,
    Metal,
    Cpu,
}

impl AcceleratorKind {
    /// Map a `GpuInfo`/`DeviceProfile` vendor string (as produced by
    /// `probe_gpu`, e.g. `"nvidia"`, `"apple"`, `"amd"`, `"unknown"`) to the
    /// accelerator kind.
    #[must_use]
    pub fn from_vendor(vendor: &str) -> Self {
        match vendor.to_ascii_lowercase().as_str() {
            "nvidia" => Self::Cuda,
            "apple" => Self::Metal,
            _ => Self::Cpu,
        }
    }
}

/// Human-readable summary of detected VRAM.
///
/// No longer names an auto-selected preset — the VRAM-tiered preset ladder
/// (`auto_preset`/`auto_preset_for`) was deleted (Task 8): it covered 8B,
/// 14B-QLoRA, 14B-LoRA and 32B with no 27B rung, so a 27B request silently
/// landed on the 14B preset. Real per-host sizing now comes from
/// `memory_model::sweep`/`plan_for` after the model is on disk, not a
/// pre-download VRAM-to-preset guess.
pub fn vram_summary(_device_is_cuda: bool) -> String {
    match get_system_vram_info() {
        Some(i) => format!(
            "VRAM: {:.1} GiB total, {:.1} GiB used, {:.1} GiB free",
            i.total_gb, i.used_gb, i.free_gb
        ),
        None => {
            "Could not detect VRAM (set VOX_VRAM_OVERRIDE_GB or pass --preset manually)".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerator_kind_from_vendor_maps_known_vendors() {
        assert_eq!(
            AcceleratorKind::from_vendor("nvidia"),
            AcceleratorKind::Cuda
        );
        assert_eq!(
            AcceleratorKind::from_vendor("NVIDIA"),
            AcceleratorKind::Cuda
        );
        assert_eq!(
            AcceleratorKind::from_vendor("apple"),
            AcceleratorKind::Metal
        );
        assert_eq!(
            AcceleratorKind::from_vendor("Apple"),
            AcceleratorKind::Metal
        );
        assert_eq!(AcceleratorKind::from_vendor("amd"), AcceleratorKind::Cpu);
        assert_eq!(
            AcceleratorKind::from_vendor("unknown"),
            AcceleratorKind::Cpu
        );
        assert_eq!(AcceleratorKind::from_vendor(""), AcceleratorKind::Cpu);
    }

    #[test]
    #[serial_test::serial(vox_vram_override_env)]
    #[allow(unsafe_code)]
    fn vram_override_env_is_respected() {
        // Set a fake value and confirm it returns correctly.
        unsafe {
            std::env::set_var("VOX_VRAM_OVERRIDE_GB", "20.0");
        }
        assert_eq!(get_system_vram_gb(), Some(20.0));
        unsafe {
            std::env::remove_var("VOX_VRAM_OVERRIDE_GB");
        }
    }

    #[test]
    fn vram_summary_never_mentions_a_preset() {
        // vram_summary no longer names an auto-selected preset (the VRAM-tiered
        // ladder that used to pick one was deleted — see the fn doc comment).
        let summary = vram_summary(false);
        assert!(
            !summary.contains("qwen_4080_16g")
                && !summary.contains("4080")
                && !summary.contains("a100")
                && !summary.contains("safe"),
            "vram_summary must not mention a GPU preset; got: {summary}"
        );
    }

    #[test]
    fn test_parse_nvidia_smi_output() {
        let sample = "16376, 1401, 14645\n";
        let info = parse_nvidia_smi_output(sample).unwrap();
        assert_eq!(info.total_gb, 16376.0 / 1024.0);
        assert_eq!(info.used_gb, 1401.0 / 1024.0);
        assert_eq!(info.free_gb, 14645.0 / 1024.0);
    }

    #[test]
    fn test_parse_nvidia_smi_output_invalid() {
        assert!(parse_nvidia_smi_output("invalid").is_none());
        assert!(parse_nvidia_smi_output("16376, 1401").is_none());
    }
}
