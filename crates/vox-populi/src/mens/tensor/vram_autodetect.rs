//! VRAM auto-detection and training preset selection.
//!
//! On Windows/Linux with NVIDIA, we try to query the GPU through available
//! mechanisms (NVML env vars, sysfs, or compile-time hints). Falls back to
//! a user-supplied override or safe defaults.

/// Query available GPU VRAM in GiB.
///
/// Resolution order:
/// 1. `VOX_VRAM_OVERRIDE_GB` environment variable (float)
/// 2. Linux sysfs NVML-style `/sys/class/drm/card0/device/mem_info_vram_total`
/// 3. Windows NVML via `nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits`
/// 4. `None` — caller should fall back to user preset
pub fn get_system_vram_gb() -> Option<f32> {
    // Priority 1: env override
    if let Ok(v) = std::env::var("VOX_VRAM_OVERRIDE_GB")
        && let Ok(gb) = v.parse::<f32>()
        && gb > 0.0
    {
        return Some(gb);
    }

    // Priority 2: Linux sysfs
    if cfg!(target_os = "linux") {
        let sysfs_path = "/sys/class/drm/card0/device/mem_info_vram_total";
        if let Ok(raw) = std::fs::read_to_string(sysfs_path)
            && let Ok(bytes) = raw.trim().parse::<u64>()
        {
            return Some(bytes as f32 / (1024.0 * 1024.0 * 1024.0));
        }
    }

    // Priority 3: nvidia-smi (Windows + Linux fallback)
    if let Ok(out) = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output()
        && out.status.success()
    {
        let raw = String::from_utf8_lossy(&out.stdout);
        // May return multiple lines for multi-GPU; take the first.
        if let Some(line) = raw.lines().next()
            && let Ok(mib) = line.trim().parse::<f32>()
        {
            return Some(mib / 1024.0);
        }
    }

    None
}

/// Select the best training preset for the detected hardware.
///
/// Returns a preset name matching `preset_schema.rs` known aliases.
/// Returns `None` when CUDA is not in use or VRAM is too low.
pub fn auto_preset(device_is_cuda: bool, vram_gb: Option<f32>) -> Option<&'static str> {
    if !device_is_cuda {
        return None;
    }
    match vram_gb {
        Some(v) if v < 6.0 => None, // Too small for QLoRA
        Some(v) if v < 10.0 => Some("safe"),
        Some(v) if v <= 16.0 => Some("qwen_4080_16g"),
        Some(v) if v <= 24.0 => Some("4080"),
        Some(_) => Some("a100"),
        None => None,
    }
}

/// Human-readable summary of detected VRAM + auto-selected preset.
pub fn vram_summary(device_is_cuda: bool) -> String {
    let vram = get_system_vram_gb();
    let preset = auto_preset(device_is_cuda, vram);
    match (vram, preset) {
        (Some(v), Some(p)) => format!("{:.1} GiB VRAM detected → preset '{p}'", v),
        (Some(v), None) => format!(
            "{:.1} GiB VRAM detected (no matching preset; specify --preset manually)",
            v
        ),
        (None, _) => {
            "Could not detect VRAM (set VOX_VRAM_OVERRIDE_GB or pass --preset manually)".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_preset_maps_correctly() {
        assert_eq!(auto_preset(true, Some(16.0)), Some("qwen_4080_16g"));
        assert_eq!(auto_preset(true, Some(8.0)), Some("safe"));
        assert_eq!(auto_preset(true, Some(80.0)), Some("a100"));
        assert_eq!(auto_preset(false, Some(16.0)), None);
        assert_eq!(auto_preset(true, Some(4.0)), None);
    }

    #[test]
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
}
