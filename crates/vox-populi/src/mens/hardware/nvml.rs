use crate::mens::hardware::types::GpuTelemetry;

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
