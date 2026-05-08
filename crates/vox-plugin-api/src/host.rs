//! VoxHost trait — the capability surface a code plugin receives at init.
//! Stable-ABI for the dylib boundary via abi_stable.

use crate::errors::LogLevel;
use abi_stable::{StableAbi, sabi_trait, std_types::*};

#[derive(Debug, Clone, Copy, StableAbi)]
#[repr(u8)]
pub enum SabiLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for SabiLogLevel {
    fn from(l: LogLevel) -> Self {
        match l {
            LogLevel::Trace => Self::Trace,
            LogLevel::Debug => Self::Debug,
            LogLevel::Info => Self::Info,
            LogLevel::Warn => Self::Warn,
            LogLevel::Error => Self::Error,
        }
    }
}

#[sabi_trait]
pub trait VoxHost: Send + Sync {
    fn data_dir(&self) -> RString;
    fn log(&self, level: SabiLogLevel, msg: RStr<'_>);
    fn telemetry_event(&self, kind: RStr<'_>, payload: RStr<'_>);
}
