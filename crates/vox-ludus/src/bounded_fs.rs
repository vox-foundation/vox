//! UTF-8 reads capped by scaling policy (TOESTUB `scaling/unbounded-read` hygiene).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use vox_scaling_policy::ScalingPolicy;

pub fn read_utf8_path_capped(path: &Path) -> Result<String> {
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
