use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Cpu,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeBackend {
    Cuda,
    Metal,
    Wgpu,
    Cpu,
    Unknown,
}

impl ComputeBackend {
    pub fn as_cli_flag(&self) -> &'static str {
        match self {
            ComputeBackend::Cuda => "cuda",
            ComputeBackend::Metal => "metal",
            ComputeBackend::Wgpu => "vulkan",
            ComputeBackend::Cpu => "cpu",
            ComputeBackend::Unknown => "best",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSummary {
    pub model_name: String,
    pub vram_mb: u64,
    pub gpu_count: u32,
    pub vendor: GpuVendor,
    pub backend: ComputeBackend,
    pub driver_version: Option<String>,
    pub pci_bus_id: Option<String>,
    /// Names of probes that failed during the pipeline run that produced this summary.
    /// `None` means the summary was not produced by a pipeline run, or the pipeline
    /// ran with no failures. `Some(names)` means one or more named probes returned
    /// an error; callers should not construct `Some(vec![])` — use `None` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probe_failures: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuTelemetry {
    pub temperature_c: f32,
    pub power_usage_w: f32,
    pub fan_speed_pct: f32,
    pub memory_used_mb: u64,
    pub memory_free_mb: u64,
    pub utilization_pct: f32,
}

/// PCI Vendor IDs for common GPU manufacturers.
pub mod vendor_ids {
    pub const NVIDIA: u32 = 0x10DE;
    pub const AMD: u32 = 0x1002;
    pub const AMD_ALT: u32 = 0x1022;
    pub const INTEL: u32 = 0x8086;
    pub const APPLE: u32 = 0x05AC;
}

/// Constants for byte conversions.
pub const BYTES_TO_MB: u64 = 1024 * 1024;
pub const BYTES_TO_GB: u64 = 1024 * 1024 * 1024;
pub const MB_PER_GB: u64 = 1024;

/// Infers a `GpuVendor` from a PCI Vendor ID.
pub fn vendor_from_id(id: u32) -> GpuVendor {
    match id {
        vendor_ids::NVIDIA => GpuVendor::Nvidia,
        vendor_ids::AMD | vendor_ids::AMD_ALT => GpuVendor::Amd,
        vendor_ids::INTEL => GpuVendor::Intel,
        _ => GpuVendor::Unknown,
    }
}

pub fn vendor_from_model(model: &str) -> GpuVendor {
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
        return GpuVendor::Nvidia;
    }
    if m.contains("amd") || m.contains("radeon") {
        return GpuVendor::Amd;
    }
    if m.contains("intel") && (m.contains("arc") || m.contains("iris") || m.contains("uhd")) {
        return GpuVendor::Intel;
    }
    if m.contains("apple") || m.contains("m1") || m.contains("m2") || m.contains("m3") {
        return GpuVendor::Apple;
    }
    GpuVendor::Unknown
}
