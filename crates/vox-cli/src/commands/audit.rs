//! `vox audit` — umbrella command that iterates quality gates defined in
//! `contracts/ci/check-targets.v1.yaml`.
//!
//! # Usage
//!
//! ```text
//! vox audit                         # run all checks
//! vox audit --category lint         # run only lint checks
//! vox audit --list                  # print check IDs without running
//! vox audit --dry-run               # print commands without executing
//! ```
//!
//! The manifest at `contracts/ci/check-targets.v1.yaml` is the SSOT for every
//! quality gate.  See that file for per-check metadata: `blocking`, `runs_on`,
//! `rust_only`, and `command`.

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::audit_effort::EffortArgs;
use crate::commands::audit_route::EffortRouteArgs;

/// Nested subcommands under `vox audit <thing>`.
///
/// Today this is just `effort` (F1). The intent is for additional umbrella
/// gates to graduate here over time — see AGENTS.md §10.2 ("vox audit" umbrella
/// SSOT) for the contract.
#[derive(Subcommand, Debug, Clone)]
pub enum AuditSubcommand {
    /// Architecture layer + LoC budget validation (`vox-arch-check`).
    Arch,
    /// Source-policy / TOESTUB detectors (`vox-code-audit`).
    Code,
    /// Retirement-guard parity vs AGENTS.md §Retired Surfaces (CR-L6).
    Retirement,
    /// Run arch → code → retirement and print summary JSON (`schema_version: 1`).
    Core,
    /// AI-judged audit of recent git commit history (token-spend estimates +
    /// remediation suggestions). See
    /// `docs/superpowers/specs/2026-05-28-effort-audit-core-design.md`.
    Effort(EffortArgs),
    /// Route effort-audit findings to verified, drafted enforcement-artifact
    /// proposals (AGENTS.md rule / lint detector / arch rule / CI gate / corpus
    /// example / Vox script). See
    /// `docs/superpowers/specs/2026-05-30-effort-route-design.md`.
    #[command(name = "effort-route")]
    EffortRoute(EffortRouteArgs),
    /// Audit architecture SSOTs for empirical test citations and verify existence.
    #[command(name = "research-parity")]
    ResearchParity(crate::commands::audit_research_parity::ResearchParityArgs),
    /// Deterministic Documentation Reality Audit & Mention Extractor
    Docs(DocsAuditArgs),
}

/// Arguments for `vox audit docs`.
#[derive(clap::Args, Debug, Clone)]
pub struct DocsAuditArgs {
    /// Format of findings output: text (human summary), json, or jsonl.
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Generate prune report for the documentation corpus.
    #[arg(long)]
    pub prune_report: bool,

    /// Write machine-generated findings to contracts/reports/docs-reality-audit/findings.v1.json and update metrics.
    #[arg(long)]
    pub write_findings: bool,

    /// Hand-labelled evaluation sample path for false-positive rate calculation.
    #[arg(long)]
    pub eval_sample: Option<PathBuf>,

    /// Filter scan to specific relative doc paths.
    #[arg(long)]
    pub paths: Option<Vec<PathBuf>>,

    /// Output destination file for prune report (markdown or json).
    #[arg(long)]
    pub prune_out: Option<PathBuf>,

    /// Update or generate contracts/reports/docs-freshness/ledger.v1.json.
    #[arg(long)]
    pub update_ledger: bool,

    /// Check drift gate against baseline and suppressions.
    #[arg(long)]
    pub check_drift: bool,

    /// Tighten baseline by removing deficits that are no longer present.
    #[arg(long)]
    pub tighten_baseline: bool,

    /// Generate or initialize contracts/reports/docs-freshness/baseline.v1.json.
    #[arg(long)]
    pub generate_baseline: bool,

    /// Re-attest a documentation page against HEAD commit in the Freshness Ledger.
    #[arg(long)]
    pub attest: Option<PathBuf>,

    /// Subcommand action for docs audit: pr-impact or dod.
    #[arg(value_name = "ACTION")]
    pub action: Option<String>,

    /// Generate PR impact comment for reader-facing documentation affected by code changes.
    #[arg(long)]
    pub pr_impact: bool,

    /// Evaluate Docs Definition-of-Done rules for modified contracts and registries.
    #[arg(long)]
    pub dod: bool,

    /// Git base reference for PR diff (e.g. origin/main or commit SHA).
    #[arg(long)]
    pub diff_base: Option<String>,

    /// Git head reference for PR diff (e.g. HEAD or commit SHA).
    #[arg(long)]
    pub diff_head: Option<String>,

    /// Destination file for PR impact markdown output.
    #[arg(long)]
    pub diff_out: Option<PathBuf>,

    /// Dry run mode using synthetic or representative sample.
    #[arg(long)]
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// Check-manifest types now live in `vox-cli-contracts` (shared with vox-cli-ci);
/// re-exported here so existing `commands::audit::CheckManifest` paths keep working.
pub use vox_cli_contracts::{CheckEntry, CheckManifest};

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------

/// Run quality-gate checks defined in `contracts/ci/check-targets.v1.yaml`,
/// OR run a CR-L gate from the `vox-audit` umbrella registry.
///
/// Two dispatch modes, mutually exclusive:
///   • Default — iterates `check-targets.v1.yaml` and runs each entry.
///   • `--gate <NAME>` / `--list-gates` — dispatches to
///     `vox_audit::registry()` for the v1.0 LLM-target CR-L gates (CR-L0..L8).
#[derive(Args, Debug, Clone)]
pub struct AuditArgs {
    /// Optional nested subcommand (`vox audit effort`, future
    /// `vox audit <thing>` slots). When present, the flag-based
    /// check-targets / CR-L dispatch path is bypassed entirely.
    #[command(subcommand)]
    pub command: Option<AuditSubcommand>,

    /// Filter to a specific category (lint, test, audit, doc, arch).
    /// Ignored when `--gate` is set.
    #[arg(long, value_name = "CATEGORY")]
    pub category: Option<String>,

    /// Print all matching check IDs and exit without running anything.
    /// Ignored when `--gate` is set.
    #[arg(long)]
    pub list: bool,

    /// Print each command without executing it.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip checks marked `quick_skip: true` in the manifest.
    /// Ignored when `--gate` is set.
    #[arg(long)]
    pub quick: bool,

    /// CR-L gate to run from the `vox-audit` registry (e.g. `retirement`,
    /// `corpus-feedback`, `all`). When set, the check-targets dispatch path
    /// is skipped and we route through `vox_audit::registry()` instead. This
    /// aligns with `contracts/ci/vox-audit-contract.v1.yaml` §cli_surface
    /// ("`vox audit <thing>`" is the user-facing shape).
    #[arg(long, value_name = "GATE", conflicts_with_all = ["list", "category", "quick"])]
    pub gate: Option<String>,

    /// List every registered CR-L gate with its description, then exit.
    /// Mutually exclusive with the check-targets `--list`.
    #[arg(long, conflicts_with = "list")]
    pub list_gates: bool,

    /// Output format for CR-L gate reports (`json`/`markdown`/`html`).
    /// Only meaningful with `--gate`. Defaults to `json`.
    #[arg(long, value_name = "FORMAT", default_value = "json")]
    pub format: String,

    /// Override the corpus / contract path for the targeted gate.
    /// Only meaningful with `--gate`.
    #[arg(long, value_name = "PATH")]
    pub corpus: Option<PathBuf>,

    /// Override the bar from the corpus manifest (e.g. `--threshold 0.8`).
    /// Only meaningful with `--gate`.
    #[arg(long)]
    pub threshold: Option<f64>,

    /// Suppress writing the canonical `contracts/reports/<thing>/<date>.json`.
    /// Only meaningful with `--gate`.
    #[arg(long)]
    pub no_canonical_report: bool,

    /// With `--gate all`: evaluate the GA roll-up (foundation-first, with
    /// `blocked_by_foundation`), write `contracts/reports/_snapshot/<UTC>.json`,
    /// and exit non-zero when GA is not met. This is the v1.0 acceptance
    /// command (CR-F0). Only meaningful with `--gate all`.
    #[arg(long)]
    pub strict_block_ga: bool,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Entry point called from the CLI dispatch (flag-based path; no nested subcommand).
pub fn run(args: &AuditArgs) -> Result<()> {
    // CR-L gate routing takes precedence over check-targets dispatch.
    if args.list_gates {
        // Honor --format so dashboards / tooling can consume structured JSON.
        // Default (no --format) defaults to "json" via clap, which gives
        // structured output for the most-common programmatic consumer; the
        // text-style listing remains available via `--format markdown` etc.
        let format = vox_audit::report::ReportFormat::parse(&args.format)
            .map_err(|msg| anyhow::anyhow!(msg))?;
        return run_list_cr_l_gates_with_format(&format);
    }
    if let Some(gate_name) = args.gate.as_deref() {
        if gate_name == "core" {
            return run_core_gates_summary();
        }
        return run_cr_l_gate(gate_name, args);
    }

    let root = vox_repository::resolve_repo_root_for_ci();
    let manifest = load_manifest(&root)?;
    let checks = filter_checks(&manifest.checks, args);

    if args.list {
        for check in &checks {
            println!("{}: {} [{}]", check.id, check.description, check.category);
        }
        return Ok(());
    }

    for check in &checks {
        println!("==> {}: {}", check.id, check.description);
        if args.dry_run {
            println!("    DRY-RUN: {}", check.command.join(" "));
            continue;
        }
        run_check(check, &root)?;
        println!("    OK");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Core umbrella gates (arch / code / retirement — AGENTS.md §`vox audit`).
// ---------------------------------------------------------------------------

/// Run a single core gate or the full trio (`vox audit core` / `--gate core`).
pub fn run_audit_subcommand(cmd: &AuditSubcommand) -> Result<()> {
    let root = vox_repository::resolve_repo_root_for_ci();
    let result = match cmd {
        AuditSubcommand::Arch => vox_audit::core_gates::run_arch_gate(&root),
        AuditSubcommand::Code => vox_audit::core_gates::run_code_gate(&root),
        AuditSubcommand::Retirement => vox_audit::core_gates::run_retirement_gate(),
        AuditSubcommand::Core => {
            let summary = vox_audit::core_gates::run_core_all(&root);
            let json = serde_json::to_string_pretty(&summary)
                .map_err(|e| anyhow::anyhow!("serialize core gates summary: {e}"))?;
            println!("{json}");
            if summary.ok {
                return Ok(());
            }
            anyhow::bail!("core audit gates failed");
        }
        AuditSubcommand::ResearchParity(args) => {
            crate::commands::audit_research_parity::run_research_parity_audit_sync(args)?;
            return Ok(());
        }
        AuditSubcommand::Docs(args) => {
            run_docs_audit(&root, args)?;
            return Ok(());
        }
        AuditSubcommand::Effort(_) | AuditSubcommand::EffortRoute(_) => {
            anyhow::bail!("async audit subcommand must be dispatched from cli_dispatch");
        }
    };
    if result.ok {
        Ok(())
    } else {
        let detail = result.detail.unwrap_or_default();
        anyhow::bail!(
            "audit {} failed (exit {}){}",
            result.gate,
            result.exit_code,
            if detail.is_empty() {
                String::new()
            } else {
                format!(": {detail}")
            }
        )
    }
}

fn run_core_gates_summary() -> Result<()> {
    run_audit_subcommand(&AuditSubcommand::Core)
}

fn run_docs_audit(root: &Path, args: &DocsAuditArgs) -> Result<()> {
    // 1. Hand-labelled evaluation sample path
    if let Some(ref sample_path) = args.eval_sample {
        let registry = vox_doc_pipeline::claims::GroundTruthRegistry::load(root)?;
        let full_sample = if sample_path.is_absolute() {
            sample_path.clone()
        } else {
            root.join(sample_path)
        };
        let report = vox_doc_pipeline::claims::evaluate_sample_dataset(&full_sample, &registry)?;
        println!(
            "Docs Claims Extractor Evaluation:\n  Samples: {}\n  Precision: {:.2}%\n  Recall: {:.2}%\n  False-Positive Rate: {:.2}%\n  Acceptable: {}",
            report.total_samples,
            report.precision * 100.0,
            report.recall * 100.0,
            report.false_positive_rate * 100.0,
            if report.is_acceptable() { "YES" } else { "NO" }
        );
        return Ok(());
    }

    // 2. PR impact and Definition-of-Done
    let is_pr_impact = args.pr_impact || args.action.as_deref() == Some("pr-impact");
    let is_dod = args.dod || args.action.as_deref() == Some("dod");

    if is_pr_impact || is_dod {
        let changed: Vec<String> = if args.dry_run {
            vec![
                "crates/vox-cli/src/main.rs".to_string(),
                "contracts/cli/command-registry.yaml".to_string(),
            ]
        } else {
            let base = args.diff_base.as_deref().unwrap_or("HEAD~1");
            let head = args.diff_head.as_deref().unwrap_or("HEAD");
            // vox-arch-check: allow git-exec
            let output = std::process::Command::new("git")
                .args(["diff", "--name-only", base, head])
                .current_dir(root)
                .output()?;
            let text = String::from_utf8_lossy(&output.stdout);
            text.lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };

        if is_pr_impact {
            let ledger_path = root.join("contracts/reports/docs-freshness/ledger.v1.json");
            let ledger = vox_doc_pipeline::claims::FreshnessLedger::load_from_file(&ledger_path)
                .unwrap_or_default();
            let index = vox_doc_pipeline::claims::ReverseIndex::build(&ledger);
            let report = index.query_impact(&changed, &ledger);

            if let Some(ref out_path) = args.diff_out {
                let full_out = if out_path.is_absolute() {
                    out_path.clone()
                } else {
                    root.join(out_path)
                };
                if let Some(parent) = full_out.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&full_out, &report.markdown_comment)?;
                eprintln!("Wrote PR impact report to {}", full_out.display());
            } else {
                println!("{}", report.markdown_comment);
            }
        }

        if is_dod {
            let warnings = vox_doc_pipeline::claims::check_definition_of_done(&changed);
            if warnings.is_empty() {
                println!(
                    "\n✅ Docs Definition-of-Done PASSED: All modified contracts/registries have documentation updates."
                );
            } else {
                eprintln!(
                    "\n⚠️ Docs Definition-of-Done Warnings ({}):",
                    warnings.len()
                );
                for w in &warnings {
                    eprintln!("  • [{}] {} — {}", w.rule_id, w.trigger_file, w.message);
                }
            }
        }

        let only_pr_or_dod = !args.check_drift
            && !args.write_findings
            && !args.update_ledger
            && !args.prune_report
            && !args.generate_baseline
            && !args.tighten_baseline
            && args.attest.is_none();
        if only_pr_or_dod {
            return Ok(());
        }
    }

    // 2. Scan corpus
    let resolved = vox_doc_pipeline::claims::scan_corpus(root, args.paths.as_deref())?;
    let findings = vox_doc_pipeline::claims::generate_findings(&resolved);

    let total_mentions = resolved.len();
    let resolved_count = resolved
        .iter()
        .filter(|m| m.status == vox_doc_pipeline::claims::ResolutionStatus::Resolved)
        .count();
    let retired_count = resolved
        .iter()
        .filter(|m| m.status == vox_doc_pipeline::claims::ResolutionStatus::Retired)
        .count();
    let missing_count = resolved
        .iter()
        .filter(|m| m.status == vox_doc_pipeline::claims::ResolutionStatus::Missing)
        .count();
    let ambiguous_count = resolved
        .iter()
        .filter(|m| m.status == vox_doc_pipeline::claims::ResolutionStatus::Ambiguous)
        .count();

    let mut pages_set = std::collections::HashSet::new();
    for m in &resolved {
        pages_set.insert(&m.extracted.source_file);
    }
    let pages_scanned = pages_set.len();

    if args.format == "jsonl" {
        for f in &findings {
            let record = serde_json::json!({
                "schema_version": 1,
                "discriminator": "doc_deficit",
                "finding": f,
            });
            println!("{}", serde_json::to_string(&record)?);
        }
    } else if args.format == "json" {
        let file_record = vox_doc_pipeline::claims::FindingsFile {
            schema_version: 1,
            findings: findings.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&file_record)?);
    } else {
        println!("=== Vox Documentation Reality Audit ===");
        println!("Pages scanned:             {}", pages_scanned);
        println!("Total mentions extracted:  {}", total_mentions);
        println!("  Resolved:                {}", resolved_count);
        println!("  Retired:                 {}", retired_count);
        println!("  Missing:                 {}", missing_count);
        println!("  Ambiguous:               {}", ambiguous_count);
        println!("Actionable deficits found: {}", findings.len());

        if !findings.is_empty() {
            println!("\nTop Actionable Deficits:");
            for f in findings.iter().take(10) {
                println!(
                    "  [{}] {} ({}) - {}",
                    f.priority_band,
                    f.id,
                    f.doc_paths.first().map(|s| s.as_str()).unwrap_or("?"),
                    f.recommended_action
                );
            }
            if findings.len() > 10 {
                println!("  ... and {} more deficits.", findings.len() - 10);
            }
        }
    }

    if args.write_findings {
        let findings_path = root.join("contracts/reports/docs-reality-audit/findings.v1.json");
        let metrics_path = root.join("contracts/reports/docs-reality-audit/metrics.v1.json");

        let file_record = vox_doc_pipeline::claims::FindingsFile {
            schema_version: 1,
            findings: findings.clone(),
        };
        std::fs::write(&findings_path, serde_json::to_string_pretty(&file_record)?)?;

        let metrics = vox_doc_pipeline::claims::compute_metrics(
            &findings,
            pages_scanned,
            total_mentions,
            resolved_count,
        );
        std::fs::write(&metrics_path, serde_json::to_string_pretty(&metrics)?)?;
        eprintln!(
            "Updated findings at {} and metrics at {}",
            findings_path.display(),
            metrics_path.display()
        );
    }

    if args.prune_report {
        let prune_report = vox_doc_pipeline::claims::generate_prune_report(root, &resolved)?;
        println!("\n=== Documentation Prune Report ===");
        println!(
            "Total pages evaluated:    {}",
            prune_report.total_pages_scanned
        );
        for (disp, count) in &prune_report.count_by_disposition {
            println!("  {:<20} {}", disp, count);
        }
        println!("Topic clusters detected:  {}", prune_report.clusters.len());

        if let Some(ref out_path) = args.prune_out {
            let full_out = if out_path.is_absolute() {
                out_path.clone()
            } else {
                root.join(out_path)
            };
            if let Some(parent) = full_out.parent() {
                std::fs::create_dir_all(parent)?;
            }

            if full_out.extension().and_then(|s| s.to_str()) == Some("json") {
                let json_data = serde_json::to_string_pretty(&prune_report)?;
                std::fs::write(&full_out, json_data)?;
                eprintln!("Prune report JSON written to {}", full_out.display());
            } else {
                let mut md = String::new();
                md.push_str("---\n");
                md.push_str("title: \"Documentation Corpus Prune Report\"\n");
                md.push_str("description: \"Evidence-based prune dispositions for all pages in the Vox documentation corpus.\"\n");
                md.push_str("category: \"Architecture SSOTs\"\n");
                md.push_str("status: \"current\"\n");
                md.push_str("---\n\n");
                md.push_str("# Documentation Corpus Prune Report\n\n");
                md.push_str(&format!(
                    "- **Total Pages Scanned**: {}\n",
                    prune_report.total_pages_scanned
                ));
                for (disp, count) in &prune_report.count_by_disposition {
                    md.push_str(&format!("- **{}**: {}\n", disp, count));
                }
                md.push_str(&format!(
                    "- **Topic Clusters**: {}\n\n",
                    prune_report.clusters.len()
                ));
                md.push_str("## Page Dispositions\n\n");
                md.push_str("| Page | Category | Status | Disposition | Dead Mentions | Inbound Links | Age (days) | Rationale |\n");
                md.push_str("|---|---|---|---|---|---|---|---|\n");
                for p in &prune_report.pages {
                    md.push_str(&format!(
                        "| `{}` | {} | {} | **{:?}** | {} | {} | {} | {} |\n",
                        p.relative_path,
                        p.category,
                        p.status,
                        p.recommended_disposition,
                        p.dead_mentions_count,
                        p.inbound_links_count,
                        p.days_since_substantive_edit,
                        p.disposition_rationale
                    ));
                }
                std::fs::write(&full_out, md)?;
                eprintln!("Prune report Markdown written to {}", full_out.display());
            }
        }
    }

    if args.update_ledger {
        let ledger_path = root.join("contracts/reports/docs-freshness/ledger.v1.json");
        let existing = vox_doc_pipeline::claims::FreshnessLedger::load_from_file(&ledger_path);
        // vox-arch-check: allow git-exec
        let head_commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());

        let ledger = vox_doc_pipeline::claims::generate_freshness_ledger(
            root,
            &head_commit,
            existing.as_ref(),
        );
        ledger.save_to_file(&ledger_path)?;
        eprintln!(
            "Freshness ledger written to {} ({} pages, {} verified, {} drifting, {} broken, {} historical, {} unanchored)",
            ledger_path.display(),
            ledger.metrics.total_pages,
            ledger.metrics.verified_count,
            ledger.metrics.drifting_count,
            ledger.metrics.broken_count,
            ledger.metrics.historical_count,
            ledger.metrics.unanchored_count,
        );
    }

    if args.generate_baseline {
        let baseline_path = root.join("contracts/reports/docs-freshness/baseline.v1.json");
        let mut deficits = Vec::new();
        for f in &findings {
            if f.doc_paths
                .iter()
                .any(|p| vox_doc_pipeline::claims::is_reader_facing_page(p))
            {
                deficits.push(vox_doc_pipeline::claims::GrandfatheredDeficit {
                    fingerprint: f.fingerprint.clone().unwrap_or_else(|| f.id.clone()),
                    page_path: f.doc_paths.first().cloned().unwrap_or_default(),
                    raw_text: f.claim_ids.first().cloned().unwrap_or_default(),
                });
            }
        }
        let baseline = vox_doc_pipeline::claims::BaselineFile {
            schema_version: 1,
            generated_at: chrono::Utc::now().to_rfc3339(),
            deficits,
        };
        baseline.save_to_file(&baseline_path)?;
        eprintln!(
            "Generated grandfathered baseline at {} with {} deficits",
            baseline_path.display(),
            baseline.deficits.len()
        );
    }

    if args.tighten_baseline {
        let baseline_path = root.join("contracts/reports/docs-freshness/baseline.v1.json");
        let mut baseline = vox_doc_pipeline::claims::BaselineFile::load_from_file(&baseline_path)
            .map_err(|e| anyhow::anyhow!(e))?;
        let active_fps: std::collections::HashSet<String> = findings
            .iter()
            .map(|f| f.fingerprint.clone().unwrap_or_else(|| f.id.clone()))
            .collect();
        let tightened = baseline.tighten(&active_fps);
        baseline.save_to_file(&baseline_path)?;
        eprintln!(
            "Tightened baseline at {}: removed {} resolved deficits (remaining: {})",
            baseline_path.display(),
            tightened,
            baseline.deficits.len()
        );
    }

    if let Some(ref attest_path) = args.attest {
        let ledger_path = root.join("contracts/reports/docs-freshness/ledger.v1.json");
        let mut ledger = vox_doc_pipeline::claims::FreshnessLedger::load_from_file(&ledger_path)
            .ok_or_else(|| {
                anyhow::anyhow!("Freshness ledger not found at {}", ledger_path.display())
            })?;

        // vox-arch-check: allow git-exec
        let head_commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());

        let norm_path = attest_path
            .strip_prefix(root)
            .unwrap_or(attest_path)
            .to_string_lossy()
            .replace('\\', "/");

        let page_rec = ledger
            .pages
            .get_mut(&norm_path)
            .ok_or_else(|| anyhow::anyhow!("Page '{}' not found in freshness ledger", norm_path))?;

        for anchor in &mut page_rec.anchors {
            anchor.verified_hash = anchor.content_hash.clone();
        }
        page_rec.last_verified_commit = Some(head_commit);
        if page_rec.broken_mentions.is_empty() {
            page_rec.state = vox_doc_pipeline::claims::DocState::Verified;
        }
        let new_state = page_rec.state.clone();
        ledger.recompute_metrics();
        ledger.save_to_file(&ledger_path)?;
        eprintln!(
            "Attested '{}': state set to {:?}, updated anchors to match disk hashes",
            norm_path, new_state
        );
    }

    if args.check_drift {
        let baseline_path = root.join("contracts/reports/docs-freshness/baseline.v1.json");
        let suppressions_path = root.join("contracts/reports/docs-freshness/suppressions.v1.json");

        let baseline = vox_doc_pipeline::claims::BaselineFile::load_from_file(&baseline_path)
            .map_err(|e| anyhow::anyhow!(e))?;
        let suppressions =
            vox_doc_pipeline::claims::SuppressionsFile::load_from_file(&suppressions_path)
                .map_err(|e| anyhow::anyhow!(e))?;

        let today = chrono::Utc::now().date_naive();
        let result = vox_doc_pipeline::claims::evaluate_drift_gate(
            &findings,
            &baseline,
            &suppressions,
            today,
        );

        if !result.passed {
            eprintln!("\n❌ Documentation Drift Gate FAILED");
            if !result.expired_suppression_errors.is_empty() {
                eprintln!("\nExpired or Invalid Suppressions:");
                for err in &result.expired_suppression_errors {
                    eprintln!("  • {}", err);
                }
            }
            if !result.new_deficits.is_empty() {
                eprintln!("\nNew Unsuppressed Reader-Facing Deficits:");
                for f in &result.new_deficits {
                    eprintln!(
                        "  • [{}] {} in {} — {}",
                        f.priority_band,
                        f.id,
                        f.doc_paths.first().map(|s| s.as_str()).unwrap_or("?"),
                        f.recommended_action
                    );
                }
            }
            eprintln!(
                "\nActive grandfathered deficits: {}, Resolved baselined deficits: {}",
                result.active_baselined_count, result.resolved_baselined_count
            );
            anyhow::bail!(
                "Drift gate failed with {} new deficits",
                result.new_deficits.len()
            );
        }

        println!("\n✅ Documentation Drift Gate PASSED");
        println!(
            "Active grandfathered deficits: {}, Resolved: {}, Expired suppressions: 0",
            result.active_baselined_count, result.resolved_baselined_count
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// CR-L gate dispatch (B1, ratified 2026-05-15).
// ---------------------------------------------------------------------------

/// Render every registered CR-L gate.
///
/// Honors `--format`:
/// - `json` (default for programmatic consumers like the dashboard): emits a
///   stable JSON array of `GateDescriptor` records;
/// - `markdown`/`md`: emits a Markdown bullet list;
/// - default text: the original human-readable two-line-per-gate format.
fn run_list_cr_l_gates_with_format(format: &vox_audit::report::ReportFormat) -> Result<()> {
    use vox_audit::report::ReportFormat;
    let registry = vox_audit::registry();
    let descriptors: Vec<GateDescriptor> = registry
        .iter()
        .map(|sub| {
            let gate = sub.gate();
            GateDescriptor {
                name: gate.thing_name().to_string(),
                gate: format!("{gate:?}"),
                block_ga: gate.block_ga(),
                cost_metered: gate.cost_metered(),
                description: sub.description().to_string(),
            }
        })
        .collect();

    match format {
        ReportFormat::Json => {
            // A12: wrap in {schema_version, gates: [...]} envelope. Matches
            // the contract-wide JSON envelope convention so dashboards can
            // branch on schema versions.
            let envelope = GateListingEnvelope {
                schema_version: 1,
                gates: &descriptors,
            };
            let json = serde_json::to_string_pretty(&envelope)
                .map_err(|err| anyhow::anyhow!("serialize gate list: {err}"))?;
            println!("{json}");
        }
        ReportFormat::Markdown => {
            println!("# Registered CR-L gates ({})\n", descriptors.len());
            for d in &descriptors {
                println!(
                    "- **{name}** — `block_ga={block}`, `cost_metered={cost}`\n  > {desc}",
                    name = d.name,
                    block = d.block_ga,
                    cost = d.cost_metered,
                    desc = d.description,
                );
            }
        }
        ReportFormat::Html => {
            println!(
                "<table class=\"vox-audit-gates\"><thead><tr>\
                 <th>name</th><th>block_ga</th><th>cost_metered</th><th>description</th>\
                 </tr></thead><tbody>"
            );
            for d in &descriptors {
                println!(
                    "<tr><td>{name}</td><td>{block}</td><td>{cost}</td><td>{desc}</td></tr>",
                    name = html_escape(&d.name),
                    block = d.block_ga,
                    cost = d.cost_metered,
                    desc = html_escape(&d.description),
                );
            }
            println!("</tbody></table>");
        }
    }
    Ok(())
}

/// Stable JSON shape for `--list-gates --format json` consumers (dashboards,
/// other tooling). Field names are intentionally `snake_case` to match the
/// repo-wide telemetry/contract conventions.
#[derive(Debug, serde::Serialize)]
struct GateDescriptor {
    name: String,
    gate: String,
    block_ga: bool,
    cost_metered: bool,
    description: String,
}

/// Envelope shape (A12, ratified 2026-05-15) that wraps the array of
/// [`GateDescriptor`]s. Matches the contract-wide `{schema_version, ...}`
/// convention used by `AuditReport` and other CR-L surfaces, so dashboard
/// consumers can branch on schema versions across future revisions.
#[derive(Debug, serde::Serialize)]
struct GateListingEnvelope<'a> {
    schema_version: u32,
    gates: &'a [GateDescriptor],
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Resolve `gate_name` to a `CrlGate` and dispatch via the umbrella registry.
/// `gate_name == "all"` runs every gate and aggregates the exit code per
/// `contracts/ci/vox-audit-contract.v1.yaml` §umbrella.
fn run_cr_l_gate(gate_name: &str, args: &AuditArgs) -> Result<()> {
    let common = build_common_args(args)?;

    if gate_name == "all" {
        if args.strict_block_ga {
            // v1.0 GA acceptance: foundation-first roll-up with
            // blocked_by_foundation + _snapshot artifact (CR-F0).
            let snap = vox_audit::run_ga_snapshot(&common, true);
            println!(
                "{}",
                serde_json::to_string_pretty(&snap)
                    .map_err(|err| anyhow::anyhow!("render GA snapshot: {err}"))?
            );
            std::process::exit(snap.exit_code);
        }
        let outcomes = vox_audit::run_all(&common);
        for outcome in &outcomes {
            render_outcome(&outcome.report, &common.format)?;
        }
        let exit_code = vox_audit::aggregate_exit_code(&outcomes);
        std::process::exit(exit_code.as_i32());
    }

    let Some(gate) = vox_audit::gate_from_name(gate_name) else {
        bail!(
            "unknown CR-L gate `{gate_name}`. Use `vox audit --list-gates` to see registered gates."
        );
    };
    let outcome = vox_audit::run_gate(gate, &common);
    render_outcome(&outcome.report, &common.format)?;
    std::process::exit(outcome.exit_code.as_i32());
}

fn render_outcome(
    report: &vox_audit::report::AuditReport,
    format: &vox_audit::report::ReportFormat,
) -> Result<()> {
    let text = report
        .render(*format)
        .map_err(|err| anyhow::anyhow!("render audit report: {err}"))?;
    println!("{text}");
    Ok(())
}

fn build_common_args(args: &AuditArgs) -> Result<vox_audit::CommonArgs> {
    let format =
        vox_audit::report::ReportFormat::parse(&args.format).map_err(|msg| anyhow::anyhow!(msg))?;
    Ok(vox_audit::CommonArgs {
        format,
        baseline: None,
        threshold: args.threshold,
        corpus: args.corpus.clone(),
        llm_panel: None,
        dry_run: args.dry_run,
        write_canonical_report: !args.no_canonical_report,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load and parse the manifest from `<root>/contracts/ci/check-targets.v1.yaml`.
fn load_manifest(root: &Path) -> Result<CheckManifest> {
    let path = root
        .join("contracts")
        .join("ci")
        .join("check-targets.v1.yaml");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("read manifest at {}", path.display()))?;
    let manifest: CheckManifest =
        serde_yaml::from_str(&content).with_context(|| "parse check-targets.v1.yaml")?;
    if manifest.schema_version != 1 {
        bail!(
            "unsupported manifest schema_version {}; expected 1",
            manifest.schema_version
        );
    }
    Ok(manifest)
}

/// Return the subset of checks that match the supplied CLI filters.
fn filter_checks<'a>(checks: &'a [CheckEntry], args: &AuditArgs) -> Vec<&'a CheckEntry> {
    checks
        .iter()
        .filter(|c| {
            if let Some(ref cat) = args.category {
                if c.category != *cat {
                    return false;
                }
            }
            if args.quick && c.quick_skip {
                return false;
            }
            true
        })
        .collect()
}

/// Execute a single check; bail on non-zero exit.
fn run_check(check: &CheckEntry, root: &Path) -> Result<()> {
    let (program, rest) = check
        .command
        .split_first()
        .context("check command is empty")?;

    let status = Command::new(program)
        .args(rest)
        .current_dir(root)
        .status()
        .with_context(|| format!("spawn command for check `{}`", check.id))?;

    if !status.success() {
        bail!(
            "check `{}` failed with exit code {:?}",
            check.id,
            status.code()
        );
    }
    Ok(())
}

/// Walk parent directories from `start` until a `Cargo.toml` is found.
/// Returns `None` if the filesystem root is reached without finding one.
///
/// This is a lightweight fallback for environments where
/// `vox_repository::resolve_repo_root_for_ci` is unavailable.
#[allow(dead_code)]
fn find_cargo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join("Cargo.toml").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest file must be present and parse without error.
    #[test]
    fn manifest_parses_from_repo_root() {
        // Walk from the manifest file's known location relative to this source
        // file: crates/vox-cli/src/commands/audit.rs → repo root is 5 levels up.
        let src = std::path::Path::new(file!());
        // file!() gives a relative path from repo root; convert to absolute.
        let abs = std::env::current_dir()
            .expect("cwd")
            .join(src)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(file!()));

        // Walk upward to find Cargo.toml (workspace root).
        let mut root = abs.as_path();
        let manifest_path = loop {
            let candidate = root.join("contracts/ci/check-targets.v1.yaml");
            if candidate.exists() {
                break candidate;
            }
            match root.parent() {
                Some(p) => root = p,
                None => {
                    // If we can't find it (e.g. in a detached worktree without the
                    // contracts dir), skip the test rather than fail.
                    eprintln!("skip: contracts/ci/check-targets.v1.yaml not found from {abs:?}");
                    return;
                }
            }
        };

        let content = std::fs::read_to_string(&manifest_path).expect("read check-targets.v1.yaml");
        let manifest: CheckManifest =
            serde_yaml::from_str(&content).expect("parse check-targets.v1.yaml");
        assert_eq!(manifest.schema_version, 1, "schema_version must be 1");
        assert!(
            !manifest.checks.is_empty(),
            "manifest must list at least one check"
        );
        // All checks must have non-empty ids and commands.
        for check in &manifest.checks {
            assert!(!check.id.is_empty(), "check id must not be empty");
            assert!(
                !check.command.is_empty(),
                "check `{}` command must not be empty",
                check.id
            );
        }
    }

    /// `filter_checks` by category must return only matching entries.
    #[test]
    fn filter_by_category() {
        let checks = vec![
            CheckEntry {
                id: "fmt".into(),
                description: "fmt".into(),
                category: "lint".into(),
                blocking: true,
                runs_on: vec!["ci".into()],
                rust_only: true,
                command: vec!["cargo".into(), "fmt".into()],
                quick_skip: false,
            },
            CheckEntry {
                id: "arch-check".into(),
                description: "arch".into(),
                category: "arch".into(),
                blocking: true,
                runs_on: vec!["ci".into()],
                rust_only: true,
                command: vec!["cargo".into(), "run".into()],
                quick_skip: false,
            },
        ];

        let args = AuditArgs {
            command: None,
            category: Some("lint".into()),
            list: false,
            dry_run: false,
            quick: false,
            gate: None,
            list_gates: false,
            format: "json".into(),
            corpus: None,
            threshold: None,
            no_canonical_report: false,
            strict_block_ga: false,
        };
        let filtered = filter_checks(&checks, &args);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "fmt");
    }

    // -----------------------------------------------------------------------
    // CR-L gate bridge (B1) — verify routing into `vox_audit::registry()`.
    // -----------------------------------------------------------------------

    fn args_for_gate(gate: &str) -> AuditArgs {
        AuditArgs {
            command: None,
            category: None,
            list: false,
            dry_run: false,
            quick: false,
            gate: Some(gate.into()),
            list_gates: false,
            format: "json".into(),
            corpus: None,
            threshold: None,
            no_canonical_report: true,
            strict_block_ga: false,
        }
    }

    #[test]
    fn build_common_args_round_trips_format_and_corpus() {
        let args = AuditArgs {
            // vox-arch-check: allow abs-path
            corpus: Some(std::path::PathBuf::from("/tmp/contracts")),
            threshold: Some(0.5),
            ..args_for_gate("retirement")
        };
        let common = build_common_args(&args).expect("build common args");
        assert_eq!(common.threshold, Some(0.5));
        assert_eq!(
            common.corpus.as_deref(),
            // vox-arch-check: allow abs-path
            Some(std::path::Path::new("/tmp/contracts"))
        );
        assert!(
            !common.write_canonical_report,
            "no_canonical_report: true should disable writing"
        );
    }

    #[test]
    fn build_common_args_rejects_bad_format_string() {
        let args = AuditArgs {
            format: "xml".into(),
            ..args_for_gate("retirement")
        };
        assert!(build_common_args(&args).is_err());
    }

    #[test]
    fn gate_descriptor_serializes_to_stable_json_shape() {
        // The dashboard and other tooling consume this JSON; lock the
        // field names so a rename here is a contract change.
        let d = GateDescriptor {
            name: "retirement".into(),
            gate: "L6Retirement".into(),
            block_ga: true,
            cost_metered: false,
            description: "CR-L6 description".into(),
        };
        let json = serde_json::to_string(&d).expect("ser");
        assert!(json.contains("\"name\":\"retirement\""));
        assert!(json.contains("\"gate\":\"L6Retirement\""));
        assert!(json.contains("\"block_ga\":true"));
        assert!(json.contains("\"cost_metered\":false"));
        assert!(json.contains("\"description\":\"CR-L6 description\""));
    }

    #[test]
    fn gate_listing_envelope_serializes_with_schema_version() {
        // A12: dashboards consume the envelope, not a bare array. The
        // top-level shape must match `{schema_version: 1, gates: [...]}`.
        let descriptors = vec![GateDescriptor {
            name: "retirement".into(),
            gate: "L6Retirement".into(),
            block_ga: true,
            cost_metered: false,
            description: "x".into(),
        }];
        let envelope = GateListingEnvelope {
            schema_version: 1,
            gates: &descriptors,
        };
        let json = serde_json::to_string(&envelope).expect("ser");
        assert!(
            json.starts_with("{\"schema_version\":1"),
            "envelope must start with schema_version field; got {json}"
        );
        assert!(json.contains("\"gates\":["));
        assert!(json.contains("\"name\":\"retirement\""));
    }

    #[test]
    fn html_escape_handles_special_chars() {
        assert_eq!(
            html_escape("<a href=\"x\">&amp;</a>"),
            "&lt;a href=&quot;x&quot;&gt;&amp;amp;&lt;/a&gt;"
        );
    }

    #[test]
    fn gate_from_name_resolves_known_gates_via_vox_audit() {
        // Smoke-check that the bridge depends on the same registry as the
        // umbrella binary — every thing_name must round-trip.
        for gate in vox_audit::CrlGate::all() {
            let name = gate.thing_name();
            assert_eq!(vox_audit::gate_from_name(name), Some(gate));
        }
        assert_eq!(vox_audit::gate_from_name("does-not-exist"), None);
    }

    /// `filter_checks` with `--quick` skips `quick_skip: true` entries.
    #[test]
    fn filter_quick_skip() {
        let checks = vec![
            CheckEntry {
                id: "slow".into(),
                description: "slow check".into(),
                category: "test".into(),
                blocking: false,
                runs_on: vec!["ci".into()],
                rust_only: true,
                command: vec!["cargo".into(), "test".into()],
                quick_skip: true,
            },
            CheckEntry {
                id: "fast".into(),
                description: "fast check".into(),
                category: "lint".into(),
                blocking: true,
                runs_on: vec!["ci".into()],
                rust_only: false,
                command: vec!["cargo".into(), "fmt".into()],
                quick_skip: false,
            },
        ];

        let args = AuditArgs {
            command: None,
            category: None,
            list: false,
            dry_run: false,
            quick: true,
            gate: None,
            list_gates: false,
            format: "json".into(),
            corpus: None,
            threshold: None,
            no_canonical_report: false,
            strict_block_ga: false,
        };
        let filtered = filter_checks(&checks, &args);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "fast");
    }
}
