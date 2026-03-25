//! Bounded filesystem reads for eval gate artifacts (aligns with scaling policy `max_file_bytes_hint`).

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

use vox_scaling_policy::ScalingPolicy;

fn max_artifact_bytes() -> u64 {
    ScalingPolicy::embedded().thresholds.max_file_bytes_hint
}

pub fn read_utf8_path_capped(path: &Path) -> Result<String> {
    let cap = max_artifact_bytes();
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

/// Reads newline-delimited records; rejects files larger than [`ScalingPolicy::embedded`] threshold.
pub fn read_jsonl_nonempty_lines(path: &Path) -> Result<Vec<String>> {
    let cap = max_artifact_bytes();
    let meta = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if meta.len() > cap {
        anyhow::bail!(
            "{} is {} bytes; exceeds scaling policy max_file_bytes_hint ({})",
            path.display(),
            meta.len(),
            cap
        );
    }
    let f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line.with_context(|| format!("read line {}", path.display()))?;
        if !line.is_empty() {
            out.push(line);
        }
    }
    Ok(out)
}
