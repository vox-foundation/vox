//! `vox ci suppression-review-gate` — advisory when a PR looks suppression-only.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

fn looks_like_suppression_path(path: &str) -> bool {
    let p = path.replace('\\', "/");
    p.contains("suppressions")
        || p.ends_with("toestub/suppressions.v1.json")
        || p.contains("unreasoned_suppression")
}

fn git_diff_names(root: &Path, base: &str) -> Result<Vec<String>> {
    let out = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "diff", "--name-only", base])
        .output()
        .context("git diff")?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

pub struct SuppressionReviewArgs {
    pub base: String,
    pub paths: Option<PathBuf>,
    pub strict: bool,
}

pub fn run(root: &Path, args: SuppressionReviewArgs) -> Result<()> {
    let files = if let Some(list) = args.paths {
        let p = if list.is_absolute() {
            list
        } else {
            root.join(list)
        };
        std::fs::read_to_string(p)?
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        git_diff_names(root, &args.base).unwrap_or_default()
    };

    if files.is_empty() {
        println!("suppression-review-gate: no changed files — ok");
        return Ok(());
    }

    let suppressionish: Vec<_> = files
        .iter()
        .filter(|f| looks_like_suppression_path(f))
        .cloned()
        .collect();
    let only_suppressions = !suppressionish.is_empty()
        && files.iter().all(|f| {
            looks_like_suppression_path(f)
                || f.ends_with(".md")
                || f.starts_with(".planning/")
        });

    let labeled = std::env::var("NEEDS_HUMAN_REVIEW")
        .ok()
        .as_deref()
        == Some("1")
        || std::env::var("GITHUB_PR_LABELS")
            .unwrap_or_default()
            .contains("needs-human-review");

    if only_suppressions && !labeled {
        eprintln!(
            "::warning title=suppression-review::suppression-only diff ({}) — add needs-human-review label or CODEOWNERS approval",
            suppressionish.len()
        );
        if args.strict {
            anyhow::bail!("suppression-only PR requires needs-human-review");
        }
    } else {
        println!("suppression-review-gate: ok");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_suppression_paths() {
        assert!(looks_like_suppression_path(
            "contracts/toestub/suppressions.v1.json"
        ));
        assert!(!looks_like_suppression_path("crates/vox-cli/src/main.rs"));
    }
}
