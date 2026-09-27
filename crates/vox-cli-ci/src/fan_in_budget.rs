//! `vox ci fan-in-budget` — deny-new-growth gate for workspace fan-in.
//!
//! Reads `contracts/ci/fan-in-snapshot.v1.json` and computes the LIVE fan-in
//! straight from `cargo metadata` via `crate_edges::collect_live_edges` — the
//! same non-dev (Normal + Build only) edge set `vox ci crate-edges` trusts, and
//! for the same reason: a regenerated mirror must not be able to shrink what
//! this gate sees. Dev-only edges (test/dev-dependencies) never ship in the
//! binary closure, so they must not count toward fan-in either.
//! Fails when any crate's in-tree dependent COUNT grows beyond its committed snapshot value.
//! Crates that shrink their fan-in are not flagged (ratchet, not two-way).

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::crate_edges::{self, EXEMPT};

#[derive(Debug, Deserialize)]
pub struct FanInSnapshot {
    #[allow(dead_code)]
    pub schema_version: u32,
    pub snapshot: HashMap<String, usize>,
}

/// Compute fan-in from a live, non-dev in-tree edge set (`(from, to)` pairs, `from`
/// depends on `to`). `workspace-hack` is exempt in both positions, mirroring
/// `crate_edges::EXEMPT` — it's a hakari feature-unification crate, not a real
/// dependent or dependency.
/// Returns `(crate_name, dependents_count)` for all crates listed in `snapshot`.
pub fn compute_fan_in(
    edges: &BTreeSet<(String, String)>,
    snapshot: &FanInSnapshot,
) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    // Init to 0 for all snapshot keys so missing crates show up as 0 (not missing)
    for k in snapshot.snapshot.keys() {
        counts.insert(k.clone(), 0);
    }
    for (from, to) in edges {
        if from == EXEMPT || to == EXEMPT {
            continue;
        }
        if let Some(c) = counts.get_mut(to) {
            *c += 1;
        }
    }
    counts
}

/// Check for fan-in regressions (crate gained new dependents beyond snapshot).
/// Returns violation messages for crates whose count > snapshot.
pub fn check_regressions(actual: &HashMap<String, usize>, snapshot: &FanInSnapshot) -> Vec<String> {
    let mut violations = Vec::new();
    let mut keys: Vec<&String> = snapshot.snapshot.keys().collect();
    keys.sort();
    for k in keys {
        let snap = *snapshot.snapshot.get(k).unwrap_or(&0);
        let act = *actual.get(k).unwrap_or(&0);
        if act > snap {
            violations.push(format!(
                "  {} fan-in grew: {} → {} ({}  new dependent(s))",
                k,
                snap,
                act,
                act - snap
            ));
        }
    }
    violations
}

pub fn run_fan_in_budget(root: &Path, exit_zero: bool) -> Result<()> {
    let snapshot_path = root.join("contracts/ci/fan-in-snapshot.v1.json");

    let snapshot_raw = std::fs::read_to_string(&snapshot_path)
        .with_context(|| format!("read {}", snapshot_path.display()))?;
    let snapshot: FanInSnapshot = serde_json::from_str(&snapshot_raw)
        .with_context(|| format!("parse {}", snapshot_path.display()))?;

    let edges = crate_edges::collect_live_edges(root)
        .context("collect live in-tree (non-dev) edges via cargo metadata")?;
    let actual = compute_fan_in(&edges, &snapshot);

    // Report all
    let mut keys: Vec<&String> = snapshot.snapshot.keys().collect();
    keys.sort();
    for k in &keys {
        let snap = *snapshot.snapshot.get(*k).unwrap_or(&0);
        let act = *actual.get(*k).unwrap_or(&0);
        let status = if act > snap {
            "GREW"
        } else if act < snap {
            "shrank"
        } else {
            "ok  "
        };
        println!("{status}  {k}: {act} (snapshot {snap})");
    }

    let violations = check_regressions(&actual, &snapshot);
    if violations.is_empty() {
        println!("fan-in-budget: no regressions.");
        return Ok(());
    }

    eprintln!("fan-in-budget REGRESSIONS ({}):", violations.len());
    for v in &violations {
        eprintln!("{v}");
    }
    eprintln!(
        "To allow: bump the count in contracts/ci/fan-in-snapshot.v1.json and update crate-graph.v1.json."
    );

    if exit_zero {
        eprintln!("(advisory — exiting 0 due to --exit-zero)");
        return Ok(());
    }

    anyhow::bail!("{} fan-in regression(s) detected", violations.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(entries: Vec<(&str, usize)>) -> FanInSnapshot {
        FanInSnapshot {
            schema_version: 1,
            snapshot: entries
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    fn make_actual(entries: Vec<(&str, usize)>) -> HashMap<String, usize> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    fn edges(pairs: &[(&str, &str)]) -> BTreeSet<(String, String)> {
        pairs
            .iter()
            .map(|(f, t)| (f.to_string(), t.to_string()))
            .collect()
    }

    #[test]
    fn no_regressions_when_all_at_snapshot() {
        let snap = make_snapshot(vec![("vox-config", 40), ("vox-db", 25)]);
        let actual = make_actual(vec![("vox-config", 40), ("vox-db", 25)]);
        assert!(check_regressions(&actual, &snap).is_empty());
    }

    #[test]
    fn growth_detected() {
        let snap = make_snapshot(vec![("vox-config", 40)]);
        let actual = make_actual(vec![("vox-config", 42)]);
        let v = check_regressions(&actual, &snap);
        assert_eq!(v.len(), 1);
        assert!(v[0].contains("vox-config"));
        assert!(v[0].contains("40"));
        assert!(v[0].contains("42"));
    }

    #[test]
    fn shrinkage_not_flagged() {
        let snap = make_snapshot(vec![("vox-config", 40)]);
        let actual = make_actual(vec![("vox-config", 38)]);
        assert!(check_regressions(&actual, &snap).is_empty());
    }

    #[test]
    fn compute_fan_in_counts_correctly() {
        let snap = make_snapshot(vec![("vox-db", 0)]);
        let e = edges(&[
            ("vox-cli", "vox-db"),
            ("vox-cli", "vox-compiler"),
            ("vox-scientia", "vox-db"),
            ("vox-search", "vox-compiler"),
        ]);
        let result = compute_fan_in(&e, &snap);
        assert_eq!(result["vox-db"], 2);
    }

    #[test]
    fn missing_crate_in_graph_counts_as_zero() {
        let snap = make_snapshot(vec![("vox-nobody", 5)]);
        let e = edges(&[]);
        let result = compute_fan_in(&e, &snap);
        assert_eq!(result.get("vox-nobody").copied().unwrap_or(0), 0);
        // Shrinkage (0 < 5) — not a regression
        let violations = check_regressions(&result, &snap);
        assert!(violations.is_empty());
    }

    /// Decision (2026-09-27): `workspace-hack` is exempt from fan-in counting in
    /// both positions, mirroring `crate_edges::EXEMPT` — it's a hakari
    /// feature-unification crate that every other crate depends on by design, not
    /// a real dependent relationship the budget should ratchet.
    #[test]
    fn workspace_hack_exempt_from_fan_in() {
        let snap = make_snapshot(vec![("workspace-hack", 0), ("vox-db", 0)]);
        let e = edges(&[("vox-cli", "workspace-hack"), ("vox-cli", "vox-db")]);
        let result = compute_fan_in(&e, &snap);
        assert_eq!(
            result["workspace-hack"], 0,
            "workspace-hack must be exempt from fan-in counting"
        );
        assert_eq!(result["vox-db"], 1);
    }

    /// Decision (2026-09-27): fan-in counts only non-dev edges. Build a minimal
    /// two-crate cargo workspace where `a`'s only edge to `b` is a
    /// `[dev-dependencies]` entry, run it through the SAME live-edge collector
    /// `crate-edges` trusts, and confirm the dev-only edge neither appears in the
    /// live set nor counts toward `b`'s fan-in.
    #[test]
    fn dev_only_edge_not_counted_in_fan_in() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("a/src")).unwrap();
        std::fs::write(
            dir.path().join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dev-dependencies]\nb = { path = \"../b\" }\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("a/src/lib.rs"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("b/src")).unwrap();
        std::fs::write(
            dir.path().join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("b/src/lib.rs"), "").unwrap();

        let live = crate_edges::collect_live_edges(dir.path()).unwrap();
        assert!(
            !live.contains(&("a".to_string(), "b".to_string())),
            "dev-only dependency must not appear in the live non-dev edge set: {live:?}"
        );

        let snap = make_snapshot(vec![("b", 0)]);
        let counts = compute_fan_in(&live, &snap);
        assert_eq!(
            counts["b"], 0,
            "dev-only dependency must not count toward fan-in"
        );
    }
}
