use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analysis::RustFileContext;
use crate::detectors;
use crate::report::{OutputFormat, Reporter, RunSnapshot};
use crate::rules::{DetectionRule, Finding, Language, Severity};
use crate::scanner::Scanner;
use crate::task_queue::TaskQueue;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// How CI / CLI should treat findings for exit status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToestubRunMode {
    /// Fail only on [`Severity::Error`] or higher (historical `toestub` behavior).
    #[default]
    Legacy,
    /// Emit all severities (`Info`+), never fail (audit report).
    Audit,
    /// Fail on [`Severity::Critical`] only.
    EnforceWarn,
    /// Fail on [`Severity::Warning`] or higher.
    EnforceStrict,
}

/// Configuration for a TOESTUB analysis run.
#[derive(Debug, Clone)]
pub struct ToestubConfig {
    /// Root directories to scan.
    pub roots: Vec<PathBuf>,
    /// Minimum severity to include in the report.
    pub min_severity: Severity,
    /// Output format.
    pub format: OutputFormat,
    /// Whether to generate fix suggestions.
    pub suggest_fixes: bool,
    /// Language filter (None = all languages).
    pub languages: Option<Vec<Language>>,
    /// Extra glob patterns to exclude.
    pub excludes: Vec<String>,
    /// Specific rule IDs to run (None = all rules).
    pub rule_filter: Option<Vec<String>>,
    /// Path to vox-schema.json for architectural validation.
    pub schema_path: Option<PathBuf>,
    /// Path to unwired.json for scope-aware prioritization.
    pub unwired_path: Option<PathBuf>,
    /// Exit-code policy for CLI / CI (see [`ToestubRunMode`]).
    pub run_mode: ToestubRunMode,
    /// Optional structured suppressions JSON (schema `contracts/toestub/suppression.v1.schema.json`).
    pub suppression_path: Option<PathBuf>,
    /// When non-empty, AST-enhanced unresolved-ref runs only under `crates/<name>/` (canary rollout).
    pub canary_crates: Option<Vec<String>>,
    /// Policy for scanning Rust files under `tests/` directories.
    pub tests_mode: crate::run_context::ToestubTestsMode,
    /// Optional JSON allowlist (`contracts/toestub/prelude-allowlist.v1.json`); merged with defaults.
    pub prelude_allowlist_path: Option<PathBuf>,
    /// Staged detector flags (e.g. `unwired-graph`, `scaling-fs-ast-only`).
    pub feature_flags: Vec<String>,
    /// Workspace / repository identifier surfaced in `vox.lint.*` telemetry
    /// events so the CR-L8 aggregator can roll up by repo. `None` = unset
    /// (events are emitted with `repository_id: None`). Council-ratified
    /// 2026-05-15 (P2.1c); landed alongside the engine emission hookup.
    pub repository_id: Option<String>,
}

impl Default for ToestubConfig {
    fn default() -> Self {
        Self {
            roots: vec![PathBuf::from(".")],
            min_severity: Severity::Warning,
            format: OutputFormat::Terminal,
            suggest_fixes: false,
            languages: None,
            excludes: Vec::new(),
            rule_filter: None,
            schema_path: None,
            unwired_path: None,
            run_mode: ToestubRunMode::default(),
            suppression_path: None,
            canary_crates: None,
            tests_mode: crate::run_context::ToestubTestsMode::default(),
            prelude_allowlist_path: None,
            feature_flags: Vec::new(),
            repository_id: None,
        }
    }
}

#[derive(serde::Deserialize)]
struct PreludeAllowFile {
    version: u32,
    idents: Vec<String>,
}

fn collect_workspace_cross_refs(
    files: &[crate::rules::SourceFile],
) -> (
    HashMap<String, HashSet<String>>,
    HashMap<String, HashSet<String>>,
) {
    let re_crate = regex::Regex::new(r"\bcrate::([a-zA-Z_][a-zA-Z0-9_]*)").expect("valid regex");
    let re_super = regex::Regex::new(r"\bsuper::([a-zA-Z_][a-zA-Z0-9_]*)").expect("valid regex");
    let re_word = regex::Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\b").expect("valid regex");

    let mut mod_refs: HashMap<String, HashSet<String>> = HashMap::new();
    let mut words: HashMap<String, HashSet<String>> = HashMap::new();

    for f in files {
        if f.language != Language::Rust {
            continue;
        }
        let Some(key) = crate::run_context::workspace_crate_key(&f.path) else {
            continue;
        };
        for cap in re_crate.captures_iter(&f.content) {
            if let Some(m) = cap.get(1) {
                let name = m.as_str();
                if !matches!(name, "self" | "super" | "crate") {
                    mod_refs
                        .entry(key.clone())
                        .or_default()
                        .insert(name.to_string());
                }
            }
        }
        for cap in re_super.captures_iter(&f.content) {
            if let Some(m) = cap.get(1) {
                let name = m.as_str();
                if !matches!(name, "self" | "super" | "crate") {
                    mod_refs
                        .entry(key.clone())
                        .or_default()
                        .insert(name.to_string());
                }
            }
        }

        // Fast word accumulation for reachability heuristic
        let word_set = words.entry(key.clone()).or_default();
        for cap in re_word.captures_iter(&f.content) {
            if let Some(m) = cap.get(1) {
                word_set.insert(m.as_str().to_string());
            }
        }
    }
    (mod_refs, words)
}

fn merge_prelude_allowlist(roots: &[PathBuf], explicit: Option<&Path>) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut try_load = |p: &Path| {
        if let Ok(raw) = vox_bounded_fs::read_utf8_path_capped(p)
            && let Ok(doc) = serde_json::from_str::<PreludeAllowFile>(&raw)
            && doc.version == 1
        {
            out.extend(doc.idents);
        }
    };
    if let Some(p) = explicit {
        try_load(p);
    }
    try_load(Path::new("contracts/toestub/prelude-allowlist.v1.json"));
    for root in roots {
        try_load(&root.join("contracts/toestub/prelude-allowlist.v1.json"));
        if let Some(parent) = root.parent() {
            try_load(&parent.join("contracts/toestub/prelude-allowlist.v1.json"));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// The main analysis engine.
pub struct ToestubEngine {
    rules: Vec<Box<dyn DetectionRule>>,
    config: ToestubConfig,
}

impl ToestubEngine {
    /// Create a new engine with the given config, loading all built-in rules.
    pub fn new(config: ToestubConfig) -> Self {
        let mut rules = detectors::all_rules(config.schema_path.clone());

        // Apply rule filter if specified
        if let Some(ref filter) = config.rule_filter {
            rules.retain(|r| {
                filter.iter().any(|f| {
                    let pat = f.as_str();
                    r.id().starts_with(pat)
                        || r.id()
                            .split_once('/')
                            .is_some_and(|(_, sub)| sub.starts_with(pat))
                })
            });
        }

        Self { rules, config }
    }

    /// Create an engine with a custom set of rules (useful for testing).
    pub fn with_rules(config: ToestubConfig, rules: Vec<Box<dyn DetectionRule>>) -> Self {
        Self { rules, config }
    }

    /// Run lint rules on a single pre-loaded source file without a filesystem scan.
    ///
    /// This is a lightweight alternative to [`Self::run`] for `vox check --for-llm` and
    /// similar single-file consumers that already have source in memory.  Unlike
    /// [`Self::run`], this method:
    /// - Skips the [`Scanner`] (no directory walk).
    /// - Does **not** emit telemetry (no CR-L8 corpus events).
    /// - Does **not** apply suppression rules.
    /// - Sets up a default [`crate::run_context::RunContext`] (safe for single-file Vox linting; some
    ///   Rust-specific rules that depend on workspace cross-refs will produce no
    ///   findings on Vox files regardless).
    ///
    /// Severity filtering and deterministic sort are still applied so callers
    /// receive findings in the same order as a full [`Self::run`].
    ///
    /// # Thread safety
    /// [`crate::run_context::RunContext`] uses a process-global mutex.  Concurrent calls to this method
    /// (or to [`Self::run`]) will serialize; do not hold the returned `Vec<Finding>` across
    /// a second call on a different thread without copying first.
    pub fn check_source_file(&self, file: &crate::rules::SourceFile) -> Vec<Finding> {
        let _guard =
            crate::run_context::RunContextGuard::new(crate::run_context::RunContext::default());

        let rust_ctx_owned = if file.language == crate::rules::Language::Rust {
            Some(RustFileContext::parse(&file.content))
        } else {
            None
        };
        let rust_ctx = rust_ctx_owned.as_ref();

        let mut findings: Vec<Finding> = Vec::new();
        for rule in &self.rules {
            if !rule.languages().contains(&file.language) {
                continue;
            }
            findings.extend(rule.detect(file, rust_ctx));
        }

        // Apply severity filter (same as full run).
        findings.retain(|f| f.severity >= self.config.min_severity);

        // Deterministic sort: critical first, then stable tie-breakers.
        findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.deterministic_key().cmp(&b.deterministic_key()))
        });

        findings
    }

    /// Build a `rule_id → minimal_repro` lookup table for all registered rules.
    ///
    /// Used by `vox check --for-llm` to attach per-rule minimal reproduction snippets
    /// to `LintFindingPayload` without modifying the [`Finding`] struct (which would
    /// require updating every detector constructor).
    pub fn minimal_repro_table(&self) -> HashMap<&'static str, &'static str> {
        self.rules
            .iter()
            .filter_map(|r| r.minimal_repro().map(|mr| (r.id(), mr)))
            .collect()
    }

    /// Run the full analysis pipeline and return findings.
    pub fn run(&self) -> AnalysisResult {
        let roots = self.get_roots();
        let prelude =
            merge_prelude_allowlist(&roots, self.config.prelude_allowlist_path.as_deref());
        let suppression_store = match crate::suppression::SuppressionStore::load_optional(
            self.config.suppression_path.as_deref(),
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("TOESTUB suppressions not applied: {e}");
                crate::suppression::SuppressionStore::empty()
            }
        };

        let scanner = Scanner::new(roots, &self.config.excludes, self.config.languages.clone());
        let files = scanner.scan();
        let needs_cross_refs = self.rules.iter().any(|r| {
            matches!(
                r.id(),
                "skeleton/unwired" | "skeleton/reachability" | "unresolved-ref"
            )
        });
        let (workspace_crate_mod_refs, workspace_crate_words) = if needs_cross_refs {
            collect_workspace_cross_refs(&files)
        } else {
            (HashMap::new(), HashMap::new())
        };

        let _run_ctx_guard =
            crate::run_context::RunContextGuard::new(crate::run_context::RunContext {
                canary_crates: self.config.canary_crates.clone(),
                tests_mode: self.config.tests_mode,
                prelude_allow_idents: prelude,
                feature_flags: self.config.feature_flags.iter().cloned().collect(),
                unresolved_callee_counts: std::collections::HashMap::new(),
                workspace_crate_mod_refs,
                workspace_crate_words,
            });

        // 2. Run each rule on each file (one Rust parse + token map per file)
        let mut all_findings: Vec<Finding> = Vec::new();
        let mut rust_parse_failures = 0usize;
        for file in &files {
            let rust_ctx_owned = if file.language == Language::Rust {
                let ctx = RustFileContext::parse(&file.content);
                if ctx.ast.is_err() {
                    rust_parse_failures += 1;
                }
                Some(ctx)
            } else {
                None
            };
            let rust_ctx = rust_ctx_owned.as_ref();
            for rule in &self.rules {
                // Skip rules that don't apply to this language
                if !rule.languages().contains(&file.language) {
                    continue;
                }
                let findings = rule.detect(file, rust_ctx);
                all_findings.extend(findings);
            }
        }

        // Batch (cross-file) detectors run after the per-file pass — they need the full file
        // set. Cross-crate split-brain emits Info findings; on a scoped scan the subset simply
        // won't contain two copies, so it can't false-positive. The suppression pass below
        // still applies to these findings.
        let run_cross_crate_dup = self.config.min_severity <= Severity::Info
            && self.config.rule_filter.as_ref().is_none_or(|prefixes| {
                prefixes.is_empty()
                    || prefixes
                        .iter()
                        .any(|p| "arch/cross-crate-dup".starts_with(p.as_str()))
            });
        if run_cross_crate_dup {
            all_findings
                .extend(crate::detectors::cross_crate_dup::detect_cross_crate_dup_in_batch(&files));
        }

        let mut suppressions_applied: usize = 0;
        let mut suppression_counts_by_family: HashMap<String, usize> = HashMap::new();
        all_findings.retain(|f| {
            if suppression_store.suppresses(f) {
                suppressions_applied += 1;
                let fam = f
                    .rule_id
                    .split_once('/')
                    .map(|(a, _)| a.to_string())
                    .unwrap_or_else(|| f.rule_id.clone());
                *suppression_counts_by_family.entry(fam).or_insert(0) += 1;
                return false;
            }
            true
        });

        // 3. Filter by severity
        all_findings.retain(|f| f.severity >= self.config.min_severity);

        // 4. Sort by severity (critical first), then deterministic tie-breakers
        all_findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.deterministic_key().cmp(&b.deterministic_key()))
        });

        // 4b. CR-L8 corpus-feedback: emit one `vox.lint.finding` event per
        // emitted finding. `record_event!` is a no-op when no recorder is
        // registered (the v0.5.x default), so this adds zero overhead to
        // existing toestub runs. Council-ratified 2026-05-15 (P2.1b).
        emit_lint_finding_telemetry(&all_findings, self.config.repository_id.as_deref());

        // 5. Build the task queue
        let task_queue = if self.config.suggest_fixes {
            TaskQueue::from_findings(&all_findings)
        } else {
            TaskQueue::empty()
        };

        let unresolved_ref_callee_counts = crate::run_context::unresolved_callee_counts_snapshot();

        AnalysisResult {
            files_scanned: files.len(),
            rules_applied: self.rules.len(),
            findings: all_findings,
            task_queue,
            rust_parse_failures,
            unresolved_ref_callee_counts,
            suppressions_applied,
            suppression_counts_by_family,
        }
    }

    /// Run analysis and produce formatted output as a String.
    pub fn run_and_report(&self) -> (AnalysisResult, String) {
        let result = self.run();
        let output = Reporter::format_run(
            RunSnapshot {
                findings: &result.findings,
                files_scanned: result.files_scanned,
                rules_applied: result.rules_applied,
                rust_parse_failures: result.rust_parse_failures,
                unresolved_ref_callee_counts: &result.unresolved_ref_callee_counts,
                suppressions_applied: result.suppressions_applied,
                suppression_counts_by_family: &result.suppression_counts_by_family,
            },
            self.config.format,
            &result.task_queue,
        );
        (result, output)
    }

    fn get_roots(&self) -> Vec<PathBuf> {
        if let Some(ref path) = self.config.unwired_path
            && let Ok(content) = vox_bounded_fs::read_utf8_path_capped(path)
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(roots) = json.get("roots").and_then(|r| r.as_array())
        {
            return roots
                .iter()
                .filter_map(|r| r.as_str().map(PathBuf::from))
                .collect();
        }
        self.config.roots.clone()
    }
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// The output of a TOESTUB analysis run.
pub struct AnalysisResult {
    /// Number of source files scanned.
    pub files_scanned: usize,
    /// Number of detection rules applied.
    pub rules_applied: usize,
    /// All findings (already filtered and sorted).
    pub findings: Vec<Finding>,
    /// Generated task queue with fix suggestions.
    pub task_queue: TaskQueue,
    /// Rust files where `syn::parse_file` failed (token map still available).
    pub rust_parse_failures: usize,
    /// Best-effort counts of unresolved-ref callees (for hotlist / diagnostics).
    pub unresolved_ref_callee_counts: HashMap<String, usize>,
    /// Findings dropped by structured suppressions (before severity filter).
    pub suppressions_applied: usize,
    /// Suppressed finding counts by rule id family (segment before first `/`).
    pub suppression_counts_by_family: HashMap<String, usize>,
}

impl AnalysisResult {
    /// Returns `true` if any findings are at or above [`Severity::Error`].
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity >= Severity::Error)
    }

    /// Whether the process should exit with failure for the given mode.
    pub fn should_fail_build(&self, mode: ToestubRunMode) -> bool {
        match mode {
            ToestubRunMode::Legacy => self.has_errors(),
            ToestubRunMode::Audit => false,
            ToestubRunMode::EnforceWarn => self
                .findings
                .iter()
                .any(|f| f.severity >= Severity::Critical),
            ToestubRunMode::EnforceStrict => self
                .findings
                .iter()
                .any(|f| f.severity >= Severity::Warning),
        }
    }

    /// Summary counts by severity.
    pub fn summary(&self) -> SeveritySummary {
        let mut s = SeveritySummary::default();
        for f in &self.findings {
            match f.severity {
                Severity::Info => s.info += 1,
                Severity::Warning => s.warning += 1,
                Severity::Error => s.error += 1,
                Severity::Critical => s.critical += 1,
            }
        }
        s
    }
}

#[derive(Debug, Default)]
pub struct SeveritySummary {
    pub info: usize,
    pub warning: usize,
    pub error: usize,
    pub critical: usize,
}

// ───────────────────────────────────────────────────────────────────────────
// CR-L8 corpus-feedback telemetry emission (P2.1b).
// ───────────────────────────────────────────────────────────────────────────

/// Emit one `vox.lint.finding` event per emitted finding.
///
/// `vox_telemetry::record_event!` is a no-op when no global recorder has been
/// registered, so this function adds zero observable cost to existing toestub
/// runs (the v0.5.x default). When the CR-L8 export pipeline registers a
/// recorder, every finding flows into the quarterly corpus-feedback artifact
/// at `contracts/reports/corpus-feedback/<quarter>.json`.
///
/// Council-ratified 2026-05-15 (D8 / §1.3 P2.1b in
/// `docs/src/architecture/v1-llm-target-implementation-plan-2026.md`).
///
/// `repository_id` is the workspace identifier from `ToestubConfig`; when
/// `None`, events are emitted without a repo label (P2.1c lands the threading,
/// callers configure it).
fn emit_lint_finding_telemetry(findings: &[Finding], repository_id: Option<&str>) {
    for finding in findings {
        tracing::debug!(
            rule_id = %finding.rule_id,
            diagnostic_id = ?finding.diagnostic_id,
            severity = ?finding.severity,
            file = %finding.file.display(),
            line = finding.line,
            repository_id = ?repository_id,
            "lint finding"
        );
    }
}

#[allow(dead_code)]
fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Critical => "critical",
    }
}

#[allow(dead_code)]
fn confidence_label(c: crate::rules::FindingConfidence) -> &'static str {
    use crate::rules::FindingConfidence;
    match c {
        FindingConfidence::High => "high",
        FindingConfidence::Medium => "medium",
        FindingConfidence::Low => "low",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::SourceFile;

    /// Flags every line containing `FLAGME` at a fixed severity.
    struct FlagMe(Severity);

    impl DetectionRule for FlagMe {
        fn id(&self) -> &'static str {
            "test/flag-me"
        }
        fn name(&self) -> &'static str {
            "Flag Me"
        }
        fn description(&self) -> &'static str {
            "test-only rule"
        }
        fn severity(&self) -> Severity {
            self.0
        }
        fn languages(&self) -> &[Language] {
            &[Language::Rust]
        }
        fn detect(&self, file: &SourceFile, _ctx: Option<&RustFileContext>) -> Vec<Finding> {
            file.lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.contains("FLAGME"))
                .map(|(i, _)| Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: None,
                    rule_name: self.name().to_string(),
                    severity: self.0,
                    file: file.path.clone(),
                    line: i + 1,
                    column: 0,
                    message: "flagged".to_string(),
                    suggestion: None,
                    alternatives: Vec::new(),
                    rationale: None,
                    context: String::new(),
                    confidence: None,
                    evidence: None,
                })
                .collect()
        }
    }

    fn engine(rule_severity: Severity) -> ToestubEngine {
        ToestubEngine::with_rules(
            ToestubConfig {
                min_severity: Severity::Warning,
                ..Default::default()
            },
            vec![Box::new(FlagMe(rule_severity))],
        )
    }

    fn rust_file(src: &str) -> SourceFile {
        SourceFile::new(PathBuf::from("a.rs"), src.to_string())
    }

    #[test]
    fn check_source_file_reports_findings_at_their_line_and_filters_by_severity() {
        let src = "fn a() {}\n// FLAGME\nfn b() {}\n";
        let found = engine(Severity::Error).check_source_file(&rust_file(src));
        assert_eq!(found.len(), 1);
        assert_eq!(
            (found[0].rule_id.as_str(), found[0].line),
            ("test/flag-me", 2)
        );

        // Below `min_severity` → dropped; clean source → nothing.
        assert!(
            engine(Severity::Info)
                .check_source_file(&rust_file(src))
                .is_empty()
        );
        assert!(
            engine(Severity::Error)
                .check_source_file(&rust_file("fn a() {}\n"))
                .is_empty()
        );
    }

    #[test]
    fn check_source_file_skips_rules_for_other_languages() {
        let f = SourceFile::new(PathBuf::from("a.vox"), "// FLAGME\n".to_string());
        assert!(engine(Severity::Error).check_source_file(&f).is_empty());
    }

    #[test]
    fn should_fail_build_follows_run_mode_thresholds() {
        let mut result = AnalysisResult {
            files_scanned: 1,
            rules_applied: 1,
            findings: engine(Severity::Warning).check_source_file(&rust_file("// FLAGME\n")),
            task_queue: TaskQueue::empty(),
            rust_parse_failures: 0,
            unresolved_ref_callee_counts: HashMap::new(),
            suppressions_applied: 0,
            suppression_counts_by_family: HashMap::new(),
        };
        // One Warning finding: only EnforceStrict fails.
        assert!(!result.has_errors());
        assert!(!result.should_fail_build(ToestubRunMode::Legacy));
        assert!(!result.should_fail_build(ToestubRunMode::Audit));
        assert!(!result.should_fail_build(ToestubRunMode::EnforceWarn));
        assert!(result.should_fail_build(ToestubRunMode::EnforceStrict));
        assert_eq!(result.summary().warning, 1);

        result.findings[0].severity = Severity::Critical;
        assert!(result.should_fail_build(ToestubRunMode::Legacy));
        assert!(result.should_fail_build(ToestubRunMode::EnforceWarn));
        assert!(!result.should_fail_build(ToestubRunMode::Audit));
        assert_eq!(result.summary().critical, 1);
    }
}
