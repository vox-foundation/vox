use crate::mens::hardware::probe::{HardwareProbe, ProbeError};
use crate::mens::hardware::types::{ComputeBackend, GpuTelemetry, GpuVendor, HardwareSummary, BYTES_TO_MB};
use async_trait::async_trait;

/// Hardware probe backend that uses the NVIDIA Management Library (NVML).
pub struct NvmlProbe;

#[async_trait]
impl HardwareProbe for NvmlProbe {
    fn name(&self) -> &'static str {
        "nvml"
    }
    fn applicable(&self) -> bool {
        true
    }
    async fn probe(&self) -> Result<Option<HardwareSummary>, ProbeError> {
        use nvml_wrapper::Nvml;
        let nvml = match Nvml::init() {
            Ok(n) => n,
            Err(e) => return Err(ProbeError::LibraryUnavailable(e.to_string())),
        };
        let count = nvml
            .device_count()
            .map_err(|e| ProbeError::DeviceError(e.to_string()))?;
        if count == 0 {
            return Ok(None);
        }
        let device = nvml
            .device_by_index(0)
            .map_err(|e| ProbeError::DeviceError(e.to_string()))?;
        let name = device
            .name()
            .map_err(|e| ProbeError::DeviceError(e.to_string()))?;
        let mem = device
            .memory_info()
            .map_err(|e| ProbeError::DeviceError(e.to_string()))?;
        let total_mb = (mem.used + mem.free) / BYTES_TO_MB;
        let driver_version = nvml.sys_driver_version().ok();
        Ok(Some(HardwareSummary {
            model_name: name,
            vram_mb: total_mb,
            gpu_count: count,
            vendor: GpuVendor::Nvidia,
            backend: ComputeBackend::Cuda,
            driver_version,
            pci_bus_id: None,
            probe_failures: None,
        }))
    }
}

pub fn monitor_nvml() -> Option<GpuTelemetry> {
    use nvml_wrapper::Nvml;

    let nvml = Nvml::init().ok()?;
    let device = nvml.device_by_index(0).ok()?;

    let temp = device
        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        .unwrap_or(0) as f32;
    let power = device.power_usage().unwrap_or(0) as f32 / 1000.0; // mW to W
    let fan = device.fan_speed(0).unwrap_or(0) as f32;
    let mem = device.memory_info().ok()?;
    let util = device
        .utilization_rates()
        .ok()
        .map(|u| u.gpu as f32)
        .unwrap_or(0.0);

    Some(GpuTelemetry {
        temperature_c: temp,
        power_usage_w: power,
        fan_speed_pct: fan,
        memory_used_mb: mem.used / crate::mens::hardware::types::BYTES_TO_MB,
        memory_free_mb: mem.free / crate::mens::hardware::types::BYTES_TO_MB,
        utilization_pct: util,
    })
}
