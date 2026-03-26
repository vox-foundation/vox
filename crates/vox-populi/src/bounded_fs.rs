//! Capped UTF-8 file reads aligned with scaling policy `max_file_bytes_hint`.

use std::fs;
use std::io;
use std::path::Path;

use vox_scaling_policy::ScalingPolicy;

fn max_file_bytes_hint() -> u64 {
    ScalingPolicy::embedded().thresholds.max_file_bytes_hint
}

pub(crate) fn read_utf8_path_capped(path: &Path) -> io::Result<String> {
    let cap = max_file_bytes_hint();
    let meta = fs::metadata(path)?;
    if meta.len() > cap {
        return Err(io::Error::other(format!(
            "{} is {} bytes; exceeds scaling policy max_file_bytes_hint ({})",
            path.display(),
            meta.len(),
            cap
        )));
    }
    let bytes = fs::read(path)?;
    String::from_utf8(bytes)
        .map_err(|e| io::Error::other(format!("{}: invalid UTF-8: {}", path.display(), e)))
}
