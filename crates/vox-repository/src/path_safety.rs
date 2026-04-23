//! Repository-scoped path resolution shared by MCP writes, catalog queries, and CLI.

use std::path::{Path, PathBuf};

/// Resolve `user_rel` strictly under `repo_root`: must be relative, no `..`, then return the
/// joined path (may not exist yet). Callers that need an on-disk path under root should
/// canonicalize after create/write.
pub fn resolve_strict_repo_relative_path(
    repo_root: &Path,
    user_rel: &str,
) -> Result<PathBuf, String> {
    let root =
        std::fs::canonicalize(repo_root).map_err(|e| format!("canonicalize repo root: {e}"))?;
    let rel = user_rel.trim();
    if rel.is_empty() {
        return Err("path must not be empty".into());
    }
    if Path::new(rel).is_absolute() {
        return Err("path must be relative to the repository root".into());
    }
    let mut acc = root.clone();
    for c in Path::new(rel).components() {
        match c {
            std::path::Component::Normal(p) => acc.push(p),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err("path must not contain '..'".into());
            }
            _ => return Err("invalid path".into()),
        }
    }
    Ok(acc)
}

/// Resolve a user path that may be absolute or workspace-relative; both must canonicalize under
/// `repo_root` (paths must exist on disk).
pub fn resolve_local_path_under_repo_root(
    repo_root: &Path,
    rel_or_abs: &str,
) -> Result<PathBuf, String> {
    let requested = Path::new(rel_or_abs);
    let joined = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        repo_root.join(requested)
    };
    let root_canon =
        std::fs::canonicalize(repo_root).map_err(|e| format!("canonicalize root: {e}"))?;
    let file_canon =
        std::fs::canonicalize(&joined).map_err(|e| format!("canonicalize path: {e}"))?;
    if !file_canon.starts_with(&root_canon) {
        return Err("path resolves outside repository root".to_string());
    }
    Ok(file_canon)
}

/// `/`-separated path relative to canonical `repo_root`.
pub fn path_relative_to_repo_root(
    repo_root: &Path,
    absolute_file: &Path,
) -> Result<String, String> {
    let root_canon =
        std::fs::canonicalize(repo_root).map_err(|e| format!("canonicalize root: {e}"))?;
    let file_canon =
        std::fs::canonicalize(absolute_file).map_err(|e| format!("canonicalize file: {e}"))?;
    if !file_canon.starts_with(&root_canon) {
        return Err("file is not under repository root".into());
    }
    Ok(file_canon
        .strip_prefix(&root_canon)
        .unwrap_or(&file_canon)
        .to_string_lossy()
        .replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn strict_rejects_parent_dir() {
        let d = TempDir::new().expect("tempdir");
        let err = resolve_strict_repo_relative_path(d.path(), "a/../../etc/passwd").unwrap_err();
        assert!(err.contains(".."));
    }

    #[test]
    fn strict_builds_under_root() {
        let d = TempDir::new().expect("tempdir");
        let p = resolve_strict_repo_relative_path(d.path(), "src/foo.vox").expect("ok");
        assert!(p.ends_with("src/foo.vox"));
    }

    #[test]
    fn local_resolves_existing_file() {
        let d = TempDir::new().expect("tempdir");
        fs::write(d.path().join("f.txt"), b"x").expect("write");
        let p = resolve_local_path_under_repo_root(d.path(), "f.txt").expect("ok");
        assert!(p.ends_with("f.txt"));
    }
}
