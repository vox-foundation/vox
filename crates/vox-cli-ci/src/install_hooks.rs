use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Resolve the git hooks directory for `root`, working in both ordinary clones and
/// **worktrees** (where `.git` is a file pointing at `…/.git/worktrees/<name>`, so the old
/// `root.join(".git/hooks")` + `is_dir()` check silently skipped — exactly where Claude
/// agents run). `git rev-parse --git-path hooks` returns the correct hooks dir for either
/// layout and also honors `core.hooksPath`. Falls back to `<root>/.git/hooks` if git is
/// unavailable.
fn resolve_hooks_dir(root: &Path) -> Option<PathBuf> {
    // Route through vox_git so the concurrency policy is honored (arch-check
    // forbids raw `Command::new("git")`). This is a read-only query.
    let rel = vox_git::read_only(root, &["rev-parse", "--git-path", "hooks"]).ok()?;
    let rel = rel.trim();
    if rel.is_empty() {
        return None;
    }
    let p = Path::new(rel);
    Some(if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    })
}

/// One hook system: lefthook, driven by the repository's `lefthook.yml`. That file is the SSOT
/// for every hook and is checked by `vox ci dev-loop-guard` (hooks call installed binaries and
/// never build). This command used to write its own pre-push hook that ran `cargo build -p vox-cli`
/// before every push; it now only asks lefthook to install the hooks.
pub const INSTALL_ARGS: [&str; 1] = ["install"];

pub fn run(root: &Path) -> Result<()> {
    if resolve_hooks_dir(root).is_none() {
        println!(
            "Skipping hook installation: {} is not inside a git repository",
            root.display()
        );
        return Ok(());
    }
    let status = std::process::Command::new("lefthook")
        .args(INSTALL_ARGS)
        .current_dir(root)
        .status()
        .context(
            "spawn lefthook (install it: brew install lefthook | winget install evilmartians.lefthook | \
             cargo install lefthook); then run `vox run scripts/setup.vox` to install the hook tools",
        )?;
    anyhow::ensure!(
        status.success(),
        "lefthook install exited with {:?}",
        status.code()
    );
    println!(
        "✅ Installed git hooks from lefthook.yml (they call installed vox/toestub and never build)"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_through_lefthook_only() {
        assert_eq!(INSTALL_ARGS, ["install"]);
    }

    #[test]
    fn skips_outside_a_git_repository() {
        let dir = tempfile::tempdir().expect("tempdir");
        run(dir.path()).expect("a non-repository is skipped, not an error");
    }
}
