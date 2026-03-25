//! Capped UTF-8 file reads aligned with scaling policy `max_file_bytes_hint`.

use std::fs;
use std::path::Path;

use vox_scaling_policy::ScalingPolicy;

use crate::errors::SecretError;

fn max_file_bytes_hint() -> u64 {
    ScalingPolicy::embedded().thresholds.max_file_bytes_hint
}

pub(crate) fn read_utf8_path_capped(path: &Path) -> Result<String, SecretError> {
    let cap = max_file_bytes_hint();
    let meta =
        fs::metadata(path).map_err(|e| SecretError::Io(format!("stat {}: {e}", path.display())))?;
    if meta.len() > cap {
        return Err(SecretError::Io(format!(
            "{} is {} bytes; exceeds scaling policy max_file_bytes_hint ({})",
            path.display(),
            meta.len(),
            cap
        )));
    }
    let bytes =
        fs::read(path).map_err(|e| SecretError::Io(format!("read {}: {e}", path.display())))?;
    String::from_utf8(bytes)
        .map_err(|e| SecretError::Io(format!("{}: invalid UTF-8: {e}", path.display())))
}

/// Same as [`read_utf8_path_capped`] but returns `None` on any failure (matches prior `read_to_string` + `.ok()` usage).
pub(crate) fn read_utf8_path_capped_opt(path: &Path) -> Option<String> {
    read_utf8_path_capped(path).ok()
}
