//! GitHub adapter: PR lifecycle via `vox-forge`, local git via CLI (worktrees).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use vox_forge::github::GitHubProvider;
use vox_forge::{GitForgeProvider, NewChangeRequest};
use vox_git::GitBridge;

use super::path_policy;

/// Parse owner/repo from a GitHub URL (https or git).
pub(crate) fn parse_github_owner_repo(url: &str) -> Option<(String, String)> {
    let url = url.trim().trim_end_matches('/');
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("git@github.com:"))?;
    let rest = rest.trim_end_matches(".git");
    let mut parts = rest.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

/// Resolve GitHub token: `GITHUB_TOKEN` / `GH_TOKEN`.
pub(crate) fn github_token() -> Result<String> {
    std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .context("GITHUB_TOKEN or GH_TOKEN required. Set env or use `gh auth token` piped export.")
}

fn owner_repo_from_path(path: &Path) -> Result<(String, String)> {
    let bridge = GitBridge::open(path).context("open git repo")?;
    let remote_url = bridge.remote_url().context("remote URL")?;
    parse_github_owner_repo(&remote_url).context("parse GitHub owner/repo from remote URL")
}

/// Submit: branch, optional commit, push, PR, trigger CodeRabbit.
///
/// Prefer `--no-commit` and commit selectively; `no_commit == false` runs `git add -A` (broad).
pub async fn submit(
    path: &Path,
    branch: Option<&str>,
    title: Option<&str>,
    no_commit: bool,
) -> Result<u64> {
    let (owner, repo) = owner_repo_from_path(path)?;

    let branch_name = branch
        .map(String::from)
        .unwrap_or_else(|| format!("coderabbit/{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")));

    let status = tokio::process::Command::new("git")
        .args(["checkout", "-b", &branch_name])
        .current_dir(path)
        .status()
        .await
        .context("git checkout -b")?;
    if !status.success() {
        let status2 = tokio::process::Command::new("git")
            .args(["checkout", &branch_name])
            .current_dir(path)
            .status()
            .await
            .context("git checkout")?;
        if !status2.success() {
            anyhow::bail!("git checkout -b failed");
        }
    }

    if !no_commit {
        let status = tokio::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(path)
            .status()
            .await
            .context("git add -A")?;
        if !status.success() {
            anyhow::bail!("git add -A failed");
        }

        let status = tokio::process::Command::new("git")
            .args(["commit", "-m", "chore: batch for CodeRabbit review"])
            .current_dir(path)
            .status()
            .await
            .context("git commit")?;
        if !status.success() {
            anyhow::bail!("git commit failed (nothing to commit or commit error)");
        }
    }

    let status = tokio::process::Command::new("git")
        .args(["push", "-u", "origin", &branch_name])
        .current_dir(path)
        .status()
        .await
        .context("git push")?;
    if !status.success() {
        anyhow::bail!("git push failed");
    }

    let token = github_token()?;
    let provider = GitHubProvider::new(&token).map_err(|e| anyhow::anyhow!("{e}"))?;

    let repo_info = provider
        .repo_info(&owner, &repo)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let target_branch = repo_info.default_branch;

    let pr_title = title
        .map(String::from)
        .unwrap_or_else(|| format!("CodeRabbit review: {branch_name}"));
    let body_str = "Automated PR for CodeRabbit review.\n\n@coderabbitai review";
    let cr = provider
        .create_change_request(
            &owner,
            &repo,
            NewChangeRequest {
                title: &pr_title,
                body: body_str,
                source_branch: &branch_name,
                target_branch: &target_branch,
                draft: false,
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let _ = trigger_coderabbit(&token, &owner, &repo, cr.number, false).await;

    eprintln!("PR #{} opened: {}", cr.number, cr.web_url);
    Ok(cr.number)
}

/// Post a comment to trigger CodeRabbit review.
pub async fn trigger_coderabbit(
    token: &str,
    owner: &str,
    repo: &str,
    pr_number: u64,
    full_review: bool,
) -> Result<()> {
    let body = if full_review {
        "@coderabbitai full review"
    } else {
        "@coderabbitai review"
    };

    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{pr_number}/comments");
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&serde_json::json!({ "body": body }))
        .send()
        .await
        .context("POST issue comment")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub API {status}: {text}");
    }
    Ok(())
}

/// Push `refs/heads/<baseline_name>` to the same commit as `origin/<default_branch>` (after fetch).
pub fn push_baseline_from_origin(
    repo: &Path,
    baseline_name: &str,
    default_branch: &str,
) -> Result<String> {
    let status = Command::new("git")
        .args(["fetch", "origin", default_branch])
        .current_dir(repo)
        .status()
        .context("git fetch origin")?;
    if !status.success() {
        anyhow::bail!("git fetch origin {default_branch} failed");
    }
    let remote_ref = format!("origin/{default_branch}");
    let out = Command::new("git")
        .args(["rev-parse", &remote_ref])
        .current_dir(repo)
        .output()
        .context("git rev-parse")?;
    if !out.status.success() {
        anyhow::bail!(
            "rev-parse {remote_ref}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let refspec = format!("{sha}:refs/heads/{baseline_name}");
    let status = Command::new("git")
        .args(["push", "origin", &refspec])
        .current_dir(repo)
        .status()
        .context("git push baseline")?;
    if !status.success() {
        anyhow::bail!("git push origin {refspec} failed");
    }
    eprintln!("[baseline] {baseline_name} -> {sha} (from {remote_ref})");
    Ok(sha)
}

/// Worktree checkout directory for `review_branch` (slashes escaped for filesystem safety).
#[must_use]
pub fn worktree_dir(repo: &Path, review_branch: &str) -> PathBuf {
    let safe = review_branch.replace(['/', '\\'], "__");
    repo.join(".coderabbit").join("worktrees").join(safe)
}

/// Remove a registered git worktree and delete its directory; retry once on failure.
pub async fn git_worktree_remove(repo: &Path, path: &Path) -> Result<()> {
    for attempt in 1..=2u8 {
        let st = tokio::process::Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(path)
            .current_dir(repo)
            .status()
            .await
            .with_context(|| format!("git worktree remove {}", path.display()))?;
        if st.success() {
            break;
        }
        if attempt == 2 {
            anyhow::bail!(
                "git worktree remove failed for {} after retry (exit {:?}). \
                 Try closing programs using that folder, then `git worktree remove --force {}` from the repo root.",
                path.display(),
                st.code(),
                path.display()
            );
        }
        eprintln!(
            "[warn] git worktree remove attempt {attempt} failed for {}; retrying…",
            path.display()
        );
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    if path.exists() {
        tokio::fs::remove_dir_all(path)
            .await
            .with_context(|| format!("remove worktree dir {}", path.display()))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    use walkdir::WalkDir;
    for entry in WalkDir::new(src)
        .follow_links(false)
        .max_depth(20) // safety: prevent infinite recursion if a dir somehow nests back into itself
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if entry.file_type().is_symlink() {
            continue;
        }
        let rel = p.strip_prefix(src).unwrap_or(p);
        let rel_str = rel.to_string_lossy();
        if path_policy::should_skip_overlay_copy_path(&rel_str) {
            continue;
        }
        // Also skip any path component named `.coderabbit`.  When `src` is already
        // inside `.coderabbit/`, rel paths like `worktrees/…` wouldn't match the
        // `.coderabbit/` prefix check above — this per-component guard catches them.
        if rel
            .components()
            .any(|c| c.as_os_str().to_string_lossy().as_ref() == ".coderabbit")
        {
            continue;
        }
        let target = dst.join(rel);
        if p.is_dir() {
            fs::create_dir_all(&target)?;
        } else if p.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(p, &target)
                .with_context(|| format!("copy {} -> {}", p.display(), target.display()))?;
        }
    }
    Ok(())
}


/// Create a review PR from an isolated worktree: baseline → worktree, overlay files from `source_tree`, push, open PR.
///
/// Does not modify the main working tree other than registering a git worktree under `.coderabbit/worktrees/`.
pub async fn create_chunk_pr_via_worktree(
    repo_root: &Path,
    source_tree: &Path,
    _default_branch: &str,
    baseline_branch: &str,
    review_branch: &str,
    files: &[String],
    full_review: bool,
) -> Result<u64> {
    let (owner, repo_name) = owner_repo_from_path(repo_root)?;
    let token = github_token()?;
    let provider = GitHubProvider::new(&token).map_err(|e| anyhow::anyhow!("{e}"))?;

    let wt = worktree_dir(repo_root, review_branch);
    if wt.exists() {
        git_worktree_remove(repo_root, &wt)
            .await
            .with_context(|| format!("remove stale worktree {}", wt.display()))?;
    }
    if let Some(parent) = wt.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }

    let fetch_baseline = tokio::process::Command::new("git")
        .args(["fetch", "origin", baseline_branch])
        .current_dir(repo_root)
        .status()
        .await
        .context("git fetch baseline branch")?;
    if !fetch_baseline.success() {
        anyhow::bail!("git fetch origin {baseline_branch} failed");
    }

    let wt_str = wt.to_str().with_context(|| "worktree path utf-8")?;
    let status = tokio::process::Command::new("git")
        .args([
            "worktree",
            "add",
            "-B",
            review_branch,
            wt_str,
            &format!("origin/{baseline_branch}"),
        ])
        .current_dir(repo_root)
        .status()
        .await
        .context("git worktree add")?;
    if !status.success() {
        anyhow::bail!("git worktree add failed for {review_branch}");
    }

    for rel in files {
        let rel_norm = rel.replace('\\', "/");
        if path_policy::is_coderabbit_local_tool_path(&rel_norm) {
            continue;
        }
        let src = source_tree.join(&rel_norm);
        let dst = wt.join(&rel_norm);
        if src.is_file() {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dst).with_context(|| format!("overlay {}", rel_norm))?;
        } else if src.is_dir() {
            if dst.exists() {
                fs::remove_dir_all(&dst)?;
            }
            copy_dir_recursive(&src, &dst)?;
        } else {
            if dst.exists() {
                if dst.is_dir() {
                    fs::remove_dir_all(&dst)?;
                } else {
                    fs::remove_file(&dst)?;
                }
            }
        }
    }

    if !files.is_empty() {
        for batch in files.chunks(80) {
            let mut args: Vec<&str> = vec!["add", "--"];
            let batch_str: Vec<String> = batch.iter().map(|s| s.replace('\\', "/")).collect();
            let refs: Vec<&str> = batch_str.iter().map(|s| s.as_str()).collect();
            args.extend(refs);
            let st = tokio::process::Command::new("git")
                .args(&args)
                .current_dir(&wt)
                .status()
                .await
                .context("git add")?;
            if !st.success() {
                eprintln!("[warn] git add batch had failures (paths may be deleted)");
            }
        }
    }

    let commit_msg = format!("feat: CodeRabbit chunk {review_branch}");
    let cst = tokio::process::Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(&wt)
        .status()
        .await
        .context("git commit")?;
    if !cst.success() {
        anyhow::bail!("git commit failed in worktree (nothing staged?)");
    }

    let push_st = tokio::process::Command::new("git")
        .args(["push", "-u", "origin", review_branch])
        .current_dir(&wt)
        .status()
        .await
        .context("git push from worktree")?;
    if !push_st.success() {
        anyhow::bail!("git push failed for {review_branch}");
    }

    let pr_title = format!("CodeRabbit review: {review_branch}");
    let body = format!(
        "Automated semantic PR for CodeRabbit.\n\n@coderabbitai {}",
        if full_review { "full review" } else { "review" }
    );
    let cr = provider
        .create_change_request(
            &owner,
            &repo_name,
            NewChangeRequest {
                title: &pr_title,
                body: body.as_str(),
                source_branch: review_branch,
                target_branch: baseline_branch,
                draft: false,
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let _ = trigger_coderabbit(&token, &owner, &repo_name, cr.number, full_review).await;
    eprintln!(
        "PR #{} opened: {} (base={})",
        cr.number, cr.web_url, baseline_branch
    );
    Ok(cr.number)
}

/// Stacked chunk PR: branch from `base_branch`, checkout files from `default_branch` tip, commit, push, PR into `base_branch`.
pub async fn create_stack_chunk_pr(
    path: &Path,
    default_branch: &str,
    base_branch: &str,
    new_branch: &str,
    files: &[String],
    full_review: bool,
) -> Result<u64> {
    let status = tokio::process::Command::new("git")
        .args(["checkout", "-b", new_branch, base_branch])
        .current_dir(path)
        .status()
        .await
        .context("git checkout -b for stack chunk")?;
    if !status.success() {
        anyhow::bail!("git checkout -b {new_branch} {base_branch} failed");
    }

    if !files.is_empty() {
        let mut args = vec!["checkout", default_branch, "--"];
        args.extend(files.iter().map(|s| s.as_str()));
        let status = tokio::process::Command::new("git")
            .args(&args)
            .current_dir(path)
            .status()
            .await
            .context("git checkout files from default branch")?;
        if !status.success() {
            anyhow::bail!("git checkout {default_branch} -- <files> failed");
        }

        let status = tokio::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(path)
            .status()
            .await
            .context("git add -A")?;
        if !status.success() {
            anyhow::bail!("git add -A failed");
        }

        let status = tokio::process::Command::new("git")
            .args([
                "commit",
                "-m",
                &format!(
                    "feat: CodeRabbit review chunk {}",
                    new_branch.trim_start_matches("cr-review-")
                ),
            ])
            .current_dir(path)
            .status()
            .await
            .context("git commit")?;
        if !status.success() {
            anyhow::bail!("git commit failed (nothing to commit or commit error)");
        }
    }

    let status = tokio::process::Command::new("git")
        .args(["push", "-u", "origin", new_branch])
        .current_dir(path)
        .status()
        .await
        .context("git push")?;
    if !status.success() {
        anyhow::bail!("git push failed");
    }

    let (owner, repo) = owner_repo_from_path(path)?;
    let token = github_token()?;
    let provider = GitHubProvider::new(&token).map_err(|e| anyhow::anyhow!("{e}"))?;

    let pr_title = format!("CodeRabbit review: {new_branch}");
    let body = format!(
        "Automated stacked PR for CodeRabbit review.\n\n@coderabbitai {}",
        if full_review { "full review" } else { "review" }
    );
    let cr = provider
        .create_change_request(
            &owner,
            &repo,
            NewChangeRequest {
                title: &pr_title,
                body: body.as_str(),
                source_branch: new_branch,
                target_branch: base_branch,
                draft: false,
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let _ = trigger_coderabbit(&token, &owner, &repo, cr.number, full_review).await;

    eprintln!(
        "PR #{} opened: {} (base={})",
        cr.number, cr.web_url, base_branch
    );
    Ok(cr.number)
}

/// Create an empty orphan branch for stack baseline (requires **clean** working tree).
pub async fn create_orphan_baseline(path: &Path, branch_name: &str) -> Result<()> {
    let out = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .await
        .context("git status --porcelain")?;
    if !out.stdout.is_empty() {
        anyhow::bail!(
            "Working tree must be clean for `stack-submit` orphan baseline (destructive checkout).\n\
             Commit your work first, or use `vox review coderabbit semantic-submit` (worktrees; no orphan baseline).\n\
             Uncommitted changes:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
    }

    eprintln!(
        "[warn] stack-submit: `git checkout --orphan {branch_name}` will replace the current checkout with a minimal tree until the command finishes and restores your branch."
    );

    let status = tokio::process::Command::new("git")
        .args(["checkout", "--orphan", branch_name])
        .current_dir(path)
        .status()
        .await
        .context("git checkout --orphan")?;
    if !status.success() {
        anyhow::bail!("git checkout --orphan {branch_name} failed");
    }

    let _ = tokio::process::Command::new("git")
        .args(["read-tree", "--empty"])
        .current_dir(path)
        .status()
        .await;

    let readme = "# CodeRabbit stack baseline\n\nEmpty baseline for stacked review PRs.\n";
    fs::write(path.join("README.md"), readme).context("Write baseline README")?;

    let status = tokio::process::Command::new("git")
        .args(["add", "README.md"])
        .current_dir(path)
        .status()
        .await
        .context("git add README.md")?;
    if !status.success() {
        anyhow::bail!("git add README.md failed");
    }

    let status = tokio::process::Command::new("git")
        .args(["commit", "-m", "chore: empty baseline for CodeRabbit stack"])
        .current_dir(path)
        .status()
        .await
        .context("git commit")?;
    if !status.success() {
        anyhow::bail!("git commit failed");
    }

    let status = tokio::process::Command::new("git")
        .args(["push", "-u", "origin", branch_name])
        .current_dir(path)
        .status()
        .await
        .context("git push")?;
    if !status.success() {
        anyhow::bail!("git push failed");
    }

    Ok(())
}

/// Wait for CodeRabbit bot review on a PR (polls GitHub reviews).
pub async fn wait_for_review(pr_number: u64, timeout_secs: u64, path: &Path) -> Result<()> {
    let (owner, repo) = owner_repo_from_path(path)?;
    let token = github_token()?;
    let provider = GitHubProvider::new(&token).map_err(|e| anyhow::anyhow!("{e}"))?;

    let start = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_secs(30);

    while start.elapsed().as_secs() < timeout_secs {
        let reviews = provider
            .list_reviews(&owner, &repo, pr_number)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let coderabbit_review = reviews.iter().find(|r| {
            let login = r.reviewer.to_lowercase();
            login.contains("coderabbit") || login.contains("code-rabbit")
        });

        if coderabbit_review.is_some() {
            eprintln!("CodeRabbit review completed for PR #{pr_number}");
            return Ok(());
        }

        tokio::time::sleep(poll_interval).await;
    }

    anyhow::bail!("Timeout waiting for CodeRabbit review on PR #{pr_number} ({timeout_secs}s)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn worktree_dir_sanitizes_branch_slashes() {
        let repo = Path::new("/repo");
        let w = worktree_dir(repo, "cr/review-02_foo");
        let s = w.to_string_lossy();
        assert!(s.contains("cr__review-02_foo"), "got {s}");
    }
}
