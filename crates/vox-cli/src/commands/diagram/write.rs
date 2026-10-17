//! Safe file output for `vox diagram`: `fmt --write` and `-o`.
//!
//! One path for both: resolve symlinks (the real file is rewritten, the link survives), refuse a
//! read-only target, create the temp sibling exclusively with the source's mode, then rename.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, bail};

/// Refuse an output path that resolves to the input file (it would destroy the diagram).
pub(super) fn ensure_not_input(output: &Path, input: &str) -> anyhow::Result<()> {
    if input == "-" {
        return Ok(());
    }
    let Ok(input_real) = std::fs::canonicalize(input) else {
        return Ok(()); // unreadable input: the read reports it
    };
    if std::fs::canonicalize(output).is_ok_and(|o| o == input_real) {
        bail!(
            "{}: is the input file; refusing to overwrite it",
            output.display()
        );
    }
    Ok(())
}

/// Write `data` to `path` via an exclusively created sibling temp file and a rename, so a failed
/// write never truncates the target. A symlink is written through; a read-only file is refused.
pub(super) fn write_atomic(path: &Path, data: &str) -> anyhow::Result<()> {
    let real = match std::fs::canonicalize(path) {
        Ok(p) => p,
        // Not there yet: create it. A dangling symlink is an error, not something to replace.
        Err(e)
            if e.kind() == std::io::ErrorKind::NotFound
                && std::fs::symlink_metadata(path).is_err() =>
        {
            path.to_path_buf()
        }
        Err(e) => return Err(e).with_context(|| path.display().to_string()),
    };
    let perms = std::fs::metadata(&real).ok().map(|m| m.permissions());
    if perms.as_ref().is_some_and(|p| p.readonly()) {
        bail!("{}: file is read-only", path.display());
    }
    let name = real
        .file_name()
        .context("not a file path")?
        .to_string_lossy();
    let tmp = real.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true); // O_EXCL: never follows a planted symlink
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Some(p) = &perms {
            opts.mode(p.mode() & 0o7777); // never wider than the source, even briefly
        }
    }
    let mut f = opts.open(&tmp).with_context(|| tmp.display().to_string())?;
    let wrote = (|| {
        f.write_all(data.as_bytes())?;
        f.sync_all()?;
        match perms {
            Some(p) => f.set_permissions(p),
            None => Ok(()),
        }
    })();
    drop(f);
    if let Err(e) = wrote.and_then(|()| std::fs::rename(&tmp, &real)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e).with_context(|| path.display().to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_planted_temp_symlink_is_never_followed() {
        let dir = tempfile::tempdir().unwrap();
        let (target, victim) = (dir.path().join("f.vdg"), dir.path().join("victim"));
        std::fs::write(&target, "old").unwrap();
        std::fs::write(&victim, "precious").unwrap();
        let planted = dir
            .path()
            .join(format!(".f.vdg.{}.tmp", std::process::id()));
        std::os::unix::fs::symlink(&victim, &planted).unwrap();
        assert!(write_atomic(&target, "new").is_err());
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "precious");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
    }

    #[test]
    fn output_equal_to_input_is_refused_by_any_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.vdg");
        std::fs::write(&input, "x").unwrap();
        let dotted = dir.path().join(".").join("in.vdg");
        let err = ensure_not_input(&dotted, input.to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("input"), "{err}");
        ensure_not_input(&dir.path().join("out.svg"), input.to_str().unwrap()).unwrap();
        ensure_not_input(&input, "-").unwrap();
    }
}
