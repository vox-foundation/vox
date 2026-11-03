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

/// True when the `[build]` table of a cargo config sets `jobs`.
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
}
