use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mens::cloud::{
    CloudProvider, CloudProviderConfig, GpuOffer, JobHandle, JobStatus,
};
use crate::mens::hardware::types::HardwareSummary;

/// Provider that spawns an in-process local worker acting as a mesh node.
pub struct LocalProvider {
    pub hardware: Arc<HardwareSummary>,
    pub config: Arc<CloudProviderConfig>,
    /// We hold the child process handle to kill it upon termination.
    pub child: Arc<Mutex<Option<tokio::process::Child>>>,
}

impl LocalProvider {
    pub fn new(hardware: Arc<HardwareSummary>, config: Arc<CloudProviderConfig>) -> Self {
        Self {
            hardware,
            config,
            child: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl CloudProvider for LocalProvider {
    fn kind(&self) -> crate::mens::cloud::ProviderKind {
        crate::mens::cloud::ProviderKind::Local
    }

    async fn list_offers(&self, min_vram_mb: u64) -> anyhow::Result<Vec<GpuOffer>> {
        if self.hardware.vram_mb < min_vram_mb {
            return Ok(vec![]);
        }

        Ok(vec![GpuOffer {
            offer_id: "local-gpu".to_string(),
            provider: crate::mens::cloud::ProviderKind::Local,
            gpu_name: self.hardware.model_name.clone(),
            gpu_count: self.hardware.gpu_count,
            vram_mb: self.hardware.vram_mb,
            price_per_hour_usd: 0.0,
            auto_terminate: true,
            is_spot: false,
            reliability_pct: 100.0,
            fetched_at: Some(std::time::Instant::now()),
            datacenter_region: Some("local".to_string()),
            cuda_max: None,
        }])
    }

    async fn dispatch(
        &self,
        _offer: &GpuOffer,
        spec: &crate::mens::cloud::CloudJobSpec,
    ) -> anyhow::Result<JobHandle> {
        let port = match spec.serve_port {
            0 => {
                // Find a free ephemeral port
                let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
                listener.local_addr()?.port()
            }
            p => p,
        };

        let current_exe = std::env::current_exe()?;
        let mut cmd = tokio::process::Command::new(current_exe);
        cmd.args(["oratio", "serve", "--port", &port.to_string()]);

        let child = cmd.spawn()?;
        *self.child.lock().await = Some(child);

        let handle = JobHandle {
            provider: crate::mens::cloud::ProviderKind::Local,
            job_id: port.to_string(),
            price_per_hour_usd: 0.0,
            started_at: std::time::SystemTime::now(),
            estimated_seconds: spec.max_runtime_secs.unwrap_or(3600) as f64,
            is_persistent: spec.persistent,
        };

        Ok(handle)
    }

    async fn poll_status(&self, _handle: &JobHandle) -> anyhow::Result<JobStatus> {
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut() {
            if let Some(status) = child.try_wait()? {
                if status.success() {
                    return Ok(JobStatus::Completed {
                        adapter_uploaded: false,
                    });
                } else {
                    return Ok(JobStatus::Failed(format!("Local worker exited with {}", status)));
                }
            }
            return Ok(JobStatus::Running {
                progress_pct: None,
                gpu_util_pct: None,
            });
        }
        Ok(JobStatus::Terminated)
    }

    async fn terminate(&self, _handle: &JobHandle) -> anyhow::Result<()> {
        let mut guard = self.child.lock().await;
        if let Some(mut child) = guard.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }

    async fn get_serve_url(&self, handle: &JobHandle, _port: u16) -> anyhow::Result<Option<String>> {
        // Port is stored in the job_id
        Ok(Some(format!("http://127.0.0.1:{}", handle.job_id)))
    }
}
