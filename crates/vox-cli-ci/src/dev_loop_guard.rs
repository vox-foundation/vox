//! `vox ci dev-loop-guard` — fails when the local loop is made slow or broken by configuration:
//! a git hook that builds, a hook that calls an unregistered `vox` command, or a hardcoded cargo
//! `jobs` count. In-process and build-free, so it runs in the fast tier locally and in CI.

use anyhow::{Result, bail};
use std::collections::HashSet;
use std::path::Path;

const BUILD_COMMANDS: [&str; 5] = [
    "cargo run",
    "cargo build",
    "cargo clippy",
    "cargo test",
    "cargo nextest",
];
const MAX_AGENT_TARGET_SLUG_LEN: usize = 48;

/// `run:` values in a lefthook file that invoke a cargo build.
pub fn hook_build_offenders(lefthook: &str) -> Vec<String> {
    lefthook
        .lines()
        .filter_map(|l| l.trim().strip_prefix("run:").map(str::trim))
        .filter(|cmd| BUILD_COMMANDS.iter().any(|b| cmd.contains(b)))
        .map(str::to_string)
        .collect()
}

/// `vox <words>` invocations in a lefthook file whose command path is not in `registry`. The path is
/// the words after `vox` up to the first flag, placeholder, path or shell operator.
pub fn unregistered_hook_commands(lefthook: &str, registry: &HashSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    for cmd in lefthook
        .lines()
        .filter_map(|l| l.trim().strip_prefix("run:").map(str::trim))
    {
        let mut words = cmd.split_whitespace();
        if words.next() != Some("vox") {
            continue;
        }
        let path: Vec<&str> = words
            .take_while(|w| {
                !w.starts_with('-')
                    && !w.starts_with('{')
                    && !w.contains('/')
                    && !w.contains('.')
                    && !w.starts_with('|')
            })
            .collect();
        let joined = path.join(" ");
        if !joined.is_empty() && !registry.contains(&joined) {
            out.push(joined);
        }
    }
    out
}

/// True when `[target.aarch64-apple-darwin]` defines `-Wl,-dead_strip` without global `panic=abort`.
pub fn macos_linker_strip_missing(cargo_config: &str) -> bool {
    let mut in_darwin = false;
    let mut has_dead_strip = false;
    let mut has_panic_abort = false;
    for line in cargo_config.lines().map(str::trim) {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_darwin = line == "[target.aarch64-apple-darwin]";
        } else if in_darwin && line.contains("-Wl,-dead_strip") {
            has_dead_strip = true;
        }
        let cleaned: String = line
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
            .collect();
        if cleaned.contains("panic=abort") {
            has_panic_abort = true;
        }
    }
    !has_dead_strip || has_panic_abort
}

/// True when `.cargo/config.toml` sets `build.jobs = N` explicitly instead of letting the broker/host size workers.
pub fn hardcoded_build_jobs(cargo_config: &str) -> bool {
    let mut in_build = false;
    for line in cargo_config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_build = line == "[build]";
        } else if in_build && line.split('=').next().map(str::trim) == Some("jobs") {
            return true;
        }
    }
    false
}

fn toml_key(line: &str) -> &str {
    line.split('=')
        .next()
        .map(str::trim)
        .unwrap_or("")
        .trim_matches(['"', '\''])
}

/// True when `.cargo/config.toml` commits `[build] rustc-wrapper` or `[env] RUSTC_WRAPPER`.
pub fn hardcoded_rustc_wrapper(cargo_config: &str) -> bool {
    let mut section = "";
    for line in cargo_config.lines().map(str::trim) {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            section = line.split('#').next().map(str::trim).unwrap_or("");
        } else {
            let key = toml_key(line);
            if (section == "[build]" && key == "rustc-wrapper")
                || (section == "[env]" && key == "RUSTC_WRAPPER")
            {
                return true;
            }
        }
    }
    false
}

/// True when `[env]` in `.cargo/config.toml` omits `SCCACHE_DIR` or `CARGO_TARGET_DIR`.
pub fn shared_sccache_or_target_missing(cargo_config: &str) -> bool {
    let (mut in_env, mut has_sccache, mut has_target) = (false, false, false);
    for line in cargo_config.lines().map(str::trim) {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_env = line.split('#').next().map(str::trim) == Some("[env]");
        } else if in_env {
            let key = toml_key(line);
            has_sccache |= key == "SCCACHE_DIR";
            has_target |= key == "CARGO_TARGET_DIR";
        }
    }
    !has_sccache || !has_target
}

/// Returns a reason when an optional `<worktree>/.vox/agent-target` slug violates `[A-Za-z0-9_-]{1,48}`.
pub fn invalid_agent_target_slug(raw: &str) -> Option<&'static str> {
    let slug = raw.trim();
    if slug.is_empty() {
        return Some("empty slug");
    }
    if slug.len() > MAX_AGENT_TARGET_SLUG_LEN {
        return Some("slug exceeds 48 characters");
    }
    if !slug
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Some("slug must match ASCII [A-Za-z0-9_-]");
    }
    None
}

/// Missing broker contention/timeout guidance across `.agents/cargo-safety.md` and `build-broker-usage.md`.
pub fn broker_safety_docs_missing_guidance(
    safety_doc: &str,
    broker_doc: &str,
) -> Vec<&'static str> {
    const SAFETY: [(&str, &str); 4] = [
        (
            "VOX_BROKER_RUN_TIMEOUT_SECS",
            "cargo-safety.md missing VOX_BROKER_RUN_TIMEOUT_SECS",
        ),
        ("VOX_WORKSTREAM", "cargo-safety.md missing VOX_WORKSTREAM"),
        (
            "VOX_ISOLATED_TARGET",
            "cargo-safety.md missing VOX_ISOLATED_TARGET",
        ),
        (
            "cargo test -p",
            "cargo-safety.md missing batched cargo test -p guidance",
        ),
    ];
    const BROKER: [(&str, &str); 6] = [
        (
            "VOX_BROKER_RUN_TIMEOUT_SECS",
            "build-broker-usage.md missing VOX_BROKER_RUN_TIMEOUT_SECS",
        ),
        (
            "VOX_WORKSTREAM",
            "build-broker-usage.md missing VOX_WORKSTREAM",
        ),
        (
            "VOX_BROKER_NO_SCCACHE",
            "build-broker-usage.md missing VOX_BROKER_NO_SCCACHE",
        ),
        (
            "VOX_BROKER_MAX_CONCURRENT",
            "build-broker-usage.md missing VOX_BROKER_MAX_CONCURRENT",
        ),
        ("target_p95", "build-broker-usage.md missing target_p95"),
        ("target_wait", "build-broker-usage.md missing target_wait"),
    ];
    SAFETY
        .into_iter()
        .filter(|(k, _)| !safety_doc.contains(k))
        .chain(BROKER.into_iter().filter(|(k, _)| !broker_doc.contains(k)))
        .map(|(_, msg)| msg)
        .collect()
}

fn sccache_on_path(path_var: Option<&std::ffi::OsStr>) -> bool {
    let bin = if cfg!(windows) {
        "sccache.exe"
    } else {
        "sccache"
    };
    path_var
        .into_iter()
        .flat_map(std::env::split_paths)
        .any(|dir| dir.join(bin).is_file())
}

/// Fast-tier source that `cargo run`s a workspace tool: an args array beginning
/// `"run", "-q", "-p"` or `"run", "-p"` (whitespace-insensitive). Returns a short excerpt per hit.
pub fn tier_build_offenders(src: &str) -> Vec<String> {
    let squashed: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    ["\"run\",\"-q\",\"-p\",", "\"run\",\"-p\","]
        .iter()
        .flat_map(|pat| {
            squashed
                .match_indices(pat)
                .map(|(i, _)| squashed[i..].chars().take(48).collect::<String>())
        })
        .collect()
}

fn registry_paths(root: &Path) -> Result<HashSet<String>> {
    #[derive(serde::Deserialize)]
    struct Op {
        surface: String,
        path: Vec<String>,
    }
    #[derive(serde::Deserialize)]
    struct Reg {
        operations: Vec<Op>,
    }
    let reg: Reg = serde_yaml::from_str(&std::fs::read_to_string(
        root.join("contracts/cli/command-registry.yaml"),
    )?)?;
    Ok(reg
        .operations
        .into_iter()
        .filter(|o| o.surface == "vox-cli")
        .map(|o| o.path.join(" "))
        .collect())
}

#[inline(never)]
pub fn run(root: &Path) -> Result<()> {
    let hooks = std::fs::read_to_string(root.join("lefthook.yml"))?;
    let cargo = std::fs::read_to_string(root.join(".cargo/config.toml"))?;
    let mut errors = Vec::new();
    for c in hook_build_offenders(&hooks) {
        errors.push(format!(
            "lefthook.yml builds in a hook: `{c}` — call the installed binary instead"
        ));
    }
    for c in unregistered_hook_commands(&hooks, &registry_paths(root)?) {
        errors.push(format!(
            "lefthook.yml calls `vox {c}`, which is not in contracts/cli/command-registry.yaml"
        ));
    }
    let pre_push =
        std::fs::read_to_string(root.join("crates/vox-cli/src/commands/ci/pre_push.rs"))?;
    for c in tier_build_offenders(&pre_push) {
        errors.push(format!("pre_push.rs runs a cargo build in the fast tier: `{c}` — use vox_cli_ci::installed_tool"));
    }
    if hardcoded_build_jobs(&cargo) {
        errors.push(
            ".cargo/config.toml sets [build] jobs — let cargo use the machine's parallelism".into(),
        );
    }
    if macos_linker_strip_missing(&cargo) {
        errors.push(
            ".cargo/config.toml [target.aarch64-apple-darwin] must enable -Wl,-dead_strip (and omit global panic=abort)".into(),
        );
    }
    if hardcoded_rustc_wrapper(&cargo) {
        errors.push(
            ".cargo/config.toml sets hardcoded rustc-wrapper / RUSTC_WRAPPER — let vox-cargo-shim inject sccache dynamically".into(),
        );
    }
    if shared_sccache_or_target_missing(&cargo) {
        errors.push(
            ".cargo/config.toml [env] must define both SCCACHE_DIR and CARGO_TARGET_DIR".into(),
        );
    }
    let agent_target = root.join(".vox").join("agent-target");
    if agent_target.is_file() {
        let raw = std::fs::read_to_string(&agent_target)?;
        if let Some(reason) = invalid_agent_target_slug(&raw) {
            errors.push(format!(
                "invalid agent-target slug in {}: {reason}",
                agent_target.display()
            ));
        }
    }
    let safety_doc = std::fs::read_to_string(root.join(".agents").join("cargo-safety.md"))?;
    let broker_doc =
        std::fs::read_to_string(root.join("docs/src/contributors/build-broker-usage.md"))?;
    for missing in broker_safety_docs_missing_guidance(&safety_doc, &broker_doc) {
        errors.push(format!("broker docs parity: {missing}"));
    }
    if !sccache_on_path(std::env::var_os("PATH").as_deref()) && std::env::var_os("CI").is_none() {
        eprintln!(
            "dev-loop-guard: advisory: sccache not found on PATH (install sccache so vox-cargo-shim can cache rustc artifacts across worktrees)"
        );
    }
    if errors.is_empty() {
        println!("dev-loop-guard: ok");
        return Ok(());
    }
    for e in &errors {
        eprintln!("dev-loop-guard: {e}");
    }
    bail!("dev-loop-guard: {} problem(s)", errors.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn flags_hooks_that_build() {
        let y = "pre-commit:\n  commands:\n    a:\n      run: cargo run -p vox-cli --quiet -- ci command-sync\n    b:\n      run: rustfmt {staged_files}\n    # cargo build in a comment is fine\n";
        assert_eq!(
            hook_build_offenders(y),
            vec!["cargo run -p vox-cli --quiet -- ci command-sync".to_string()]
        );
    }

    #[test]
    fn passes_hooks_that_only_call_installed_binaries() {
        let y = "pre-push:\n  commands:\n    p:\n      run: vox ci pre-push\n";
        assert!(hook_build_offenders(y).is_empty());
    }

    #[test]
    fn flags_vox_subcommands_missing_from_the_registry() {
        let reg: HashSet<String> = ["ci pre-push", "run"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let y = "x:\n  run: vox ci status --hook || true\ny:\n  run: vox ci pre-push\nz:\n  run: vox run scripts/fmt.vox -- --all\n";
        assert_eq!(
            unregistered_hook_commands(y, &reg),
            vec!["ci status".to_string()]
        );
    }

    #[test]
    fn flags_a_hardcoded_build_jobs_count() {
        assert!(hardcoded_build_jobs(
            "[build]\njobs = 24\nrustdocflags = []\n"
        ));
        assert!(!hardcoded_build_jobs(
            "[build]\nrustdocflags = []\n[net]\njobs = 3\n"
        ));
        assert!(!hardcoded_build_jobs("# jobs = 24\n[build]\n"));
    }

    #[test]
    fn flags_a_fast_tier_step_that_cargo_runs_a_tool() {
        let src = "cargo()\n    .args([\n        \"run\",\n        \"-q\",\n        \"-p\",\n        \"vox-drift-check\",\n    ])";
        let hits = tier_build_offenders(src);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].contains("vox-drift-check"));
        assert!(
            tier_build_offenders("Command::new(bin).args([\"run\", \"--profile\", \"ci\"])")
                .is_empty()
        );
    }

    #[test]
    fn flags_missing_macos_dead_strip_or_global_panic_abort() {
        assert!(macos_linker_strip_missing(
            "[build]\ntarget-dir = \"target\"\n"
        ));
        assert!(!macos_linker_strip_missing(
            "[target.aarch64-apple-darwin]\nrustflags = [\"-C\", \"link-arg=-Wl,-dead_strip\"]\n"
        ));
        assert!(macos_linker_strip_missing(
            "[target.aarch64-apple-darwin]\nrustflags = [\"-C\", \"link-arg=-Wl,-dead_strip\"]\n[profile.release]\npanic = \"abort\"\n"
        ));
    }

    /// The checked-in lefthook.yml, cargo config and command registry pass, so a hook that builds
    /// or calls an unregistered command fails the test suite as well as the fast tier.
    #[test]
    fn the_repository_passes_the_dev_loop_guard() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        run(root).expect("dev-loop-guard on the repository");
    }

    #[test]
    fn flags_hardcoded_rustc_wrapper_and_missing_env_keys() {
        assert!(hardcoded_rustc_wrapper(
            "[build]\nrustc-wrapper = \"sccache\"\n"
        ));
        assert!(hardcoded_rustc_wrapper(
            "[env]\nRUSTC_WRAPPER = \"sccache\"\n"
        ));
        assert!(!hardcoded_rustc_wrapper(
            "# rustc-wrapper = \"sccache\"\n[build]\nrustdocflags = []\n"
        ));
        assert!(shared_sccache_or_target_missing(
            "[env]\nSCCACHE_DIR = \".sccache\"\n"
        ));
        assert!(shared_sccache_or_target_missing(
            "[env]\nCARGO_TARGET_DIR = \"target\"\n"
        ));
        assert!(!shared_sccache_or_target_missing(
            "[env]\nSCCACHE_DIR = { value = \".sccache\", relative = true }\nCARGO_TARGET_DIR = { value = \"target\", relative = true }\n"
        ));
    }

    #[test]
    fn validates_agent_target_slug_and_broker_docs_parity() {
        assert_eq!(invalid_agent_target_slug("  axis-warp_v75\n"), None);
        assert!(invalid_agent_target_slug("   ").is_some());
        assert!(invalid_agent_target_slug("../escape").is_some());
        assert!(invalid_agent_target_slug("a/b").is_some());
        assert!(invalid_agent_target_slug(&"x".repeat(49)).is_some());
        assert!(!broker_safety_docs_missing_guidance("old safety", "old broker").is_empty());
        assert!(
            broker_safety_docs_missing_guidance(
                "VOX_BROKER_RUN_TIMEOUT_SECS VOX_WORKSTREAM VOX_ISOLATED_TARGET cargo test -p a -p b",
                "VOX_BROKER_RUN_TIMEOUT_SECS VOX_WORKSTREAM VOX_BROKER_NO_SCCACHE VOX_BROKER_MAX_CONCURRENT target_p95 target_wait",
            )
            .is_empty()
        );
    }
}
