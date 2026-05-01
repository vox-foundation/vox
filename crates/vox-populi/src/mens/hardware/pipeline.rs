use crate::mens::hardware::probe::{HardwareProbe, ProbeAttempt, ProbeOutcome, ProbeReport};
use crate::mens::hardware::types::{ComputeBackend, GpuVendor, HardwareSummary};
use std::time::Instant;

/// Runs a sequence of [`HardwareProbe`]s in order, collecting an attempt log.
///
/// Call [`ProbePipeline::empty`] + [`ProbePipeline::with_probe`] to build a custom pipeline
/// (useful in tests and operator-override scenarios).
pub struct ProbePipeline {
    pub(crate) probes: Vec<Box<dyn HardwareProbe>>,
}

impl ProbePipeline {
    /// An empty pipeline with no probes. Add probes with [`Self::with_probe`].
    pub fn empty() -> Self {
        Self { probes: Vec::new() }
    }

    /// Appends a probe to the end of the pipeline order (builder style).
    pub fn with_probe(mut self, probe: Box<dyn HardwareProbe>) -> Self {
        self.probes.push(probe);
        self
    }

    /// Run all probes in order.
    ///
    /// Returns a [`ProbeReport`] whose `summary` is the first successful result,
    /// or a CPU-only fallback if every probe returned `NoDevice`, `NotApplicable`,
    /// or `Failed`. Failed probe names are collected into `summary.probe_failures`.
    pub async fn run(&self) -> ProbeReport {
        let mut attempts = Vec::new();
        let mut summary: Option<HardwareSummary> = None;
        let mut failures: Vec<String> = Vec::new();

        for probe in &self.probes {
            let name = probe.name();
            if !probe.applicable() {
                attempts.push(ProbeAttempt {
                    probe_name: name,
                    outcome: ProbeOutcome::NotApplicable,
                    duration_ms: 0,
                });
                continue;
            }
            let start = Instant::now();
            let res = probe.probe().await;
            let duration_ms = start.elapsed().as_millis() as u64;
            match res {
                Ok(Some(s)) => {
                    let s_clone = s.clone();
                    attempts.push(ProbeAttempt {
                        probe_name: name,
                        outcome: ProbeOutcome::Found(Box::new(s)),
                        duration_ms,
                    });
                    if summary.is_none() {
                        summary = Some(s_clone);
                    }
                }
                Ok(None) => {
                    attempts.push(ProbeAttempt {
                        probe_name: name,
                        outcome: ProbeOutcome::NoDevice,
                        duration_ms,
                    });
                }
                Err(e) => {
                    failures.push(name.to_string());
                    attempts.push(ProbeAttempt {
                        probe_name: name,
                        outcome: ProbeOutcome::Failed(e.to_string()),
                        duration_ms,
                    });
                }
            }
        }

        let mut summary = summary.unwrap_or_else(cpu_fallback);
        if !failures.is_empty() {
            summary.probe_failures = Some(failures);
        }
        ProbeReport { summary, attempts }
    }
}

impl ProbePipeline {
    /// Returns the platform-default probe order.
    ///
    /// On Windows: DXGI → wgpu → NVML (feature-gated).
    /// On Linux: DRM → wgpu → NVML (feature-gated).
    /// On macOS: Metal → wgpu.
    pub fn default_for_platform() -> Self {
        let mut pipeline = Self::empty();

        #[cfg(all(target_os = "windows", feature = "mens-gpu"))]
        {
            pipeline = pipeline.with_probe(Box::new(
                crate::mens::hardware::win_dxgi::WinDxgiProbe,
            ));
        }
        #[cfg(target_os = "linux")]
        {
            pipeline = pipeline.with_probe(Box::new(
                crate::mens::hardware::linux_drm::LinuxDrmProbe,
            ));
        }
        #[cfg(target_os = "macos")]
        {
            pipeline = pipeline.with_probe(Box::new(
                crate::mens::hardware::macos_metal::MacosMetalProbe,
            ));
        }
        #[cfg(feature = "mens-gpu")]
        {
            pipeline = pipeline.with_probe(Box::new(
                crate::mens::hardware::wgpu_probe::WgpuProbe,
            ));
        }
        #[cfg(feature = "nvml-gpu-probe")]
        {
            pipeline = pipeline.with_probe(Box::new(
                crate::mens::hardware::nvml::NvmlProbe,
            ));
        }

        pipeline
    }
}

fn cpu_fallback() -> HardwareSummary {
    HardwareSummary {
        model_name: "Host CPU".into(),
        vram_mb: 0,
        gpu_count: 0,
        vendor: GpuVendor::Cpu,
        backend: ComputeBackend::Cpu,
        driver_version: None,
        pci_bus_id: None,
        probe_failures: None,
    }
}
