use super::*;

/// `contracts/reports/arch/criteria-format/<UTC>.json` and exits 0 (clean) /
/// 1 (violations) / 2 (cannot read the doc).
pub(crate) fn run_criteria_format_lint() -> ExitCode {
    use vox_arch_check::criteria_format::check_criteria_format;

    // The crate lives at `crates/vox-arch-check`, so `../..` is the workspace root.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let doc_path = root.join("docs/src/architecture/v1-release-criteria.md");
    let doc = match std::fs::read_to_string(&doc_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "vox-arch-check --lint criteria-format: cannot read {}: {e}",
                doc_path.display()
            );
            return ExitCode::from(2);
        }
    };

    let (met, errors) = match check_criteria_format(&doc) {
        Ok(()) => (true, Vec::new()),
        Err(errs) => (false, errs),
    };

    // The artifact is part of the lint contract — a write failure must surface
    // (exit 2 = infra error), not let the command exit "clean" with no report.
    let out_dir = root.join("contracts/reports/arch/criteria-format");
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!(
            "vox-arch-check --lint criteria-format: cannot create {}: {e}",
            out_dir.display()
        );
        return ExitCode::from(2);
    }
    let body = serde_json::json!({
        "schema_version": 1,
        "criterion": "CR-META",
        "measured_at": chrono::Utc::now().to_rfc3339(),
        "errors": errors,
        "threshold": { "target": "all blocks well-formed", "met": met },
    });
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let out_path = out_dir.join(format!("{date}.json"));
    let body_pretty = match serde_json::to_string_pretty(&body) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("vox-arch-check --lint criteria-format: cannot serialize report: {e}");
            return ExitCode::from(2);
        }
    };
    if let Err(e) = std::fs::write(&out_path, body_pretty) {
        eprintln!(
            "vox-arch-check --lint criteria-format: cannot write {}: {e}",
            out_path.display()
        );
        return ExitCode::from(2);
    }

    if met {
        println!("CR-META: criteria doc well-formed.");
        ExitCode::SUCCESS
    } else {
        for e in &errors {
            eprintln!("CR-META: {e}");
        }
        ExitCode::from(1)
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_secondary_rules(
    report: &mut Report,
    metadata_full: &cargo_metadata::Metadata,
    layers: &LayersConfig,
    workspace_root: &Path,
    prune_dirs: &HashSet<String>,
    cache_key: &Option<String>,
    cached: &Option<cache::CachedData>,
    workspace_dep_count: &HashMap<String, usize>,
    prof: &dyn Fn(&str, &mut std::time::Instant),
    mut prof_last: std::time::Instant,
    profile_on: bool,
    profile_start: std::time::Instant,
) {
    let workspace_root = workspace_root.to_path_buf();
    // ── Rule 8: Staleness ──
    // Flags crates with no commits since the last release date in CHANGELOG.md.
    // Plugins (independent versioning) and staleness_exempt crates are skipped.
    let changelog_path = workspace_root.join("CHANGELOG.md");
    // Cache-aware: use cached git paths on hit; run git and cache the result on miss.
    let mut touched_paths_for_cache: Option<Vec<String>> = None;
    if let Some((release_version, release_date)) = parse_release_date(&changelog_path) {
        report.staleness_since = format!("v{release_version} ({release_date})");
        let touched_from_cache = cached
            .as_ref()
            .and_then(|c| c.git_touched_paths.as_ref())
            .map(|paths| paths.iter().cloned().collect::<HashSet<String>>());
        let touched_result =
            touched_from_cache.or_else(|| git_paths_touched_since(&workspace_root, &release_date));
        if let Some(ref touched_paths) = touched_result {
            touched_paths_for_cache = Some(touched_paths.iter().cloned().collect());
        }
        if let Some(ref touched) = touched_result {
            for pkg in metadata_full.workspace_packages() {
                let name = pkg.name.as_str();
                let entry = match layers.crates.get(name) {
                    Some(e) => e,
                    None => continue,
                };
                if entry.staleness_exempt || entry.kind == "plugin" {
                    continue;
                }
                let manifest_path = Path::new(pkg.manifest_path.as_str());
                let Some(rel_dir) = manifest_parent_rel_to_repo(&workspace_root, manifest_path)
                else {
                    continue;
                };
                let touched_this_crate = touched
                    .iter()
                    .any(|p| git_path_touches_crate_root(p, &rel_dir));
                if !touched_this_crate {
                    report.staleness_warns.push((
                        name.to_string(),
                        format!("no commits touching crate on/after {release_date}"),
                    ));
                }
            }
        } else {
            // vox-arch-check: allow git-exec — batched log failed; fall back to per-crate probe.
            for pkg in metadata_full.workspace_packages() {
                let name = pkg.name.as_str();
                let entry = match layers.crates.get(name) {
                    Some(e) => e,
                    None => continue,
                };
                if entry.staleness_exempt || entry.kind == "plugin" {
                    continue;
                }
                let manifest_dir = Path::new(pkg.manifest_path.as_str())
                    .parent()
                    .unwrap_or(Path::new("."));
                if let Some(last_commit) = git_last_commit_date(manifest_dir)
                    && last_commit < release_date
                {
                    report.staleness_warns.push((name.to_string(), last_commit));
                }
            }
        }
        report.staleness_warns.sort();
    }

    // Persist cache on miss so next run is faster (non-fatal on failure).
    if let (Some(key), None) = (&cache_key, &cached) {
        let data = cache::CachedData {
            key: key.clone(),
            git_touched_paths: touched_paths_for_cache,
        };
        let _ = cache::store(&workspace_root, &data);
    }

    prof("rule 8 (staleness/git, with cache)", &mut prof_last);
    // ── Rule 9: Generated-file drift ──
    report.generated_file_drift_warns = check_generated_file_drift(&workspace_root, &prune_dirs)
        .unwrap_or_else(|e| {
            eprintln!("warn: generated-file-drift check skipped: {e:#}");
            Vec::new()
        });

    prof("rule 9 (generated-file drift)", &mut prof_last);
    // ── Rule 10: Forbidden direct dependencies ──
    if !layers.forbidden_deps.is_empty() {
        let forbidden_set: Vec<(&str, Vec<&str>)> = layers
            .forbidden_deps
            .iter()
            .map(|r| {
                (
                    r.krate.as_str(),
                    r.forbidden.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();
        for pkg in metadata_full.workspace_packages() {
            let krate_name = pkg.name.as_str();
            let rules_for_crate: Vec<&Vec<&str>> = forbidden_set
                .iter()
                .filter(|(k, _)| *k == krate_name)
                .map(|(_, f)| f)
                .collect();
            if rules_for_crate.is_empty() {
                continue;
            }
            for dep in &pkg.dependencies {
                let dep_name = dep.name.as_str();
                for forbidden_list in &rules_for_crate {
                    if forbidden_list.contains(&dep_name) {
                        report
                            .forbidden_dep_violations
                            .push((krate_name.to_string(), dep_name.to_string()));
                    }
                }
            }
        }
        report.forbidden_dep_violations.sort();
        report.forbidden_dep_violations.dedup();
    }

    prof("rule 10 (forbidden deps)", &mut prof_last);
    // ── Rule 11: Forbidden code patterns (P3-T7) ──
    // Single batched walk of the workspace; all 9 patterns evaluated per file.
    if !layers.forbidden_pattern.is_empty() {
        match scan_forbidden_patterns_all(&workspace_root, &layers.forbidden_pattern, &prune_dirs) {
            Ok(hits) => {
                for hit in hits {
                    report.forbidden_pattern_hits.push((
                        hit.rule,
                        hit.file,
                        hit.line,
                        hit.matched,
                        hit.reason,
                    ));
                }
            }
            Err(e) => {
                eprintln!("warn: forbidden_pattern scan skipped: {e:#}");
            }
        }
        report
            .forbidden_pattern_hits
            .sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
    }

    prof("rule 11 (forbidden patterns, batched)", &mut prof_last);
    // ── Rule 12: WTL / layers.toml / disk three-way parity ──
    report.wtl_parity_warns = check_wtl_parity(&layers, &workspace_root);

    prof("rule 12 (WTL parity)", &mut prof_last);
    // ── Rule 13: LoC delta regression ──
    // Warn if any budgeted crate has grown >15% vs. the last tagged release.
    // Only fires for crates >2000 LoC to avoid noise from tiny utilities.
    if let Some((release_version, _)) = parse_release_date(&workspace_root.join("CHANGELOG.md")) {
        let tag = format!("v{release_version}");
        // First pass: collect (name, manifest_dir, current_loc) for budgeted crates
        // whose current LoC is above the 2000-line floor.
        let mut candidates: Vec<(String, PathBuf, usize)> = Vec::new();
        for pkg in metadata_full.workspace_packages() {
            let name = pkg.name.as_str();
            let entry = match layers.crates.get(name) {
                Some(e) => e,
                None => continue,
            };
            if entry.max_loc.is_none() {
                continue;
            }
            let manifest_dir = Path::new(pkg.manifest_path.as_str())
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf();
            let src_dir = manifest_dir.join("src");
            let current_loc = count_loc(&src_dir, &prune_dirs).unwrap_or(0);
            if current_loc < 2000 {
                continue;
            }
            candidates.push((name.to_string(), manifest_dir, current_loc));
        }
        // Second pass: fetch baseline LoC for ALL candidates in a single
        // `git cat-file --batch` invocation, then compute deltas.
        let manifest_dirs: Vec<PathBuf> = candidates.iter().map(|(_, m, _)| m.clone()).collect();
        if let Some(baselines) = git_loc_at_tag_batch(&tag, &workspace_root, &manifest_dirs) {
            for (name, manifest_dir, current_loc) in &candidates {
                let Some(&baseline) = baselines.get(manifest_dir) else {
                    continue;
                };
                if baseline == 0 {
                    continue;
                }
                let growth = (*current_loc as f64 - baseline as f64) / baseline as f64 * 100.0;
                if growth > 15.0 {
                    report
                        .loc_delta_warns
                        .push((name.clone(), *current_loc, baseline, growth));
                }
            }
        }
        report
            .loc_delta_warns
            .sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    }

    prof("rule 13 (LoC delta, per-crate git show)", &mut prof_last);
    // ── Rule 14: No cdylib-as-normal-dep ──
    // Plugin cdylibs are loaded dynamically at runtime via vox-plugin-host.
    // Linking them statically as a normal compile-time dep breaks the plugin
    // boundary. Only non-optional, non-dev deps are checked.
    //
    // Scope: only crates marked `kind = "plugin"` in layers.toml are subject to
    // this rule. A crate may legitimately produce a `cdylib`/`staticlib` target
    // for non-plugin reasons (e.g. a uniffi mobile FFI bridge built alongside a
    // normal `rlib`); such dual-purpose `library` crates are linked as `rlib`
    // in-tree and must not be flagged. The plugin boundary is defined by
    // `kind = "plugin"`, not merely by the presence of a `cdylib` target.
    {
        let cdylib_pkg_names: HashSet<&str> = metadata_full
            .workspace_packages()
            .iter()
            .filter(|p| {
                p.targets
                    .iter()
                    .any(|t| t.kind.contains(&cargo_metadata::TargetKind::CDyLib))
                    && layers
                        .crates
                        .get(p.name.as_str())
                        .map(|e| e.kind == "plugin")
                        .unwrap_or(false)
            })
            .map(|p| p.name.as_str())
            .collect();

        if !cdylib_pkg_names.is_empty() {
            for pkg in metadata_full.workspace_packages() {
                if pkg
                    .targets
                    .iter()
                    .any(|t| t.kind.contains(&cargo_metadata::TargetKind::CDyLib))
                {
                    continue; // cdylib can depend on another cdylib (rare but not our concern here)
                }
                for dep in &pkg.dependencies {
                    if dep.kind != cargo_metadata::DependencyKind::Normal || dep.optional {
                        continue;
                    }
                    if cdylib_pkg_names.contains(dep.name.as_str()) {
                        report
                            .cdylib_dep_warns
                            .push((pkg.name.to_string(), dep.name.clone()));
                    }
                }
            }
            report.cdylib_dep_warns.sort();
            report.cdylib_dep_warns.dedup();
        }
    }

    prof("rule 14 (cdylib dep)", &mut prof_last);
    // ── Rule 15: Workspace-dep budget ──
    for (name, entry) in &layers.crates {
        if let Some(budget) = entry.max_workspace_deps {
            let count = workspace_dep_count.get(name).copied().unwrap_or(0);
            if count > budget {
                report
                    .workspace_dep_warns
                    .push((name.clone(), count, budget));
            }
        }
    }
    report
        .workspace_dep_warns
        .sort_by_key(|w| std::cmp::Reverse(w.1));
    prof("rule 15 (workspace-dep budget)", &mut prof_last);
    if profile_on {
        eprintln!("[profile] TOTAL: {}ms", profile_start.elapsed().as_millis());
    }

    // ── Rule 14: evidence-ledger integrity ─────────────────────────────────
    // Runs against `contracts/reports/evidence-ledger.v1.json`; emits
    // findings about missing/stale artifacts. Defaults to warn-only — the
    // strict flip happens once block-GA gates are all met (per the honest
    // plan §10). The lint is `strict_evidence_ledger = false` until then.
    if workspace_root
        .join("contracts/reports/evidence-ledger.v1.json")
        .exists()
    {
        match evidence_ledger::check_evidence_ledger(&workspace_root) {
            Ok(findings) => {
                report.evidence_findings = findings;
            }
            Err(e) => {
                eprintln!("[evidence-ledger] WARN: failed to load ledger: {e:#}");
            }
        }
    }
    // Default strictness: false. Tighten via `--strict-evidence` flag once
    // §10 acceptance fires.
    report.strict_evidence_ledger = std::env::args().any(|a| a == "--strict-evidence");
}
