//! Pins baseline file line counts so edits cannot happen silently.
//!
//! When the baseline legitimately shrinks (a finding is registered) or grows (a new exemption),
//! update the corresponding const and commit the change with an explanation.
use std::fs;
use std::path::Path;

fn non_comment_lines(path: &Path) -> usize {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .count()
}

fn workspace_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR is crates/vox-cli; workspace root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must exist")
}

#[test]
fn config_hygiene_baseline_has_pinned_count() {
    let path = workspace_root().join("contracts/config/config-hygiene-baseline.txt");
    let count = non_comment_lines(&path);
    // Update this number when the baseline legitimately shrinks (registered) or grows (new exemption).
    assert_eq!(
        count, EXPECTED_HYGIENE_COUNT,
        "config-hygiene-baseline.txt changed — update EXPECTED_HYGIENE_COUNT if intentional"
    );
}
// 2026-09-22: 379 -> 375, the four CI-runner autoscaler findings from
// `runner_scale.rs` went away with the self-hosted fleet tooling.
// 2026-10-06: 375 -> 240, burn-down only (no key added): Check D now reads the
// federated registry (CONFIG_KEYS too), integration-test dirs and the
// vox-cli-ci repo-tree guards are out of scope, and stale keys were pruned.
const EXPECTED_HYGIENE_COUNT: usize = 240;

#[test]
fn config_registry_baseline_has_pinned_count() {
    let path = workspace_root().join("contracts/config/config-registry-baseline.txt");
    let count = non_comment_lines(&path);
    assert_eq!(
        count, EXPECTED_REGISTRY_COUNT,
        "config-registry-baseline.txt changed — update EXPECTED_REGISTRY_COUNT if intentional"
    );
}
// 2026-09-22: 347 -> 343, the same four autoscaler knobs.
// 2026-10-06: 343 -> 219, burn-down only (no name added): the scanner counts only
// whole quoted `"VOX_*"` literals outside tests/ dirs, and stale names were pruned.
const EXPECTED_REGISTRY_COUNT: usize = 219;
