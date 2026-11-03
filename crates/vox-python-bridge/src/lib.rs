//! Framed stdio host for one lazy Python worker.
//!
//! Owns request framing, the worker process lifecycle, and handle generations.
//! The stdlib-only worker package lives in `worker/vox_bridge/`.

use std::ffi::OsString;
use std::time::Duration;

pub use vox_foundation::protocol::{DispatchRequest, DispatchResponse};

/// Worker spawn and protocol limits. `worker_cmd` selects the real interpreter
/// or a test double; it is argv, never a shell string.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub worker_cmd: Vec<OsString>,
    pub max_fetch_mb: u64,
    pub timeout: Duration,
    pub env: Vec<(OsString, OsString)>,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            worker_cmd: vec!["python3".into(), "-m".into(), "vox_bridge".into()],
            max_fetch_mb: 256,
            timeout: Duration::from_secs(300),
            env: Vec::new(),
        }
    }
}

/// One worker-resident object. A generation from a dead worker is stale.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HandleRef {
    pub generation: u32,
    pub id: u64,
    pub kind: String,
    pub dtype: String,
}

/// Successful reply. `value` is the JSON result payload; `body` is opaque.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub value: serde_json::Value,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("{0}")]
    Protocol(String),
    #[error("py: worker io: {0}")]
    Io(#[from] std::io::Error),
}

impl BridgeError {
    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }
}

/// One lazy worker. Requests are serialized and handles carry a generation.
pub struct Bridge {
    config: BridgeConfig,
    stderr: std::sync::Mutex<String>,
}

impl Bridge {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            stderr: std::sync::Mutex::new(String::new()),
        }
    }

    pub fn generation(&self) -> u32 {
        let _ = &self.config;
        0
    }

    /// Queue handle ids. The next [`Self::call`] sends them as one `release`.
    pub fn enqueue_release(&self, id: u64) {
        let _ = (self, id);
    }

    /// Stderr captured from the worker since spawn. `debug.print` lands here.
    pub fn stderr_text(&self) -> String {
        self.stderr.lock().expect("stderr lock").clone()
    }

    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
        body: Option<&[u8]>,
    ) -> Result<Reply, BridgeError> {
        let _ = (method, params, body, &self.config);
        Err(BridgeError::protocol("not implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_worker_is_python_module_with_256_mb_cap() {
        let config = BridgeConfig::default();
        assert_eq!(config.max_fetch_mb, 256);
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert_eq!(
            config
                .worker_cmd
                .iter()
                .map(|part| part.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec!["python3", "-m", "vox_bridge"]
        );
    }
}
