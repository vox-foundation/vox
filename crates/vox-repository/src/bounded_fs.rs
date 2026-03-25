//! Capped UTF-8 file reads aligned with scaling policy `max_file_bytes_hint`.

use std::fs;
use std::path::Path;

use vox_scaling_policy::ScalingPolicy;

fn max_file_bytes_hint() -> u64 {
    ScalingPolicy::embedded().thresholds.max_file_bytes_hint
}

/// Reads the entire file as UTF-8 when its size is at most `ScalingPolicy::embedded().thresholds.max_file_bytes_hint`.
pub(crate) fn read_utf8_file_capped(path: &Path) -> Option<String> {
    let cap = max_file_bytes_hint();
    let meta = fs::metadata(path).ok()?;
    if meta.len() > cap {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    String::from_utf8(bytes).ok()
}
