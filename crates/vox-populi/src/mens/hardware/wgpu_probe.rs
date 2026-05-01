use crate::mens::hardware::types::{ComputeBackend, GpuVendor, HardwareSummary};

pub async fn probe_wgpu() -> Option<HardwareSummary> {
    use wgpu::{Backends, Instance, InstanceDescriptor};

    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    });

    let adapters = instance.enumerate_adapters(Backends::all());
    let mut best: Option<HardwareSummary> = None;

    let mut count = 0;
    for adapter in adapters {
        count += 1;
        let info = adapter.get_info();
        let model_name = info.name;
        let vendor = crate::mens::hardware::types::vendor_from_id(info.vendor);

        // Heuristic: pick the one with "discrete" type if possible, or NVIDIA
        let is_discrete = matches!(info.device_type, wgpu::DeviceType::DiscreteGpu);
        let is_nvidia = matches!(vendor, GpuVendor::Nvidia);

        if best.is_none()
            || (is_discrete && !matches!(best.as_ref().unwrap().vendor, GpuVendor::Nvidia))
            || (is_nvidia && !matches!(best.as_ref().unwrap().vendor, GpuVendor::Nvidia))
        {
            best = Some(HardwareSummary {
                model_name,
                vram_mb: 0, // wgpu doesn't easily expose total VRAM in 26.x without memory_heaps (experimental)
                gpu_count: 0, // Placeholder
                vendor,
                backend: match vendor {
                    GpuVendor::Nvidia => ComputeBackend::Cuda,
                    GpuVendor::Apple => ComputeBackend::Metal,
                    _ => ComputeBackend::Wgpu,
                },
                driver_version: Some(info.driver),
                pci_bus_id: None,
                probe_failures: None,
            });
        }
    }
    if let Some(ref mut b) = best {
        b.gpu_count = count as u32;
    }
    best
}
