//! HARN-01 serve target resolution: `--model` must be a run **directory**, never the
//! adapter file (Phase 320 gate bypass hazard).

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};

pub(crate) const ADAPTER_FILENAME: &str = "candle_qlora_adapter.safetensors";
pub(crate) const MANIFEST_FILENAME: &str = "training_manifest.json";

/// Canonical run directory for local `vox mens serve --model`.
///
/// Refuses a path whose final component is [`ADAPTER_FILENAME`] (filePath gate bypass).
/// Refuses symlinks at the supplied path. Missing directories are returned as
/// canonicalized as far as resolution allows (same discipline as `reasoning_compare`).
pub(crate) fn resolve_serve_run_dir(model: &Path) -> Result<PathBuf> {
    if model
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case(ADAPTER_FILENAME))
    {
        anyhow::bail!(
            "--model must be the adapter run directory (e.g. mens/runs/qwen3-0.6b), \
             not the {ADAPTER_FILENAME} file — serving the file skips collateral gate checks \
             (see Phase 320 LADDER-REPORT finding / D-321-10)"
        );
    }
    if std::fs::symlink_metadata(model)
        .ok()
        .is_some_and(|m| m.file_type().is_symlink())
    {
        anyhow::bail!(
            "--model {} is a symlink; refusing (use a plain run directory path)",
            model.display()
        );
    }
    let meta = std::fs::metadata(model).with_context(|| {
        format!(
            "--model {} must exist (run directory for serve)",
            model.display()
        )
    })?;
    if meta.is_file() {
        anyhow::bail!(
            "--model {} is a file, not a run directory; pass the directory containing {}",
            model.display(),
            ADAPTER_FILENAME
        );
    }
    resolved(model)
}

fn resolved(p: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(p).with_context(|| format!("resolving {}", p.display()))?;
    let mut cur = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                cur.pop();
            }
            other => {
                cur.push(other);
                match cur.canonicalize() {
                    Ok(real) => cur = real,
                    Err(_) if std::fs::symlink_metadata(&cur).is_ok() => anyhow::bail!(
                        "{} exists but does not resolve (dangling symlink?); refusing",
                        cur.display()
                    ),
                    Err(_) => {}
                }
            }
        }
    }
    Ok(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_adapter_tree(dir: &Path) {
        fs::write(dir.join(ADAPTER_FILENAME), b"stub").expect("adapter");
        fs::write(
            dir.join(MANIFEST_FILENAME),
            br#"{"schema":"vox_mens_training_manifest_v1"}"#,
        )
        .expect("manifest");
    }

    #[test]
    fn accepts_run_directory_with_adapter_markers() {
        let tmp = TempDir::new().expect("tempdir");
        write_adapter_tree(tmp.path());
        let got = resolve_serve_run_dir(tmp.path()).expect("ok");
        assert!(got.join(ADAPTER_FILENAME).exists());
    }

    #[test]
    fn accepts_directory_without_adapter_markers() {
        let tmp = TempDir::new().expect("tempdir");
        let got = resolve_serve_run_dir(tmp.path()).expect("ok");
        assert_eq!(got, tmp.path().canonicalize().expect("canon"));
    }

    #[test]
    fn refuses_adapter_file_path() {
        let tmp = TempDir::new().expect("tempdir");
        let adapter = tmp.path().join(ADAPTER_FILENAME);
        fs::write(&adapter, b"x").expect("write");
        let err = resolve_serve_run_dir(&adapter).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("run directory"), "{msg}");
        assert!(msg.contains(ADAPTER_FILENAME), "{msg}");
    }

    #[test]
    fn refuses_symlink_run_dir() {
        let tmp = TempDir::new().expect("tempdir");
        let real = tmp.path().join("real_run");
        fs::create_dir(&real).expect("mkdir");
        write_adapter_tree(&real);
        let link = tmp.path().join("link_run");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).expect("symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&real, &link).expect("symlink");
        let err = resolve_serve_run_dir(&link).unwrap_err();
        assert!(format!("{err:#}").contains("symlink"));
    }
}
