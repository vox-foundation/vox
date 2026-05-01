#[cfg(test)]
mod tests {
    use crate::mens::hardware::mock::MockProbe;
    use crate::mens::hardware::probe::{HardwareProbe, ProbeError};
    use crate::mens::hardware::types::{ComputeBackend, GpuVendor, HardwareSummary};

    fn dummy_summary() -> HardwareSummary {
        HardwareSummary {
            model_name: "Test GPU".into(),
            vram_mb: 8192,
            gpu_count: 1,
            vendor: GpuVendor::Nvidia,
            backend: ComputeBackend::Cuda,
            driver_version: None,
            pci_bus_id: None,
            probe_failures: None,
        }
    }

    #[tokio::test]
    async fn mock_probe_returns_configured_result() {
        let probe = MockProbe {
            name: "test",
            applicable: true,
            result: Ok(Some(dummy_summary())),
        };
        assert_eq!(probe.name(), "test");
        assert!(probe.applicable());
        let res = probe.probe().await.unwrap();
        assert_eq!(res.unwrap().model_name, "Test GPU");
    }

    #[tokio::test]
    async fn mock_probe_propagates_error() {
        let probe = MockProbe {
            name: "broken",
            applicable: true,
            result: Err(ProbeError::LibraryUnavailable("nvml".into())),
        };
        assert_eq!(
            probe.probe().await.unwrap_err(),
            ProbeError::LibraryUnavailable("nvml".into())
        );
    }
}
