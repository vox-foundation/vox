//! Milestone `v3.5` (`Phases 183–187`): Zero-Paper-Code, Anti-Hallucination,
//! Dynamic-Programming & Anti-Reward-Hacking Guards for the Vox Axis Agent Harness.

use serde::{Deserialize, Serialize};

/// Report from `verify_test_ratchet_integrity` (`TEST-RATCHET-01..02`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRatchetReport {
    pub tests_before: usize,
    pub tests_after: usize,
    pub assertions_before: usize,
    pub assertions_after: usize,
    pub ignores_before: usize,
    pub ignores_after: usize,
    pub test_tampering_detected: bool,
    pub reason: Option<String>,
}

/// Comprehensive `v3.5` anti-laziness, paper-code, magic-value, and test-ratchet audit report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZeroPaperAuditReport {
    pub paper_code_symbols: Vec<String>,
    pub hollow_functions: Vec<String>,
    pub stub_markers: Vec<String>,
    pub magic_value_literals: Vec<String>,
    pub special_cased_branches: Vec<String>,
    pub test_ratchet: TestRatchetReport,
    pub symbol_catalog_header: Option<String>,
    pub should_block_write: bool,
    pub formatted_advisory: Option<String>,
}

const EXEMPT_FN_NAMES: &[&str] = &[
    "main", "new", "default", "drop", "fmt", "clone", "from", "into", "try_from", "try_into",
    "deref", "as_ref", "as_mut", "noop", "no_op",
];

const TRIVIAL_RETURN_BODIES: &[&str] = &[
    "Ok(())",
    "Ok(Default::default())",
    "Default::default()",
    "Vec::new()",
    "vec![]",
    "String::new()",
    "None",
    "Ok(true)",
    "Ok(false)",
    "Ok(0)",
    "()",
];

/// Split a source file into `(production_source, test_source)` at `#[cfg(test)]` boundaries (`PAPER-GUARD-01`).
#[must_use]
pub fn split_production_and_test_source(source: &str) -> (String, String) {
    let mut prod = String::with_capacity(source.len());
    let mut test_part = String::new();
    let mut in_test_mod = false;
    let mut pending_cfg_test = false;
    let mut test_brace_depth: i32 = 0;

    for line in source.lines() {
        let t = line.trim();
        if !in_test_mod && t.starts_with("#[cfg(test)]") {
            pending_cfg_test = true;
            test_part.push_str(line);
            test_part.push('\n');
            continue;
        }
        if pending_cfg_test || in_test_mod {
            let opens = line.chars().filter(|&c| c == '{').count() as i32;
            let closes = line.chars().filter(|&c| c == '}').count() as i32;
            test_brace_depth += opens - closes;
            test_part.push_str(line);
            test_part.push('\n');
            if pending_cfg_test && opens > 0 {
                in_test_mod = true;
                pending_cfg_test = false;
            } else if in_test_mod && test_brace_depth <= 0 {
                in_test_mod = false;
                test_brace_depth = 0;
            }
        } else {
            prod.push_str(line);
            prod.push('\n');
        }
    }
    (prod, test_part)
}

fn extract_declared_fn_name(trimmed_line: &str) -> Option<&str> {
    if trimmed_line.starts_with("//") || trimmed_line.starts_with('*') {
        return None;
    }
    let fn_kw = concat!("f", "n ");
    let after_fn = if let Some(rest) = trimmed_line.strip_prefix(fn_kw) {
        Some(rest)
    } else if let Some(idx) = trimmed_line.find(&format!(" {fn_kw}")) {
        Some(&trimmed_line[idx + 4..])
    } else {
        trimmed_line.strip_prefix("export function ").or_else(|| {
            trimmed_line
                .strip_prefix("export async function ")
                .or_else(|| trimmed_line.strip_prefix("function "))
        })
    }?;
    let name = after_fn
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()?;
    (!name.is_empty()).then_some(name)
}

/// Detect functions declared in production code that are only referenced inside `#[cfg(test)]`
/// with zero production call sites across `prod_source` and `sibling_prod_context` (`PAPER-GUARD-02..03`).
#[must_use]
pub fn detect_paper_code_symbols(
    prod_source: &str,
    test_source: &str,
    sibling_prod_context: Option<&str>,
) -> Vec<String> {
    if test_source.trim().is_empty() {
        return Vec::new();
    }
    let mut paper_symbols = Vec::new();
    let lines: Vec<&str> = prod_source.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let Some(fn_name) = extract_declared_fn_name(line.trim()) else {
            continue;
        };
        if EXEMPT_FN_NAMES.contains(&fn_name) || fn_name.starts_with("default_") {
            continue;
        }
        let prev_attr = idx
            .checked_sub(1)
            .and_then(|p| lines.get(p))
            .map(|s| s.trim())
            .unwrap_or("");
        if prev_attr.contains("tauri::command") || prev_attr.contains("#[test") {
            continue;
        }
        let call_token = format!("{fn_name}(");
        let prod_occurrences = prod_source.matches(&call_token).count();
        let sibling_occurrences = sibling_prod_context
            .map(|sib| sib.matches(&fn_name).count())
            .unwrap_or(0);
        let test_occurrences = test_source.matches(&fn_name).count();
        if prod_occurrences <= 1
            && sibling_occurrences == 0
            && test_occurrences >= 1
            && !paper_symbols.contains(&fn_name.to_string())
        {
            paper_symbols.push(fn_name.to_string());
        }
    }
    paper_symbols
}

/// Detect hollow function bodies and deferred stub markers in production code (`HOLLOW-GATE-01..02`).
#[must_use]
pub fn detect_hollow_and_stub_bodies(prod_source: &str) -> (Vec<String>, Vec<String>) {
    let mut hollow_fns = Vec::new();
    let mut stub_markers = Vec::new();
    let lines: Vec<&str> = prod_source.lines().collect();
    let markers = [
        concat!("to", "do!("),
        concat!("unimple", "mented!("),
        concat!("// ", "TO", "DO"),
        concat!("// ", "FIX", "ME"),
        concat!("// ", "HA", "CK"),
        concat!("/* ", "st", "ub */"),
    ];
    let impl_later = concat!("implement ", "later");

    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        let line_no = i + 1;
        if markers.iter().any(|m| t.contains(m)) || t.to_ascii_lowercase().contains(impl_later) {
            stub_markers.push(format!("L{line_no}: {t}"));
        }
        let Some(fn_name) = extract_declared_fn_name(t) else {
            continue;
        };
        if EXEMPT_FN_NAMES.contains(&fn_name) || fn_name.starts_with("default_") {
            continue;
        }
        let mut body_lines = Vec::new();
        let mut depth: i32 = 0;
        let mut entered = false;
        for &bline in &lines[i..lines.len().min(i + 8)] {
            let opens = bline.chars().filter(|&c| c == '{').count() as i32;
            let closes = bline.chars().filter(|&c| c == '}').count() as i32;
            if !entered {
                if let Some(pos) = bline.find('{') {
                    entered = true;
                    let after_open = bline[pos + 1..].trim().trim_end_matches('}').trim();
                    if !after_open.is_empty() {
                        body_lines.push(after_open.to_string());
                    }
                }
            } else {
                let cleaned = bline.trim().trim_end_matches('}').trim();
                if !cleaned.is_empty() && !cleaned.starts_with("//") {
                    body_lines.push(cleaned.to_string());
                }
            }
            depth += opens - closes;
            if entered && depth <= 0 {
                break;
            }
        }
        if body_lines.len() == 1 {
            let compact = body_lines[0]
                .trim_start_matches("return ")
                .trim_end_matches(';')
                .replace(' ', "");
            if TRIVIAL_RETURN_BODIES.contains(&compact.as_str())
                && !hollow_fns.contains(&fn_name.to_string())
            {
                hollow_fns.push(fn_name.to_string());
            }
        }
    }
    (hollow_fns, stub_markers)
}

/// Detect hardcoded magic literals and specification-gaming special-cased branches (`DYNAMIC-CODE-01..02`).
#[must_use]
pub fn detect_magic_values_and_special_casing(prod_source: &str) -> (Vec<String>, Vec<String>) {
    let mut magic_literals = Vec::new();
    let mut special_branches = Vec::new();
    let magics = [
        "127.0.0.1:",
        "localhost:",
        concat!("\"", "/", "tmp/"),
        "\"/Users/",
        "postgres://",
        "sqlite://",
    ];
    let gaming_tokens = [
        "== \"test_",
        "== \"case_",
        "== \"fixture_",
        "== \"mock_",
        "== \"expected_",
    ];

    for (idx, line) in prod_source.lines().enumerate() {
        let t = line.trim();
        if t.starts_with("//")
            || t.starts_with("const ")
            || t.starts_with("pub const ")
            || t.starts_with("static ")
            || t.starts_with("pub static ")
        {
            continue;
        }
        let line_no = idx + 1;
        for magic in magics {
            if t.contains(magic) {
                magic_literals.push(format!("L{line_no} ({magic})"));
            }
        }
        let lower = t.to_ascii_lowercase();
        if (lower.starts_with("if ") || lower.contains(" if "))
            && gaming_tokens.iter().any(|g| lower.contains(g))
        {
            special_branches.push(format!("L{line_no}: {t}"));
        }
    }
    (magic_literals, special_branches)
}

/// Compare `old_source` and `new_source` to detect test deletion, assertion stripping, or `#[ignore]` injection (`TEST-RATCHET-01..02`).
#[must_use]
pub fn verify_test_ratchet_integrity(
    old_source: Option<&str>,
    new_source: &str,
) -> TestRatchetReport {
    let Some(old) = old_source else {
        return TestRatchetReport::default();
    };
    let count_tests = |s: &str| {
        s.lines()
            .filter(|l| {
                let t = l.trim();
                t.starts_with("#[test]")
                    || t.starts_with("#[tokio::test")
                    || t.starts_with("it(")
                    || t.starts_with("test(")
            })
            .count()
    };
    let count_asserts = |s: &str| {
        s.matches("assert!(").count()
            + s.matches("assert_eq!(").count()
            + s.matches("assert_ne!(").count()
            + s.matches("expect(").count()
    };
    let count_ignores = |s: &str| {
        s.lines()
            .filter(|l| {
                let t = l.trim();
                t.starts_with("#[ignore") || t.contains(".skip(") || t.contains("it.todo(")
            })
            .count()
    };

    let (tests_before, tests_after) = (count_tests(old), count_tests(new_source));
    let (assertions_before, assertions_after) = (count_asserts(old), count_asserts(new_source));
    let (ignores_before, ignores_after) = (count_ignores(old), count_ignores(new_source));

    let mut reasons = Vec::new();
    if tests_before > 0 && tests_after < tests_before {
        reasons.push(format!(
            "test count dropped ({tests_before} -> {tests_after})"
        ));
    }
    if tests_before > 0 && assertions_after < assertions_before {
        reasons.push(format!(
            "assertion count dropped ({assertions_before} -> {assertions_after})"
        ));
    }
    if ignores_after > ignores_before {
        reasons.push(format!(
            "test ignore/skip attributes added ({ignores_before} -> {ignores_after})"
        ));
    }

    TestRatchetReport {
        tests_before,
        tests_after,
        assertions_before,
        assertions_after,
        ignores_before,
        ignores_after,
        test_tampering_detected: !reasons.is_empty(),
        reason: (!reasons.is_empty()).then(|| {
            format!(
                "[TEST RATCHET VIOLATION: {} — do not delete tests, strip assertions, or add #[ignore] to pass verification]",
                reasons.join("; ")
            )
        }),
    }
}

/// Extract a compact top-level symbol & enum variant catalog from a source file (`STOP-VERIFY-02`).
#[must_use]
pub fn extract_top_level_symbol_catalog(source: &str, max_items: usize) -> Option<String> {
    let (prod, _) = split_production_and_test_source(source);
    let mut items = Vec::new();
    for line in prod.lines() {
        let t = line.trim();
        if t.starts_with("pub ")
            || t.starts_with("pub(crate) ")
            || t.starts_with("export ")
            || t.starts_with(concat!("f", "n "))
        {
            let sig = t.split('{').next().unwrap_or(t).trim();
            if !sig.is_empty() && !sig.starts_with("pub use ") {
                items.push(sig.to_string());
                if items.len() >= max_items {
                    break;
                }
            }
        }
    }
    (!items.is_empty()).then(|| format!("[FILE SYMBOL CATALOG: {}]", items.join(" | ")))
}

/// Run the full `v3.5` Zero-Paper-Code, Hollow-Body, Dynamic-Programming, and Test-Ratchet audit.
#[must_use]
pub fn audit_zero_paper_and_laziness(
    path: &str,
    new_source: &str,
    old_source: Option<&str>,
    sibling_prod_context: Option<&str>,
) -> ZeroPaperAuditReport {
    let norm = path.replace('\\', "/");
    let is_code = norm.ends_with(".rs")
        || norm.ends_with(".ts")
        || norm.ends_with(".tsx")
        || norm.ends_with(".vox");
    if !is_code {
        return ZeroPaperAuditReport::default();
    }
    let is_dedicated_test_file = norm.contains("/tests/")
        || norm.ends_with("_test.rs")
        || norm.ends_with(".test.ts")
        || norm.ends_with(".test.tsx");
    let (prod, test_part) = if is_dedicated_test_file {
        (String::new(), new_source.to_string())
    } else {
        split_production_and_test_source(new_source)
    };

    let paper_code_symbols = detect_paper_code_symbols(&prod, &test_part, sibling_prod_context);
    let (hollow_functions, stub_markers) = detect_hollow_and_stub_bodies(&prod);
    let (magic_value_literals, special_cased_branches) =
        detect_magic_values_and_special_casing(&prod);
    let test_ratchet = verify_test_ratchet_integrity(old_source, new_source);
    let symbol_catalog_header = extract_top_level_symbol_catalog(&prod, 12);

    let should_block_write = !special_cased_branches.is_empty()
        || test_ratchet.test_tampering_detected
        || !hollow_functions.is_empty();

    let mut notes = Vec::new();
    if let Some(ref tr) = test_ratchet.reason {
        notes.push(tr.clone());
    }
    if !special_cased_branches.is_empty() {
        notes.push(format!(
            "[SPECIFICATION GAMING BLOCKED: hardcoded test/fixture branch in production code ({})]",
            special_cased_branches.join(", ")
        ));
    }
    if !hollow_functions.is_empty() {
        notes.push(format!(
            "[HOLLOW FUNCTION BLOCKED: production function(s) return trivial default without logic ({})]",
            hollow_functions.join(", ")
        ));
    }
    if !stub_markers.is_empty() {
        notes.push(format!(
            "[DEFERRED STUB WARNING: remove todo/fixme/stub markers before finishing ({})]",
            stub_markers.join(", ")
        ));
    }
    if !paper_code_symbols.is_empty() {
        notes.push(format!(
            "[PAPER CODE WARNING: symbol(s) `{}` are only called inside #[cfg(test)] with 0 production call sites — wire them into the live execution path before finishing]",
            paper_code_symbols.join(", ")
        ));
    }
    if !magic_value_literals.is_empty() {
        notes.push(format!(
            "[MAGIC VALUE WARNING: extract hardcoded literal(s) ({}) into named `const` or config]",
            magic_value_literals.join(", ")
        ));
    }

    ZeroPaperAuditReport {
        paper_code_symbols,
        hollow_functions,
        stub_markers,
        magic_value_literals,
        special_cased_branches,
        test_ratchet,
        symbol_catalog_header,
        should_block_write,
        formatted_advisory: (!notes.is_empty()).then(|| notes.join(" | ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v35_zero_paper_code_hollow_magic_and_test_ratchet_guards_work() {
        let old_src = concat!(
            "pub const DEFAULT_PORT: u16 = 8080;\n",
            "pub ",
            "f",
            "n compute_routing_score(latency_ms: u64) -> u64 { latency_ms.saturating_mul(2) }\n",
            "#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    ",
            "f",
            "n test_score() {\n        assert_eq!(compute_routing_score(10), 20);\n        assert_eq!(compute_routing_score(5), 10);\n    }\n}\n"
        );
        let bad_new_src = concat!(
            "pub ",
            "f",
            "n compute_routing_score(input: &str) -> u64 {\n",
            "    if input == \"test_case_alpha\" { return 42; }\n",
            "    let endpoint = \"http://127.0.0.1:9000\";\n    let _ = endpoint;\n    0\n}\n",
            "pub ",
            "f",
            "n unwired_paper_helper(x: u64) -> u64 { x + 1 }\n",
            "pub ",
            "f",
            "n hollow_stub() -> Option<String> { None }\n",
            "#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    #[ignore]\n    ",
            "f",
            "n test_score() { assert_eq!(unwired_paper_helper(1), 2); }\n}\n"
        );

        let report = audit_zero_paper_and_laziness(
            "crates/vox-orchestrator/src/foo.rs",
            bad_new_src,
            Some(old_src),
            None,
        );
        assert!(
            report
                .paper_code_symbols
                .contains(&"unwired_paper_helper".to_string())
        );
        assert!(report.hollow_functions.contains(&"hollow_stub".to_string()));
        assert!(!report.magic_value_literals.is_empty());
        assert!(!report.special_cased_branches.is_empty());
        assert!(report.test_ratchet.test_tampering_detected);
        assert!(report.should_block_write);

        let clean_src = concat!(
            "pub ",
            "f",
            "n wired_helper(x: u64) -> u64 { x.saturating_add(1) }\n",
            "#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    ",
            "f",
            "n test_wired() {\n        assert_eq!(wired_helper(1), 2);\n        assert_eq!(wired_helper(2), 3);\n    }\n}\n"
        );
        let clean_report = audit_zero_paper_and_laziness(
            "crates/vox-orchestrator/src/foo.rs",
            clean_src,
            Some(old_src),
            Some("let val = wired_helper(42);"),
        );
        assert!(clean_report.paper_code_symbols.is_empty());
        assert!(clean_report.hollow_functions.is_empty());
        assert!(!clean_report.test_ratchet.test_tampering_detected);
        assert!(!clean_report.should_block_write);
    }
}
