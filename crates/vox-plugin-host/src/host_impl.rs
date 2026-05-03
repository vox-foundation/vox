//! Default VoxHost implementation. Wraps system data dir resolution and
//! routes log/telemetry calls to tracing.

use crate::telemetry;
use abi_stable::std_types::*;
use vox_plugin_api::host::{SabiLogLevel, VoxHost};

pub struct DefaultVoxHost {
    data_dir: String,
}

impl DefaultVoxHost {
    pub fn new() -> Self {
        let data_dir = dirs::data_local_dir()
            .map(|p| p.join("vox").join("plugins").to_string_lossy().to_string())
            .unwrap_or_else(|| "./vox-plugins".into());
        Self { data_dir }
    }

    pub fn with_data_dir(data_dir: impl Into<String>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }
}

impl Default for DefaultVoxHost {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxHost for DefaultVoxHost {
    fn data_dir(&self) -> RString {
        self.data_dir.clone().into()
    }
    fn log(&self, level: SabiLogLevel, msg: RStr<'_>) {
        match level {
            SabiLogLevel::Trace => tracing::trace!("{}", msg.as_str()),
            SabiLogLevel::Debug => tracing::debug!("{}", msg.as_str()),
            SabiLogLevel::Info => tracing::info!("{}", msg.as_str()),
            SabiLogLevel::Warn => tracing::warn!("{}", msg.as_str()),
            SabiLogLevel::Error => tracing::error!("{}", msg.as_str()),
        }
    }
    fn telemetry_event(&self, kind: RStr<'_>, payload: RStr<'_>) {
        telemetry::loaded(kind.as_str(), payload.as_str(), "telemetry", 0);
    }
}
