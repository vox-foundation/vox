//! Hook registry for future workflow / telemetry integration.

use std::sync::Mutex;

/// Lightweight hook registry (extensible without breaking callers).
#[derive(Debug, Default)]
pub struct HookRegistry {
    _lock: Mutex<()>,
}

impl HookRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            _lock: Mutex::new(()),
        }
    }
}
