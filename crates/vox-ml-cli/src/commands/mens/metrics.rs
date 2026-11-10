//! Quantitative degradation and quality metrics for Mens model evaluation.

use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use vox_compiler::pipeline::FrontendResult;

/// Per-answer anti-stub floor on `ast_eval` construct richness: the most a single
/// declaration can score (1/8). Re-exported by `eval_local`; rationale lives there.
pub const ANTI_STUB_MIN_CONSTRUCT_RICHNESS: f64 = 0.125;

static TOKEN_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"[\w]+|[^\w\s]").expect("valid token regex"));

/// Computes the Distinct-4 repetition index: unique 4-grams divided by total 4-grams.
/// Returns 1.0 for inputs with fewer than 4 lexemes (length-guarded).
pub fn calculate_distinct_4(code: &str) -> f64 {
    let tokens: Vec<&str> = TOKEN_RE.find_iter(code).map(|m| m.as_str()).collect();
    if tokens.len() < 4 {
        return 1.0;
    }
    let total_4grams = tokens.len() - 3;
    let mut unique_4grams = HashSet::with_capacity(total_4grams);
    for window in tokens.windows(4) {
        unique_4grams.insert((window[0], window[1], window[2], window[3]));
    }
    unique_4grams.len() as f64 / total_4grams as f64
}

/// Verifies strict negative constraints:
/// - Zero deprecated decorators (`@endpoint`, `@mutation`, `@server`, `@query`, `@table`, etc.)
/// - Zero conversational / chatty commentary (e.g. "here is the", "certainly!", etc.)
pub fn check_negative_constraints(code: &str) -> bool {
    let lower = code.to_ascii_lowercase();

    // 1. Deprecated decorators (retired in v0.6.0; hard error per AGENTS.md)
    let deprecated_decorators = [
        "@endpoint",
        "@mutation",
        "@server",
        "@query",
        "@table",
        "@tool",
        "@resource",
        "@form",
        "@index",
    ];
    for dec in deprecated_decorators {
        if lower.contains(dec) {
            return false;
        }
    }

    // 2. Chatty conversational commentary
    let chatty_phrases = [
        "here is the",
        "here's the",
        "here is a",
        "here's a",
        "certainly!",
        "certainly,",
        "sure!",
        "sure,",
        "i hope this helps",
        "let me know if",
        "as an ai",
        "in this solution",
        "below is the code",
    ];
    for phrase in chatty_phrases {
        if lower.contains(phrase) {
            return false;
        }
    }

    true
}

/// Evaluates multi-file symbol binding accuracy (SBA).
/// SBA = |S_bound| / (|S_bound| + |S_unresolved|)
/// Counts unresolved symbol diagnostics matching `codes::TYPES_UNDEFINED_VARIABLE`,
/// `codes::TYPES_UNRESOLVED_TYPE`, and `codes::TYPES_FIELD_NOT_FOUND`.
pub fn verify_symbol_binding(result: &FrontendResult) -> f64 {
    use vox_compiler::typeck::diagnostics::codes;
    let unresolved_count = result
        .diagnostics
        .iter()
        .filter(|d| {
            if let Some(code) = &d.code {
                code == codes::TYPES_UNDEFINED_VARIABLE
                    || code == codes::TYPES_UNRESOLVED_TYPE
                    || code == codes::TYPES_FIELD_NOT_FOUND
            } else {
                let msg = d.message.to_ascii_lowercase();
                msg.contains("undefined variable")
                    || msg.contains("unresolved type")
                    || msg.contains("field not found")
            }
        })
        .count();

    if unresolved_count == 0 {
        return 1.0;
    }

    let total_idents = vox_compiler::lexer::lex(&result.source)
        .into_iter()
        .filter(|s| matches!(s.token, vox_compiler::lexer::Token::Ident(_)))
        .count();

    let bound = total_idents.saturating_sub(unresolved_count);
    if bound + unresolved_count == 0 {
        0.0
    } else {
        bound as f64 / (bound + unresolved_count) as f64
    }
}

/// Executes dynamic runtime test assertions for runnable tasks containing `@test`.
/// Returns `None` if the module contains no `@test` functions (non-runnable task).
/// Returns `Some(true)` if all test functions pass, or `Some(false)` if any fails.
pub fn run_test_assertions(hir: &vox_compiler::hir::HirModule) -> Option<bool> {
    if hir.tests.is_empty() {
        return None;
    }

    let mut interp = vox_compiler::eval::Interpreter::new(100_000);
    if interp.run_module(hir).is_err() {
        return Some(false);
    }

    for test_fn in &hir.tests {
        if interp.call(&test_fn.name, vec![]).is_err() {
            return Some(false);
        }
    }

    Some(true)
}

/// Strips reasoning tags and markdown code fences to isolate the raw generated Vox code.
pub fn extract_code(completion: &str) -> String {
    // 1. Strip <think>...</think> if present
    let text = if let Some(idx) = completion.rfind("</think>") {
        &completion[idx + "</think>".len()..]
    } else if completion.trim_start().starts_with("<think>") {
        ""
    } else {
        completion
    };
    // 2. Extract ```vox ... ``` or ``` ... ``` if present
    if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        let code_start = if let Some(first_newline) = after_start.find('\n') {
            first_newline + 1
        } else {
            0
        };
        let code_body = &after_start[code_start..];
        if let Some(end) = code_body.find("```") {
            return code_body[..end].trim().to_string();
        } else {
            return code_body.trim().to_string();
        }
    }
    text.trim().to_string()
}

/// Counts distinct placeholder-marker *categories* hit in code (anti-stub metric).
///
/// Matches genuine stub forms with word boundaries / macro shapes so short valid
/// Vox is not over-flagged, while `todo!()`, `unimplemented!()`, `TODO`, and
/// explicit stub phrases still fail.
pub fn placeholder_marker_hits(source: &str) -> usize {
    // One compiled regex per category; count how many categories match.
    static CATEGORIES: std::sync::LazyLock<[Regex; 8]> = std::sync::LazyLock::new(|| {
        [
            Regex::new(r"(?i)todo!\s*\(").expect("todo! regex"),
            Regex::new(r"(?i)unimplemented!\s*\(").expect("unimplemented! regex"),
            Regex::new(r"(?i)\btodo\b").expect("todo word regex"),
            Regex::new(r"(?i)\btbd\b").expect("tbd word regex"),
            Regex::new(r"(?i)\bplaceholder\b").expect("placeholder word regex"),
            Regex::new(r"(?i)\bstub\b").expect("stub word regex"),
            Regex::new(r"(?i)not\s+implemented").expect("not implemented regex"),
            Regex::new(r"(?i)coming\s+soon").expect("coming soon regex"),
        ]
    });
    CATEGORIES.iter().filter(|re| re.is_match(source)).count()
}

/// True when a non-comment code line is itself a hollow stub token.
fn is_hollow_stub_line(line: &str) -> bool {
    let s = line.trim().trim_end_matches(';').trim();
    if s.is_empty() {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    lower == "..."
        || lower == "todo!()"
        || lower == "unimplemented!()"
        || lower == "pass"
        || lower == "return"
        || lower == "return ()"
        || lower == "()"
}

/// Checks if output is a trivial placeholder/empty/hollow stub body.
pub fn is_trivial_placeholder_output(source: &str) -> bool {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.eq_ignore_ascii_case("return")
        || trimmed == ";"
        || trimmed == "{}"
        || trimmed == "..."
    {
        return true;
    }
    let code_lines: Vec<&str> = trimmed
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect();
    if code_lines.is_empty() {
        return true;
    }
    if code_lines.iter().all(|l| is_hollow_stub_line(l)) {
        return true;
    }
    // Declaration with an empty or ellipsis-only body: `fn f() { }` / `{ ... }`.
    if let (Some(open), Some(close)) = (trimmed.find('{'), trimmed.rfind('}'))
        && close > open
    {
        let body = trimmed[open + 1..close].trim();
        let body_lines: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .collect();
        if body_lines.is_empty() || body_lines.iter().all(|l| is_hollow_stub_line(l)) {
            return true;
        }
    }
    if code_lines.len() <= 1 {
        let stripped: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if stripped.len() <= 5 || stripped.ends_with("{}") || stripped.ends_with("();") {
            return true;
        }
    }
    false
}

/// Live anti-stub verdict: no placeholder markers and not a trivial/hollow body.
///
/// Intentionally independent of parse/typecheck success — those are other gates.
#[must_use]
pub fn anti_stub_pass_from_flags(placeholder_hits: usize, trivial_placeholder: bool) -> bool {
    placeholder_hits == 0 && !trivial_placeholder
}

/// Aggregate anti-stub run metrics from an `eval_local_report.json` value.
///
/// Prefer recomputing from per-sample `checks` so gate evaluation stays correct
/// after scoring-rule fixes even when top-level header fields were written by an
/// older binary (or drifted). Falls back to header fields when `results` is absent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AntiStubRunMetrics {
    pub anti_stub_task_success: f64,
    pub placeholder_event_rate: f64,
    pub trivial_placeholder_event_rate: f64,
    /// Mean best construct-richness among tasks that had at least one `parse_ok` sample.
    pub construct_richness_mean: f64,
    pub source: AntiStubMetricsSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiStubMetricsSource {
    RecomputedFromSamples,
    ReportHeader,
}

#[must_use]
pub fn anti_stub_metrics_from_eval_local_report(report: &serde_json::Value) -> AntiStubRunMetrics {
    if let Some(results) = report.get("results").and_then(|v| v.as_array())
        && !results.is_empty()
    {
        let total = results.len() as f64;
        let mut anti_pass = 0usize;
        let mut ph_tasks = 0usize;
        let mut triv_tasks = 0usize;
        let mut rich_sum = 0.0_f64;
        let mut rich_n = 0usize;
        for task in results {
            let samples = task
                .get("samples")
                .and_then(|s| s.as_array())
                .map(|a| a.as_slice())
                .unwrap_or(&[]);
            let mut task_anti = false;
            let mut task_ph = false;
            let mut task_triv = false;
            let mut best_parse_ok_richness: Option<f64> = None;
            for sample in samples {
                let checks = sample.get("checks");
                let ph = checks
                    .and_then(|c| c.get("placeholder_marker_hits"))
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as usize;
                let triv = checks
                    .and_then(|c| c.get("trivial_placeholder_output"))
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                if checks.is_some() {
                    if anti_stub_pass_from_flags(ph, triv) {
                        task_anti = true;
                    }
                } else if sample
                    .get("anti_stub_pass")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false)
                {
                    task_anti = true;
                }
                if ph > 0 {
                    task_ph = true;
                }
                if triv {
                    task_triv = true;
                }
                let parse_ok = checks
                    .and_then(|c| c.get("parse_ok"))
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                if parse_ok
                    && let Some(r) = checks
                        .and_then(|c| c.get("construct_richness_score"))
                        .and_then(|x| x.as_f64())
                {
                    best_parse_ok_richness = Some(best_parse_ok_richness.map_or(r, |b| b.max(r)));
                }
            }
            if task_anti {
                anti_pass += 1;
            }
            if task_ph {
                ph_tasks += 1;
            }
            if task_triv {
                triv_tasks += 1;
            }
            if let Some(r) = best_parse_ok_richness {
                rich_sum += r;
                rich_n += 1;
            }
        }
        return AntiStubRunMetrics {
            anti_stub_task_success: anti_pass as f64 / total,
            placeholder_event_rate: ph_tasks as f64 / total,
            trivial_placeholder_event_rate: triv_tasks as f64 / total,
            construct_richness_mean: if rich_n > 0 {
                rich_sum / rich_n as f64
            } else {
                0.0
            },
            source: AntiStubMetricsSource::RecomputedFromSamples,
        };
    }
    AntiStubRunMetrics {
        anti_stub_task_success: report
            .get("anti_stub_task_success")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        placeholder_event_rate: report
            .get("placeholder_event_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        trivial_placeholder_event_rate: report
            .get("trivial_placeholder_event_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        construct_richness_mean: report
            .get("construct_richness_mean")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        source: AntiStubMetricsSource::ReportHeader,
    }
}

/// Verification results for a model completion candidate.
pub struct CompletionVerification {
    pub pass: bool,
    pub pass_compile: bool,
    pub pass_ast: bool,
    pub pass_exec: Option<bool>,
    pub semantic_pass: bool,
    pub anti_stub_pass: bool,
    pub distinct_4_ratio: f64,
    pub constraint_adherence: bool,
    pub symbol_binding_accuracy: f64,
    pub checks: serde_json::Value,
}

/// Evaluates a completion against 3-tier oracle and degradation stress metrics.
pub fn verify_completion(
    completion: &str,
    bench_root: &Path,
    file_hint: &str,
    sample_id: &str,
    manifest_index: usize,
    semantic_expected_contains: &[String],
) -> CompletionVerification {
    let extracted = extract_code(completion);
    let normalized = vox_compiler::generated_vox::normalize_generated_vox(
        &extracted,
        vox_compiler::generated_vox::OutputSurfaceMode::RawCodeOnly,
    );
    let code = if !normalized.normalized.trim().is_empty() {
        normalized.normalized.as_str()
    } else {
        &extracted
    };

    let non_empty = !code.trim().is_empty();
    let mut parse_ok = false;
    let mut typecheck_ok = false;
    let mut parse_error: Option<String> = None;
    let mut diag_errors = 0usize;
    let mut symbol_binding_acc = 1.0_f64;
    let mut exec_pass: Option<bool> = None;

    let candidate_path = if !file_hint.is_empty() {
        bench_root.join(file_hint)
    } else {
        bench_root.join(format!("eval_local_{manifest_index}_{sample_id}.vox"))
    };

    let frontend = crate::pipeline::run_frontend_str(code, &candidate_path, false);
    match frontend {
        Ok(res) => {
            parse_ok = true;
            diag_errors = res.error_count();
            typecheck_ok = !res.has_errors();
            symbol_binding_acc = verify_symbol_binding(&res);
            exec_pass = run_test_assertions(&res.hir);
        }
        Err(err) => {
            parse_error = Some(err.to_string());
        }
    }

    let pass_compile = non_empty && parse_ok && typecheck_ok;
    let placeholder_hits = placeholder_marker_hits(code);
    let trivial_placeholder = is_trivial_placeholder_output(code);
    let ast_report = vox_compiler::ast_eval(code);
    let construct_richness = ast_report.coverage_score();
    // Anti-stub is about stubs (placeholders / trivial empty bodies), not parse
    // quality. Folding AST richness / node_count into anti_stub_pass made every
    // unparseable answer look like a stub and double-counted the same failure
    // under pass_compile and anti_stub. Richness stays a separate run-mean gate.
    let anti_stub_pass = anti_stub_pass_from_flags(placeholder_hits, trivial_placeholder);

    let pass_ast = pass_compile && anti_stub_pass;
    let pass = pass_ast && exec_pass.unwrap_or(true);

    let distinct_4 = calculate_distinct_4(code);
    let constraint_adherence = check_negative_constraints(completion);

    let semantic_pass = pass
        && semantic_expected_contains
            .iter()
            .all(|needle| code.contains(needle));

    CompletionVerification {
        pass,
        pass_compile,
        pass_ast,
        pass_exec: exec_pass,
        semantic_pass,
        anti_stub_pass,
        distinct_4_ratio: distinct_4,
        constraint_adherence,
        symbol_binding_accuracy: symbol_binding_acc,
        checks: serde_json::json!({
            "non_empty": non_empty,
            "parse_ok": parse_ok,
            "typecheck_ok": typecheck_ok,
            "diag_errors": diag_errors,
            "parse_error": parse_error,
            "placeholder_marker_hits": placeholder_hits,
            "trivial_placeholder_output": trivial_placeholder,
            "construct_richness_score": construct_richness,
            "anti_stub_pass": anti_stub_pass,
            "pass_compile": pass_compile,
            "pass_ast": pass_ast,
            "pass_exec": exec_pass,
            "distinct_4_ratio": distinct_4,
            "constraint_adherence": constraint_adherence,
            "symbol_binding_accuracy": symbol_binding_acc,
            "semantic_expected_contains": semantic_expected_contains,
            "semantic_pass": semantic_pass
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distinct_4_ratio_catches_repetition() {
        let loop_text = " - - - - - - - - - - - - - - - - - - - -";
        assert!(calculate_distinct_4(loop_text) < 0.25);
        let healthy = "fn add(a: int, b: int) to int { return a + b; }";
        assert!(calculate_distinct_4(healthy) >= 0.65);
    }

    #[test]
    fn test_negative_constraint_checking() {
        let bad_code = "@endpoint fn old_api() {}";
        assert!(!check_negative_constraints(bad_code));
        let bad_mutation = "@mutation fn create_item() {}";
        assert!(!check_negative_constraints(bad_mutation));
        let bad_chat = "Here is the code you requested:\nfn api() {}";
        assert!(!check_negative_constraints(bad_chat));
        let good_code = "fn new_api() {}";
        assert!(check_negative_constraints(good_code));
    }

    #[test]
    fn test_symbol_binding_accuracy_clean_code() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_sba.vox");
        let code = "fn add(a: int, b: int) to int { return a + b; }";
        let res = crate::pipeline::run_frontend_str(code, &path, false).unwrap();
        assert_eq!(verify_symbol_binding(&res), 1.0);
    }

    #[test]
    fn test_symbol_binding_accuracy_unresolved_variable() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_sba_unresolved.vox");
        let code = "fn bad(a: int) to int { return a + undeclared_var_xyz; }";
        let res = crate::pipeline::run_frontend_str(code, &path, false).unwrap();
        let score = verify_symbol_binding(&res);
        assert!(score < 1.0, "Expected score < 1.0, got {score}");
    }

    #[test]
    fn anti_stub_pass_does_not_require_parse_or_ast_richness() {
        // Real (non-placeholder) code that the frontend cannot parse must not be
        // labeled a stub — that failure belongs to pass_compile / pass_at_k.
        let broken =
            "fn is_symmetric_matrix(matrix: [[int]] ) to bool {\n    let n = matrix.length\n";
        let dir = tempfile::tempdir().unwrap();
        let v = verify_completion(broken, dir.path(), "", "broken_sym", 0, &[]);
        assert!(!v.pass_compile, "fixture must fail compile: {:?}", v.checks);
        assert!(
            v.anti_stub_pass,
            "unparseable non-placeholder output must still pass anti-stub: {:?}",
            v.checks
        );
        assert_eq!(v.checks["placeholder_marker_hits"], 0);
    }

    #[test]
    fn anti_stub_rejects_todo_unimplemented_ellipsis_and_hollow_bodies() {
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            "fn f() to int { todo!() }",
            "fn f() to int { unimplemented!() }",
            "fn f() to int { ... }",
            "fn f() to int { }",
            "workflow fetch_and_save(url: str) to str {\n    // TODO: implement\n}",
            "// not implemented yet\nfn f() to int { return 1 }",
        ];
        for src in cases {
            let v = verify_completion(src, dir.path(), "", "stub_case", 0, &[]);
            assert!(
                !v.anti_stub_pass,
                "expected anti_stub fail for {src:?}: {:?}",
                v.checks
            );
        }
    }

    #[test]
    fn anti_stub_passes_clean_zero_placeholder_single_decl() {
        let dir = tempfile::tempdir().unwrap();
        let clean = "fn add(a: int, b: int) to int {\n    return a + b\n}";
        let v = verify_completion(clean, dir.path(), "", "clean", 0, &[]);
        assert_eq!(placeholder_marker_hits(clean), 0);
        assert!(!is_trivial_placeholder_output(clean));
        assert!(
            v.anti_stub_pass,
            "clean implementation must anti_stub_pass: {:?}",
            v.checks
        );
    }

    #[test]
    fn anti_stub_metrics_recompute_ignores_stale_header_and_parse_failures() {
        let report = serde_json::json!({
            "anti_stub_task_success": 0.1,
            "placeholder_event_rate": 0.9,
            "trivial_placeholder_event_rate": 0.9,
            "construct_richness_mean": 0.01,
            "results": [
                {
                    "id": "a",
                    "samples": [{
                        "anti_stub_pass": false,
                        "checks": {
                            "placeholder_marker_hits": 0,
                            "trivial_placeholder_output": false,
                            "parse_ok": false,
                            "construct_richness_score": 0.0
                        }
                    }]
                },
                {
                    "id": "b",
                    "samples": [{
                        "anti_stub_pass": false,
                        "checks": {
                            "placeholder_marker_hits": 0,
                            "trivial_placeholder_output": false,
                            "parse_ok": true,
                            "construct_richness_score": 0.125
                        }
                    }]
                }
            ]
        });
        let m = anti_stub_metrics_from_eval_local_report(&report);
        assert_eq!(m.source, AntiStubMetricsSource::RecomputedFromSamples);
        assert!((m.anti_stub_task_success - 1.0).abs() < 1e-9, "{m:?}");
        assert!((m.placeholder_event_rate - 0.0).abs() < 1e-9, "{m:?}");
        assert!(
            (m.trivial_placeholder_event_rate - 0.0).abs() < 1e-9,
            "{m:?}"
        );
        assert!((m.construct_richness_mean - 0.125).abs() < 1e-9, "{m:?}");
    }

    #[test]
    fn anti_stub_metrics_recompute_still_fails_true_stub_samples() {
        let report = serde_json::json!({
            "results": [{
                "id": "stub",
                "samples": [{
                    "checks": {
                        "placeholder_marker_hits": 2,
                        "trivial_placeholder_output": true,
                        "parse_ok": true,
                        "construct_richness_score": 0.125
                    }
                }]
            }]
        });
        let m = anti_stub_metrics_from_eval_local_report(&report);
        assert!((m.anti_stub_task_success - 0.0).abs() < 1e-9, "{m:?}");
        assert!(m.placeholder_event_rate > 0.0);
        assert!(m.trivial_placeholder_event_rate > 0.0);
    }

    #[test]
    fn test_run_test_assertions_pass_and_fail() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_assertions.vox");
        let code_pass = "fn add(a: int, b: int) to int { return a + b; }\n@test fn check() to Unit { assert(add(1, 2) == 3); }";
        let res_pass = crate::pipeline::run_frontend_str(code_pass, &path, false).unwrap();
        assert_eq!(run_test_assertions(&res_pass.hir), Some(true));

        let code_no_test = "fn add(a: int, b: int) to int { return a + b; }";
        let res_no_test = crate::pipeline::run_frontend_str(code_no_test, &path, false).unwrap();
        assert_eq!(run_test_assertions(&res_no_test.hir), None);

        let code_fail = "fn add(a: int, b: int) to int { return a + b; }\n@test fn check_fail() to Unit { assert(1 == 2); }";
        let res_fail = crate::pipeline::run_frontend_str(code_fail, &path, false).unwrap();
        assert_eq!(run_test_assertions(&res_fail.hir), Some(false));
    }
}
