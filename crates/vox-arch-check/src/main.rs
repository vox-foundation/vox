//! Architecture check: enforces workspace-wide structural rules.
//!
//! Reads `docs/src/architecture/layers.toml` and runs thirteen rules over the
//! current `cargo metadata` snapshot:
//!
//!   1. **Layer ordering** (strict by default) — a crate at layer N may depend
//!      only on crates at layer ≤ N. Inversions in `[[known_inversions]]` are
//!      tolerated.
//!   2. **Fan-in tracker** (warn) — workspace dependents per crate vs.
//!      `max_dependents`.
//!   3. **LoC budget** (warn) — `wc -l` over `src/**/*.rs` vs. `max_loc`.
//!   4. **Orphan detector** (warn) — flags crates with 0 in-tree consumers
//!      AND `kind != "plugin" | "binary" | "test-only"`.
//!   5. **Docstring lint** (strict for L0-L2, warn for L3+) — flags `lib.rs`
//!      files that don't open with `//!`.
//!   6. **Description present** (warn) — flags L1+ library crates with no
//!      Cargo.toml `description` or one shorter than 40 characters.
//!   7. **Where-things-live coverage** (warn) — flags workspace members not
//!      listed in `docs/src/architecture/where-things-live.md`.
//!   8. **Staleness** (warn) — flags crates with no commits since the last
//!      release date in `CHANGELOG.md`. Mark stable utility crates with
//!      `staleness_exempt = true` in `layers.toml` to silence the warning.
//!      Implemented with one batched `git log` over the repo when possible,
//!      falling back to per-crate `git log` if the batch query fails.
//!   9. **Generated-file drift** (warn) — files containing a
//!      `@generated-hash <hex>` header whose content hash no longer matches,
//!      indicating a hand-edit of a machine-generated file.
//!  10. **Forbidden direct dependencies** (error) — crates must not directly
//!      depend on any deps listed under `[[forbidden_deps]]` in `layers.toml`.
//!  11. **Forbidden code patterns** (warn) — patterns in `[[forbidden_pattern]]`
//!      that must not appear in source (e.g. raw `Command::new("git")`).
//!  12. **WTL / layers.toml / disk three-way parity** (error) — (a) every
//!      `[crates]` entry in `layers.toml` whose directory is absent from
//!      `crates/` AND is not in the `[planned]` table; (b) every
//!      `crates/<name>/` reference in `where-things-live.md` whose directory
//!      is absent from disk AND is not in `[planned]`. Add ghost entries to
//!      `[planned]` with a `plan =` pointer to suppress the warning.
//!  13. **LoC delta regression** (warn) — for crates with `max_loc` budgets,
//!      warns if the current LoC is more than 15% higher than the LoC at the
//!      last tagged release (`v{version}` from `CHANGELOG.md`). Only fires for
//!      crates >2000 LoC to avoid noise. Catches regrowth at PR time rather
//!      than at the hard ceiling.
//!  14. **No-cdylib-as-normal-dep** (error by default) — a workspace crate
//!      must not take a non-optional, non-dev compile-time dependency on a
//!      workspace crate whose output is a `cdylib`. Plugin cdylibs are loaded
//!      dynamically at runtime via `vox-plugin-host`; linking them statically
//!      breaks the plugin boundary and bloats the binary.
//!  15. **Workspace-dep budget** (warn) — caps how many workspace-member
//!      crates a given crate may take as normal (compile-time) dependencies.
//!      Set `max_workspace_deps` in `layers.toml` per-crate. Useful for
//!      detecting "kitchen-sink" aggregator crates before they hit the hard
//!      fan-in / LoC ceilings (e.g. `vox-cli = 60`).
//!
//! Layer ordering is the only rule that fails the build by default; the other
//! fourteen rules are warn-only unless promoted via `[guards]` in `layers.toml`.
//!
//! Modes:
//!   default        — strict layer-ordering; warn-only on the other eight
//!   --warn-only    — warn on layer-ordering too (used during transition phases)
//!
//! Exit codes:
//!   0 — clean (or warn-only)
//!   1 — strict rule failed, OR config error

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, anyhow};
use cargo_metadata::MetadataCommand;

mod cache;
mod checks;
mod forbidden_patterns;
mod json_report;
use forbidden_patterns::scan_all as scan_forbidden_patterns_all;

mod rule_runners;
use rule_runners::{run_criteria_format_lint, run_secondary_rules};
/// Rule 14: evidence-ledger integrity check.
/// See `evidence_ledger.rs` for the contract.
mod evidence_ledger;
use evidence_ledger::{EvidenceFinding, FindingKind};

mod config_types;
mod git_and_walk;
mod report_model;
use config_types::*;
use git_and_walk::*;
use report_model::*;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();

    // CR-META: `vox-arch-check --lint criteria-format` runs only the
    // criteria-doc format lint and exits, bypassing the layered-arch run.
    if let Some(i) = argv.iter().position(|a| a == "--lint")
        && argv.get(i + 1).map(String::as_str) == Some("criteria-format")
    {
        return run_criteria_format_lint();
    }

    let warn_only = argv.iter().any(|a| a == "--warn-only");
    let json = argv.iter().any(|a| a == "--json");

    match run(warn_only) {
        Ok(report) => {
            if json {
                // Machine-readable per-rule results on stdout for the policy-status
                // overlay; the human text summary still goes to stderr so plain
                // (non-`--json`) behavior is byte-for-byte unchanged.
                let results = report.to_rule_results();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into())
                );
            } else {
                report.print_summary();
            }
            if report.strict_failed() && !warn_only {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("vox-arch-check: {e:#}");
            ExitCode::FAILURE
        }
    }
}

// CR-META: lint `docs/src/architecture/v1-release-criteria.md` so every
// `[CR-*]` definition block declares `verify_cmd`, `artifact_path`, and a
// non-empty `if_failing`.

fn run(warn_only_flag: bool) -> Result<Report> {
    // `--no-deps` skips transitive dependency resolution. arch-check never reads
    // `metadata.resolve`; each package's declared `dependencies` is still
    // populated so Rules 10/11/14/15 work unchanged.
    let metadata_full = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("cargo metadata failed")?;

    let workspace_root: PathBuf = metadata_full.workspace_root.clone().into();

    // Load or prime the git-paths cache. Non-fatal: a miss just means we run git normally.
    let cache_key = cache::compute_key(&workspace_root).ok();
    let cached = cache_key
        .as_deref()
        .and_then(|k| cache::load(&workspace_root, k));

    let layers_path = workspace_root.join("docs/src/architecture/layers.toml");

    let layers_text = std::fs::read_to_string(&layers_path)
        .with_context(|| format!("reading {}", layers_path.display()))?;
    let layers: LayersConfig = toml::from_str(&layers_text)
        .with_context(|| format!("parsing {}", layers_path.display()))?;
    let prune_dirs = walk_prune_dir_names(&layers);

    let workspace_members: HashSet<&str> = metadata_full
        .workspace_packages()
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    // Layer ordering is strict by default; the others default to warn-only.
    // --warn-only flag downgrades layer ordering to warn too.
    let mut report = Report {
        strict_layer: !warn_only_flag,
        strict_fan_in: parse_strictness(layers.guards.fan_in.as_ref(), false),
        strict_loc: parse_strictness(layers.guards.loc_budget.as_ref(), false),
        loc_budget_off: loc_budget_disabled(layers.guards.loc_budget.as_ref()),
        strict_orphan: parse_strictness(layers.guards.orphan.as_ref(), false),
        strict_docstring: parse_strictness(layers.guards.docstring.as_ref(), false),
        strict_description: parse_strictness(layers.guards.description.as_ref(), false),
        strict_where_things_live: parse_strictness(layers.guards.where_things_live.as_ref(), false),
        strict_staleness: parse_strictness(layers.guards.staleness.as_ref(), false),
        strict_generated_file_drift: parse_strictness(
            layers.guards.generated_file_drift.as_ref(),
            false,
        ),
        strict_forbidden_deps: parse_strictness(layers.guards.forbidden_deps.as_ref(), false),
        strict_forbidden_pattern: parse_strictness(layers.guards.forbidden_pattern.as_ref(), false),
        strict_wtl_parity: parse_strictness(layers.guards.wtl_parity.as_ref(), false),
        strict_loc_delta: parse_strictness(layers.guards.loc_delta.as_ref(), false),
        strict_cdylib_dep: parse_strictness(layers.guards.cdylib_dep.as_ref(), true),
        strict_workspace_deps: parse_strictness(layers.guards.workspace_deps.as_ref(), false),
        ..Report::default()
    };

    let profile_on = std::env::var("VOX_ARCH_CHECK_PROFILE").is_ok();
    let profile_start = std::time::Instant::now();
    let mut prof_last = profile_start;
    let prof = |label: &str, prev: &mut std::time::Instant| {
        if profile_on {
            let now = std::time::Instant::now();
            eprintln!(
                "[profile] {}: {}ms",
                label,
                now.duration_since(*prev).as_millis()
            );
            *prev = now;
        }
    };
    prof("setup (metadata+layers+cache)", &mut prof_last);

    // ── Rule 1: Layer ordering + Rule 2: Fan-in + Rule 4: Orphan + Rule 15: Workspace-dep budget + Rule 17: Cycles (single pass) ──
    // `dependent_count` (Rule 4, orphan detection: "does anyone at all depend on
    // this, even via a dev-dep") counts every dependency edge, unfiltered.
    // `raw_fan_in_edges` narrows to Rule 2's architectural fan-in question
    // ("how many production consumers") via `compute_dependent_counts` below.
    let mut dependent_count: HashMap<String, usize> = HashMap::new();
    let mut workspace_dep_count: HashMap<String, usize> = HashMap::new();
    let mut unlisted: Vec<String> = Vec::new();
    // Rule 17: collect normal dep edges for Tarjan SCC cycle detection.
    let mut normal_dep_edges: Vec<(String, String)> = Vec::new();
    // Rule 2: raw (from, to, kind) edges, filtered down to a live fan-in count below.
    let mut raw_fan_in_edges: Vec<(String, String, cargo_metadata::DependencyKind)> = Vec::new();

    for pkg in metadata_full.workspace_packages() {
        let from_name = pkg.name.as_str();
        let from_layer = match layers.crates.get(from_name) {
            Some(e) => e.layer,
            None => {
                unlisted.push(from_name.to_string());
                continue;
            }
        };
        for dep in &pkg.dependencies {
            let to_name = dep.name.as_str();
            if !workspace_members.contains(to_name) {
                continue;
            }
            *dependent_count.entry(to_name.to_string()).or_insert(0) += 1;
            raw_fan_in_edges.push((from_name.to_string(), to_name.to_string(), dep.kind));
            if dep.kind == cargo_metadata::DependencyKind::Normal {
                *workspace_dep_count
                    .entry(from_name.to_string())
                    .or_insert(0) += 1;
                // Collect for Rule 17 Tarjan cycle detection.
                normal_dep_edges.push((from_name.to_string(), to_name.to_string()));
            }

            // Layer-inversion check only applies to normal (production) deps.
            // Dev-deps and build-deps don't affect the runtime binary's layer purity.
            if dep.kind != cargo_metadata::DependencyKind::Normal {
                continue;
            }
            let to_layer = match layers.crates.get(to_name) {
                Some(e) => e.layer,
                None => continue,
            };
            if to_layer > from_layer {
                let is_known = layers
                    .known_inversions
                    .iter()
                    .any(|k| k.from == from_name && k.to == to_name);
                if !is_known {
                    report.inversions.push((
                        from_name.to_string(),
                        to_name.to_string(),
                        from_layer,
                        to_layer,
                    ));
                }
            }
        }
    }

    let fan_in_count = compute_dependent_counts(&raw_fan_in_edges);

    if !unlisted.is_empty() {
        unlisted.sort();
        unlisted.dedup();
        return Err(anyhow!(
            "{} workspace crate(s) missing from layers.toml: {}",
            unlisted.len(),
            unlisted.join(", ")
        ));
    }

    // ── Rule 17: Cycle detection (Tarjan SCC) ──
    report.cycle_errors = checks::cycles::find_cycles(&normal_dep_edges);

    // ── Rule 18: Publishability closure gate ──
    {
        // Build adjacency from required (non-optional) workspace normal deps only.
        // Optional deps are excluded: `cargo publish` does not require them to be on
        // crates.io (they can be satisfied by a feature flag that is never enabled).
        let mut ws_required_deps: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for pkg in metadata_full.workspace_packages() {
            for dep in &pkg.dependencies {
                if dep.kind != cargo_metadata::DependencyKind::Normal {
                    continue;
                }
                if dep.optional {
                    continue;
                }
                let to = dep.name.as_str();
                if !workspace_members.contains(to) {
                    continue;
                }
                ws_required_deps
                    .entry(pkg.name.to_string())
                    .or_default()
                    .push(to.to_string());
            }
        }
        // Detect publish_false via cargo_metadata: publish == Some([]) means publish = false.
        let crate_recs: Vec<checks::publishable::CrateRec> = metadata_full
            .workspace_packages()
            .iter()
            .map(|pkg| {
                let publish_false = pkg.publish.as_deref() == Some(&[]);
                let publishable = layers
                    .crates
                    .get(pkg.name.as_str())
                    .map(|e| e.publishable)
                    .unwrap_or(false);
                checks::publishable::CrateRec {
                    name: pkg.name.to_string(),
                    deps: ws_required_deps
                        .get(pkg.name.as_str())
                        .cloned()
                        .unwrap_or_default(),
                    publish_false,
                    publishable,
                }
            })
            .collect();
        report.publishability_errors = checks::publishable::check(&crate_recs);
    }

    // ── Rule 19: Dep-closure-size budget gate ──
    {
        let budgets_path = workspace_root.join("contracts/reports/closure-budgets.v1.json");
        if budgets_path.exists() {
            let raw = std::fs::read_to_string(&budgets_path)
                .with_context(|| format!("reading {}", budgets_path.display()))?;
            let parsed: serde_json::Value =
                serde_json::from_str(&raw).with_context(|| "parsing closure-budgets.v1.json")?;
            let budgets: Vec<checks::closure_budget::Budget> = parsed["budgets"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|entry| {
                    let name = entry["crate_name"].as_str()?.to_string();
                    let max = entry["max_closure"].as_u64()? as usize;
                    Some(checks::closure_budget::Budget {
                        crate_name: name,
                        max_closure: max,
                    })
                })
                .collect();
            report.closure_budget_errors =
                checks::closure_budget::check(&normal_dep_edges, &budgets);
        }
    }

    // ── Rule 20: Profile forbidden-crate gate ──
    for (profile_name, profile_cfg) in &layers.profiles {
        if profile_cfg.forbidden.is_empty() {
            continue;
        }
        // Run cargo tree for the profile; skip silently if cargo tree fails.
        let tree_out = std::process::Command::new("cargo")
            .args([
                "tree",
                "-p",
                "vox-cli",
                "--no-default-features",
                "--features",
                "script-execution",
                "-e",
                "normal",
                "--prefix",
                "none",
            ])
            .output();
        let lean_tree: Vec<String> = match tree_out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.split_whitespace().next())
                .map(|s| s.to_string())
                .collect(),
            // Feature may not exist; try without --features
            _ => {
                let o2 = std::process::Command::new("cargo")
                    .args([
                        "tree",
                        "-p",
                        "vox-cli",
                        "--no-default-features",
                        "-e",
                        "normal",
                        "--prefix",
                        "none",
                    ])
                    .output();
                match o2 {
                    Ok(o2) if o2.status.success() => String::from_utf8_lossy(&o2.stdout)
                        .lines()
                        .filter_map(|l| l.split_whitespace().next())
                        .map(|s| s.to_string())
                        .collect(),
                    _ => vec![],
                }
            }
        };
        if !lean_tree.is_empty() {
            let mut errs =
                check_profile_forbidden(profile_name, &lean_tree, &profile_cfg.forbidden);
            report.profile_forbidden_errors.append(&mut errs);
        }
    }

    // Rule 2: fan-in budget
    for (name, entry) in &layers.crates {
        if let Some(budget) = entry.max_dependents {
            let count = fan_in_count.get(name).copied().unwrap_or(0);
            if count > budget {
                report.fan_in_warns.push((name.clone(), count, budget));
            }
        }
    }

    prof("rules 1+2+15 (layer/fan-in/wsdep)", &mut prof_last);
    // ── Rule 3: LoC budget ──
    // Skipped wholesale when `loc_budget = "off"`. Rule 13 (`loc_delta`, below)
    // still runs and still reads the same `max_loc` values as its growth
    // baseline — that is the whole reason the budgets stay in layers.toml.
    if !report.loc_budget_off {
        for pkg in metadata_full.workspace_packages() {
            let name = pkg.name.as_str();
            let entry = match layers.crates.get(name) {
                Some(e) => e,
                None => continue,
            };
            let budget = match entry.max_loc {
                Some(b) => b,
                None => continue,
            };
            let manifest_dir = Path::new(pkg.manifest_path.as_str())
                .parent()
                .unwrap_or(Path::new("."));
            let src_dir = manifest_dir.join("src");
            let loc = count_loc(&src_dir, &prune_dirs).unwrap_or(0);
            if loc > budget {
                report.loc_warns.push((name.to_string(), loc, budget));
            }
        }
    }

    prof(
        if report.loc_budget_off {
            "rule 3 (LoC budget) — skipped, guard off"
        } else {
            "rule 3 (LoC budget)"
        },
        &mut prof_last,
    );
    // ── Rule 4: Orphan detector ──
    for (name, entry) in &layers.crates {
        if entry.kind != "library" || entry.orphan_exempt {
            continue;
        }
        let count = dependent_count.get(name).copied().unwrap_or(0);
        if count == 0 && workspace_members.contains(name.as_str()) {
            report.orphan_warns.push(name.clone());
        }
    }
    report.orphan_warns.sort();

    prof("rule 4 (orphan)", &mut prof_last);
    // ── Rule 5: Docstring lint (strict for L0-L2, warn for L3+) ──
    for pkg in metadata_full.workspace_packages() {
        let name = pkg.name.as_str();
        let layer = match layers.crates.get(name) {
            Some(e) => e.layer,
            None => continue,
        };
        let manifest_dir = Path::new(pkg.manifest_path.as_str())
            .parent()
            .unwrap_or(Path::new("."));
        let lib_rs = manifest_dir.join("src").join("lib.rs");
        if !lib_rs.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&lib_rs) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let first_nonempty = content.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        if !first_nonempty.trim_start().starts_with("//!") {
            // L0-L2: strict (always fail); L3+: governed by strict_docstring guard
            let is_strict = layer <= 2;
            report.docstring_warns.push((name.to_string(), is_strict));
        }
    }
    report.docstring_warns.sort_by(|a, b| a.0.cmp(&b.0));

    prof("rule 5 (docstring lint)", &mut prof_last);
    // ── Rule 6: Description present ──
    report.description_warns = check_description_present(&metadata_full, &layers);

    prof("rule 6 (description)", &mut prof_last);
    // ── Rule 7: Where-things-live coverage ──
    report.where_things_live_warns =
        check_where_things_live_coverage(&metadata_full, &layers, &workspace_root).unwrap_or_else(
            |e| {
                eprintln!("warn: where-things-live check skipped: {e:#}");
                Vec::new()
            },
        );

    prof("rule 7 (WTL coverage)", &mut prof_last);
    run_secondary_rules(
        &mut report,
        &metadata_full,
        &layers,
        &workspace_root,
        &prune_dirs,
        &cache_key,
        &cached,
        &workspace_dep_count,
        &prof,
        prof_last,
        profile_on,
        profile_start,
    );
    Ok(report)
}

#[cfg(test)]
mod walk_and_staleness_tests {
    use super::*;

    #[test]
    fn compute_dependent_counts_dedupes_same_pair_across_kinds() {
        // vox-codegen-ts depends on vox-compiler as both a normal and a dev
        // dependency in real Cargo.tomls; that must count as one dependent,
        // not two.
        let edges = vec![
            (
                "vox-codegen-ts".to_string(),
                "vox-compiler".to_string(),
                cargo_metadata::DependencyKind::Normal,
            ),
            (
                "vox-codegen-ts".to_string(),
                "vox-compiler".to_string(),
                cargo_metadata::DependencyKind::Development,
            ),
        ];
        let counts = compute_dependent_counts(&edges);
        assert_eq!(counts.get("vox-compiler"), Some(&1));
    }

    #[test]
    fn compute_dependent_counts_excludes_dev_only_edges() {
        // A crate that only reaches another via a dev-dependency (e.g. an
        // integration-test crate) doesn't ship in the binary closure, so it
        // must not count toward the fan-in budget.
        let edges = vec![(
            "vox-integration-tests".to_string(),
            "vox-compiler".to_string(),
            cargo_metadata::DependencyKind::Development,
        )];
        let counts = compute_dependent_counts(&edges);
        assert_eq!(counts.get("vox-compiler"), None);
    }

    #[test]
    fn compute_dependent_counts_includes_build_kind() {
        let edges = vec![(
            "vox-foo".to_string(),
            "vox-bar".to_string(),
            cargo_metadata::DependencyKind::Build,
        )];
        let counts = compute_dependent_counts(&edges);
        assert_eq!(counts.get("vox-bar"), Some(&1));
    }

    #[test]
    fn compute_dependent_counts_excludes_workspace_hack_either_direction() {
        let edges = vec![
            (
                "vox-foo".to_string(),
                "workspace-hack".to_string(),
                cargo_metadata::DependencyKind::Normal,
            ),
            (
                "workspace-hack".to_string(),
                "vox-bar".to_string(),
                cargo_metadata::DependencyKind::Normal,
            ),
        ];
        let counts = compute_dependent_counts(&edges);
        assert!(counts.is_empty(), "{counts:?}");
    }

    #[test]
    fn walk_prune_skips_target_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("crates/foo/src")).unwrap();
        std::fs::write(root.join("crates/foo/src/lib.rs"), "x").unwrap();
        std::fs::create_dir_all(root.join("crates/foo/target/debug")).unwrap();
        std::fs::write(root.join("crates/foo/target/debug/huge.bin"), [0u8; 4096]).unwrap();
        let prune = built_in_walk_prune_names();
        let files: Vec<_> = walk_repo_files(root, &prune)
            .into_iter()
            .map(|p| p.strip_prefix(root).unwrap().to_path_buf())
            .collect();
        assert!(
            files
                .iter()
                .any(|p| p == Path::new("crates/foo/src/lib.rs")),
            "{files:?}"
        );
        assert!(
            !files.iter().any(|p| p.to_string_lossy().contains("target")),
            "must not descend into target/: {files:?}"
        );
    }

    /// Locks in the Rule 13 line-count semantics. The batched
    /// `git cat-file --batch` reader counts lines manually from raw bytes, so
    /// we explicitly assert it matches `str::lines().count()` for every edge
    /// case that mattered in the original per-file `git show` path.
    #[test]
    fn count_lines_in_bytes_matches_str_lines() {
        for s in &[
            "",
            "\n",
            "foo",
            "foo\n",
            "foo\nbar",
            "foo\nbar\n",
            "foo\nbar\nbaz\n",
            "\n\n\n",
            "a\r\nb",
            "a\r\nb\r\n",
        ] {
            assert_eq!(
                count_lines_in_bytes(s.as_bytes()),
                s.lines().count(),
                "line count mismatch for {s:?}"
            );
        }
    }

    #[test]
    fn walk_prune_extra_from_layers_toml() {
        let cfg: LayersConfig = toml::from_str(
            r#"
[crates.dummy]
layer = 0
[arch_check.walk_prune]
extra_skip_dir_names = ["my_vendor"]
"#,
        )
        .expect("parse minimal layers");
        let prune = walk_prune_dir_names(&cfg);
        assert!(prune.contains("target"));
        assert!(prune.contains("my_vendor"));
    }

    #[test]
    fn git_path_touches_crate_root_prefix() {
        assert!(git_path_touches_crate_root(
            "crates/vox-cli/src/main.rs",
            "crates/vox-cli"
        ));
        assert!(git_path_touches_crate_root(
            "crates/vox-cli/Cargo.toml",
            "crates/vox-cli"
        ));
        assert!(!git_path_touches_crate_root(
            "crates/vox-other/src/lib.rs",
            "crates/vox-cli"
        ));
    }

    #[test]
    fn manifest_parent_rel_to_repo_normalizes() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        let mf = repo.join("crates/foo/Cargo.toml");
        assert_eq!(
            manifest_parent_rel_to_repo(repo, &mf).as_deref(),
            Some("crates/foo")
        );
    }

    #[test]
    fn parses_publishable_and_plugin_candidate() {
        let cfg: LayersConfig = toml::from_str(
            r#"
[crates.vox-crypto]
layer = 1
publishable = true

[crates.vox-gamify]
layer = 3
plugin_candidate = true
"#,
        )
        .expect("parse with publishable/plugin_candidate");
        let crypto = &cfg.crates["vox-crypto"];
        assert!(crypto.publishable, "vox-crypto should be publishable");
        assert!(
            !crypto.plugin_candidate,
            "vox-crypto should not be a plugin_candidate"
        );
        let gamify = &cfg.crates["vox-gamify"];
        assert!(!gamify.publishable, "vox-gamify should not be publishable");
        assert!(
            gamify.plugin_candidate,
            "vox-gamify should be a plugin_candidate"
        );
    }

    #[test]
    fn defaults_are_false_when_absent() {
        let cfg: LayersConfig = toml::from_str(
            r#"
[crates.plain-crate]
layer = 2
"#,
        )
        .expect("parse minimal crate entry");
        let entry = &cfg.crates["plain-crate"];
        assert!(!entry.publishable, "publishable defaults to false");
        assert!(
            !entry.plugin_candidate,
            "plugin_candidate defaults to false"
        );
    }

    #[test]
    fn rule20_flags_forbidden_crate_in_lean_profile() {
        let lean_tree = vec!["vox-cli".to_string(), "vox-gui".to_string()];
        let forbidden = vec!["vox-gui".to_string(), "vox-gamify".to_string()];
        let violations = check_profile_forbidden("lean", &lean_tree, &forbidden);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("vox-gui"));
    }

    #[test]
    fn rule20_clean_profile_passes() {
        let lean_tree = vec!["vox-cli".to_string(), "vox-ast".to_string()];
        let forbidden = vec!["vox-gui".to_string(), "vox-gamify".to_string()];
        let violations = check_profile_forbidden("lean", &lean_tree, &forbidden);
        assert!(violations.is_empty());
    }

    #[test]
    fn rule20_multiple_forbidden_all_reported() {
        let lean_tree = vec!["vox-gamify".to_string(), "vox-gui".to_string()];
        let forbidden = vec!["vox-gui".to_string(), "vox-gamify".to_string()];
        let violations = check_profile_forbidden("lean", &lean_tree, &forbidden);
        assert_eq!(violations.len(), 2);
    }
}

#[cfg(test)]
mod loc_budget_guard_tests {
    use super::loc_budget_disabled;

    #[test]
    fn loc_budget_off_disables_only_the_absolute_check() {
        assert!(loc_budget_disabled(Some(&"off".to_string())));
        assert!(loc_budget_disabled(Some(&"none".to_string())));
        // Existing values keep their meaning.
        assert!(!loc_budget_disabled(Some(&"warn".to_string())));
        assert!(!loc_budget_disabled(Some(&"error".to_string())));
        // Absent -> enabled, so removing the key cannot silently disable a guard.
        assert!(!loc_budget_disabled(None));
    }

    #[test]
    fn unrecognised_values_do_not_disable_the_guard() {
        // parse_strictness maps anything unknown to its default, so a typo like
        // "OFF" or "disabled" must NOT be treated as off — failing closed keeps
        // a misspelling from silently removing a gate.
        assert!(!loc_budget_disabled(Some(&"OFF".to_string())));
        assert!(!loc_budget_disabled(Some(&"disabled".to_string())));
        assert!(!loc_budget_disabled(Some(&String::new())));
    }
}
