use super::*;

#[derive(Default)]
pub(crate) struct Report {
    pub(crate) inversions: Vec<(String, String, u8, u8)>,
    pub(crate) fan_in_warns: Vec<(String, usize, usize)>,
    pub(crate) loc_warns: Vec<(String, usize, usize)>,
    pub(crate) orphan_warns: Vec<String>,
    /// Docstring findings split by strictness: (name, strict)
    pub(crate) docstring_warns: Vec<(String, bool)>,
    pub(crate) description_warns: Vec<String>,
    pub(crate) where_things_live_warns: Vec<String>,
    /// Rule 8: (crate_name, last_commit_date YYYY-MM-DD).
    pub(crate) staleness_warns: Vec<(String, String)>,
    /// "vX.Y.Z (YYYY-MM-DD)" used in the staleness summary line.
    pub(crate) staleness_since: String,
    /// Rule 9: (file_path_relative, expected_hash, actual_hash).
    pub(crate) generated_file_drift_warns: Vec<(PathBuf, String, String)>,
    /// Rule 10: (crate, forbidden_dep) pairs for direct forbidden-dep violations.
    pub(crate) forbidden_dep_violations: Vec<(String, String)>,
    /// Rule 11 (P3-T7): (rule_name, file, line, matched, reason) tuples.
    pub(crate) forbidden_pattern_hits: Vec<(String, PathBuf, usize, String, String)>,
    /// Rule 12: WTL / layers.toml / disk three-way parity violations.
    pub(crate) wtl_parity_warns: Vec<String>,
    /// Rule 13: (crate, current_loc, baseline_loc, pct_growth).
    pub(crate) loc_delta_warns: Vec<(String, usize, usize, f64)>,
    /// Rule 14: (consumer, cdylib_dep) pairs where a normal dep links a cdylib.
    pub(crate) cdylib_dep_warns: Vec<(String, String)>,
    /// Rule 15: (crate, ws_dep_count, budget) tuples over the workspace-dep budget.
    pub(crate) workspace_dep_warns: Vec<(String, usize, usize)>,
    /// Rule 16: evidence-ledger integrity findings (missing / stale artifacts).
    pub(crate) evidence_findings: Vec<EvidenceFinding>,
    /// Whether each rule's failure should be treated as strict (vs. warn-only).
    pub(crate) strict_layer: bool,
    pub(crate) strict_fan_in: bool,
    pub(crate) strict_loc: bool,
    /// `loc_budget = "off"`: skip the absolute `max_loc` threshold entirely.
    /// Distinct from `!strict_loc`, which still collects and prints warnings.
    pub(crate) loc_budget_off: bool,
    pub(crate) strict_orphan: bool,
    pub(crate) strict_docstring: bool,
    pub(crate) strict_description: bool,
    pub(crate) strict_where_things_live: bool,
    pub(crate) strict_staleness: bool,
    pub(crate) strict_generated_file_drift: bool,
    pub(crate) strict_forbidden_deps: bool,
    pub(crate) strict_forbidden_pattern: bool,
    pub(crate) strict_wtl_parity: bool,
    pub(crate) strict_loc_delta: bool,
    pub(crate) strict_cdylib_dep: bool,
    pub(crate) strict_workspace_deps: bool,
    /// Rule 16 strictness. Default: false — ledger is freshly seeded and many
    /// rows point at not-yet-existing artifacts. Flip to true once
    /// `vox audit --gate all --strict-block-ga` exits 0 (i.e. when block-GA
    /// gates are met by real measurements rather than corpus-inventory).
    pub(crate) strict_evidence_ledger: bool,
    /// Rule 17: dependency cycles detected by Tarjan SCC. Always a hard error.
    /// Each entry is one cycle: a sorted list of crate names forming the SCC.
    pub(crate) cycle_errors: Vec<Vec<String>>,
    /// Rule 18: publishability closure gate (hard error).
    /// Pairs of (publishable_crate, unpublishable_dep).
    pub(crate) publishability_errors: Vec<(String, String)>,
    /// Rule 19: dep-closure-size budget gate. Always a hard error when fired.
    pub(crate) closure_budget_errors: Vec<String>,
    /// Rule 20: named build profiles must not contain forbidden crates.
    pub(crate) profile_forbidden_errors: Vec<String>,
}

impl Report {
    pub(crate) fn strict_failed(&self) -> bool {
        (self.strict_layer && !self.inversions.is_empty())
            || (self.strict_fan_in && !self.fan_in_warns.is_empty())
            || (self.strict_loc && !self.loc_warns.is_empty())
            || (self.strict_orphan && !self.orphan_warns.is_empty())
            || self.docstring_warns.iter().any(|(_, strict)| *strict)
            || (self.strict_description && !self.description_warns.is_empty())
            || (self.strict_where_things_live && !self.where_things_live_warns.is_empty())
            || (self.strict_staleness && !self.staleness_warns.is_empty())
            || (self.strict_generated_file_drift && !self.generated_file_drift_warns.is_empty())
            || (self.strict_forbidden_deps && !self.forbidden_dep_violations.is_empty())
            || (self.strict_forbidden_pattern && !self.forbidden_pattern_hits.is_empty())
            || (self.strict_wtl_parity && !self.wtl_parity_warns.is_empty())
            || (self.strict_loc_delta && !self.loc_delta_warns.is_empty())
            || (self.strict_cdylib_dep && !self.cdylib_dep_warns.is_empty())
            || (self.strict_workspace_deps && !self.workspace_dep_warns.is_empty())
            || (self.strict_evidence_ledger
                && self
                    .evidence_findings
                    .iter()
                    .any(|f| f.kind.severity() == "ERROR"))
            || !self.cycle_errors.is_empty()
            || !self.closure_budget_errors.is_empty()
            || !self.publishability_errors.is_empty()
            || !self.profile_forbidden_errors.is_empty()
    }

    /// Project the report into per-rule results for the policy-status overlay.
    /// Ids match the `arch-rule` registry namespace (`arch-rule/<guard-key>`),
    /// covering exactly the 11 guards catalogued from `layers.toml [guards]`.
    ///
    /// DEVIATION FROM PLAN: the plan used `arch/<guard>` ids and listed
    /// `where_things_live`, `layers`, and `docstring`. The committed registry
    /// (Plan 1b) uses `arch-rule/<guard>` and catalogs exactly these 11 guard
    /// keys: description, docstring, fan_in, forbidden_deps,
    /// generated_file_drift, loc_budget, loc_delta, orphan, staleness,
    /// where_things_live, wtl_parity. There is no `arch-rule/layers` entry, so
    /// the layer-ordering check is intentionally not projected (untracked grey).
    pub(crate) fn to_rule_results(&self) -> Vec<crate::json_report::ArchRuleResult> {
        use crate::json_report::{ArchRuleResult, status_str};
        let mk = |id: &str, has: bool, strict: bool, count: usize| ArchRuleResult {
            id: format!("arch-rule/{id}"),
            status: status_str(has, strict).to_string(),
            count,
        };
        vec![
            mk(
                "description",
                !self.description_warns.is_empty(),
                self.strict_description,
                self.description_warns.len(),
            ),
            mk(
                "docstring",
                !self.docstring_warns.is_empty(),
                self.docstring_warns.iter().any(|(_, strict)| *strict),
                self.docstring_warns.len(),
            ),
            mk(
                "fan_in",
                !self.fan_in_warns.is_empty(),
                self.strict_fan_in,
                self.fan_in_warns.len(),
            ),
            mk(
                "forbidden_deps",
                !self.forbidden_dep_violations.is_empty(),
                self.strict_forbidden_deps,
                self.forbidden_dep_violations.len(),
            ),
            mk(
                "generated_file_drift",
                !self.generated_file_drift_warns.is_empty(),
                self.strict_generated_file_drift,
                self.generated_file_drift_warns.len(),
            ),
            mk(
                "loc_budget",
                !self.loc_warns.is_empty(),
                self.strict_loc,
                self.loc_warns.len(),
            ),
            mk(
                "loc_delta",
                !self.loc_delta_warns.is_empty(),
                self.strict_loc_delta,
                self.loc_delta_warns.len(),
            ),
            mk(
                "orphan",
                !self.orphan_warns.is_empty(),
                self.strict_orphan,
                self.orphan_warns.len(),
            ),
            mk(
                "staleness",
                !self.staleness_warns.is_empty(),
                self.strict_staleness,
                self.staleness_warns.len(),
            ),
            mk(
                "where_things_live",
                !self.where_things_live_warns.is_empty(),
                self.strict_where_things_live,
                self.where_things_live_warns.len(),
            ),
            mk(
                "wtl_parity",
                !self.wtl_parity_warns.is_empty(),
                self.strict_wtl_parity,
                self.wtl_parity_warns.len(),
            ),
        ]
    }

    pub(crate) fn print_summary(&self) {
        let mut any = false;
        if !self.cycle_errors.is_empty() {
            any = true;
            eprintln!(
                "[ERROR] Rule 17: dependency cycle(s) detected ({}):",
                self.cycle_errors.len()
            );
            for cycle in &self.cycle_errors {
                eprintln!("  cycle: {}", cycle.join(" ↔ "));
            }
        }
        if !self.publishability_errors.is_empty() {
            any = true;
            eprintln!(
                "[ERROR] Rule 18: publishable crates with unpublishable deps ({}):",
                self.publishability_errors.len()
            );
            for (krate, dep) in &self.publishability_errors {
                eprintln!("  {krate} → {dep} (publish = false)");
            }
        }
        if !self.closure_budget_errors.is_empty() {
            any = true;
            eprintln!(
                "[ERROR] Rule 19: dep-closure-size budget exceeded ({}):",
                self.closure_budget_errors.len()
            );
            for msg in &self.closure_budget_errors {
                eprintln!("  {msg}");
            }
        }
        if !self.profile_forbidden_errors.is_empty() {
            any = true;
            eprintln!(
                "[ERROR] Rule 20: profile forbidden-crate violations ({}):",
                self.profile_forbidden_errors.len()
            );
            for msg in &self.profile_forbidden_errors {
                eprintln!("  {msg}");
            }
        }
        if !self.inversions.is_empty() {
            any = true;
            let label = if self.strict_layer { "ERROR" } else { "warn" };
            eprintln!("[{label}] layer inversions ({}):", self.inversions.len());
            for (from, to, fl, tl) in &self.inversions {
                eprintln!("  {from} (L{fl}) → {to} (L{tl})");
            }
        }
        if !self.fan_in_warns.is_empty() {
            any = true;
            let label = if self.strict_fan_in { "ERROR" } else { "warn" };
            eprintln!(
                "[{label}] fan-in over budget ({}):",
                self.fan_in_warns.len()
            );
            for (name, count, budget) in &self.fan_in_warns {
                eprintln!("  {name}: {count} dependents (budget {budget})");
            }
        }
        if !self.loc_warns.is_empty() {
            any = true;
            let label = if self.strict_loc { "ERROR" } else { "warn" };
            eprintln!("[{label}] LoC budget exceeded ({}):", self.loc_warns.len());
            for (name, loc, budget) in &self.loc_warns {
                eprintln!("  {name}: {loc} LoC (budget {budget})");
            }
        }
        if !self.orphan_warns.is_empty() {
            any = true;
            let label = if self.strict_orphan { "ERROR" } else { "warn" };
            eprintln!(
                "[{label}] orphan crates ({}) — 0 in-tree consumers and kind=library:",
                self.orphan_warns.len()
            );
            for name in &self.orphan_warns {
                eprintln!("  {name}");
            }
        }
        // Docstring: split strict vs. warn findings for display
        let docstring_strict: Vec<&str> = self
            .docstring_warns
            .iter()
            .filter(|(_, s)| *s)
            .map(|(n, _)| n.as_str())
            .collect();
        let docstring_warn: Vec<&str> = self
            .docstring_warns
            .iter()
            .filter(|(_, s)| !*s)
            .map(|(n, _)| n.as_str())
            .collect();
        if !docstring_strict.is_empty() {
            any = true;
            eprintln!(
                "[ERROR] lib.rs without `//!` opening docstring — L0-L2 (strict) ({}):",
                docstring_strict.len()
            );
            for name in &docstring_strict {
                eprintln!("  {name}");
            }
        }
        if !docstring_warn.is_empty() {
            any = true;
            let label = if self.strict_docstring {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] lib.rs without `//!` opening docstring — L3+ ({}):",
                docstring_warn.len()
            );
            for name in &docstring_warn {
                eprintln!("  {name}");
            }
        }
        if !self.description_warns.is_empty() {
            any = true;
            let label = if self.strict_description {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] Cargo.toml description missing or too short ({}):",
                self.description_warns.len()
            );
            for msg in &self.description_warns {
                eprintln!("  {msg}");
            }
        }
        if !self.where_things_live_warns.is_empty() {
            any = true;
            let label = if self.strict_where_things_live {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] crates not listed in where-things-live.md ({}):",
                self.where_things_live_warns.len()
            );
            for msg in &self.where_things_live_warns {
                eprintln!("  {msg}");
            }
        }
        if !self.staleness_warns.is_empty() {
            any = true;
            let label = if self.strict_staleness {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] crates unchanged since {} ({}) — add `staleness_exempt = true` in layers.toml to silence:",
                self.staleness_since,
                self.staleness_warns.len()
            );
            for (name, date) in &self.staleness_warns {
                eprintln!("  {name}: last changed {date}");
            }
        }
        if !self.generated_file_drift_warns.is_empty() {
            any = true;
            let label = if self.strict_generated_file_drift {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] generated-file drift — hand-edited @generated files ({}):",
                self.generated_file_drift_warns.len()
            );
            for (path, expected, actual) in &self.generated_file_drift_warns {
                eprintln!(
                    "  {}: expected hash {expected}, got {actual}  (re-run the generator)",
                    path.display()
                );
            }
        }
        if !self.forbidden_dep_violations.is_empty() {
            any = true;
            let label = if self.strict_forbidden_deps {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] forbidden direct dependencies ({}):",
                self.forbidden_dep_violations.len()
            );
            for (krate, forbidden) in &self.forbidden_dep_violations {
                eprintln!("  {krate} → {forbidden}  (see [[forbidden_deps]] in layers.toml)");
            }
        }
        if !self.forbidden_pattern_hits.is_empty() {
            any = true;
            let label = if self.strict_forbidden_pattern {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] forbidden_pattern violations ({}):",
                self.forbidden_pattern_hits.len()
            );
            for (rule, file, line, matched, reason) in &self.forbidden_pattern_hits {
                eprintln!(
                    "  [{}] {}:{} — {}\n    reason: {reason}",
                    rule,
                    file.display(),
                    line,
                    matched
                );
            }
        }
        if !self.wtl_parity_warns.is_empty() {
            any = true;
            let label = if self.strict_wtl_parity {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] WTL/layers.toml/disk parity violations ({}):",
                self.wtl_parity_warns.len()
            );
            for msg in &self.wtl_parity_warns {
                eprintln!("  {msg}");
            }
            eprintln!(
                "  → Add missing entries to [planned] in layers.toml with a `plan =` pointer, or create the crate directory."
            );
        }
        if !self.loc_delta_warns.is_empty() {
            any = true;
            let label = if self.strict_loc_delta {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] LoC delta >15% since last release ({}) — review before merging:",
                self.loc_delta_warns.len()
            );
            for (name, current, baseline, pct) in &self.loc_delta_warns {
                eprintln!("  {name}: {current} LoC (was {baseline} at last release, +{pct:.0}%)");
            }
            eprintln!("  → Large deltas indicate scope creep. Consider extracting a sub-crate.");
        }
        if !self.cdylib_dep_warns.is_empty() {
            any = true;
            let label = if self.strict_cdylib_dep {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] cdylib linked as normal compile-time dep ({}) — use vox-plugin-host instead:",
                self.cdylib_dep_warns.len()
            );
            for (consumer, plugin) in &self.cdylib_dep_warns {
                eprintln!(
                    "  {consumer} → {plugin}  (cdylib must be loaded dynamically, not linked)"
                );
            }
        }
        if !self.workspace_dep_warns.is_empty() {
            any = true;
            let label = if self.strict_workspace_deps {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] workspace-dep budget exceeded ({}):",
                self.workspace_dep_warns.len()
            );
            for (name, count, budget) in &self.workspace_dep_warns {
                eprintln!("  {name}: {count} workspace deps (budget {budget})");
            }
        }
        if !self.evidence_findings.is_empty() {
            any = true;
            let strict_label = if self.strict_evidence_ledger {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[evidence-ledger] {} finding(s) — claims in `docs/src/architecture/vox-as-llm-target-audit-and-plan-2026.md` point at:",
                self.evidence_findings.len()
            );
            for f in &self.evidence_findings {
                let kind_label = match &f.kind {
                    FindingKind::MissingArtifact => "missing".to_string(),
                    FindingKind::DirectoryHasNoDatedReports => "no dated reports".to_string(),
                    FindingKind::Stale {
                        age_days,
                        max_age_days,
                    } => {
                        format!("stale ({age_days}d > {max_age_days}d budget)")
                    }
                    FindingKind::UnknownArtifactKind(k) => format!("unknown kind `{k}`"),
                };
                let sev = f.kind.severity();
                let row_label = if sev == "ERROR" { strict_label } else { sev };
                eprintln!(
                    "  [{row_label}] {claim} → {kind} @ {path}",
                    claim = f.claim_id,
                    kind = kind_label,
                    path = f.artifact_path.display(),
                );
            }
            eprintln!(
                "  → See contracts/reports/evidence-ledger.v1.json and the honest plan at docs/superpowers/specs/2026-05-21-v1-honest-completion-plan.md §1.2."
            );
        }
        if !any {
            eprintln!(
                "vox-arch-check {}: clean ✓",
                concat!(
                    env!("CARGO_PKG_VERSION"),
                    "+build.",
                    env!("VOX_BUILD_NUMBER"),
                    " (",
                    env!("VOX_GIT_HASH"),
                    ")"
                )
            );
        }
    }
}
