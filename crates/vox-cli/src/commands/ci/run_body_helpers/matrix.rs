use anyhow::{Context, Result, anyhow};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

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
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("target"));
    base.join("nested-ci")
}

/// Options for `vox ci mens-gate` isolated runner (temp `vox` copy).
#[derive(Clone, Default, Debug)]
pub(crate) struct MensGateOpts {
    pub isolated_runner: bool,
    pub gate_build_target_dir: Option<PathBuf>,
    pub gate_log_file: Option<PathBuf>,
}

const VOX_MENS_GATE_INNER: &str = "VOX_MENS_GATE_INNER";

struct TempGateExe(PathBuf);

impl Drop for TempGateExe {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Run Mens profiles from `gates.yaml`. When `VOX_MENS_GATE_INNER=1`, always runs steps in-process.
pub(crate) fn run_mens_gate(root: &Path, profile: &str, opts: &MensGateOpts) -> Result<()> {
    if env::var(VOX_MENS_GATE_INNER).ok().as_deref() == Some("1") {
        return run_mens_gate_steps(root, profile);
    }

    if opts.isolated_runner {
        return run_mens_gate_isolated(root, profile, opts);
    }

    run_mens_gate_steps(root, profile)
}

fn run_mens_gate_isolated(root: &Path, profile: &str, opts: &MensGateOpts) -> Result<()> {
    #[cfg(windows)]
    {
        return run_mens_gate_windows_isolated(root, profile, opts);
    }
    #[cfg(unix)]
    {
        return run_mens_gate_unix_isolated(root, profile, opts);
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = (root, profile, opts);
        Err(anyhow!(
            "--isolated-runner is only supported on Windows and Unix targets"
        ))
    }
}

#[cfg(windows)]
fn run_mens_gate_windows_isolated(root: &Path, profile: &str, opts: &MensGateOpts) -> Result<()> {
    let cargo = cargo_bin();
    let target_dir = opts
        .gate_build_target_dir
        .clone()
        .unwrap_or_else(|| root.join("target").join("mens-gate-safe"));

    eprintln!(
        ">> isolated mens-gate: cargo build -p vox-cli --target-dir {}",
        target_dir.display()
    );
    let st = Command::new(&cargo)
        .current_dir(root)
        .args(["build", "-p", "vox-cli", "--target-dir"])
        .arg(&target_dir)
        .status()?;
    if !st.success() {
        return Err(anyhow!("isolated cargo build -p vox-cli failed"));
    }

    let built = target_dir.join("debug").join("vox.exe");
    if !built.is_file() {
        return Err(anyhow!(
            "expected gate runner at {} (debug build)",
            built.display()
        ));
    }

    let tmp = env::temp_dir().join(format!(
        "vox-gate-{}.exe",
        uuid::Uuid::new_v4().simple()
    ));
    fs::copy(&built, &tmp).with_context(|| {
        format!(
            "copy runner {} -> {}",
            built.display(),
            tmp.display()
        )
    })?;
    let _rm_temp = TempGateExe(tmp.clone());

    if let Some(ref log_path) = opts.gate_log_file {
        if let Some(parent) = log_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("mkdir {}", parent.display()))?;
            }
        }

        let mut child = Command::new(&tmp)
            .current_dir(root)
            .env(VOX_MENS_GATE_INNER, "1")
            .args(["ci", "mesh-gate", "--profile", profile])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn isolated mens-gate")?;

        let stdout = child.stdout.take().context("isolated mens-gate stdout")?;
        let stderr = child.stderr.take().context("isolated mens-gate stderr")?;

        let log = Arc::new(Mutex::new(
            fs::File::create(log_path)
                .with_context(|| format!("create log {}", log_path.display()))?,
        ));

        let log_out = Arc::clone(&log);
        let h_out = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                println!("{line}");
                if let Ok(mut w) = log_out.lock() {
                    let _ = writeln!(w, "{line}");
                    let _ = w.flush();
                }
            }
        });

        let log_err = Arc::clone(&log);
        let h_err = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                eprintln!("{line}");
                if let Ok(mut w) = log_err.lock() {
                    let _ = writeln!(w, "{line}");
                    let _ = w.flush();
                }
            }
        });

        let st = child.wait().context("wait isolated mens-gate")?;
        let _ = h_out.join();
        let _ = h_err.join();
        if !st.success() {
            return Err(anyhow!("mens-gate (isolated runner) failed: {st}"));
        }
    } else {
        let st = Command::new(&tmp)
            .current_dir(root)
            .env(VOX_MENS_GATE_INNER, "1")
            .args(["ci", "mesh-gate", "--profile", profile])
            .status()
            .context("run isolated mens-gate")?;
        if !st.success() {
            return Err(anyhow!("mens-gate (isolated runner) failed: {st}"));
        }
    }

    Ok(())
}

#[cfg(unix)]
fn run_mens_gate_unix_isolated(root: &Path, profile: &str, opts: &MensGateOpts) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let cargo = cargo_bin();
    let target_dir = opts
        .gate_build_target_dir
        .clone()
        .unwrap_or_else(|| root.join("target").join("mens-gate-safe"));

    eprintln!(
        ">> isolated mens-gate: cargo build -p vox-cli --target-dir {}",
        target_dir.display()
    );
    let st = Command::new(&cargo)
        .current_dir(root)
        .args(["build", "-p", "vox-cli", "--target-dir"])
        .arg(&target_dir)
        .status()?;
    if !st.success() {
        return Err(anyhow!("isolated cargo build -p vox-cli failed"));
    }

    let built = target_dir.join("debug").join("vox");
    if !built.is_file() {
        return Err(anyhow!(
            "expected gate runner at {} (debug build)",
            built.display()
        ));
    }

    let tmp = env::temp_dir().join(format!("vox-gate-{}", uuid::Uuid::new_v4().simple()));
    fs::copy(&built, &tmp).with_context(|| {
        format!(
            "copy runner {} -> {}",
            built.display(),
            tmp.display()
        )
    })?;
    let mut perms = fs::metadata(&tmp)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&tmp, perms)?;
    let _rm_temp = TempGateExe(tmp.clone());

    if let Some(ref log_path) = opts.gate_log_file {
        if let Some(parent) = log_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("mkdir {}", parent.display()))?;
            }
        }

        let mut child = Command::new(&tmp)
            .current_dir(root)
            .env(VOX_MENS_GATE_INNER, "1")
            .args(["ci", "mesh-gate", "--profile", profile])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn isolated mens-gate")?;

        let stdout = child.stdout.take().context("isolated mens-gate stdout")?;
        let stderr = child.stderr.take().context("isolated mens-gate stderr")?;

        let log = Arc::new(Mutex::new(
            fs::File::create(log_path)
                .with_context(|| format!("create log {}", log_path.display()))?,
        ));

        let log_out = Arc::clone(&log);
        let h_out = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                println!("{line}");
                if let Ok(mut w) = log_out.lock() {
                    let _ = writeln!(w, "{line}");
                    let _ = w.flush();
                }
            }
        });

        let log_err = Arc::clone(&log);
        let h_err = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                eprintln!("{line}");
                if let Ok(mut w) = log_err.lock() {
                    let _ = writeln!(w, "{line}");
                    let _ = w.flush();
                }
            }
        });

        let st = child.wait().context("wait isolated mens-gate")?;
        let _ = h_out.join();
        let _ = h_err.join();
        if !st.success() {
            return Err(anyhow!("mens-gate (isolated runner) failed: {st}"));
        }
    } else {
        let st = Command::new(&tmp)
            .current_dir(root)
            .env(VOX_MENS_GATE_INNER, "1")
            .args(["ci", "mesh-gate", "--profile", profile])
            .status()
            .context("run isolated mens-gate")?;
        if !st.success() {
            return Err(anyhow!("mens-gate (isolated runner) failed: {st}"));
        }
    }

    Ok(())
}

fn run_mens_gate_steps(root: &Path, profile: &str) -> Result<()> {
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

pub(crate) fn run_toestub_self_apply(repo: &Path) -> Result<()> {
    let cargo = cargo_bin();
    let st = Command::new(&cargo)
        .current_dir(repo)
        .args(["build", "-p", "vox-toestub", "--release"])
        .status()?;
    if !st.success() {
        return Err(anyhow!("cargo build -p vox-toestub --release failed"));
    }
    let st = Command::new(&cargo)
        .current_dir(repo)
        .args(["run", "-q", "-p", "vox-toestub", "--bin", "toestub"])
        .status()?;
    if !st.success() {
        return Err(anyhow!("toestub self-apply run failed"));
    }
    println!("TOESTUB self-apply OK");
    Ok(())
}

pub(crate) fn run_toestub_scoped(repo: &Path, scan_root: &Path, mode: ToestubCiMode) -> Result<()> {
    let root: PathBuf = if scan_root.is_absolute() {
        scan_root.to_path_buf()
    } else {
        repo.join(scan_root)
    };
    let cargo = cargo_bin();
    let nested_target = nested_cargo_target_dir(repo);
    let mut c = Command::new(&cargo);
    c.current_dir(repo)
        .env("CARGO_TARGET_DIR", &nested_target)
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
    let nested_target = nested_cargo_target_dir(root);
    for f in FEATURE_SETS {
        if f.is_empty() {
            eprintln!("==> cargo check -p vox-cli (default features)");
            let st = Command::new(&cargo)
                .current_dir(root)
                .env("CARGO_TARGET_DIR", &nested_target)
                .args(["check", "-p", "vox-cli"])
                .status()?;
            if !st.success() {
                return Err(anyhow!("cargo check -p vox-cli failed"));
            }
        } else {
            eprintln!("==> cargo check -p vox-cli --features {f}");
            let st = Command::new(&cargo)
                .current_dir(root)
                .env("CARGO_TARGET_DIR", &nested_target)
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
mod tests;
