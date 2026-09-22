//! Build-id handshake between `vox` and the `vox-ml-cli` binary it delegates
//! every ML command to.
//!
//! `vox` resolves `vox-ml-cli` as a separate executable (sibling, then
//! `~/.vox/bin`, then `PATH`), so a stale `vox-ml-cli` left in `~/.cargo/bin`
//! silently runs month-old code behind a fresh `vox`. The parent passes its own
//! build id in [`PARENT_BUILD_ID_ENV`]; the child compares it against its own
//! and warns (or, with [`REQUIRE_MATCH_ENV`], refuses to run) on skew.
//!
//! The build id is the short git hash baked in by `vox_build_meta::emit`.

use std::path::Path;

/// Env var `vox` sets to its own build id when spawning `vox-ml-cli`.
pub const PARENT_BUILD_ID_ENV: &str = "VOX_PARENT_BUILD_ID";

/// Set to a non-empty value to turn a build mismatch into a hard failure.
pub const REQUIRE_MATCH_ENV: &str = "VOX_REQUIRE_MATCHING_ML_CLI";

/// Rebuild command for a contributor (running inside a Vox source checkout).
pub const CONTRIBUTOR_REBUILD: &str =
    "cargo install --path crates/vox-ml-cli --features gpu --force";

/// Refresh command for an installed (non-contributor) user. Names no `cargo`
/// and no repo-relative path (spec §9.1).
pub const INSTALLED_REBUILD: &str = "vox upgrade";

/// A build id is comparable only when it is a real (hex) git hash. `"unknown"`
/// (no git at build time), empty, or anything else is never compared, so
/// non-git builds never produce a false warning.
fn known(id: &str) -> Option<&str> {
    let t = id.trim();
    (!t.is_empty() && t.chars().all(|c| c.is_ascii_hexdigit())).then_some(t)
}

/// Whether `id` is a comparable build id (see [`builds_mismatch`]).
pub fn is_known_build_id(id: &str) -> bool {
    known(id).is_some()
}

/// True only when both ids are known hashes that name different commits.
///
/// Prefix-equal hashes match: `git rev-parse --short` grows its length as a
/// repo grows, so the same commit can be `abc1234` in one build and
/// `abc12345` in another.
pub fn builds_mismatch(a: &str, b: &str) -> bool {
    match (known(a), known(b)) {
        (Some(a), Some(b)) => !(a.starts_with(b) || b.starts_with(a)),
        _ => false,
    }
}

/// The remedy command for the current persona.
pub fn rebuild_command(contributor: bool) -> &'static str {
    if contributor {
        CONTRIBUTOR_REBUILD
    } else {
        INSTALLED_REBUILD
    }
}

/// One-paragraph warning naming both builds, the running binary, and the fix.
pub fn mismatch_message(parent: &str, child: &str, child_path: &Path, contributor: bool) -> String {
    format!(
        "vox-ml-cli build {child} does not match vox build {parent}; the ML command is \
         running code from a different commit.\n  running: {}\n  fix: {}\n  \
         (set {REQUIRE_MATCH_ENV}=1 to make this an error)",
        child_path.display(),
        rebuild_command(contributor),
    )
}

/// Outcome of the child-side handshake.
#[derive(Debug, PartialEq, Eq)]
pub enum Handshake {
    /// Builds match, or there is nothing comparable (no parent / unknown hash).
    Ok,
    /// Mismatch: print this to stderr and continue.
    Warn(String),
    /// Mismatch with [`REQUIRE_MATCH_ENV`] set: print and exit non-zero.
    Fail(String),
}

/// Pure handshake decision. `parent` is the value of [`PARENT_BUILD_ID_ENV`]
/// (absent when `vox-ml-cli` was run directly, not via `vox`).
pub fn evaluate(
    child: &str,
    parent: Option<&str>,
    child_path: &Path,
    contributor: bool,
    require_match: bool,
) -> Handshake {
    let Some(parent) = parent.filter(|p| builds_mismatch(p, child)) else {
        return Handshake::Ok;
    };
    let msg = mismatch_message(parent, child, child_path, contributor);
    if require_match {
        Handshake::Fail(msg)
    } else {
        Handshake::Warn(msg)
    }
}

/// Extract the git hash from a version line such as
/// `vox-ml-cli 0.6.0+build.601 (abc1234)`. `None` when the line carries no
/// hash in parentheses (e.g. a binary that predates this handshake).
pub fn git_hash_from_version_line(line: &str) -> Option<&str> {
    let inner = line.trim().strip_suffix(')')?;
    let hash = &inner[inner.rfind('(')? + 1..];
    known(hash)
}

/// Child-side entry point: run the handshake against the environment and
/// print/exit as needed. Call once at `vox-ml-cli` startup.
pub fn enforce_from_env(child: &str) {
    let parent = std::env::var(PARENT_BUILD_ID_ENV).ok();
    let require = std::env::var_os(REQUIRE_MATCH_ENV).is_some_and(|v| !v.is_empty());
    let path = std::env::current_exe().unwrap_or_else(|_| "vox-ml-cli".into());
    // Contributor = cwd is inside a checkout that contains this crate.
    let contributor = std::env::current_dir()
        .ok()
        .and_then(|cwd| vox_config::paths::find_repo_root(&cwd))
        .is_some_and(|root| root.join("crates").join("vox-ml-cli").is_dir());
    match evaluate(child, parent.as_deref(), &path, contributor, require) {
        Handshake::Ok => {}
        Handshake::Warn(msg) => eprintln!("warning: {msg}"),
        Handshake::Fail(msg) => {
            eprintln!("error: {msg}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatch_only_between_two_known_different_hashes() {
        assert!(builds_mismatch("abc1234", "def5678"));
        assert!(!builds_mismatch("abc1234", "abc1234"));
        // Short-hash length drift is the same commit.
        assert!(!builds_mismatch("abc1234", "abc12345ff"));
        // Unknown / non-git builds never warn.
        assert!(!builds_mismatch("unknown", "abc1234"));
        assert!(!builds_mismatch("abc1234", ""));
        assert!(!builds_mismatch("unknown", "unknown"));
        assert!(is_known_build_id("abc1234") && !is_known_build_id("unknown"));
    }

    #[test]
    fn message_names_both_builds_path_and_persona_remedy() {
        let p = Path::new("/home/u/.cargo/bin/vox-ml-cli");
        let m = mismatch_message("abc1234", "def5678", p, true);
        assert!(m.contains("abc1234") && m.contains("def5678"));
        assert!(m.contains("/home/u/.cargo/bin/vox-ml-cli"));
        assert!(m.contains(CONTRIBUTOR_REBUILD));
        assert!(m.contains(REQUIRE_MATCH_ENV));

        let installed = mismatch_message("abc1234", "def5678", p, false);
        assert!(installed.contains(INSTALLED_REBUILD));
        assert!(!installed.contains("cargo install"));
    }

    #[test]
    fn evaluate_warns_fails_or_passes() {
        let p = Path::new("vox-ml-cli");
        assert_eq!(evaluate("abc1234", None, p, false, true), Handshake::Ok);
        assert_eq!(
            evaluate("abc1234", Some("abc1234"), p, false, true),
            Handshake::Ok
        );
        assert_eq!(
            evaluate("unknown", Some("abc1234"), p, false, true),
            Handshake::Ok
        );
        assert!(matches!(
            evaluate("abc1234", Some("def5678"), p, false, false),
            Handshake::Warn(_)
        ));
        assert!(matches!(
            evaluate("abc1234", Some("def5678"), p, false, true),
            Handshake::Fail(_)
        ));
    }

    #[test]
    fn hash_parsed_from_version_line() {
        assert_eq!(
            git_hash_from_version_line("vox-ml-cli 0.6.0+build.601 (abc1234)\n"),
            Some("abc1234")
        );
        assert_eq!(git_hash_from_version_line("vox-ml-cli 0.6.0"), None);
        assert_eq!(
            git_hash_from_version_line("vox-ml-cli 0.6.0+build.dev (unknown)"),
            None
        );
    }
}
