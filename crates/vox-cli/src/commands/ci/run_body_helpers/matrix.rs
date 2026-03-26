use anyhow::{Context, Result, anyhow};
use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::ci::bounded_read::read_utf8_path_capped;
use crate::commands::ci::cargo_bin;
use crate::commands::ci::cmd_enums::ToestubCiMode;
use crate::commands::ci::constants::FEATURE_SETS;

pub(crate) fn visit_rs_files(dir: &Path, f: &mut impl FnMut(&Path) -> Result<()>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        let t = entry.file_type()?;
        if t.is_dir() {
            visit_rs_files(&p, f)?;
        } else if t.is_file() && p.extension().and_then(|x| x.to_str()) == Some("rs") {
            f(&p)?;
        }
    }
    Ok(())
}

pub(crate) fn check_no_vox_dei(root: &Path) -> Result<()> {
    let src = root.join("crates/vox-cli/src");
    let re = regex::Regex::new(r"\bvox_dei::")?;
    visit_rs_files(&src, &mut |p: &Path| {
        let text = read_utf8_path_capped(p)?;
        if re.is_match(&text) {
            return Err(anyhow!(
                "vox-cli must not reference vox_dei:: (crate is workspace-excluded). Offender: {}",
                p.display()
            ));
        }
        Ok(())
    })?;
    println!("vox-cli no-vox_dei guard OK");
    Ok(())
}

pub(crate) fn check_workflow_scripts(root: &Path, allowlist_path: &Path) -> Result<()> {
    let allow_path = root.join(allowlist_path);
    let allowed: std::collections::HashSet<String> = if allow_path.is_file() {
        read_utf8_path_capped(&allow_path)?
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    } else {
        return Err(anyhow!("missing allowlist: {}", allow_path.display()));
    };

    let wf_dir = root.join(".github/workflows");
    let re = regex::Regex::new(r"scripts/[A-Za-z0-9_./-]+")?;
    let mut violations = Vec::new();
    for entry in fs::read_dir(&wf_dir).with_context(|| format!("read {}", wf_dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        if p.extension().and_then(|x| x.to_str()) != Some("yml")
            && p.extension().and_then(|x| x.to_str()) != Some("yaml")
        {
            continue;
        }
        let text = read_utf8_path_capped(&p)?;
        for cap in re.find_iter(&text) {
            let path = cap.as_str().to_string();
            if !allowed.contains(&path) {
                violations.push(format!("{}: {}", p.display(), path));
            }
        }
    }
    if !violations.is_empty() {
        return Err(anyhow!(
            "workflow references scripts/ not in allowlist:\n{}",
            violations.join("\n")
        ));
    }
    println!("workflow-scripts allowlist OK");
    Ok(())
}

fn resolve_mens_gate_manifest_path(root: &Path) -> PathBuf {
    let canonical = root.join("scripts/populi/gates.yaml");
    if canonical.is_file() {
        canonical
    } else {
        // Back-compat fallback for older repos/worktrees.
        root.join("scripts/mens/gates.yaml")
    }
}

fn nested_cargo_target_dir(root: &Path) -> PathBuf {
    let base = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|p| {
            if p.is_absolute() { p } else { root.join(p) }
        })
        .unwrap_or_else(|| root.join("target"));
    base.join("nested-ci")
}

pub(crate) fn run_mens_gate(root: &Path, profile: &str) -> Result<()> {
    let manifest_path = resolve_mens_gate_manifest_path(root);
    let raw = read_utf8_path_capped(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&raw)?;
    let profiles = doc
        .get("profiles")
        .and_then(|p| p.as_mapping())
        .ok_or_else(|| anyhow!("gates.yaml: missing profiles"))?;
    let prof = profiles
        .get(serde_yaml::Value::String(profile.to_string()))
        .ok_or_else(|| anyhow!("unknown profile: {profile}"))?;
    let steps = prof
        .get("steps")
        .and_then(|s| s.as_sequence())
        .ok_or_else(|| anyhow!("profile {profile}: missing steps"))?;

    let cargo = cargo_bin();
    let nested_target = nested_cargo_target_dir(root);
    for step in steps {
        let cmd = step
            .get("command")
            .and_then(|c| c.as_str())
            .unwrap_or("cargo");
        let args = step
            .get("args")
            .and_then(|a| a.as_sequence())
            .ok_or_else(|| anyhow!("step missing args"))?;
        let arg_strs: Vec<String> = args
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        eprintln!(">> {cmd} {}", arg_strs.join(" "));
        let st = if cmd == "cargo" {
            let mut child = Command::new(&cargo);
            child
                .current_dir(root)
                .env("CARGO_TARGET_DIR", &nested_target)
                .args(&arg_strs);
            child.status()?
        } else {
            Command::new(cmd)
                .current_dir(root)
                .args(&arg_strs)
                .status()?
        };
        if !st.success() {
            return Err(anyhow!("mens-gate step failed: {cmd} {:?}", arg_strs));
        }
    }
    println!("Mens gate OK ({profile})");
    Ok(())
}

pub(crate) fn run_toestub_scoped(repo: &Path, scan_root: &Path, mode: ToestubCiMode) -> Result<()> {
    let root: PathBuf = if scan_root.is_absolute() {
        scan_root.to_path_buf()
    } else {
        repo.join(scan_root)
    };
    let cargo = cargo_bin();
    let mut c = Command::new(&cargo);
    c.current_dir(repo)
        .args(["run", "-p", "vox-toestub", "--bin", "toestub", "--"]);
    if mode != ToestubCiMode::Legacy {
        c.arg("--mode").arg(mode.as_cli_str());
    }
    c.arg(root.to_string_lossy().as_ref());
    let st = c.status()?;
    if !st.success() {
        return Err(anyhow!("toestub scoped run failed"));
    }
    Ok(())
}

pub(crate) fn run_feature_matrix(root: &Path) -> Result<()> {
    let cargo = cargo_bin();
    for f in FEATURE_SETS {
        if f.is_empty() {
            eprintln!("==> cargo check -p vox-cli (default features)");
            let st = Command::new(&cargo)
                .current_dir(root)
                .args(["check", "-p", "vox-cli"])
                .status()?;
            if !st.success() {
                return Err(anyhow!("cargo check -p vox-cli failed"));
            }
        } else {
            eprintln!("==> cargo check -p vox-cli --features {f}");
            let st = Command::new(&cargo)
                .current_dir(root)
                .args(["check", "-p", "vox-cli", "--features", f])
                .status()?;
            if !st.success() {
                return Err(anyhow!("cargo check -p vox-cli --features {f} failed"));
            }
        }
    }
    println!("vox-cli feature matrix OK");
    Ok(())
}

#[cfg(test)]
mod feature_matrix_contract_tests {
    use std::path::{Path, PathBuf};

    use crate::commands::ci::constants::FEATURE_SETS;
    use super::{nested_cargo_target_dir, resolve_mens_gate_manifest_path};

    #[test]
    fn feature_sets_include_script_execution_lane() {
        assert!(
            FEATURE_SETS.contains(&"script-execution"),
            "CI feature matrix must compile the script-execution lane"
        );
        assert!(
            FEATURE_SETS.contains(&"script-execution,stub-check"),
            "CI feature matrix must include a mixed script-execution + stub-check build"
        );
    }

    #[test]
    fn feature_sets_include_populi_oratio_lane() {
        assert!(
            FEATURE_SETS.contains(&"oratio"),
            "CI feature matrix must compile the oratio (Oratio STT) lane"
        );
    }

    #[test]
    fn canonical_mens_gate_manifest_exists_in_repo() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let resolved = resolve_mens_gate_manifest_path(&root);
        assert!(
            resolved.ends_with(PathBuf::from("scripts/populi/gates.yaml")),
            "expected canonical mens gate manifest path to resolve first, got {}",
            resolved.display()
        );
        assert!(resolved.is_file(), "missing gate manifest: {}", resolved.display());
    }

    #[test]
    fn mens_gate_manifest_resolution_uses_legacy_fallback() {
        let td = tempfile::tempdir().expect("tempdir");
        let root = td.path();
        std::fs::create_dir_all(root.join("scripts/mens")).expect("mkdir scripts/mens");
        std::fs::write(root.join("scripts/mens/gates.yaml"), "profiles: {}\n")
            .expect("write legacy gates");
        let resolved = resolve_mens_gate_manifest_path(root);
        assert!(
            resolved.ends_with(PathBuf::from("scripts/mens/gates.yaml")),
            "expected legacy fallback, got {}",
            resolved.display()
        );
    }

    #[test]
    fn nested_cargo_target_uses_nested_ci_suffix() {
        let td = tempfile::tempdir().expect("tempdir");
        let root = td.path();
        let nested = nested_cargo_target_dir(root);
        assert!(
            nested.ends_with(PathBuf::from("target/nested-ci")),
            "unexpected nested target path: {}",
            nested.display()
        );
    }
}
