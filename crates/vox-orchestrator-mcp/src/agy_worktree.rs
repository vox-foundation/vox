//! Isolated git worktree that jails an `agy` delegation (our real safety
//! boundary under --dangerously-skip-permissions), plus diff capture, NTFS
//! junction-safe worktree cleanup (`T3-WORKTREE-VCS-01`), and untracked `.env`
//! copy-on-provision (`T3-WORKTREE-VCS-03`).

use crate::agy_exec::sanitize_slug;
use crate::git_exec::{GitExec, GitExecError};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Per-repository mutex map serializing worktree creation, removal, and pruning (`T3-WORKTREE-VCS-01` & `T3-WORKTREE-CKPT-01`).
#[derive(Default)]
pub struct WorkspaceLeaseMap {
    leases: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    async_leases: Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>,
}

impl WorkspaceLeaseMap {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<WorkspaceLeaseMap> = OnceLock::new();
        INSTANCE.get_or_init(Self::default)
    }

    pub fn lease_for(&self, repo_root: &Path) -> Arc<Mutex<()>> {
        let canonical = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf());
        let mut guard = self.leases.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .entry(canonical)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub fn async_lease_for(&self, repo_root: &Path) -> Arc<tokio::sync::Mutex<()>> {
        let canonical = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf());
        let mut guard = self.async_leases.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .entry(canonical)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

/// Validates that `worktree_path` is strictly nested 2 path segments under `managed_root`
/// (`<repoHash>/<worktreeDir>`) and contains no parent-directory traversal (`T3-WORKTREE-VCS-01`).
pub fn validate_managed_worktree_subpath(
    managed_root: &Path,
    worktree_path: &Path,
) -> Result<(), String> {
    let rel = worktree_path
        .strip_prefix(managed_root)
        .map_err(|_| "worktree_path is not inside managed_root".to_string())?;
    let mut segments = Vec::new();
    for comp in rel.components() {
        match comp {
            Component::Normal(s) => {
                let s_str = s.to_string_lossy();
                if s_str.is_empty() || s_str == "." || s_str == ".." {
                    return Err("invalid segment in managed worktree path".to_string());
                }
                segments.push(s_str.into_owned());
            }
            _ => return Err("non-normal path component in managed worktree path".to_string()),
        }
    }
    if segments.len() != 2 {
        return Err(format!(
            "managed worktree path must have exactly 2 relative segments (<repoHash>/<worktreeDir>), got {}",
            segments.len()
        ));
    }
    Ok(())
}

/// Removes a linked git worktree safely on Windows/NTFS (`fs::remove_dir_all` + `git worktree prune`)
/// instead of `git worktree remove --force`, which fails when pnpm/Vite junctions exist (`T3-WORKTREE-VCS-01`).
pub async fn remove_linked_worktree(
    repo_root: &Path,
    managed_root: &Path,
    worktree_path: &Path,
) -> Result<(), GitExecError> {
    validate_managed_worktree_subpath(managed_root, worktree_path).map_err(|msg| {
        GitExecError::NonZero {
            code: 1,
            stdout: String::new(),
            stderr: msg,
        }
    })?;
    let lease = WorkspaceLeaseMap::global().lease_for(repo_root);
    {
        let _guard = lease.lock().unwrap_or_else(|e| e.into_inner());
        if worktree_path.exists() {
            std::fs::remove_dir_all(worktree_path).map_err(GitExecError::Spawn)?;
        }
    }
    let _ = GitExec::new(repo_root).run(&["worktree", "prune"]).await;
    Ok(())
}

/// Copies untracked `.env` and `.env.*` files (excluding `.env.example`) from `repo_root`
/// to `worktree_path` without overwriting existing files (`T3-WORKTREE-VCS-03`).
pub fn provision_untracked_env_files(
    repo_root: &Path,
    worktree_path: &Path,
    untracked_rel_paths: &[String],
) -> std::io::Result<usize> {
    let mut copied = 0usize;
    for rel in untracked_rel_paths {
        let path = Path::new(rel);
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let is_env =
            (file_name == ".env" || file_name.starts_with(".env.")) && file_name != ".env.example";
        if !is_env {
            continue;
        }
        let src = repo_root.join(path);
        let dst = worktree_path.join(path);
        if !src.is_file() || dst.exists() {
            continue;
        }
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dst)
        {
            Ok(mut out_file) => {
                let mut in_file = std::fs::File::open(&src)?;
                std::io::copy(&mut in_file, &mut out_file)?;
                copied += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(copied)
}

pub fn delegation_worktree_path(repo_root: &Path, slug: &str) -> PathBuf {
    repo_root.join(".vox").join("agy-worktrees").join(slug)
}

pub fn count_changed(tracked_diff: &str, untracked_list: &str) -> usize {
    let tracked = tracked_diff.matches("diff --git").count();
    let untracked = untracked_list
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    tracked + untracked
}

pub struct DelegationWorktree {
    pub path: PathBuf,
    pub branch: String,
    git: GitExec,
}

impl DelegationWorktree {
    /// Create a fresh worktree+branch off HEAD. `slug` MUST be unique per call
    /// (callers derive it from a monotonic counter; see agy_tools).
    pub async fn create(repo_root: &Path, slug: &str) -> Result<Self, GitExecError> {
        let slug = sanitize_slug(slug);
        let path = delegation_worktree_path(repo_root, &slug);
        let branch = format!("agy/{slug}");
        let path_s = path.to_string_lossy().to_string();
        // Acquire per-repository async lease to serialize worktree creation and .env provisioning (`T3-REGEN-WORKTREE-02` & `T3-WORKTREE-CKPT-01`).
        let async_lease = WorkspaceLeaseMap::global().async_lease_for(repo_root);
        let _async_guard = async_lease.lock().await;
        GitExec::new(repo_root)
            .run(&["worktree", "add", "-b", &branch, &path_s, "HEAD"])
            .await?;
        if let Ok(untracked) = GitExec::new(repo_root)
            .run(&["ls-files", "--others", "--exclude-standard"])
            .await
        {
            let rel_paths: Vec<String> = untracked
                .stdout
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            let _ = provision_untracked_env_files(repo_root, &path, &rel_paths);
        }
        Ok(Self {
            path: path.clone(),
            branch,
            git: GitExec::new(path),
        })
    }

    /// (unified-diff text, changed-file count). Includes tracked + untracked.
    pub async fn capture(&self) -> Result<(String, usize), GitExecError> {
        let tracked = self.git.run(&["diff", "HEAD"]).await?;
        let untracked = self
            .git
            .run(&["ls-files", "--others", "--exclude-standard"])
            .await?;
        let n = count_changed(&tracked.stdout, &untracked.stdout);
        let text = format!(
            "# tracked\n{}\n# new files\n{}",
            tracked.stdout, untracked.stdout
        );
        Ok((text, n))
    }

    /// Removes the delegation worktree using NTFS junction-safe directory removal + `git worktree prune`.
    pub async fn cleanup(&self, repo_root: &Path) -> Result<(), GitExecError> {
        let managed_root = repo_root.join(".vox");
        remove_linked_worktree(repo_root, &managed_root, &self.path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_path_is_jailed_under_dot_vox() {
        let p = delegation_worktree_path(std::path::Path::new("/repo"), "d-123-foo");
        assert!(p.starts_with("/repo/.vox/agy-worktrees"));
        assert!(p.to_string_lossy().contains("d-123-foo"));
    }

    #[test]
    fn counts_changed_files_from_diff_parts() {
        let tracked = "diff --git a/x b/x\n...\ndiff --git a/y b/y\n...";
        let untracked = "newfile.txt\n";
        assert_eq!(count_changed(tracked, untracked), 3);
    }

    #[test]
    fn validate_managed_worktree_subpath_enforces_two_segments() {
        let managed = Path::new("/repo/.vox");
        assert!(
            validate_managed_worktree_subpath(managed, Path::new("/repo/.vox/agy-worktrees/wt-01"))
                .is_ok()
        );
        assert!(validate_managed_worktree_subpath(managed, Path::new("/repo/.vox/wt-01")).is_err());
        assert!(
            validate_managed_worktree_subpath(
                managed,
                Path::new("/repo/.vox/agy-worktrees/wt-01/extra")
            )
            .is_err()
        );
        assert!(validate_managed_worktree_subpath(managed, Path::new("/other/repo/wt")).is_err());
    }

    #[test]
    fn provision_untracked_env_files_copies_env_and_skips_example() {
        let tmp = std::env::temp_dir().join(format!("vox_env_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let src = tmp.join("repo");
        let dst = tmp.join("wt");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        std::fs::write(src.join(".env"), "A=1\n").unwrap();
        std::fs::write(src.join(".env.local"), "B=2\n").unwrap();
        std::fs::write(src.join(".env.example"), "C=3\n").unwrap();

        let copied = provision_untracked_env_files(
            &src,
            &dst,
            &[
                ".env".to_string(),
                ".env.local".to_string(),
                ".env.example".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(copied, 2);
        assert!(dst.join(".env").exists());
        assert!(dst.join(".env.local").exists());
        assert!(!dst.join(".env.example").exists());

        // Second run does not overwrite existing files
        let copied_again =
            provision_untracked_env_files(&src, &dst, &[".env".to_string()]).unwrap();
        assert_eq!(copied_again, 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
