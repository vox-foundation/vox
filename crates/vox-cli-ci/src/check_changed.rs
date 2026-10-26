//! `vox ci check-changed` — t3code `check:changed` analog over Vox path owners.
//!
//! Loads `contracts/ci/changed-path-owners.v1.yaml`, classifies a changed-file
//! set as inert (no cargo) or owned (emit gate plan). Never invents a second
//! affected-crate engine — pair with `vox ci affected-crates` for crate closure.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const DEFAULT_OWNERS: &str = "contracts/ci/changed-path-owners.v1.yaml";

#[derive(Debug, Clone, Deserialize)]
pub struct PathOwnersFile {
    pub schema_version: u32,
    #[serde(default)]
    pub inert_prefixes: Vec<String>,
    #[serde(default)]
    pub owners: Vec<OwnerRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OwnerRule {
    pub glob: String,
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default)]
    pub gates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePlan {
    pub inert: bool,
    pub owners: BTreeSet<String>,
    pub gates: BTreeSet<String>,
    pub files: Vec<String>,
}

pub fn load_owners(path: &Path) -> Result<PathOwnersFile> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read path-owners contract {}", path.display()))?;
    let parsed: PathOwnersFile =
        serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    if parsed.schema_version != 1 {
        bail!(
            "unsupported schema_version {} in {}",
            parsed.schema_version,
            path.display()
        );
    }
    Ok(parsed)
}

fn normalize(p: &str) -> String {
    p.replace('\\', "/")
}

fn is_inert(file: &str, prefixes: &[String]) -> bool {
    let f = normalize(file);
    prefixes.iter().any(|pre| {
        let pre = normalize(pre);
        f.starts_with(&pre)
    })
}

/// Simple glob: `**` any depth, `*` one segment (no `/`).
fn glob_match(pattern: &str, path: &str) -> bool {
    let pat = normalize(pattern);
    let path = normalize(path);
    if pat == "**/*" || pat == "**" {
        return true;
    }
    // Exact prefix forms ending with /**
    if let Some(prefix) = pat.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if !pat.contains('*') {
        return path == pat;
    }
    // Segment-wise match
    let p_segs: Vec<&str> = pat.split('/').collect();
    let f_segs: Vec<&str> = path.split('/').collect();
    match_segs(&p_segs, &f_segs)
}

fn match_segs(pat: &[&str], file: &[&str]) -> bool {
    if pat.is_empty() {
        return file.is_empty();
    }
    match pat[0] {
        "**" => {
            if match_segs(&pat[1..], file) {
                return true;
            }
            if file.is_empty() {
                return false;
            }
            match_segs(pat, &file[1..])
        }
        "*" => {
            if file.is_empty() {
                return false;
            }
            match_segs(&pat[1..], &file[1..])
        }
        lit => {
            if file.first() != Some(&lit) {
                return false;
            }
            match_segs(&pat[1..], &file[1..])
        }
    }
}

pub fn plan_for_files(owners: &PathOwnersFile, files: &[String]) -> ChangePlan {
    let files: Vec<String> = files.iter().map(|f| normalize(f)).collect();
    if files.is_empty() {
        return ChangePlan {
            inert: true,
            owners: BTreeSet::new(),
            gates: BTreeSet::new(),
            files,
        };
    }
    let all_inert = files
        .iter()
        .all(|f| is_inert(f, &owners.inert_prefixes));
    if all_inert {
        return ChangePlan {
            inert: true,
            owners: BTreeSet::new(),
            gates: BTreeSet::new(),
            files,
        };
    }
    let mut owner_set = BTreeSet::new();
    let mut gate_set = BTreeSet::new();
    for f in &files {
        if is_inert(f, &owners.inert_prefixes) {
            continue;
        }
        for rule in &owners.owners {
            if glob_match(&rule.glob, f) {
                owner_set.extend(rule.owners.iter().cloned());
                gate_set.extend(rule.gates.iter().cloned());
                break;
            }
        }
    }
    ChangePlan {
        inert: false,
        owners: owner_set,
        gates: gate_set,
        files,
    }
}

fn git_changed_files(root: &Path, base_ref: &str) -> Result<Vec<String>> {
    let mb = Command::new("git")
        .args(["-c", "core.autocrlf=false", "merge-base", "HEAD", base_ref])
        .current_dir(root)
        .output()
        .context("git merge-base")?;
    if !mb.status.success() {
        bail!(
            "check-changed: no merge base with {base_ref}. Fetch the remote or pass --base."
        );
    }
    let base = String::from_utf8_lossy(&mb.stdout).trim().to_string();
    let out = Command::new("git")
        .args([
            "-c",
            "core.autocrlf=false",
            "diff",
            "--name-only",
            "--no-renames",
            &base,
            "--",
        ])
        .current_dir(root)
        .output()
        .context("git diff --name-only")?;
    if !out.status.success() {
        bail!("git diff failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let mut files: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect();
    // Untracked
    let untracked = Command::new("git")
        .args([
            "-c",
            "core.autocrlf=false",
            "ls-files",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output()
        .context("git ls-files")?;
    if untracked.status.success() {
        for line in String::from_utf8_lossy(&untracked.stdout).lines() {
            if !line.is_empty() {
                files.push(line.to_string());
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn read_changed_list(path: &Path) -> Result<Vec<String>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read --changed {}", path.display()))?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect())
}

pub struct CheckChangedArgs {
    pub base: String,
    pub dry_run: bool,
    pub changed: Option<PathBuf>,
    pub owners_file: Option<PathBuf>,
    pub github_output: Option<PathBuf>,
}

/// Run `vox ci check-changed`. Returns Ok when inert or plan printed; never
/// spawns cargo on inert-only diffs (enforced by not calling cargo here at all
/// — non-inert dry-run only prints the plan; non-inert non-dry-run still only
/// plans in v5.4 Phase 301 — execution of gates is left to pre-push/CI).
pub fn run(root: &Path, args: CheckChangedArgs) -> Result<()> {
    let owners_path = args
        .owners_file
        .unwrap_or_else(|| root.join(DEFAULT_OWNERS));
    let owners = load_owners(&owners_path)?;
    let files = if let Some(list) = args.changed.as_ref() {
        read_changed_list(list)?
    } else {
        git_changed_files(root, &args.base)?
    };
    let plan = plan_for_files(&owners, &files);
    println!(
        "check-changed: {} file(s) vs {}",
        plan.files.len(),
        if args.changed.is_some() {
            "--changed".to_string()
        } else {
            args.base.clone()
        }
    );
    if plan.inert {
        println!("check-changed: inert — no cargo / playwright invocations required");
        write_github_output(args.github_output.as_deref(), true, &plan)?;
        return Ok(());
    }
    println!(
        "check-changed: owners=[{}]",
        plan.owners.iter().cloned().collect::<Vec<_>>().join(", ")
    );
    if !plan.gates.is_empty() {
        println!(
            "check-changed: gates=[{}]",
            plan.gates.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }
    if args.dry_run {
        println!("check-changed: dry-run — plan only (no cargo)");
    } else {
        println!(
            "check-changed: non-inert — run `vox ci pre-push` / affected-crates for execution; this command does not spawn cargo"
        );
    }
    write_github_output(args.github_output.as_deref(), false, &plan)?;
    Ok(())
}

fn write_github_output(path: Option<&Path>, inert: bool, plan: &ChangePlan) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let owners = plan.owners.iter().cloned().collect::<Vec<_>>().join(",");
    let body = format!(
        "inert={}\nowners={}\nfile_count={}\n",
        if inert { "true" } else { "false" },
        owners,
        plan.files.len()
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_owners() -> PathOwnersFile {
        PathOwnersFile {
            schema_version: 1,
            inert_prefixes: vec![
                ".planning/".into(),
                "docs/src/archive/".into(),
                "crates/vox-gui/ui/review-bundle/".into(),
            ],
            owners: vec![
                OwnerRule {
                    glob: "crates/vox-gui/**".into(),
                    owners: vec!["gui".into()],
                    gates: vec!["gui-coverage-gate".into()],
                },
                OwnerRule {
                    glob: "crates/**".into(),
                    owners: vec!["rust".into()],
                    gates: vec!["fmt".into()],
                },
                OwnerRule {
                    glob: "contracts/**".into(),
                    owners: vec!["contracts".into()],
                    gates: vec!["ssot-drift".into()],
                },
                OwnerRule {
                    glob: "**/*".into(),
                    owners: vec!["misc".into()],
                    gates: vec![],
                },
            ],
        }
    }

    #[test]
    fn inert_only_planning_paths_need_no_cargo() {
        let o = sample_owners();
        let plan = plan_for_files(
            &o,
            &[
                ".planning/STATE.md".into(),
                ".planning/ROADMAP.md".into(),
                "docs/src/archive/research-2026-q1/foo.md".into(),
            ],
        );
        assert!(plan.inert, "planning+archive must be inert");
        assert!(plan.owners.is_empty());
        assert!(plan.gates.is_empty());
    }

    #[test]
    fn rust_crate_change_is_not_inert() {
        let o = sample_owners();
        let plan = plan_for_files(&o, &["crates/vox-cli/src/lib.rs".into()]);
        assert!(!plan.inert);
        assert!(plan.owners.contains("rust"));
        assert!(plan.gates.contains("fmt"));
    }

    #[test]
    fn mixed_inert_and_rust_is_not_inert() {
        let o = sample_owners();
        let plan = plan_for_files(
            &o,
            &[
                ".planning/STATE.md".into(),
                "crates/vox-db/src/lib.rs".into(),
            ],
        );
        assert!(!plan.inert);
        assert!(plan.owners.contains("rust"));
    }

    #[test]
    fn gui_glob_beats_crates_glob() {
        let o = sample_owners();
        let plan = plan_for_files(&o, &["crates/vox-gui/ui/src/App.tsx".into()]);
        assert!(!plan.inert);
        assert!(plan.owners.contains("gui"));
        assert!(!plan.owners.contains("rust"));
    }

    #[test]
    fn committed_owners_contract_loads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let path = root.join(DEFAULT_OWNERS);
        let loaded = load_owners(&path).expect("load committed contract");
        assert_eq!(loaded.schema_version, 1);
        assert!(!loaded.inert_prefixes.is_empty());
        assert!(!loaded.owners.is_empty());
        // Fixture: inert-only should stay inert against the real contract
        let plan = plan_for_files(
            &loaded,
            &[".planning/REQUIREMENTS.md".into(), ".planning/phases/x.md".into()],
        );
        assert!(plan.inert);
    }

    #[test]
    fn glob_double_star_prefix() {
        assert!(glob_match("crates/**", "crates/vox-cli/src/a.rs"));
        assert!(glob_match("crates/vox-gui/**", "crates/vox-gui/ui/x.ts"));
        assert!(!glob_match("crates/vox-gui/**", "crates/vox-cli/src/a.rs"));
    }
}
