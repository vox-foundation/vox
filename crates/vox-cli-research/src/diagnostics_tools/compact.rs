use anyhow::Result;

use owo_colors::OwoColorize;
use std::path::Path;
use vox_bounded_fs::read_utf8_path_capped;

/// Lex and compact a `.vox` source file, printing statistics to stderr.
pub fn run(file_path: &Path) -> Result<()> {
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let src = read_utf8_path_capped(file_path)?;
    let compacted = vox_compiler::canonicalize_vox(&src)
        .map_err(|e| anyhow::anyhow!("Compaction validation failed: {e}"))?;

    println!("{}", compacted);

    let original_len = src.len();
    let compacted_len = compacted.len();
    let savings = if original_len > 0 {
        100.0 - (compacted_len as f64 / original_len as f64 * 100.0)
    } else {
        0.0
    };

    eprintln!(
        "\n{}",
        format!(
            "✓ Compacted {}/{} bytes ({:.1}% reduction)",
            compacted_len, original_len, savings
        )
        .green()
    );

    Ok(())
}
