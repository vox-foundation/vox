//! MCP tool: `vox_commit_create`
//!
//! Creates a git commit from the currently staged changes, appending
//! standardised trailers and running a secret scan on the staged diff
//! before allowing the commit through.

use std::path::Path;

use super::secret_scan::{SecretMatch, scan_for_secrets};
use crate::git_exec::{GitExec, GitExecError};
use vox_orchestrator_types::WorkspaceId;

// ── Output / error types ─────────────────────────────────────────────────────

/// Successful result of [`commit_create`].
#[derive(Debug)]
pub struct CommitOutput {
    /// The full SHA returned by `git rev-parse HEAD` after the commit.
    pub sha: String,
    /// The full commit message that was passed to git, trailers included.
    pub message_used: String,
}

/// Errors that can arise during [`commit_create`].
#[derive(Debug, thiserror::Error)]
pub enum CommitError {
    #[error("secrets detected in staged diff ({} match(es))", .0.len())]
    SecretsDetected(Vec<SecretMatch>),

    #[error("git operation failed: {0}")]
    GitFailed(#[from] GitExecError),

    #[error("nothing staged — run `git add` first")]
    NoStagedChanges,
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Build the full commit message with standardised trailers.
///
/// Extracted as a free function so the trailer-formatting logic can be unit-
/// tested without touching git.
fn build_commit_message(
    message: &str,
    author_name: &str,
    author_email: &str,
    model_id: &str,
    workspace_id: u64,
) -> String {
    let trailers = format!(
        "Co-authored-by: {} <{}>\nVox-Model-Id: {}\nVox-Workspace: {}",
        author_name,
        author_email,
        model_id,
        WorkspaceId(workspace_id)
    );
    format!("{}\n\n{}", message.trim_end(), trailers)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Create a git commit from the currently staged changes.
///
/// # Steps
/// 1. Checks that something is staged (`git diff --cached --name-only`).
/// 2. Scans the staged diff for secrets; aborts if any are found.
/// 3. Builds a commit message with standard trailers.
/// 4. Runs `git commit --no-verify -m <message>`.
/// 5. Returns the new commit SHA and the message that was used.
pub async fn commit_create(
    cwd: &Path,
    message: &str,
    author_name: &str,
    author_email: &str,
    model_id: &str,
    workspace_id: u64,
) -> Result<CommitOutput, CommitError> {
    let git = GitExec::new(cwd);

    // 1. Ensure something is staged.
    let names_out = git.run(&["diff", "--cached", "--name-only"]).await?;
    if names_out.stdout.trim().is_empty() {
        return Err(CommitError::NoStagedChanges);
    }

    // 2. Secret scan on the staged diff.
    let diff_out = git.run(&["diff", "--cached", "--unified=0"]).await?;
    let secrets = scan_for_secrets(&diff_out.stdout);
    if !secrets.is_empty() {
        return Err(CommitError::SecretsDetected(secrets));
    }

    // 3. Build the commit message.
    let full_message =
        build_commit_message(message, author_name, author_email, model_id, workspace_id);

    // 4. Commit.
    git.run(&["commit", "--no-verify", "-m", &full_message])
        .await?;

    // 5. Retrieve the new SHA.
    let sha_out = git.run(&["rev-parse", "HEAD"]).await?;
    let sha = sha_out.stdout.trim().to_string();

    tracing::info!(
        target: "vox.vcs.commit",
        sha = %sha,
        workspace_id = workspace_id,
        "commit created"
    );

    Ok(CommitOutput {
        sha,
        message_used: full_message,
    })
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_message_has_trailers() {
        let msg = build_commit_message("fix: bug", "Bot", "bot@vox", "claude-opus-4-7", 42);
        assert!(
            msg.contains("Co-authored-by: Bot <bot@vox>"),
            "missing Co-authored-by trailer: {msg:?}"
        );
        assert!(
            msg.contains("Vox-Model-Id: claude-opus-4-7"),
            "missing Vox-Model-Id trailer: {msg:?}"
        );
        assert!(
            msg.contains("Vox-Workspace: W-000042"),
            "missing Vox-Workspace trailer: {msg:?}"
        );
    }

    #[test]
    fn commit_message_workspace_id_zero_padded() {
        let msg = build_commit_message("chore: noop", "Bot", "bot@vox", "some-model", 1);
        assert!(
            msg.contains("Vox-Workspace: W-000001"),
            "workspace id not zero-padded to 6 digits: {msg:?}"
        );
    }

    #[test]
    fn commit_message_trims_trailing_whitespace() {
        let msg = build_commit_message("fix: bug   ", "Bot", "bot@vox", "m", 0);
        // The body should end with "fix: bug" (no trailing spaces) before the
        // blank line that separates body from trailers.
        assert!(
            msg.starts_with("fix: bug\n\n"),
            "trailing whitespace not trimmed from body: {msg:?}"
        );
    }

    #[test]
    fn commit_message_blank_line_separates_body_and_trailers() {
        let msg = build_commit_message("feat: something", "Bot", "bot@vox", "m", 0);
        assert!(
            msg.contains("\n\nCo-authored-by:"),
            "expected double-newline before Co-authored-by: {msg:?}"
        );
    }
}
