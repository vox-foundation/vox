//! Host capability discovery for agent queues (CPU-first; GPU from config until probed elsewhere).
//!
//! Merge rules: see `merge_agent_capabilities` and
//! [`docs/src/architecture/orchestration-unified-ssot.md`](../../../../docs/src/architecture/orchestration-unified-ssot.md).

use crate::contract::TaskCapabilityHints;

/// Snapshot of the current process host (best-effort, no extra crates required on the default build).
#[must_use]
pub fn probe_host_capabilities() -> TaskCapabilityHints {
    let mut h = TaskCapabilityHints {
        cpu_cores: std::thread::available_parallelism()
            .ok()
            .map(|n| n.get() as u32),
        arch: Some(std::env::consts::ARCH.to_string()),
        hostname: hostname_best_effort(),
        ..Default::default()
    };
    #[cfg(feature = "system-metrics")]
    {
        use sysinfo::{CpuRefreshKind, System};
        let mut sys = System::new();
        sys.refresh_cpu_list(CpuRefreshKind::everything());
        if let Some(name) = System::name() {
            if !name.is_empty() {
                h.hostname = Some(name);
            }
        }
    }
    apply_mesh_capability_env(&mut h);
    h
}

/// Merge operator [`TaskCapabilityHints`] from config with a host [`probe_host_capabilities`] snapshot.
///
/// Precedence:
/// 1. **Host truth** for `cpu_cores`, `arch`, `hostname` when the probe produced a value.
/// 2. **Labels**: config labels first, then probe-only labels appended (deduped by first occurrence).
/// 3. **GPU / NPU / VRAM / min_cpu_cores**: config wins when set; probe/env may set flags via
///    [`apply_mesh_capability_env`] inside `probed` before calling this function.
#[must_use]
pub fn merge_agent_capabilities(
    config: &TaskCapabilityHints,
    probed: TaskCapabilityHints,
) -> TaskCapabilityHints {
    let mut out = config.clone();
    if probed.cpu_cores.is_some() {
        out.cpu_cores = probed.cpu_cores;
    }
    if probed.arch.is_some() {
        out.arch = probed.arch;
    }
    if probed.hostname.is_some() {
        out.hostname = probed.hostname;
    }
    let mut seen: std::collections::HashSet<String> = out.labels.iter().cloned().collect();
    for lab in probed.labels {
        if seen.insert(lab.clone()) {
            out.labels.push(lab);
        }
    }
    if out.gpu_cuda {
        // config already wants CUDA
    } else if probed.gpu_cuda {
        out.gpu_cuda = true;
    }
    if out.gpu_metal {
        // config already wants Metal
    } else if probed.gpu_metal {
        out.gpu_metal = true;
    }
    if !out.gpu_vulkan && probed.gpu_vulkan {
        out.gpu_vulkan = true;
    }
    if !out.gpu_webgpu && probed.gpu_webgpu {
        out.gpu_webgpu = true;
    }
    if !out.npu && probed.npu {
        out.npu = true;
    }
    if out.device_class.is_none() {
        out.device_class = probed.device_class.clone();
    }
    if out.min_vram_mb.is_none() {
        out.min_vram_mb = probed.min_vram_mb;
    }
    if out.min_cpu_cores.is_none() {
        out.min_cpu_cores = probed.min_cpu_cores;
    }
    out
}

fn hostname_best_effort() -> Option<String> {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .filter(|s| !s.is_empty())
}

fn mesh_env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|v| {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
}

/// Mesh operator env: CUDA (legacy), Vulkan/WebGPU/NPU, optional `device_class`.
///
/// `VOX_MESH_ADVERTISE_GPU=1|true` still sets **`gpu_cuda`** for backward compatibility (workstation
/// advertisement, not an Android Vulkan probe).
fn apply_mesh_capability_env(h: &mut TaskCapabilityHints) {
    if mesh_env_truthy("VOX_MESH_ADVERTISE_GPU") {
        h.gpu_cuda = true;
    }
    if mesh_env_truthy("VOX_MESH_ADVERTISE_VULKAN") {
        h.gpu_vulkan = true;
    }
    if mesh_env_truthy("VOX_MESH_ADVERTISE_WEBGPU") {
        h.gpu_webgpu = true;
    }
    if mesh_env_truthy("VOX_MESH_ADVERTISE_NPU") {
        h.npu = true;
    }
    if let Ok(s) = std::env::var("VOX_MESH_DEVICE_CLASS") {
        let t = s.trim();
        if !t.is_empty() {
            h.device_class = Some(t.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_probed_cpu_and_keeps_config_gpu() {
        let cfg = TaskCapabilityHints {
            gpu_cuda: true,
            labels: vec!["pool=a".into()],
            ..Default::default()
        };
        let p = TaskCapabilityHints {
            cpu_cores: Some(8),
            arch: Some("aarch64".into()),
            labels: vec!["pool=b".into()],
            ..Default::default()
        };
        let m = merge_agent_capabilities(&cfg, p);
        assert_eq!(m.cpu_cores, Some(8));
        assert_eq!(m.arch.as_deref(), Some("aarch64"));
        assert!(m.gpu_cuda);
        assert!(m.labels.contains(&"pool=a".into()));
        assert!(m.labels.contains(&"pool=b".into()));
    }
}
