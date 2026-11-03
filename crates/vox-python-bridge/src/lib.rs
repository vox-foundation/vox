//! Framed stdio host for one lazy Python worker.
//!
//! Owns request framing, the worker process lifecycle, and handle generations.
//! The stdlib-only worker package lives in `worker/vox_bridge/`.

use std::ffi::OsString;
use std::time::Duration;

mod frame;
mod host;

pub use host::Bridge;
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

/// Directory that contains the `vox_bridge` package. Put it on `PYTHONPATH`.
pub fn worker_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("worker")
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

    #[test]
    fn worker_root_contains_the_bridge_module() {
        let main = worker_root().join("vox_bridge").join("__main__.py");
        assert!(main.is_file(), "missing {}", main.display());
    }
}
