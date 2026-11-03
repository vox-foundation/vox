//! `vox ci crate-edges` — exact edge-set ratchet + layer rule for workspace deps.
//!
//! Reads the LIVE graph from `cargo metadata` (never the regenerated
//! `crate-graph.v1.json` mirror — a mirror regen must not be able to admit edges).
//! Baseline + human-gated exceptions: `contracts/ci/crate-edges.allow.v1.json`.
//! Layer map (downward-only rule): `[crates]` + `[[known_inversions]]` of
//! `docs/src/architecture/layers.toml`, the same file `vox-arch-check` reads.
//! Spec: docs/superpowers/specs/2026-07-03-crate-disentanglement-ratchet-design.md

use anyhow::{Context, Result, bail};
use cargo_metadata::DependencyKind;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const ALLOW_REL: &str = "contracts/ci/crate-edges.allow.v1.json";
pub const LAYERS_REL: &str = "docs/src/architecture/layers.toml";
/// hakari feature-unification crate: exempt both directions, by design.
pub const EXEMPT: &str = "workspace-hack";

/// LLM-facing heal text. AGENTS.md "Dependency Discipline" states the same rules;
/// keep both short and in sync.
pub const HEAL: &str = "\
[diag id=arch/crate-edges heal=llm]
A workspace dependency edge is not in the committed baseline. Legal moves:
 1. Do not add the edge: check for a narrower `-types`/`-core` crate, or duplicate a
    helper under ~50 lines into your crate with a `// vox:defactored-from <crate> <date>`
    comment (see AGENTS.md 'Dependency Discipline').
 2. If the edge is genuinely needed: PROPOSE an `exceptions` entry in your PR
    description and STOP. Entries in contracts/ci/crate-edges.allow.v1.json are
    USER-AUTHORIZED-ONLY. Never write one yourself; never regenerate baselines to
    admit your own edge.
Tightening (removing edges) is always allowed: `vox ci crate-edges --tighten`.";

#[derive(Debug, Serialize, Deserialize)]
pub struct AllowFile {
    pub schema_version: u32,
    /// Frozen baseline, sorted [from, to] pairs. Machine-tightened only.
    pub edges: Vec<[String; 2]>,
    /// Human-gated ledger. Append requires explicit user authorization.
    pub exceptions: Vec<ExceptionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionEntry {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub date: String,
    pub authorized_by: String,
}

/// The `[crates]` layer map and `[[known_inversions]]` of `layers.toml` — the one
/// layer SSOT, shared with `vox-arch-check` (layer semantics are documented there).
#[derive(Debug)]
pub struct LayersFile {
    /// crate name -> layer (0 = pure types ... 5 = surfaces).
    pub layers: BTreeMap<String, u8>,
    /// Upward `[from, to]` pairs accepted by `[[known_inversions]]`. They clear the
    /// UpwardEdge verdict only; the edge must still be in the baseline or exceptions.
    pub known_inversions: BTreeSet<(String, String)>,
}

/// Parse the subset of `layers.toml` this guard needs; other tables are ignored.
pub fn parse_layers_toml(text: &str) -> Result<LayersFile> {
    #[derive(Deserialize)]
    struct Raw {
        #[serde(default)]
        crates: BTreeMap<String, RawCrate>,
        #[serde(default)]
        known_inversions: Vec<RawInversion>,
    }
    #[derive(Deserialize)]
    struct RawCrate {
        layer: u8,
    }
    #[derive(Deserialize)]
    struct RawInversion {
        from: String,
        to: String,
    }
    let raw: Raw = toml::from_str(text).with_context(|| format!("parse {LAYERS_REL}"))?;
    Ok(LayersFile {
        layers: raw.crates.into_iter().map(|(k, c)| (k, c.layer)).collect(),
        known_inversions: raw
            .known_inversions
            .into_iter()
            .map(|i| (i.from, i.to))
            .collect(),
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Violation {
    NewEdge {
        from: String,
        to: String,
    },
    UpwardEdge {
        from: String,
        from_layer: u8,
        to: String,
        to_layer: u8,
    },
    MissingLayer {
        krate: String,
    },
}

/// Pure rule engine: violations + stale-baseline warnings. No IO.
///
/// IMPORTANT: an exception suppresses the NewEdge/UpwardEdge verdicts for its
/// own pair, but layer-presence checking is UNCONDITIONAL — an excepted edge must
/// never let a crate skip classification (a new crate sneaking in via an exception
/// with no layer entry is exactly the kind of gap this rule exists to prevent).
pub fn check(
    live: &BTreeSet<(String, String)>,
    allow: &AllowFile,
    layers: &LayersFile,
) -> (Vec<Violation>, Vec<(String, String)>) {
    let baseline: BTreeSet<(String, String)> = allow
        .edges
        .iter()
        .map(|e| (e[0].clone(), e[1].clone()))
        .collect();
    let excepted: BTreeSet<(String, String)> = allow
        .exceptions
        .iter()
        .map(|x| (x.from.clone(), x.to.clone()))
        .collect();

    let mut violations = Vec::new();
    let mut missing: BTreeSet<String> = BTreeSet::new();
    for (from, to) in live {
        if from == EXEMPT || to == EXEMPT {
            continue;
        }
        let pair = (from.clone(), to.clone());
        let is_excepted = excepted.contains(&pair);
        if !baseline.contains(&pair) && !is_excepted {
            violations.push(Violation::NewEdge {
                from: from.clone(),
                to: to.clone(),
            });
        }
        // Layer-presence + upward-edge checks always run, regardless of exception
        // status; only the UpwardEdge verdict itself is suppressed when excepted.
        match (layers.layers.get(from), layers.layers.get(to)) {
            (Some(&fl), Some(&tl)) => {
                if fl < tl && !is_excepted && !layers.known_inversions.contains(&pair) {
                    violations.push(Violation::UpwardEdge {
                        from: from.clone(),
                        from_layer: fl,
                        to: to.clone(),
                        to_layer: tl,
                    });
                }
            }
            (f, t) => {
                if f.is_none() {
                    missing.insert(from.clone());
                }
                if t.is_none() {
                    missing.insert(to.clone());
                }
            }
        }
    }
    for krate in missing {
        violations.push(Violation::MissingLayer { krate });
    }
    // Stale covers BOTH the frozen baseline and the exceptions ledger — a dead
    // exception (its edge no longer exists) must also prompt cleanup via --tighten.
    let stale: Vec<(String, String)> = baseline
        .iter()
        .chain(excepted.iter())
        .filter(|e| !live.contains(*e))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    (violations, stale)
}

/// Live in-tree edge set from `cargo metadata`: workspace members only,
/// normal + build dependency kinds (dev-deps excluded in v1 — they don't ship
/// in the binary closure; `vox ci dep-cycles` covers dev-dep back-edges).
pub fn collect_live_edges(root: &Path) -> Result<BTreeSet<(String, String)>> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .exec()
        .context("run `cargo metadata`")?;
    let members: BTreeSet<&str> = metadata
        .workspace_packages()
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    let mut edges = BTreeSet::new();
    for pkg in metadata.workspace_packages() {
        for dep in &pkg.dependencies {
            if !matches!(dep.kind, DependencyKind::Normal | DependencyKind::Build) {
                continue;
            }
            if members.contains(dep.name.as_str()) {
                edges.insert((pkg.name.as_str().to_string(), dep.name.clone()));
            }
        }
    }
    Ok(edges)
}

pub fn write_allow_file(path: &Path, file: &AllowFile) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(file)? + "\n")
        .with_context(|| format!("write {}", path.display()))
}

/// Regenerate the baseline from the live set. Removal-only: refuses if any live
/// edge is absent from the prior baseline union exceptions. Keeps exceptions whose
/// edge still exists; excepted pairs are NOT duplicated into edges.
pub fn tighten(root: &Path, live: &BTreeSet<(String, String)>) -> Result<()> {
    let path = root.join(ALLOW_REL);
    let live_real: BTreeSet<(String, String)> = live
        .iter()
        .filter(|(f, t)| f != EXEMPT && t != EXEMPT)
        .cloned()
        .collect();
    let prior: Option<AllowFile> = if path.exists() {
        Some(
            serde_json::from_str(
                &std::fs::read_to_string(&path)
                    .with_context(|| format!("read {}", path.display()))?,
            )
            .context("parse crate-edges.allow.v1.json")?,
        )
    } else {
        None
    };
    let mut exceptions: Vec<ExceptionEntry> = Vec::new();
    if let Some(prior) = &prior {
        let prior_allowed: BTreeSet<(String, String)> = prior
            .edges
            .iter()
            .map(|e| (e[0].clone(), e[1].clone()))
            .chain(
                prior
                    .exceptions
                    .iter()
                    .map(|x| (x.from.clone(), x.to.clone())),
            )
            .collect();
        let additions: Vec<&(String, String)> = live_real
            .iter()
            .filter(|e| !prior_allowed.contains(e))
            .collect();
        if !additions.is_empty() {
            bail!(
                "--tighten would ADD {} edge(s) (tighten is removal-only): {:?}\n\n{HEAL}",
                additions.len(),
                additions
            );
        }
        exceptions = prior
            .exceptions
            .iter()
            .filter(|x| live_real.contains(&(x.from.clone(), x.to.clone())))
            .cloned()
            .collect();
    }
    let excepted: BTreeSet<(String, String)> = exceptions
        .iter()
        .map(|x| (x.from.clone(), x.to.clone()))
        .collect();
    let file = AllowFile {
        schema_version: 1,
        edges: live_real
            .iter()
            .filter(|e| !excepted.contains(e))
            .map(|(f, t)| [f.clone(), t.clone()])
            .collect(),
        exceptions,
    };
    write_allow_file(&path, &file)?;
    println!(
        "crate-edges: baseline tightened to {} edges (+{} exceptions) -> {}",
        file.edges.len(),
        file.exceptions.len(),
        path.display()
    );
    Ok(())
}

/// Guard entry point (`vox ci crate-edges [--tighten]`).
pub fn run(root: &Path, tighten_mode: bool) -> Result<()> {
    let live = collect_live_edges(root)?;
    let layers_path = root.join(LAYERS_REL);
    if tighten_mode {
        return tighten(root, &live);
    }
    let allow_path = root.join(ALLOW_REL);
    if !allow_path.exists() {
        bail!("missing {ALLOW_REL} — bootstrap with `vox ci crate-edges --tighten`");
    }
    let allow: AllowFile = serde_json::from_str(&std::fs::read_to_string(&allow_path)?)
        .context("parse crate-edges.allow.v1.json")?;
    let layers = parse_layers_toml(
        &std::fs::read_to_string(&layers_path)
            .with_context(|| format!("read {LAYERS_REL} (the layer SSOT)"))?,
    )?;

    let (violations, stale) = check(&live, &allow, &layers);
    for (f, t) in &stale {
        println!(
            "warning: stale baseline edge {f} -> {t} (gone; run `vox ci crate-edges --tighten`)"
        );
    }
    if violations.is_empty() {
        println!(
            "crate-edges: OK ({} live in-tree edges within baseline)",
            live.len()
        );
        return Ok(());
    }
    for v in &violations {
        match v {
            Violation::NewEdge { from, to } => {
                eprintln!("NEW EDGE not in baseline: {from} -> {to}")
            }
            Violation::UpwardEdge {
                from,
                from_layer,
                to,
                to_layer,
            } => eprintln!(
                "UPWARD EDGE (layer rule): {from} (L{from_layer}) -> {to} (L{to_layer}) — deps must point same-layer or down"
            ),
            Violation::MissingLayer { krate } => eprintln!(
                "UNASSIGNED LAYER: {krate} missing from {LAYERS_REL} [crates] — assign one per where-things-live.md"
            ),
        }
    }
    bail!("crate-edges: {} violation(s)\n\n{HEAL}", violations.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(pairs: &[(&str, &str)]) -> BTreeSet<(String, String)> {
        pairs
            .iter()
            .map(|(f, t)| (f.to_string(), t.to_string()))
            .collect()
    }
    fn allow(baseline: &[(&str, &str)], exceptions: &[(&str, &str)]) -> AllowFile {
        AllowFile {
            schema_version: 1,
            edges: baseline
                .iter()
                .map(|(f, t)| [f.to_string(), t.to_string()])
                .collect(),
            exceptions: exceptions
                .iter()
                .map(|(f, t)| ExceptionEntry {
                    from: f.to_string(),
                    to: t.to_string(),
                    reason: "test".into(),
                    date: "2026-07-03".into(),
                    authorized_by: "test".into(),
                })
                .collect(),
        }
    }
    fn layers(assign: &[(&str, u8)]) -> LayersFile {
        LayersFile {
            layers: assign.iter().map(|(k, l)| (k.to_string(), *l)).collect(),
            known_inversions: BTreeSet::new(),
        }
    }

    #[test]
    fn parse_layers_toml_reads_crates_and_known_inversions_only() {
        let l = parse_layers_toml(
            r#"
[guards]
fan_in = "warn"

[crates]
vox-lib = { layer = 0 }
vox-app = { layer = 5, kind = "binary", max_loc = 1_000 }

[planned]
vox-future = { plan = "x.md", layer = 2 }

[[known_inversions]]
from   = "vox-lib"
to     = "vox-app"
reason = "test"
"#,
        )
        .unwrap();
        assert_eq!(
            l.layers,
            BTreeMap::from([("vox-app".to_string(), 5), ("vox-lib".to_string(), 0)]),
            "[planned] entries must not count as assigned layers"
        );
        assert_eq!(
            l.known_inversions,
            edges(&[("vox-lib", "vox-app")]),
            "known inversions come from the same file vox-arch-check reads"
        );
    }

    #[test]
    fn parse_layers_toml_rejects_missing_layer() {
        let err = parse_layers_toml("[crates]\nvox-lib = { kind = \"library\" }\n").unwrap_err();
        assert!(format!("{err:#}").contains("layer"), "{err:#}");
    }

    #[test]
    fn known_inversion_suppresses_upward_verdict_but_not_new_edge() {
        let mut l = layers(&[("app", 5), ("lib", 0)]);
        l.known_inversions = edges(&[("lib", "app")]);
        let (v, _) = check(
            &edges(&[("lib", "app")]),
            &allow(&[("lib", "app")], &[]),
            &l,
        );
        assert!(v.is_empty(), "{v:?}");
        let (v, _) = check(&edges(&[("lib", "app")]), &allow(&[], &[]), &l);
        assert_eq!(
            v,
            vec![Violation::NewEdge {
                from: "lib".into(),
                to: "app".into()
            }],
            "a known inversion must never admit an edge the baseline does not hold"
        );
    }

    /// Integration: the layer map crate-edges reads is the one vox-arch-check
    /// reads, so the two gates can no longer disagree about a crate's layer.
    #[test]
    fn repo_layer_map_is_layers_toml() {
        let root = crate::repo_root();
        let l =
            parse_layers_toml(&std::fs::read_to_string(root.join(LAYERS_REL)).unwrap()).unwrap();
        assert_eq!(LAYERS_REL, "docs/src/architecture/layers.toml");
        assert!(l.layers.contains_key("vox-cli"));
        assert!(
            !root.join("contracts/ci/crate-layers.v1.json").exists(),
            "the retired duplicate layer map must not come back"
        );
    }

    /// A minimal single-package Cargo workspace (no dependencies, so `cargo
    /// metadata` never touches the network) — enough for `collect_live_edges`
    /// to succeed inside `run()` without needing the real repo.
    fn fake_workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"fake-root\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "").unwrap();
        dir
    }

    #[test]
    fn new_edge_fails() {
        let (v, _) = check(
            &edges(&[("app", "lib")]),
            &allow(&[], &[]),
            &layers(&[("app", 4), ("lib", 0)]),
        );
        assert_eq!(
            v,
            vec![Violation::NewEdge {
                from: "app".into(),
                to: "lib".into()
            }]
        );
    }

    #[test]
    fn baseline_edge_passes() {
        let (v, stale) = check(
            &edges(&[("app", "lib")]),
            &allow(&[("app", "lib")], &[]),
            &layers(&[("app", 4), ("lib", 0)]),
        );
        assert!(v.is_empty());
        assert!(stale.is_empty());
    }

    #[test]
    fn exception_admits_edge_and_suppresses_upward_verdict() {
        let (v, _) = check(
            &edges(&[("lib", "app")]),
            &allow(&[], &[("lib", "app")]),
            &layers(&[("app", 4), ("lib", 0)]),
        );
        assert!(v.is_empty());
    }

    #[test]
    fn exception_does_not_mask_missing_layer() {
        let (v, _) = check(
            &edges(&[("app", "new")]),
            &allow(&[], &[("app", "new")]),
            &layers(&[("app", 4)]),
        );
        assert_eq!(
            v,
            vec![Violation::MissingLayer {
                krate: "new".into()
            }]
        );
    }

    #[test]
    fn removed_edge_reports_stale_not_fail() {
        let (v, stale) = check(&edges(&[]), &allow(&[("app", "lib")], &[]), &layers(&[]));
        assert!(v.is_empty());
        assert_eq!(stale, vec![("app".to_string(), "lib".to_string())]);
    }

    #[test]
    fn dead_exception_reports_stale_too() {
        let (v, stale) = check(&edges(&[]), &allow(&[], &[("app", "lib")]), &layers(&[]));
        assert!(v.is_empty());
        assert_eq!(stale, vec![("app".to_string(), "lib".to_string())]);
    }

    #[test]
    fn upward_layer_edge_fails() {
        let (v, _) = check(
            &edges(&[("lib", "app")]),
            &allow(&[("lib", "app")], &[]),
            &layers(&[("app", 4), ("lib", 0)]),
        );
        assert_eq!(
            v,
            vec![Violation::UpwardEdge {
                from: "lib".into(),
                from_layer: 0,
                to: "app".into(),
                to_layer: 4
            }]
        );
    }

    #[test]
    fn same_layer_edge_ok() {
        let (v, _) = check(
            &edges(&[("db", "compiler")]),
            &allow(&[("db", "compiler")], &[]),
            &layers(&[("db", 2), ("compiler", 2)]),
        );
        assert!(v.is_empty());
    }

    #[test]
    fn missing_layer_fails() {
        let (v, _) = check(
            &edges(&[("app", "lib")]),
            &allow(&[("app", "lib")], &[]),
            &layers(&[("app", 4)]),
        );
        assert_eq!(
            v,
            vec![Violation::MissingLayer {
                krate: "lib".into()
            }]
        );
    }

    #[test]
    fn workspace_hack_exempt() {
        let (v, _) = check(
            &edges(&[("app", "workspace-hack"), ("workspace-hack", "lib")]),
            &allow(&[], &[]),
            &layers(&[]),
        );
        assert!(v.is_empty());
    }

    /// Integration: real workspace. vox-cli depends on vox-cli-ci (already true on
    /// main after the PR-4 CI extraction); if this fails the collector or the
    /// workspace layout changed.
    #[test]
    fn live_graph_contains_known_edge() {
        let root = crate::repo_root();
        let live = collect_live_edges(&root).expect("cargo metadata");
        assert!(live.contains(&("vox-cli".to_string(), "vox-cli-ci".to_string())));
        assert!(
            live.len() > 400,
            "expected hundreds of in-tree edges, got {}",
            live.len()
        );
    }

    #[test]
    fn tighten_is_removal_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("contracts/ci")).unwrap();
        write_allow_file(&dir.path().join(ALLOW_REL), &allow(&[("app", "lib")], &[])).unwrap();
        let err = tighten(dir.path(), &edges(&[("app", "lib"), ("app", "extra")])).unwrap_err();
        assert!(err.to_string().contains("removal-only"), "{err}");
        tighten(dir.path(), &edges(&[])).unwrap();
        let f: AllowFile =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join(ALLOW_REL)).unwrap())
                .unwrap();
        assert!(f.edges.is_empty());
    }

    #[test]
    fn tighten_bootstraps_when_missing_and_keeps_live_exceptions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("contracts/ci")).unwrap();
        tighten(dir.path(), &edges(&[("app", "lib")])).unwrap();
        let f: AllowFile =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join(ALLOW_REL)).unwrap())
                .unwrap();
        assert_eq!(f.edges, vec![["app".to_string(), "lib".to_string()]]);
        write_allow_file(
            &dir.path().join(ALLOW_REL),
            &allow(&[("app", "lib")], &[("app", "lib")]),
        )
        .unwrap();
        tighten(dir.path(), &edges(&[("app", "lib")])).unwrap();
        let f: AllowFile =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join(ALLOW_REL)).unwrap())
                .unwrap();
        assert!(
            f.edges.is_empty(),
            "excepted pair must not also sit in edges"
        );
        assert_eq!(f.exceptions.len(), 1);
    }

    #[test]
    fn run_verify_mode_bails_when_allow_file_missing() {
        let dir = fake_workspace();
        let err = run(dir.path(), false).unwrap_err();
        assert!(err.to_string().contains(ALLOW_REL), "{err}");
        assert!(err.to_string().contains("--tighten"), "{err}");
    }

    #[test]
    fn run_verify_mode_bails_when_layers_file_missing() {
        let dir = fake_workspace();
        std::fs::create_dir_all(dir.path().join("contracts/ci")).unwrap();
        write_allow_file(&dir.path().join(ALLOW_REL), &allow(&[], &[])).unwrap();
        let err = run(dir.path(), false).unwrap_err();
        assert!(err.to_string().contains(LAYERS_REL), "{err}");
    }

    #[test]
    fn run_tighten_mode_writes_baseline_but_never_the_layer_map() {
        let dir = fake_workspace();
        std::fs::create_dir_all(dir.path().join("contracts/ci")).unwrap();
        run(dir.path(), true).unwrap();
        assert!(
            dir.path().join(ALLOW_REL).exists(),
            "tighten mode must write the allow baseline"
        );
        assert!(
            !dir.path().join(LAYERS_REL).exists(),
            "layers.toml is hand-authored; crate-edges must never generate it"
        );
    }
}
