//! Capped UTF-8 file reads aligned with scaling policy `max_file_bytes_hint`.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use vox_scaling_policy::ScalingPolicy;

fn max_file_bytes_hint() -> u64 {
    ScalingPolicy::embedded().thresholds.max_file_bytes_hint
}

pub(crate) fn read_utf8_path_capped(path: &Path) -> Result<String> {
    let cap = max_file_bytes_hint();
    let meta = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if meta.len() > cap {
        anyhow::bail!(
            "{} is {} bytes; exceeds scaling policy max_file_bytes_hint ({})",
            path.display(),
            meta.len(),
            cap
        );
    }
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    String::from_utf8(bytes)
        .map_err(|e| anyhow::anyhow!("{}: invalid UTF-8: {}", path.display(), e))
}

/// Same as [`read_utf8_path_capped`] but returns empty string on any failure (matches prior `read_to_string` + `unwrap_or_default` usage).
pub(crate) fn read_utf8_path_capped_or_empty(path: &Path) -> String {
    read_utf8_path_capped(path).unwrap_or_default()
}
