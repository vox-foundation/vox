//! `vox fmt` — format `.vox` source using the compiler [`vox_compiler::fmt`] pipeline (fail-closed).

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use vox_bounded_fs::read_utf8_path_capped;

fn parse_errors_lines(errors: &[vox_compiler::parser::ParseError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn atomic_write_file(path: &Path, contents: &str) -> Result<()> {
    use std::io::Write;

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    let tmp: PathBuf = parent.join(format!("{base}.voxfmt.{stamp}.tmp"));
    std::fs::File::create(&tmp)
        .with_context(|| format!("create temp {}", tmp.display()))?
        .write_all(contents.as_bytes())
        .with_context(|| format!("write temp {}", tmp.display()))?;
    #[cfg(unix)]
    {
        std::fs::rename(&tmp, path)
            .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    }
    #[cfg(not(unix))]
    {
        if path.exists() {
            std::fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
        }
        std::fs::rename(&tmp, path)
            .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    }
    Ok(())
}

/// Format `file` in place, or with **`check`** only verify it already matches formatter output.
pub fn run(file: &Path, check: bool) -> Result<()> {
    let source = read_utf8_path_capped(file)
        .with_context(|| format!("Failed to read source file: {}", file.display()))?;

    let formatted = vox_compiler::fmt::try_format(&source).map_err(|e| {
        anyhow::anyhow!(
            "{}: cannot format (parse or print round-trip failed):\n{}",
            file.display(),
            parse_errors_lines(&e)
        )
    })?;

    if check {
        if source != formatted {
            bail!(
                "{}: needs format (run `vox fmt` without `--check` to write)",
                file.display()
            );
        }
        return Ok(());
    }

    if source == formatted {
        return Ok(());
    }

    atomic_write_file(file, &formatted)?;
    println!("Formatted {}", file.display());
    Ok(())
}
