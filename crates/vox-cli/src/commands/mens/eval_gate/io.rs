//! Bounded filesystem reads for eval gate artifacts (aligns with scaling policy `max_file_bytes_hint`).

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Reads a file as UTF-8 with the workspace capped-read policy.
pub fn read_utf8_path_capped(path: &Path) -> Result<String> {
    vox_bounded_fs::read_utf8_path_capped(path)
}

/// Reads newline-delimited records; rejects files larger than embedded scaling policy threshold.
pub fn read_jsonl_nonempty_lines(path: &Path) -> Result<Vec<String>> {
    let cap = vox_bounded_fs::max_file_bytes_hint();
    let meta = std::fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
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
