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
) -> (HashMap<String, HashSet<String>>, HashMap<String, HashSet<String>>) {
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
                    mod_refs.entry(key.clone()).or_default().insert(name.to_string());
                }
            }
        }
        for cap in re_super.captures_iter(&f.content) {
            if let Some(m) = cap.get(1) {
                let name = m.as_str();
                if !matches!(name, "self" | "super" | "crate") {
                    mod_refs.entry(key.clone()).or_default().insert(name.to_string());
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
            rules.retain(|r| filter.iter().any(|f| r.id().starts_with(f.as_str())));
        }

        Self { rules, config }
    }

    /// Create an engine with a custom set of rules (useful for testing).
    pub fn with_rules(config: ToestubConfig, rules: Vec<Box<dyn DetectionRule>>) -> Self {
        Self { rules, config }
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
        let (workspace_crate_mod_refs, workspace_crate_words) = collect_workspace_cross_refs(&files);

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
