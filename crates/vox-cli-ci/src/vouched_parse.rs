//! `vox ci vouched-parse` — fail-closed parse of `.github/VOUCHED.td`.

use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const DEFAULT_VOUCHED: &str = ".github/VOUCHED.td";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VouchedFile {
    pub trusted: BTreeSet<String>,
    pub denounced: BTreeSet<String>,
}

pub fn parse(raw: &str) -> Result<VouchedFile> {
    let mut trusted = BTreeSet::new();
    let mut denounced = BTreeSet::new();
    for (idx, line) in raw.lines().enumerate() {
        let line_no = idx + 1;
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some(rest) = t.strip_prefix("-github:") {
            let user = rest.split_whitespace().next().unwrap_or("").trim();
            if user.is_empty() || !is_github_login(user) {
                bail!("VOUCHED.td:{line_no}: invalid denounce entry: {t}");
            }
            denounced.insert(user.to_ascii_lowercase());
        } else if let Some(user) = t.strip_prefix("github:") {
            let user = user.trim();
            if user.is_empty() || !is_github_login(user) {
                bail!("VOUCHED.td:{line_no}: invalid trust entry: {t}");
            }
            trusted.insert(user.to_ascii_lowercase());
        } else {
            bail!("VOUCHED.td:{line_no}: expected github:user or -github:user, got: {t}");
        }
    }
    // Denounced wins if both present.
    for u in &denounced {
        trusted.remove(u);
    }
    Ok(VouchedFile { trusted, denounced })
}

fn is_github_login(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 39
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '[' || c == ']' || c == '_')
}

pub fn label_for(status: &str) -> &'static str {
    match status {
        "denounced" => "vouch:denounced",
        "bot" | "collaborator" | "vouched" => "vouch:trusted",
        _ => "vouch:unvouched",
    }
}

pub struct VouchedParseArgs {
    pub path: PathBuf,
}

pub fn run(root: &Path, args: VouchedParseArgs) -> Result<()> {
    let path = if args.path.is_absolute() {
        args.path.clone()
    } else {
        root.join(&args.path)
    };
    let raw = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let parsed = parse(&raw)?;
    println!(
        "vouched-parse: ok trusted={} denounced={}",
        parsed.trusted.len(),
        parsed.denounced.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_trust_and_denounce() {
        let f = parse("github:alice\n-github:bob spam\n# c\ngithub:carol\n").unwrap();
        assert!(f.trusted.contains("alice"));
        assert!(f.trusted.contains("carol"));
        assert!(f.denounced.contains("bob"));
        assert!(!f.trusted.contains("bob"));
    }

    #[test]
    fn fails_closed_on_garbage() {
        assert!(parse("not-a-line\n").is_err());
        assert!(parse("github:\n").is_err());
    }

    #[test]
    fn label_transitions() {
        assert_eq!(label_for("vouched"), "vouch:trusted");
        assert_eq!(label_for("denounced"), "vouch:denounced");
        assert_eq!(label_for("unknown"), "vouch:unvouched");
    }

    #[test]
    fn denounce_overrides_trust() {
        let f = parse("github:eve\n-github:eve bad\n").unwrap();
        assert!(f.denounced.contains("eve"));
        assert!(!f.trusted.contains("eve"));
    }
}
