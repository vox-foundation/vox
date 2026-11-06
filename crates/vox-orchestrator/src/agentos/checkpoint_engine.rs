//! Sparse checkpoint eligibility from mutation classification and Git plumbing
//! shadow checkpoint store (`T3-WORKTREE-VCS-02`).

use super::mutation_classifier::mutation_kind_for_tool;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Whether a durable checkpoint should be taken **before** executing this tool turn.
pub fn should_sparse_checkpoint(tool: &str) -> bool {
    matches!(
        mutation_kind_for_tool(tool),
        "local_mutation" | "external_side_effect"
    )
}

/// Metadata returned when a Git plumbing checkpoint or restore point is created (`T3-WORKTREE-VCS-02`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub thread_id: String,
    pub turn_count: u32,
    pub ref_name: String,
    pub commit_oid: String,
    pub is_restore_point: bool,
}

/// Shadow Git checkpoint store using isolated `GIT_INDEX_FILE` and `git write-tree` + `git commit-tree`
/// under `refs/vox/checkpoints/<thread_id>/turn/<n>` without mutating the user's index or HEAD (`T3-WORKTREE-VCS-02`).
#[derive(Debug, Clone, Default)]
pub struct GitPlumbingCheckpointStore;

impl GitPlumbingCheckpointStore {
    pub fn checkpoint_ref_name(thread_id: &str, turn_count: u32) -> String {
        format!("refs/vox/checkpoints/{thread_id}/turn/{turn_count}")
    }

    pub fn restore_point_ref_name(thread_id: &str, label: &str) -> String {
        format!("refs/vox/checkpoints/{thread_id}/restore-point/{label}")
    }

    fn run_git(
        repo_root: &Path,
        index_file: Option<&Path>,
        args: &[&str],
    ) -> Result<String, String> {
        let git_bin = std::env::var("VOX_GIT_BIN").unwrap_or_else(|_| "git".to_string());
        let mut cmd = Command::new(&git_bin);
        cmd.current_dir(repo_root).args(args);
        if let Some(idx) = index_file {
            cmd.env("GIT_INDEX_FILE", idx);
        }
        let out = cmd.output().map_err(|e| format!("git spawn failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Captures a full working-tree + untracked snapshot into a commit under `ref_name`
    /// using a temporary `GIT_INDEX_FILE` so the user's `.git/index` is untouched.
    pub fn capture_ref(
        repo_root: &Path,
        thread_id: &str,
        turn_count: u32,
        ref_name: &str,
        is_restore_point: bool,
    ) -> Result<CheckpointRecord, String> {
        let sanitized_thread: String = thread_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let temp_index = std::env::temp_dir().join(format!(
            "vox_ckpt_idx_{}_{}_{}",
            std::process::id(),
            sanitized_thread,
            turn_count
        ));
        let _ = std::fs::remove_file(&temp_index);

        let result = (|| {
            // Seed temp index from HEAD if repo has a commit
            let head_oid = Self::run_git(repo_root, None, &["rev-parse", "--verify", "HEAD"]).ok();
            let _ = Self::run_git(repo_root, Some(&temp_index), &["read-tree", "HEAD"]);
            Self::run_git(repo_root, Some(&temp_index), &["add", "-A", "--", "."])?;
            let tree_oid = Self::run_git(repo_root, Some(&temp_index), &["write-tree"])?;
            let msg = format!("vox checkpoint {thread_id} turn {turn_count}");
            let commit_oid = if let Some(ref parent) = head_oid {
                Self::run_git(
                    repo_root,
                    Some(&temp_index),
                    &["commit-tree", &tree_oid, "-p", parent, "-m", &msg],
                )?
            } else {
                Self::run_git(
                    repo_root,
                    Some(&temp_index),
                    &["commit-tree", &tree_oid, "-m", &msg],
                )?
            };
            Self::run_git(repo_root, None, &["update-ref", ref_name, &commit_oid])?;
            Ok(CheckpointRecord {
                thread_id: thread_id.to_string(),
                turn_count,
                ref_name: ref_name.to_string(),
                commit_oid,
                is_restore_point,
            })
        })();

        let _ = std::fs::remove_file(&temp_index);
        result
    }

    pub fn capture_checkpoint(
        repo_root: &Path,
        thread_id: &str,
        turn_count: u32,
    ) -> Result<CheckpointRecord, String> {
        let ref_name = Self::checkpoint_ref_name(thread_id, turn_count);
        Self::capture_ref(repo_root, thread_id, turn_count, &ref_name, false)
    }

    pub fn create_restore_point(
        repo_root: &Path,
        thread_id: &str,
        turn_count: u32,
        label: &str,
    ) -> Result<CheckpointRecord, String> {
        let ref_name = Self::restore_point_ref_name(thread_id, label);
        Self::capture_ref(repo_root, thread_id, turn_count, &ref_name, true)
    }

    /// Computes unified diff (`git diff <from_ref> <to_ref>`) with `--no-ext-diff --no-textconv`.
    pub fn diff_checkpoints(
        repo_root: &Path,
        from_ref: &str,
        to_ref: &str,
    ) -> Result<String, String> {
        Self::run_git(
            repo_root,
            None,
            &[
                "-c",
                "core.autocrlf=false",
                "diff",
                "--patch",
                "--minimal",
                "--no-color",
                "--no-ext-diff",
                "--no-textconv",
                from_ref,
                to_ref,
            ],
        )
    }

    /// Restores the working tree to `target_ref` and deletes checkpoint refs for turns `> target_turn`.
    pub fn restore_checkpoint(
        repo_root: &Path,
        thread_id: &str,
        target_turn: u32,
        max_known_turn: u32,
    ) -> Result<(), String> {
        let target_ref = Self::checkpoint_ref_name(thread_id, target_turn);
        Self::run_git(
            repo_root,
            None,
            &[
                "restore",
                "--source",
                &target_ref,
                "--worktree",
                "--staged",
                "--",
                ".",
            ],
        )?;
        // Remove untracked files created after the target checkpoint
        let _ = Self::run_git(repo_root, None, &["clean", "-fd"]);
        for turn in (target_turn + 1)..=max_known_turn {
            let stale_ref = Self::checkpoint_ref_name(thread_id, turn);
            let _ = Self::run_git(repo_root, None, &["update-ref", "-d", &stale_ref]);
        }
        Ok(())
    }

    /// Lists all checkpoints (`refs/vox/checkpoints/<thread_id>/**`) stored in the repository.
    pub fn list_checkpoints(
        repo_root: &Path,
        thread_id: &str,
    ) -> Result<Vec<CheckpointRecord>, String> {
        let prefix = format!("refs/vox/checkpoints/{thread_id}/");
        let raw = Self::run_git(
            repo_root,
            None,
            &["for-each-ref", "--format=%(refname) %(objectname)", &prefix],
        )?;
        let mut out = Vec::new();
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            let Some(ref_name) = parts.next() else {
                continue;
            };
            let Some(commit_oid) = parts.next() else {
                continue;
            };
            let is_restore_point = ref_name.contains("/restore-point/");
            let turn_count = ref_name
                .rsplit('/')
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            out.push(CheckpointRecord {
                thread_id: thread_id.to_string(),
                turn_count,
                ref_name: ref_name.to_string(),
                commit_oid: commit_oid.to_string(),
                is_restore_point,
            });
        }
        out.sort_by_key(|r| r.turn_count);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_skips_checkpoint() {
        assert!(!should_sparse_checkpoint("vox_git_status"));
    }

    #[test]
    fn write_requires_checkpoint() {
        assert!(should_sparse_checkpoint("vox_write_file"));
    }

    #[test]
    fn git_plumbing_checkpoint_captures_and_diffs_without_touching_head() {
        let tmp = std::env::temp_dir().join(format!("vox_ckpt_repo_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        GitPlumbingCheckpointStore::run_git(&tmp, None, &["init"]).unwrap();
        GitPlumbingCheckpointStore::run_git(
            &tmp,
            None,
            &["config", "user.email", "vox@test.local"],
        )
        .unwrap();
        GitPlumbingCheckpointStore::run_git(&tmp, None, &["config", "user.name", "Vox Test"])
            .unwrap();

        std::fs::write(tmp.join("hello.txt"), "v1\n").unwrap();
        let c1 = GitPlumbingCheckpointStore::capture_checkpoint(&tmp, "t1", 1).unwrap();

        std::fs::write(tmp.join("hello.txt"), "v2\n").unwrap();
        let c2 = GitPlumbingCheckpointStore::capture_checkpoint(&tmp, "t1", 2).unwrap();

        let diff =
            GitPlumbingCheckpointStore::diff_checkpoints(&tmp, &c1.ref_name, &c2.ref_name).unwrap();
        assert!(diff.contains("-v1"));
        assert!(diff.contains("+v2"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
