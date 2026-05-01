use crate::mens::hardware::types::{ComputeBackend, GpuVendor, HardwareSummary};

#[cfg(target_os = "windows")]
pub fn probe_dxgi() -> Option<HardwareSummary> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory, IDXGIFactory};

    let mut best_adapter: Option<HardwareSummary> = None;

    #[allow(unsafe_code)]
    // SAFETY: We are calling Windows COM APIs via the `windows` crate.
    // CreateDXGIFactory, EnumAdapters, and GetDesc are all standard COM methods
    // whose memory invariants and reference counting are handled by the crate's generated bindings.
    unsafe {
        let factory: IDXGIFactory = match CreateDXGIFactory() {
            Ok(f) => f,
            Err(_) => return None,
        };

        let mut count = 0;
        let mut i = 0;
        while let Ok(adapter) = factory.EnumAdapters(i) {
            if let Ok(desc) = adapter.GetDesc() {
                count += 1;
                let model_name = String::from_utf16_lossy(&desc.Description)
                    .trim_matches(char::from(0))
                    .to_string();

                let vram_mb =
                    desc.DedicatedVideoMemory as u64 / crate::mens::hardware::types::BYTES_TO_MB;
                let vendor = crate::mens::hardware::types::vendor_from_id(desc.VendorId);
                let backend = match vendor {
                    GpuVendor::Nvidia => ComputeBackend::Cuda,
                    _ => ComputeBackend::Wgpu,
                };

                if best_adapter.is_none() || vram_mb > best_adapter.as_ref().unwrap().vram_mb {
                    best_adapter = Some(HardwareSummary {
                        model_name,
                        vram_mb,
                        gpu_count: 0, // Placeholder
                        vendor,
                        backend,
                        driver_version: None,
                        pci_bus_id: None,
                        probe_failures: None,
                    });
                }
            }
            i += 1;
        }

        if let Some(ref mut best) = best_adapter {
            best.gpu_count = count;
        }
    }
    best_adapter
}

#[cfg(not(target_os = "windows"))]
pub fn probe_dxgi() -> Option<HardwareSummary> {
    None
}
