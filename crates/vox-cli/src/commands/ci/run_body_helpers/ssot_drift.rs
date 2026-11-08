use anyhow::{Context, Result, anyhow};
use std::path::Path;

pub(crate) struct SsotDriftStage {
    pub name: &'static str,
    pub run: fn(&Path, bool) -> Result<()>,
    pub regen: Option<&'static [&'static str]>,
}

pub(crate) const REGEN_ONLY: &[(&[&str], &str)] = &[
    (
        &["ci", "crate-build-map-parity", "--write"],
        "crate build map derived fields (compile_s) come from crate-graph; not verified as a standalone ssot-drift stage",
    ),
    (
        &[
            "ci",
            "test-inventory",
            "--output",
            "contracts/reports/test-inventory.v1.json",
        ],
        "descriptive counts checked by nightly's snapshot gate, not ssot-drift",
    ),
];

pub(crate) fn run_stages(root: &Path, auto_heal: bool, stages: &[SsotDriftStage]) -> Result<()> {
    let mut failures = Vec::new();
    for stage in stages {
        let start = std::time::Instant::now();
        let res = (stage.run)(root, auto_heal);
        let elapsed = start.elapsed().as_millis();
        eprintln!("ssot-drift  {:>6}ms  {}", elapsed, stage.name);
        if let Err(e) = res {
            eprintln!("ssot-drift  FAIL  {}: {:#}", stage.name, e);
            failures.push((stage.name, e));
        }
    }
    if !failures.is_empty() {
        let n = failures.len();
        let names = failures
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(anyhow!(
            "ssot-drift: {n} stage(s) failed: {names} (later failures may cascade from earlier ones)"
        ));
    }
    Ok(())
}

pub(crate) fn regen_parity_failures(
    vox_src: &str,
    stages: &[SsotDriftStage],
    regen_only: &[(&[&str], &str)],
) -> Vec<String> {
    let mut failures = Vec::new();

    let regen_call_re = match regex::Regex::new(r"regen\s*\(\s*\[([^\]]*)\]") {
        Ok(re) => re,
        Err(e) => {
            failures.push(format!("compile regen regex: {e}"));
            return failures;
        }
    };
    let string_lit_re = match regex::Regex::new(r#""((?:\\.|[^"\\])*)""#) {
        Ok(re) => re,
        Err(e) => {
            failures.push(format!("compile string literal regex: {e}"));
            return failures;
        }
    };

    let mut parsed_calls: Vec<Vec<String>> = Vec::new();
    for cap in regen_call_re.captures_iter(vox_src) {
        let array_body = &cap[1];
        let mut args: Vec<String> = string_lit_re
            .captures_iter(array_body)
            .map(|c| c[1].to_string())
            .collect();
        if args.first().map(|s| s.as_str()) == Some("--quiet") {
            args.remove(0);
        }
        parsed_calls.push(args);
    }

    if parsed_calls.is_empty() {
        failures.push("0 regen() calls were parsed from source".to_string());
        return failures;
    }

    // Double listing check between stages and REGEN_ONLY
    for stage in stages {
        if let Some(argv) = stage.regen {
            let argv_strings: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
            for (ro, _) in regen_only {
                let ro_strings: Vec<String> = ro.iter().map(|s| s.to_string()).collect();
                if argv_strings == ro_strings {
                    failures.push(format!(
                        "argv {argv:?} listed both on stage '{}' and in REGEN_ONLY",
                        stage.name
                    ));
                }
            }
        }
    }

    let mut expected_argvs: Vec<Vec<String>> = Vec::new();
    for stage in stages {
        if let Some(argv) = stage.regen {
            expected_argvs.push(argv.iter().map(|s| s.to_string()).collect());
        }
    }
    for (ro, _) in regen_only {
        expected_argvs.push(ro.iter().map(|s| s.to_string()).collect());
    }

    for expected in &expected_argvs {
        if !parsed_calls.contains(expected) {
            failures.push(format!(
                "argv {expected:?} is not run by scripts/ssot-regen.vox"
            ));
        }
    }

    for parsed in &parsed_calls {
        if !expected_argvs.contains(parsed) {
            failures.push(format!(
                "unknown argv {parsed:?} run by scripts/ssot-regen.vox matches no stage and no REGEN_ONLY row"
            ));
        }
    }

    failures
}

pub(crate) fn lefthook_retry_failures(
    lefthook_yml: &str,
    stages: &[SsotDriftStage],
) -> Vec<String> {
    let mut failures = Vec::new();
    let val: serde_yaml::Value = match serde_yaml::from_str(lefthook_yml) {
        Ok(v) => v,
        Err(e) => {
            failures.push(format!("parse lefthook.yml: {e}"));
            return failures;
        }
    };

    let run_cmd = match val
        .get("pre-push")
        .and_then(|pp| pp.get("commands"))
        .and_then(|c| c.get("pre-push"))
        .and_then(|p| p.get("run"))
        .and_then(|r| r.as_str())
    {
        Some(s) => s,
        None => {
            failures.push("missing pre-push.commands.pre-push.run in lefthook.yml".to_string());
            return failures;
        }
    };

    let stage_regens: Vec<Vec<String>> = stages
        .iter()
        .filter_map(|s| {
            s.regen
                .map(|r| r.iter().map(|arg| arg.to_string()).collect())
        })
        .collect();

    let segments = run_cmd
        .split("||")
        .flat_map(|part| part.split("&&"))
        .map(|s| s.trim().trim_matches(|c| c == '(' || c == ')').trim())
        .filter(|s| !s.is_empty());

    for segment in segments {
        if segment.starts_with("vox run scripts/fmt.vox") {
            continue;
        }
        if let Some(rest) = segment.strip_prefix("vox ") {
            let words: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();
            if words.len() >= 2 && words[0] == "ci" {
                let subcmd = &words[1];
                if subcmd == "line-endings" || subcmd == "sync-ignore-files" || subcmd == "pre-push"
                {
                    continue;
                }
                if !stage_regens.contains(&words) {
                    failures.push(format!(
                        "generator call in lefthook retry chain matches no stage regen argv: {segment}"
                    ));
                }
            }
        }
    }

    failures
}

pub(crate) fn ssot_drift_stages() -> Vec<SsotDriftStage> {
    vec![
        SsotDriftStage {
            name: "check_docs_ssot",
            run: |root, _| super::docs::check_docs_ssot(root),
            regen: None,
        },
        SsotDriftStage {
            name: "check_codex_ssot",
            run: |root, _| super::docs::check_codex_ssot(root),
            regen: None,
        },
        SsotDriftStage {
            name: "run_sql_surface_guard",
            run: |root, _| super::guards::run_sql_surface_guard(root, true),
            regen: None,
        },
        SsotDriftStage {
            name: "run_query_all_guard",
            run: |root, _| super::guards::run_query_all_guard(root, true),
            regen: None,
        },
        SsotDriftStage {
            name: "run_turso_import_guard",
            run: |root, _| super::guards::run_turso_import_guard(root, true),
            regen: None,
        },
        SsotDriftStage {
            name: "policy_allowlist_parity",
            run: |root, _| crate::commands::ci::policy_allowlist_parity::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "db_schema_coverage",
            run: |root, _| vox_cli_ci::db_schema_coverage::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "nomenclature_guard",
            run: |root, _| vox_cli_ci::nomenclature_guard::run(root, false),
            regen: None,
        },
        SsotDriftStage {
            name: "operations_catalog::verify",
            run: |root, _| crate::commands::ci::operations_catalog::verify(root),
            regen: Some(&["ci", "operations-sync", "--target", "all", "--write"]),
        },
        SsotDriftStage {
            name: "mcp_vox_surface_parity",
            run: |_root, _| vox_cli_ci::mcp_vox_surface_parity::run(),
            regen: None,
        },
        SsotDriftStage {
            name: "command_compliance",
            run: |root, _| crate::commands::ci::command_compliance::run(root),
            regen: Some(&["ci", "command-sync", "--write"]),
        },
        SsotDriftStage {
            name: "gui_version_sync",
            run: |root, auto_heal| match vox_cli_ci::gui_version_sync::run(root, false) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] gui_version_sync drift; writing...");
                    vox_cli_ci::gui_version_sync::run(root, true)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "gui-version-sync", "--write"]),
        },
        SsotDriftStage {
            name: "gui_catalog_parity",
            run: |root, _| crate::commands::ci::gui_catalog_parity::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "gui_surface_coverage",
            run: |root, auto_heal| match crate::commands::ci::gui_surface_coverage::run(root, false)
            {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] gui_surface_coverage drift; writing...");
                    crate::commands::ci::gui_surface_coverage::run(root, true)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "gui-surface-coverage", "--write"]),
        },
        SsotDriftStage {
            name: "gui_surface_registry",
            run: |root, auto_heal| match crate::commands::ci::gui_surface_registry::run(
                root, false, None,
            ) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] gui_surface_registry drift; writing...");
                    crate::commands::ci::gui_surface_registry::run(root, true, None)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "gui-surface-registry", "--write"]),
        },
        SsotDriftStage {
            name: "capability_sync",
            run: |root, auto_heal| match crate::commands::ci::capability_sync::run(root, false) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] capability_sync drift; writing...");
                    crate::commands::ci::capability_sync::run(root, true)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "capability-sync", "--write"]),
        },
        SsotDriftStage {
            name: "capability_index",
            run: |root, auto_heal| match crate::commands::ci::capability_index::run(root, false) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] capability_index drift; writing...");
                    crate::commands::ci::capability_index::run(root, true)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "capability-index", "--write"]),
        },
        SsotDriftStage {
            name: "plugin_surface",
            run: |root, auto_heal| match vox_cli_ci::plugin_surface::run(root, false) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] plugin_surface drift; writing...");
                    vox_cli_ci::plugin_surface::run(root, true)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "plugin-surface-sync", "--write"]),
        },
        SsotDriftStage {
            name: "plugin_catalog_sync",
            run: |root, auto_heal| match vox_cli_ci::plugin_catalog_sync::run(root, false) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] plugin_catalog_sync drift; writing...");
                    vox_cli_ci::plugin_catalog_sync::run(root, true)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "plugin-catalog-sync", "--write"]),
        },
        SsotDriftStage {
            name: "generate_plugin_catalog_docs",
            run: |_root, auto_heal| match vox_cli_ci::generate_plugin_catalog_docs::run(
                None, None, true,
            ) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] generate_plugin_catalog_docs drift; writing...");
                    vox_cli_ci::generate_plugin_catalog_docs::run(None, None, false)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "generate-plugin-catalog-docs"]),
        },
        SsotDriftStage {
            name: "crate_api_docs",
            run: |root, auto_heal| match vox_cli_ci::crate_api_docs::run(root, false) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] crate_api_docs drift; writing...");
                    vox_cli_ci::crate_api_docs::run(root, true)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "crate-api-sync", "--write"]),
        },
        SsotDriftStage {
            name: "wiki_sync",
            run: |root, auto_heal| {
                match vox_doc_pipeline::pipeline::openwiki::generate_subsystem_hubs(root, false) {
                    Ok(_) => Ok(()),
                    Err(_) if auto_heal => {
                        eprintln!("    [auto-heal] wiki_sync drift; writing...");
                        vox_doc_pipeline::pipeline::openwiki::generate_subsystem_hubs(root, true)
                            .map(|_| ())
                            .map_err(|e| anyhow!(e))
                    }
                    Err(e) => Err(anyhow!(e)),
                }
            },
            regen: Some(&["ci", "wiki-sync", "--write"]),
        },
        SsotDriftStage {
            name: "plugin_skill_parity",
            run: |_root, _| vox_cli_ci::plugin_skill_parity::run(false),
            regen: Some(&["ci", "plugin-skill-parity", "--write"]),
        },
        SsotDriftStage {
            name: "contracts_index",
            run: |root, _| vox_cli_ci::contracts_index::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "docs_reality_audit",
            run: |root, _| vox_cli_ci::docs_reality_audit::run_verify(root),
            regen: None,
        },
        SsotDriftStage {
            name: "exec_policy_contract",
            run: |root, _| crate::commands::ci::exec_policy_contract::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "completion_quality",
            run: |root, _| vox_cli_ci::completion_quality::run_audit_verify_ssot(root),
            regen: None,
        },
        SsotDriftStage {
            name: "scientia_worthiness_contract",
            run: |root, _| vox_cli_ci::scientia_worthiness_contract::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "scientia_novelty_ledger_contract",
            run: |root, _| vox_cli_ci::scientia_novelty_ledger_contract::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "run_data_ssot_guards",
            run: |root, _| super::data_ssot_guards::run_data_ssot_guards(root),
            regen: None,
        },
        SsotDriftStage {
            name: "affected_cmd::check_graph",
            run: |root, _| {
                vox_cli_ci::affected_cmd::check_graph(
                    root.join("contracts/ci/crate-graph.v1.json").as_path(),
                )
                .map_err(|e| anyhow::anyhow!(e))
            },
            regen: Some(&[
                "ci",
                "affected-crates",
                "--regen",
                "--out",
                "contracts/ci/crate-graph.v1.json",
            ]),
        },
        SsotDriftStage {
            name: "node_pnpm_ssot_guard",
            run: |root, _| vox_cli_ci::node_pnpm_ssot_guard::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "workflow_policy_guard",
            run: |root, _| vox_cli_ci::workflow_policy_guard::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "cache_key_lint",
            run: |root, _| vox_cli_ci::cache_key_lint::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "kb_facets_ts",
            run: |root, auto_heal| match vox_cli_ci::kb_facets_ts::run(root, true) {
                Ok(()) => Ok(()),
                Err(_) if auto_heal => {
                    eprintln!("    [auto-heal] kb_facets_ts drift; writing...");
                    vox_cli_ci::kb_facets_ts::run(root, false)
                }
                Err(e) => Err(e),
            },
            regen: Some(&["ci", "config-gui-codegen", "--kb-facets"]),
        },
        SsotDriftStage {
            name: "overlay_parity",
            run: |root, _| vox_cli_ci::overlay_parity::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "docs_cli_claims",
            run: |root, _| crate::commands::ci::docs_cli_claims::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "docs_orphan_pages",
            run: |root, _| crate::commands::ci::docs_orphan_pages::run(root),
            regen: None,
        },
        SsotDriftStage {
            name: "ssot_regen_parity",
            run: |root, _| {
                let vox_path = root.join("scripts/ssot-regen.vox");
                let vox_src = vox_bounded_fs::read_utf8_path_capped(&vox_path)
                    .with_context(|| format!("read {}", vox_path.display()))?;
                let stages = ssot_drift_stages();
                let mut failures = regen_parity_failures(&vox_src, &stages, REGEN_ONLY);

                let lefthook_path = root.join("lefthook.yml");
                let lefthook_yml = vox_bounded_fs::read_utf8_path_capped(&lefthook_path)
                    .with_context(|| format!("read {}", lefthook_path.display()))?;
                failures.extend(lefthook_retry_failures(&lefthook_yml, &stages));

                if !failures.is_empty() {
                    return Err(anyhow!(
                        "ssot_regen_parity failed:\n{}",
                        failures.join("\n")
                    ));
                }
                Ok(())
            },
            regen: None,
        },
    ]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER_FAIL: AtomicUsize = AtomicUsize::new(0);

    fn bump_stage(_root: &Path, _auto_heal: bool) -> Result<()> {
        Ok(())
    }

    fn bump_stage_fail(_root: &Path, _auto_heal: bool) -> Result<()> {
        COUNTER_FAIL.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn fail_alpha(_root: &Path, _auto_heal: bool) -> Result<()> {
        COUNTER_FAIL.fetch_add(1, Ordering::SeqCst);
        Err(anyhow!("alpha"))
    }

    fn fail_beta(_root: &Path, _auto_heal: bool) -> Result<()> {
        COUNTER_FAIL.fetch_add(1, Ordering::SeqCst);
        Err(anyhow!("beta"))
    }

    #[test]
    fn run_stages_executes_all_and_aggregates_failures() {
        COUNTER_FAIL.store(0, Ordering::SeqCst);
        let stages = [
            SsotDriftStage {
                name: "s1",
                run: bump_stage_fail,
                regen: None,
            },
            SsotDriftStage {
                name: "alpha",
                run: fail_alpha,
                regen: None,
            },
            SsotDriftStage {
                name: "s3",
                run: bump_stage_fail,
                regen: None,
            },
            SsotDriftStage {
                name: "beta",
                run: fail_beta,
                regen: None,
            },
        ];
        let err = run_stages(Path::new("."), false, &stages).expect_err("must fail");
        assert_eq!(COUNTER_FAIL.load(Ordering::SeqCst), 4);
        let msg = format!("{err:#}");
        assert!(msg.contains("2 stage(s) failed"), "got: {msg}");
        let pos_alpha = msg.find("alpha").expect("must mention alpha");
        let pos_beta = msg.find("beta").expect("must mention beta");
        assert!(pos_alpha < pos_beta, "alpha must precede beta in: {msg}");
    }

    #[test]
    fn run_stages_all_ok() {
        let stages = [
            SsotDriftStage {
                name: "s1",
                run: bump_stage,
                regen: None,
            },
            SsotDriftStage {
                name: "s2",
                run: bump_stage,
                regen: None,
            },
        ];
        run_stages(Path::new("."), false, &stages).expect("must succeed");
    }

    #[test]
    fn regen_parity_failures_fixture_success() {
        let fixture_stages = [
            SsotDriftStage {
                name: "stage_a",
                run: bump_stage,
                regen: Some(&["ci", "step-a", "--write"]),
            },
            SsotDriftStage {
                name: "stage_b",
                run: bump_stage,
                regen: Some(&["ci", "step-b"]),
            },
            SsotDriftStage {
                name: "stage_no_regen",
                run: bump_stage,
                regen: None,
            },
        ];
        let fixture_regen_only: &[(&[&str], &str)] =
            &[(&["ci", "regen-only-step", "--write"], "reason")];
        let vox_src = r#"
fn main() {
    let b = regen(["--quiet", "ci", "step-b"], "b")
    let ro = regen(["--quiet", "ci", "regen-only-step", "--write"], "ro")
    let a = regen(["--quiet", "ci", "step-a", "--write"], "a")
}
"#;
        let failures = regen_parity_failures(vox_src, &fixture_stages, fixture_regen_only);
        assert!(
            failures.is_empty(),
            "expected no failures, got: {failures:?}"
        );
    }

    #[test]
    fn regen_parity_failures_missing_argv() {
        let fixture_stages = [SsotDriftStage {
            name: "wiki_sync",
            run: bump_stage,
            regen: Some(&["ci", "wiki-sync", "--write"]),
        }];
        let vox_src = r#"
fn main() {
    let a = regen(["--quiet", "ci", "other-step"], "other")
}
"#;
        let failures = regen_parity_failures(vox_src, &fixture_stages, &[]);
        assert_eq!(failures.len(), 2); // 1 missing wiki-sync, 1 unknown other-step
        assert!(
            failures
                .iter()
                .any(|f| f.contains("wiki-sync") && f.contains("not run by scripts/ssot-regen.vox")),
            "expected missing failure, got: {failures:?}"
        );
    }

    #[test]
    fn regen_parity_failures_unknown_argv() {
        let fixture_stages = [SsotDriftStage {
            name: "known_stage",
            run: bump_stage,
            regen: Some(&["ci", "known", "--write"]),
        }];
        let vox_src = r#"
fn main() {
    let a = regen(["--quiet", "ci", "known", "--write"], "known")
    let b = regen(["--quiet", "ci", "unknown-tool"], "unknown")
}
"#;
        let failures = regen_parity_failures(vox_src, &fixture_stages, &[]);
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].contains("unknown") && failures[0].contains("unknown-tool"),
            "got: {failures:?}"
        );
    }

    #[test]
    fn regen_parity_failures_double_listed() {
        let fixture_stages = [SsotDriftStage {
            name: "stage_dup",
            run: bump_stage,
            regen: Some(&["ci", "duplicated", "--write"]),
        }];
        let fixture_regen_only: &[(&[&str], &str)] =
            &[(&["ci", "duplicated", "--write"], "duplicate reason")];
        let vox_src = r#"
fn main() {
    let a = regen(["--quiet", "ci", "duplicated", "--write"], "dup")
}
"#;
        let failures = regen_parity_failures(vox_src, &fixture_stages, fixture_regen_only);
        assert!(
            failures.iter().any(|f| f.contains("listed both")),
            "expected double listing error, got: {failures:?}"
        );
    }

    #[test]
    fn regen_parity_failures_zero_calls() {
        let fixture_stages = [SsotDriftStage {
            name: "stage_a",
            run: bump_stage,
            regen: Some(&["ci", "step-a"]),
        }];
        let vox_src = r#"
fn main() {
    // nothing here
}
"#;
        let failures = regen_parity_failures(vox_src, &fixture_stages, &[]);
        assert!(
            failures
                .iter()
                .any(|f| f.contains("0 regen() calls were parsed")),
            "expected zero calls error, got: {failures:?}"
        );
    }

    #[test]
    fn real_tree_ssot_drift_stages_contract() {
        let stages = ssot_drift_stages();
        let mut names = HashSet::new();
        for s in &stages {
            assert!(names.insert(s.name), "duplicate stage name: {}", s.name);
        }
        assert!(
            stages.len() >= 36,
            "expected at least 36 stages, got {}",
            stages.len()
        );
        for required in [
            "cache_key_lint",
            "kb_facets_ts",
            "capability_index",
            "crate_api_docs",
            "wiki_sync",
            "ssot_regen_parity",
        ] {
            assert!(
                names.contains(required),
                "stages missing required stage: {required}"
            );
        }
        assert_eq!(
            stages.last().unwrap().name,
            "ssot_regen_parity",
            "ssot_regen_parity must be the last stage"
        );
    }

    #[test]
    fn real_tree_regen_parity() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let vox_path = repo_root.join("scripts/ssot-regen.vox");
        let vox_src = std::fs::read_to_string(&vox_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", vox_path.display()));
        let stages = ssot_drift_stages();
        let failures = regen_parity_failures(&vox_src, &stages, REGEN_ONLY);
        assert!(
            failures.is_empty(),
            "real-tree regen parity failures:\n{failures:#?}"
        );
    }

    #[test]
    fn lefthook_retry_failures_option_a_success() {
        let fixture_stages = [SsotDriftStage {
            name: "stage_cov",
            run: bump_stage,
            regen: Some(&["ci", "gui-surface-coverage", "--write"]),
        }];
        let lefthook_yml = r#"
pre-push:
  commands:
    pre-push:
      run: vox ci pre-push || (vox run scripts/fmt.vox -- --all && vox ci line-endings --autofix && vox ci sync-ignore-files && vox ci gui-surface-coverage --write && vox ci pre-push)
"#;
        let failures = lefthook_retry_failures(lefthook_yml, &fixture_stages);
        assert!(
            failures.is_empty(),
            "expected no failures, got: {failures:?}"
        );
    }

    #[test]
    fn lefthook_retry_failures_option_a_unknown_generator() {
        let fixture_stages = [SsotDriftStage {
            name: "stage_cov",
            run: bump_stage,
            regen: Some(&["ci", "gui-surface-coverage", "--write"]),
        }];
        let lefthook_yml = r#"
pre-push:
  commands:
    pre-push:
      run: vox ci pre-push || (vox run scripts/fmt.vox -- --all && vox ci not-a-stage --write && vox ci pre-push)
"#;
        let failures = lefthook_retry_failures(lefthook_yml, &fixture_stages);
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].contains("not-a-stage"),
            "expected failure naming not-a-stage, got: {failures:?}"
        );
    }

    #[test]
    fn run_ssot_drift_dispatches_cache_key_lint() {
        let stages = ssot_drift_stages();
        assert!(stages.iter().any(|s| s.name == "cache_key_lint"));
    }

    #[test]
    fn run_ssot_drift_dispatches_kb_facets_ts() {
        let stages = ssot_drift_stages();
        assert!(stages.iter().any(|s| s.name == "kb_facets_ts"));
    }

    #[test]
    fn run_ssot_drift_dispatches_capability_index() {
        let stages = ssot_drift_stages();
        assert!(stages.iter().any(|s| s.name == "capability_index"));
    }

    #[test]
    fn real_tree_lefthook_parity() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let lefthook_path = repo_root.join("lefthook.yml");
        let lefthook_yml = std::fs::read_to_string(&lefthook_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", lefthook_path.display()));
        let stages = ssot_drift_stages();
        let failures = lefthook_retry_failures(&lefthook_yml, &stages);
        assert!(
            failures.is_empty(),
            "real-tree lefthook parity failures:\n{failures:#?}"
        );
    }
}
