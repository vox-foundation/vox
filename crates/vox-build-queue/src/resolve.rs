//! Cargo binary, worktree, target-dir, `sccache`, and process-tree helpers.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Subcommands whose runs take the cargo target lock and are worth queueing.
pub fn is_build_subcommand(sub: &str) -> bool {
    matches!(sub, "build" | "test" | "check" | "clippy" | "run" | "bench")
}

/// Walk up from `start` to the first directory containing BOTH `.cargo/config.toml` and `Cargo.toml`.
pub fn worktree_root_of(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|d| d.join(".cargo/config.toml").is_file() && d.join("Cargo.toml").is_file())
        .map(Path::to_path_buf)
}

/// The rustup cargo proxy under `$CARGO_HOME`/`~/.cargo/bin`, if present.
pub fn cargo_home_proxy() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".cargo")))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))?;
    let p = home.join("bin").join(exe);
    p.is_file().then_some(p)
}

/// Resolve real cargo by scanning `path_var`, skipping `own_exe` and same-directory siblings.
pub fn resolve_real_cargo(path_var: &str, own_exe: &Path) -> Option<PathBuf> {
    let own = own_exe.canonicalize().ok()?;
    let own_dir = own.parent().map(Path::to_path_buf);
    let exe = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    for dir in std::env::split_paths(path_var) {
        let cand = dir.join(exe);
        let Ok(canon) = cand.canonicalize() else {
            continue;
        };
        if !cand.is_file() || canon == own || canon.parent().map(Path::to_path_buf) == own_dir {
            continue;
        }
        return Some(canon);
    }
    None
}

pub fn build_args_for_run(args: &[String]) -> Option<Vec<String>> {
    if args.first().map(String::as_str) != Some("run") {
        return None;
    }
    let mut out = vec!["build".to_string()];
    out.extend(args[1..].iter().take_while(|a| a.as_str() != "--").cloned());
    Some(out)
}

fn strip_verbatim(p: PathBuf) -> PathBuf {
    let Some(s) = p.to_str() else { return p };
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    s.strip_prefix(r"\\?\").map(PathBuf::from).unwrap_or(p)
}

fn lexical_clean(p: &Path) -> PathBuf {
    use std::path::Component;
    let (mut out, mut depth) = (PathBuf::new(), 0usize);
    for comp in p.components() {
        match comp {
            Component::Prefix(pref) => out.push(pref.as_os_str()),
            Component::RootDir => out.push(comp.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir if depth > 0 => {
                out.pop();
                depth -= 1;
            }
            Component::ParentDir if !out.has_root() => out.push(".."),
            Component::ParentDir => {}
            Component::Normal(c) => {
                out.push(c);
                depth += 1;
            }
        }
    }
    out
}

/// Normalize `raw` (against `cwd` if relative) via deepest existing ancestor + verbatim strip.
pub fn normalize_target_path(cwd: &Path, raw: &Path) -> PathBuf {
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        cwd.join(raw)
    };
    let cleaned = lexical_clean(&joined);
    for anc in cleaned.ancestors() {
        if anc.as_os_str().is_empty() {
            break;
        }
        if let Ok(canon) = anc.canonicalize() {
            let base = strip_verbatim(canon);
            return match cleaned.strip_prefix(anc) {
                Ok(rel) if !rel.as_os_str().is_empty() => base.join(rel),
                _ => base,
            };
        }
    }
    strip_verbatim(cleaned)
}

fn flag_value_before_dashdash<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let pre: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .take_while(|a| *a != "--")
        .collect();
    let eq_prefix = format!("{flag}=");
    let mut i = 0;
    while i < pre.len() {
        if pre[i] == flag {
            if let Some(val) = pre.get(i + 1).copied().filter(|v| !v.is_empty()) {
                return Some(val);
            }
            i += 2;
        } else if let Some(val) = pre[i].strip_prefix(&eq_prefix).filter(|v| !v.is_empty()) {
            return Some(val);
        } else {
            i += 1;
        }
    }
    None
}

/// Resolve the canonical `CARGO_TARGET_DIR` Cargo will lock for `(cwd, args, child_env)`.
pub fn effective_target_dir(
    cwd: &Path,
    args: &[String],
    child_env: &BTreeMap<String, String>,
) -> PathBuf {
    if let Some(td) = flag_value_before_dashdash(args, "--target-dir") {
        return normalize_target_path(cwd, Path::new(td));
    }
    if let Some(td) = child_env.get("CARGO_TARGET_DIR").filter(|v| !v.is_empty()) {
        return normalize_target_path(cwd, Path::new(td));
    }
    if let Some(mp) = flag_value_before_dashdash(args, "--manifest-path") {
        let manifest_abs = normalize_target_path(cwd, Path::new(mp));
        let manifest_dir = manifest_abs.parent().unwrap_or(cwd);
        if let Some(wt) = worktree_root_of(manifest_dir) {
            return normalize_target_path(&wt, &wt.join("target"));
        }
        if worktree_root_of(cwd).is_none() {
            let ws_root = manifest_dir
                .ancestors()
                .find(|d| {
                    std::fs::read_to_string(d.join("Cargo.toml"))
                        .is_ok_and(|t| t.contains("[workspace]"))
                })
                .unwrap_or(manifest_dir);
            return normalize_target_path(ws_root, &ws_root.join("target"));
        }
    }
    let base = worktree_root_of(cwd).unwrap_or_else(|| cwd.to_path_buf());
    normalize_target_path(&base, &base.join("target"))
}

fn is_truthy(val: &str) -> bool {
    matches!(
        val.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn is_falsy(val: &str) -> bool {
    matches!(
        val.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

/// Whether `vox-cargo-shim` should probe `PATH` and inject `RUSTC_WRAPPER=sccache`.
pub fn should_inject_sccache(env: &BTreeMap<String, String>) -> bool {
    !env.contains_key("RUSTC_WRAPPER")
        && !env
            .get("VOX_BROKER_NO_SCCACHE")
            .is_some_and(|v| is_truthy(v))
        && !env.get("VOX_BROKER_SCCACHE").is_some_and(|v| is_falsy(v))
}

/// Locate an executable `sccache` binary on `path_var`, skipping `own_exe`'s directory.
pub fn resolve_sccache_wrapper(path_var: &str, own_exe: Option<&Path>) -> Option<PathBuf> {
    let own = own_exe.and_then(|p| p.canonicalize().ok());
    let own_dir = own.as_ref().and_then(|p| p.parent().map(Path::to_path_buf));
    let exe = if cfg!(windows) {
        "sccache.exe"
    } else {
        "sccache"
    };
    for dir in std::env::split_paths(path_var) {
        let cand = dir.join(exe);
        if !cand.is_file() {
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if !std::fs::metadata(&cand).is_ok_and(|m| m.permissions().mode() & 0o111 != 0) {
                continue;
            }
        }
        let Ok(canon) = cand.canonicalize() else {
            continue;
        };
        if own.as_ref() == Some(&canon)
            || (own_dir.is_some() && canon.parent().map(Path::to_path_buf) == own_dir)
        {
            continue;
        }
        return Some(strip_verbatim(canon));
    }
    None
}

/// Route `CARGO_TARGET_DIR` to `<worktree>/target/ws-<slug>` when workstream/agent
/// target isolation is requested and no explicit target/manifest override is set.
pub fn apply_workstream_target_isolation(
    cwd: &Path,
    args: &[String],
    env: &mut BTreeMap<String, String>,
    sccache_active: bool,
) -> Option<PathBuf> {
    let has_explicit = args.iter().take_while(|a| a.as_str() != "--").any(|a| {
        a == "--target-dir"
            || a.starts_with("--target-dir=")
            || a == "--manifest-path"
            || a.starts_with("--manifest-path=")
    });
    if has_explicit
        || env.get("CARGO_TARGET_DIR").is_some_and(|v| !v.is_empty())
        || env.get("VOX_ISOLATED_TARGET").is_some_and(|v| is_falsy(v))
    {
        return None;
    }

    let env_ws = env
        .get("VOX_WORKSTREAM")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let isolated_on = env
        .get("VOX_ISOLATED_TARGET")
        .is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"));

    let raw_slug = if isolated_on {
        env_ws.unwrap_or("isolated").to_string()
    } else if let Some(ws) = env_ws {
        ws.to_string()
    } else if let Some(wt) = worktree_root_of(cwd)
        && let Ok(txt) = std::fs::read_to_string(wt.join(".vox").join("agent-target"))
    {
        txt.lines().next().unwrap_or("").trim().to_string()
    } else {
        return None;
    };

    let slug: String = raw_slug
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(48)
        .collect();
    if slug.is_empty() {
        return None;
    }

    let base = worktree_root_of(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let ws_target = normalize_target_path(cwd, &base.join("target").join(format!("ws-{slug}")));
    env.insert(
        "CARGO_TARGET_DIR".into(),
        ws_target.to_string_lossy().into_owned(),
    );
    if sccache_active && !env.contains_key("CARGO_INCREMENTAL") {
        env.insert("CARGO_INCREMENTAL".into(), "0".into());
    }
    Some(ws_target)
}

/// Terminate a child process group (`killpg`) or Windows process tree (`taskkill /T /F /PID`).
#[allow(unsafe_code)]
pub fn kill_process_tree(pid: u32) {
    #[cfg(unix)]
    if pid > 0 {
        // SAFETY: Child was spawned with `CommandExt::process_group(0)`, so PGID == child PID.
        unsafe {
            let _ = libc::killpg(pid as libc::pid_t, libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sv(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn build_subcommands_classified() {
        assert!(is_build_subcommand("test") && is_build_subcommand("clippy"));
        assert!(!is_build_subcommand("fmt") && !is_build_subcommand("add"));
    }

    #[test]
    fn worktree_root_found_and_none_outside() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(worktree_root_of(tmp.path()).is_none());
        let wt = tmp.path().join("wt");
        std::fs::create_dir_all(wt.join(".cargo")).unwrap();
        std::fs::write(wt.join(".cargo/config.toml"), "[env]\n").unwrap();
        std::fs::write(wt.join("Cargo.toml"), "[workspace]\n").unwrap();
        let deep = wt.join("crates/foo/src");
        std::fs::create_dir_all(&deep).unwrap();
        assert_eq!(
            worktree_root_of(&deep).unwrap().canonicalize().unwrap(),
            wt.canonicalize().unwrap()
        );
    }

    #[test]
    fn resolve_skips_self_and_same_dir_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = if cfg!(windows) { "cargo.exe" } else { "cargo" };
        let (own_dir, real_dir) = (tmp.path().join("debug"), tmp.path().join("cargohome"));
        std::fs::create_dir_all(&own_dir).unwrap();
        std::fs::create_dir_all(&real_dir).unwrap();
        let (own, sibling, real) = (
            own_dir.join("vox-cargo"),
            own_dir.join(exe),
            real_dir.join(exe),
        );
        std::fs::write(&own, b"self").unwrap();
        std::fs::write(&sibling, b"sibling-shim").unwrap();
        std::fs::write(&real, b"real").unwrap();
        let path = std::env::join_paths([&own_dir, &real_dir]).unwrap();
        let (ps, real_c) = (path.to_str().unwrap(), real.canonicalize().unwrap());
        for caller in [&sibling, &own] {
            assert_eq!(
                resolve_real_cargo(ps, caller)
                    .unwrap()
                    .canonicalize()
                    .unwrap(),
                real_c
            );
        }
    }

    #[test]
    fn run_becomes_build_without_program_args() {
        assert_eq!(
            build_args_for_run(&sv(&["run", "-p", "vox-cli", "--quiet", "--", "ci"])).unwrap(),
            sv(&["build", "-p", "vox-cli", "--quiet"])
        );
        assert!(build_args_for_run(&sv(&["test", "-p", "x"])).is_none());
    }

    #[test]
    fn normalize_and_effective_target_dir_precedence() {
        let tmp = tempfile::tempdir().unwrap();
        let wt = tmp.path().join("wt");
        std::fs::create_dir_all(wt.join(".cargo")).unwrap();
        std::fs::write(wt.join(".cargo/config.toml"), "[env]\n").unwrap();
        std::fs::write(wt.join("Cargo.toml"), "[workspace]\n").unwrap();
        let sub = wt.join("crates/demo");
        std::fs::create_dir_all(&sub).unwrap();

        let before = normalize_target_path(&wt, Path::new("target/ws-alpha"));
        std::fs::create_dir_all(wt.join("target/ws-alpha")).unwrap();
        assert_eq!(
            before,
            normalize_target_path(&wt, Path::new("target/ws-alpha"))
        );

        let empty = BTreeMap::new();
        assert_eq!(
            effective_target_dir(&sub, &sv(&["check"]), &empty),
            normalize_target_path(&wt, &wt.join("target"))
        );

        let mut with_env = BTreeMap::new();
        with_env.insert("CARGO_TARGET_DIR".into(), "custom-env-target".into());
        assert_eq!(
            effective_target_dir(&wt, &sv(&["check"]), &with_env),
            normalize_target_path(&wt, Path::new("custom-env-target"))
        );
        assert_eq!(
            effective_target_dir(&wt, &sv(&["check", "--target-dir", "cli-t"]), &with_env),
            normalize_target_path(&wt, Path::new("cli-t"))
        );
        assert_eq!(
            effective_target_dir(&wt, &sv(&["check", "--target-dir=eq-t"]), &with_env),
            normalize_target_path(&wt, Path::new("eq-t"))
        );

        let (outside, standalone) = (tmp.path().join("outside"), tmp.path().join("standalone"));
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::create_dir_all(&standalone).unwrap();
        let (wt_mp, manifest) = (sub.join("Cargo.toml"), standalone.join("Cargo.toml"));
        std::fs::write(&wt_mp, "[package]\nname = \"demo\"\n").unwrap();
        std::fs::write(&manifest, "[workspace]\n[package]\nname = \"s\"\n").unwrap();
        let wt_args = sv(&["check", "--manifest-path", wt_mp.to_str().unwrap()]);
        let st_args = sv(&["test", "--manifest-path", manifest.to_str().unwrap()]);
        assert_eq!(
            effective_target_dir(&outside, &wt_args, &empty),
            normalize_target_path(&wt, &wt.join("target"))
        );
        assert_eq!(
            effective_target_dir(&outside, &st_args, &empty),
            normalize_target_path(&standalone, &standalone.join("target"))
        );
    }

    #[test]
    fn sccache_and_workstream_isolation_pure_helpers() {
        let tmp = tempfile::tempdir().unwrap();
        let (bin_dir, own_dir) = (tmp.path().join("bin"), tmp.path().join("shim"));
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&own_dir).unwrap();
        let exe_name = if cfg!(windows) {
            "sccache.exe"
        } else {
            "sccache"
        };
        let (own_sc, real_sc) = (own_dir.join(exe_name), bin_dir.join(exe_name));
        std::fs::write(&own_sc, b"#!/bin/sh\n").unwrap();
        std::fs::write(&real_sc, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&own_sc, std::fs::Permissions::from_mode(0o755)).unwrap();
            std::fs::set_permissions(&real_sc, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let own_cargo = own_dir.join(if cfg!(windows) { "cargo.exe" } else { "cargo" });
        std::fs::write(&own_cargo, b"shim").unwrap();

        let joined = std::env::join_paths([&own_dir, &bin_dir]).unwrap();
        let found = resolve_sccache_wrapper(joined.to_str().unwrap(), Some(&own_cargo)).unwrap();
        assert_eq!(found, normalize_target_path(tmp.path(), &real_sc));

        let mut env = BTreeMap::new();
        assert!(should_inject_sccache(&env));
        env.insert("RUSTC_WRAPPER".into(), "".into());
        assert!(!should_inject_sccache(&env));
        env.remove("RUSTC_WRAPPER");
        env.insert("VOX_BROKER_NO_SCCACHE".into(), "1".into());
        assert!(!should_inject_sccache(&env));

        let wt = tmp.path().join("wt");
        std::fs::create_dir_all(wt.join(".cargo")).unwrap();
        std::fs::write(wt.join(".cargo/config.toml"), "[env]\n").unwrap();
        std::fs::write(wt.join("Cargo.toml"), "[workspace]\n").unwrap();

        let mut ws_env = BTreeMap::new();
        ws_env.insert("VOX_ISOLATED_TARGET".into(), "1".into());
        ws_env.insert("VOX_WORKSTREAM".into(), "../axis-v75!".into());
        let isolated =
            apply_workstream_target_isolation(&wt, &sv(&["check"]), &mut ws_env, true).unwrap();
        assert_eq!(
            isolated,
            normalize_target_path(&wt, &wt.join("target/ws-axis-v75"))
        );
        assert_eq!(
            ws_env.get("CARGO_INCREMENTAL").map(String::as_str),
            Some("0")
        );

        let mut no_sc = BTreeMap::from([("VOX_WORKSTREAM".into(), "lane2".into())]);
        let _ = apply_workstream_target_isolation(&wt, &sv(&["check"]), &mut no_sc, false).unwrap();
        assert!(!no_sc.contains_key("CARGO_INCREMENTAL"));

        let mut mp_env = BTreeMap::from([("VOX_ISOLATED_TARGET".into(), "1".into())]);
        let mp_args = sv(&["build", "--manifest-path", "Cargo.toml"]);
        assert!(apply_workstream_target_isolation(&wt, &mp_args, &mut mp_env, true).is_none());
        assert!(!mp_env.contains_key("CARGO_TARGET_DIR"));
    }
}
