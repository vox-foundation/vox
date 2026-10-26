//! `vox ci evidence-asset-ban` — block committed PR evidence / screenshot spam
//! outside `contracts/ci/evidence-asset-allowlist.v1.yaml`.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_ALLOWLIST: &str = "contracts/ci/evidence-asset-allowlist.v1.yaml";

#[derive(Debug, Deserialize)]
struct AllowlistFile {
    schema_version: u32,
    #[serde(default)]
    banned_globs: Vec<String>,
    #[serde(default)]
    allowlist: Vec<String>,
}

fn normalize(p: &str) -> String {
    p.replace('\\', "/")
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let pat = normalize(pattern);
    let path = normalize(path);
    let path_s = path.as_str();

    // `**/review-bundle/**/*.{png,...}` first — must beat `split_once("/**/")`
    // which would otherwise treat left as the literal `**/review-bundle`.
    if let Some(rest) = pat.strip_prefix("**/") {
        if let Some((mid, file_pat)) = rest.split_once("/**/") {
            if !path_has_segment(path_s, mid) {
                return false;
            }
            let name = path_s.rsplit('/').next().unwrap_or(path_s);
            return match_file_pat(name, file_pat);
        }
        if let Some(prefix) = rest.strip_suffix("/**") {
            return path_has_segment(path_s, prefix);
        }
    }

    // Prefix directory allow/ban: `foo/bar/**`
    if let Some(prefix) = pat.strip_suffix("/**") {
        return path_s == prefix || path_s.starts_with(&format!("{prefix}/"));
    }

    // `dir/**/*.ext` or `dir/**/*.{png,webp}`
    if let Some((left, right)) = pat.split_once("/**/") {
        if !(path_s == left || path_s.starts_with(&format!("{left}/"))) {
            return false;
        }
        let name = path_s.rsplit('/').next().unwrap_or(path_s);
        return match_file_pat(name, right);
    }

    if !pat.contains('*') {
        return path_s == pat;
    }
    let name = path_s.rsplit('/').next().unwrap_or(path_s);
    match_file_pat(name, &pat)
}

/// True when `mid` appears as a full path segment (or the whole path).
fn path_has_segment(path: &str, mid: &str) -> bool {
    if mid.is_empty() {
        return false;
    }
    path == mid
        || path.starts_with(&format!("{mid}/"))
        || path.ends_with(&format!("/{mid}"))
        || path.contains(&format!("/{mid}/"))
}

fn match_file_pat(name: &str, file_pat: &str) -> bool {
    if file_pat == "*" || file_pat == "**" {
        return true;
    }
    if let Some(inner) = file_pat
        .strip_prefix("*.{")
        .and_then(|s| s.strip_suffix('}'))
    {
        return inner
            .split(',')
            .any(|ext| name.ends_with(&format!(".{ext}")));
    }
    if let Some(ext) = file_pat.strip_prefix("*.") {
        return name.ends_with(&format!(".{ext}"));
    }
    name == file_pat
}

fn is_allowlisted(path: &str, allow: &[String]) -> bool {
    allow.iter().any(|a| glob_match(a, path))
}

fn is_banned(path: &str, banned: &[String], allow: &[String]) -> bool {
    if is_allowlisted(path, allow) {
        return false;
    }
    banned.iter().any(|b| glob_match(b, path))
}

fn git_ls_files(root: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "ls-files", "-z"])
        .output()
        .context("git ls-files")?;
    if !out.status.success() {
        bail!("git ls-files failed");
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

fn load(path: &Path) -> Result<AllowlistFile> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let parsed: AllowlistFile = serde_yaml::from_str(&raw)?;
    if parsed.schema_version != 1 {
        bail!("unsupported schema_version {}", parsed.schema_version);
    }
    Ok(parsed)
}

pub struct EvidenceAssetBanArgs {
    pub allowlist: PathBuf,
    /// Optional newline-delimited path list (tests / changed-only).
    pub paths: Option<PathBuf>,
}

pub fn run(root: &Path, args: EvidenceAssetBanArgs) -> Result<()> {
    let allow_path = if args.allowlist.is_absolute() {
        args.allowlist.clone()
    } else {
        root.join(&args.allowlist)
    };
    let cfg = load(&allow_path)?;
    let files = if let Some(list) = args.paths {
        let p = if list.is_absolute() {
            list
        } else {
            root.join(list)
        };
        std::fs::read_to_string(&p)?
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        git_ls_files(root)?
    };

    let mut violations = Vec::new();
    for f in &files {
        if is_banned(f, &cfg.banned_globs, &cfg.allowlist) {
            violations.push(f.clone());
        }
    }
    if !violations.is_empty() {
        eprintln!("evidence-asset-ban: {} banned path(s):", violations.len());
        for v in &violations {
            eprintln!("  - {v}");
        }
        bail!(
            "committed evidence assets outside allowlist — see {}",
            DEFAULT_ALLOWLIST
        );
    }
    println!("evidence-asset-ban: ok ({} paths checked)", files.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlists_cv_flywheel_png() {
        let banned = vec!["crates/vox-gui/ui/artifacts/**/*.png".into()];
        let allow = vec!["crates/vox-gui/ui/artifacts/cv-flywheel/**".into()];
        assert!(!is_banned(
            "crates/vox-gui/ui/artifacts/cv-flywheel/surfaces/chat-after.png",
            &banned,
            &allow
        ));
        assert!(is_banned(
            "crates/vox-gui/ui/artifacts/scratch/foo.png",
            &banned,
            &allow
        ));
    }

    #[test]
    fn bans_review_bundle() {
        let banned = vec!["crates/vox-gui/ui/review-bundle/**".into()];
        let allow: Vec<String> = vec![];
        assert!(is_banned(
            "crates/vox-gui/ui/review-bundle/latest/chat.png",
            &banned,
            &allow
        ));
    }

    #[test]
    fn mutation_fails_closed_on_new_png() {
        // Intentional mutation: path not on allowlist must fail.
        let cfg = AllowlistFile {
            schema_version: 1,
            banned_globs: vec!["crates/vox-gui/ui/artifacts/**/*.png".into()],
            allowlist: vec!["crates/vox-gui/ui/artifacts/cv-flywheel/**".into()],
        };
        assert!(is_banned(
            "crates/vox-gui/ui/artifacts/pr-123/screenshot.png",
            &cfg.banned_globs,
            &cfg.allowlist
        ));
    }

    #[test]
    fn segment_bound_rejects_substring() {
        let banned = vec!["**/review-bundle/**/*.{png,webp}".into()];
        let allow: Vec<String> = vec![];
        assert!(!is_banned(
            "docs/my-review-bundle-notes.png",
            &banned,
            &allow
        ));
        assert!(is_banned("tmp/review-bundle/shot.png", &banned, &allow));
    }
}
