//! UTF-8 file reads capped by [`vox_scaling_policy::ScalingPolicy::embedded`] `max_file_bytes_hint`.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use vox_scaling_policy::ScalingPolicy;

pub(crate) fn read_utf8_path_capped(path: &Path) -> Result<String> {
    let cap = ScalingPolicy::embedded().thresholds.max_file_bytes_hint;
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

/// Capped read on the blocking pool (for `async` call sites; avoids unbounded `tokio::fs::read_to_string`).
pub(crate) async fn read_utf8_path_capped_async(path: &Path) -> Result<String> {
    let p = path.to_path_buf();
    tokio::task::spawn_blocking(move || read_utf8_path_capped(&p))
        .await
        .map_err(|e| anyhow::anyhow!("read join error: {e}"))?
}
