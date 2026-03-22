//! Serializable bootstrap / install plan (for `--json` and external tooling).

use serde::Serialize;

/// One row in the bootstrap report.
#[derive(Debug, Clone, Serialize)]
pub struct BootstrapItem {
    /// Stable machine id (e.g. `rustc`, `turso_native`).
    pub id: &'static str,
    /// Human-readable label for the check.
    pub description: &'static str,
    /// If false, failure does not fail `required_ok()`.
    pub required: bool,
    /// Whether the probe passed.
    pub ok: bool,
    /// Version string, error text, or short status.
    pub detail: String,
    /// Suggested fix command when `ok` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heal_command: Option<String>,
}

/// Full report returned by [`crate::engine::evaluate`] and serialized with `--json`.
#[derive(Debug, Clone, Serialize)]
pub struct BootstrapReport {
    /// OS family label (`windows`, `linux`, `macos`, `other`).
    pub platform: String,
    /// Ordered list of probe results.
    pub items: Vec<BootstrapItem>,
}

impl BootstrapReport {
    /// `true` if every `required` item has `ok`.
    #[must_use]
    pub fn required_ok(&self) -> bool {
        self.items.iter().filter(|i| i.required).all(|i| i.ok)
    }
}
