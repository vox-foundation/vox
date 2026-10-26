//! `vox ci preview-trust-audit` — flag unsafe privileged-preview workflow shapes.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_ALLOWLIST: &str = "contracts/ci/preview-trust-allowlist.v1.yaml";

#[derive(Debug, Deserialize)]
struct AllowFile {
    schema_version: u32,
    #[serde(default)]
    allowlist: Vec<AllowEntry>,
}

#[derive(Debug, Deserialize)]
struct AllowEntry {
    workflow: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    reason: String,
}

fn looks_risky(yml: &str) -> bool {
    let lower = yml.to_ascii_lowercase();
    let pr_target = lower.contains("pull_request_target");
    let pr_head = lower.contains("pull_request.head")
        || lower.contains("github.event.pull_request.head")
        || lower.contains("refs/pull/");
    let secrets = lower.contains("secrets.") || lower.contains("${{ secrets");
    pr_target && pr_head && secrets
}

pub struct PreviewTrustArgs {
    pub allowlist: PathBuf,
    pub workflows_dir: PathBuf,
}

pub fn run(root: &Path, args: PreviewTrustArgs) -> Result<()> {
    let allow_path = if args.allowlist.is_absolute() {
        args.allowlist.clone()
    } else {
        root.join(&args.allowlist)
    };
    let raw = std::fs::read_to_string(&allow_path)
        .with_context(|| format!("read {}", allow_path.display()))?;
    let allow: AllowFile = serde_yaml::from_str(&raw)?;
    if allow.schema_version != 1 {
        bail!("unsupported schema_version {}", allow.schema_version);
    }
    for e in &allow.allowlist {
        if e.workflow.trim().is_empty() {
            bail!("preview-trust allowlist entry missing workflow");
        }
        if e.owner.trim().is_empty() || e.reason.trim().is_empty() {
            bail!(
                "preview-trust allowlist entry for '{}' requires non-empty owner and reason",
                e.workflow
            );
        }
    }
    let allowed: std::collections::BTreeSet<_> =
        allow.allowlist.iter().map(|e| e.workflow.clone()).collect();

    let dir = if args.workflows_dir.is_absolute() {
        args.workflows_dir.clone()
    } else {
        root.join(&args.workflows_dir)
    };
    let mut violations = Vec::new();
    for entry in std::fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yml")
            && path.extension().and_then(|s| s.to_str()) != Some("yaml")
        {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let body = std::fs::read_to_string(&path)?;
        if looks_risky(&body) && !allowed.contains(&name) {
            violations.push(name);
        }
    }
    if !violations.is_empty() {
        eprintln!("preview-trust-audit: risky workflow(s):");
        for v in &violations {
            eprintln!("  - {v}");
        }
        bail!(
            "privileged pull_request_target + PR head checkout + secrets without allowlist — see {}",
            DEFAULT_ALLOWLIST
        );
    }
    println!("preview-trust-audit: ok");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_risky_combo() {
        let yml = r#"
on: pull_request_target
jobs:
  x:
    steps:
      - uses: actions/checkout@v7
        with:
          ref: ${{ github.event.pull_request.head.sha }}
      - run: echo ${{ secrets.FOO }}
"#;
        assert!(looks_risky(yml));
    }

    #[test]
    fn cleanup_shape_ok() {
        let yml = r#"
on: pull_request_target
jobs:
  x:
    steps:
      - uses: actions/checkout@v7
        with:
          ref: ${{ github.event.repository.default_branch }}
      - run: echo ${{ secrets.FOO }}
"#;
        assert!(!looks_risky(yml));
    }
}
