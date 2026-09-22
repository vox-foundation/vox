//! `vox doctor` — does the `vox-ml-cli` that ML commands delegate to come from
//! the same build as this `vox`? Advisory (listed in `OPTIONAL_CHECK_NAMES`).
//!
//! Runtime counterpart: `vox_cli_core::ml_cli_handshake`, which warns on every
//! delegated command. This row also catches binaries too old to know the
//! handshake — they print no hash in `--version`.

use std::path::Path;
use std::process::Command;

use vox_cli_core::ml_cli_handshake::{
    builds_mismatch, git_hash_from_version_line, is_known_build_id, rebuild_command,
};

use super::super::common::Check;

const CHECK_NAME: &str = "vox-ml-cli build";

pub fn run(checks: &mut Vec<Check>) {
    let path =
        vox_cli_core::daemon_ipc::process_supervision::resolve_managed_binary_path("vox-ml-cli");
    let Ok(out) = Command::new(&path).arg("--version").output() else {
        checks.push(Check::pass(
            CHECK_NAME,
            "not installed (ML commands such as `vox mens` are unavailable)",
        ));
        return;
    };
    let line = String::from_utf8_lossy(&out.stdout);
    checks.push(classify(
        &path,
        git_hash_from_version_line(&line),
        crate::freshness::EMBEDDED_GIT_HASH,
        crate::contributor_mode::is_contributor_mode(),
    ));
}

/// Pure verdict from the resolved path, the hash it reported, and ours.
fn classify(path: &Path, reported: Option<&str>, own: &str, contributor: bool) -> Check {
    let fix = rebuild_command(contributor);
    let path = path.display();
    match reported {
        // Our own build has no hash (non-git build): nothing to compare.
        _ if !is_known_build_id(own) => Check::pass(
            CHECK_NAME,
            format!("{path} (vox build id unknown; not compared)"),
        ),
        None => Check::fail(
            CHECK_NAME,
            format!(
                "{path} reports no build id — it predates the build-id handshake (almost \
                 certainly stale) or was built outside git. fix: {fix}"
            ),
        ),
        Some(hash) if builds_mismatch(own, hash) => Check::fail(
            CHECK_NAME,
            format!("{path} is build {hash}, vox is build {own}. fix: {fix}"),
        ),
        Some(hash) => Check::pass(CHECK_NAME, format!("{path} matches vox build {hash}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: &str = "/home/u/.cargo/bin/vox-ml-cli";

    #[test]
    fn matching_build_passes() {
        let c = classify(Path::new(P), Some("abc1234"), "abc1234", true);
        assert!(c.pass, "{}", c.detail);
        assert!(c.detail.contains(P));
    }

    #[test]
    fn mismatched_build_fails_with_path_and_fix() {
        let c = classify(Path::new(P), Some("def5678"), "abc1234", true);
        assert!(!c.pass);
        assert!(c.detail.contains(P) && c.detail.contains("def5678"));
        assert!(c.detail.contains("cargo install --path crates/vox-ml-cli"));
    }

    #[test]
    fn pre_handshake_binary_fails_with_installed_remedy() {
        let c = classify(Path::new(P), None, "abc1234", false);
        assert!(!c.pass);
        assert!(c.detail.contains("vox upgrade") && !c.detail.contains("cargo install"));
    }

    #[test]
    fn unknown_own_build_is_not_compared() {
        assert!(classify(Path::new(P), None, "unknown", true).pass);
        assert!(classify(Path::new(P), Some("def5678"), "unknown", true).pass);
    }
}
