//! `vox audit --gate regression-budget` — CR-F6 foundation gate.
//!
//! Scans `crates/vox-compiler/` and `crates/vox-codegen/` (Rust sources) plus
//! `examples/golden/` (`.vox` corpus files) for regression markers that should
//! be zero in the foundation crates. The gate is a **regression lock**: it
//! asserts the current clean state and fails if new markers are introduced.
//!
//! ## Detection rules (zero false-positives is a hard requirement)
//!
//! 1. **`// vox:skip` in the golden corpus** — any line matching `// vox:skip`
//!    in `examples/golden/**/*.vox`. In `.vox` files this is a real "skip the
//!    compiler" annotation; goldens must compile, so the count must be 0.
//!
//! 2. **Reachable `todo!()` / `unimplemented!()` macro calls** in
//!    `crates/vox-compiler/src/**` and `crates/vox-codegen/src/**` (`.rs`
//!    files). Exclusions:
//!    - Files whose path contains a `tests` path component are skipped entirely.
//!    - Lines inside a `#[cfg(test)]` or `mod tests {` region are skipped.
//!    - Occurrences inside `//` line comments or string literals are NOT
//!      counted (only bare macro invocations `todo!(` / `unimplemented!(` on
//!      code lines).
//!
//! 3. **Deliberate-stub markers** — the literal substring `de-stub-pending`
//!    (case-insensitive) in any `.rs` file under those two crates.
//!
//! ## Documented limitations
//!
//! Fuzzy "stub/placeholder return body" analysis (detecting functions whose
//! entire body is a placeholder expression) is explicitly out of scope for this
//! slice. That analysis can be added later via the existing `vox-code-audit`
//! `hollow_fn` detector. This gate is purely textual and deterministic.

use crate::{
    CommonArgs, CrlGate, RunOutcome, Subcommand,
    report::{AuditReport, ExitCode, Results, Threshold},
    workspace_root,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct RegressionBudgetSubcommand;

// ─────────────────────────────────────────────────────────────────────────────
// Public scanner types (pub(crate) for tests)
// ─────────────────────────────────────────────────────────────────────────────

/// The kind of regression marker found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ViolationKind {
    /// `// vox:skip` in a `.vox` golden file.
    VoxSkipInGolden,
    /// `todo!(` or `unimplemented!(` in reachable (non-test) Rust source.
    TodoOrUnimplemented,
    /// `de-stub-pending` (case-insensitive) in Rust source.
    DeStubPending,
    /// Per-spoke `pass@1` fell below the monotonic ratchet floor (`AUDIT-04`).
    SpokeRatchetRegression,
}

/// One detected violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Violation {
    pub file: String,
    pub line: u32,
    pub kind: ViolationKind,
}

// ─────────────────────────────────────────────────────────────────────────────
// Scanner core (pure, testable)
// ─────────────────────────────────────────────────────────────────────────────

/// Scan one `.vox` golden file for `// vox:skip` annotations.
///
/// The annotation must be a code-level skip, not documentation prose.
/// Because `.vox` files do not have `//!`/`///` doc comment syntax the way
/// Rust does, every occurrence of `// vox:skip` on a line (anywhere) is
/// counted as a violation.
pub(crate) fn scan_vox_golden(path: &Path, src: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        if line.contains("// vox:skip") {
            violations.push(Violation {
                file: path.to_string_lossy().into_owned(),
                line: (idx + 1) as u32,
                kind: ViolationKind::VoxSkipInGolden,
            });
        }
    }
    violations
}

/// Scan one Rust source file for `todo!(` / `unimplemented!(` macro
/// invocations and `de-stub-pending` markers.
///
/// **False-positive exclusions (enforced precisely):**
/// - The caller must skip files whose path contains a `tests` component
///   (checked by `is_test_file`).
/// - Lines inside a `#[cfg(test)]` or `mod tests {` region are skipped.
/// - A `todo!(` or `unimplemented!(` appearing only inside a `//` line
///   comment or inside a string literal delimited by `"` is not counted.
/// - The literal `FAIL_PLACEHOLDER` (or any identifier that merely contains
///   "placeholder" as part of its name) is NOT a violation; only the explicit
///   `de-stub-pending` marker is counted.
pub(crate) fn scan_rust_source(path: &Path, src: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut in_test_region = false;
    let mut test_brace_depth: i32 = 0;
    let mut region_open_depth: i32 = 0; // brace depth when the test region opened

    for (idx, line) in src.lines().enumerate() {
        let lineno = (idx + 1) as u32;

        // ── test-region tracking ──────────────────────────────────────────
        // We detect `#[cfg(test)]` and `mod tests {` as openers; we track
        // brace depth to find the matching close.
        let trimmed = line.trim();

        if !in_test_region {
            if trimmed == "#[cfg(test)]"
                || trimmed.starts_with("mod tests")
                || trimmed.starts_with("pub mod tests")
            {
                // Mark that the NEXT opening brace starts the test region.
                // We set in_test_region once we see the `{` for this block.
                // Simple approach: if `{` is on this same line, open now.
                let open_count = line.chars().filter(|&c| c == '{').count() as i32;
                let close_count = line.chars().filter(|&c| c == '}').count() as i32;
                if open_count > close_count {
                    in_test_region = true;
                    region_open_depth = test_brace_depth;
                    test_brace_depth += open_count - close_count;
                    continue;
                }
                // `{` on the next line — handled by setting a flag; use a
                // simpler approach: just re-scan the next line once we've
                // noted the annotation. We handle this by checking "pending"
                // in next iteration — but to keep the parser simple we use a
                // two-pass: check current line's net braces + flag.
                // For now, if no `{` on this line, start region on next `{`.
                // We use `in_test_region = true` and depth 0 sentinel.
                in_test_region = true;
                region_open_depth = test_brace_depth;
                // brace depth not changed yet; will be updated when `{` seen.
                continue;
            }
            // Update brace depth (outside test region).
            let open_count = line.chars().filter(|&c| c == '{').count() as i32;
            let close_count = line.chars().filter(|&c| c == '}').count() as i32;
            test_brace_depth += open_count - close_count;
        } else {
            // Inside (or just entered) test region — track depth to find exit.
            let open_count = line.chars().filter(|&c| c == '{').count() as i32;
            let close_count = line.chars().filter(|&c| c == '}').count() as i32;
            test_brace_depth += open_count - close_count;
            if test_brace_depth <= region_open_depth {
                // The test region is closed.
                in_test_region = false;
                test_brace_depth = region_open_depth;
            }
            continue; // skip lines inside test region
        }

        // ── strip line comments for macro detection ───────────────────────
        // Only strip the comment part; we do NOT try to parse strings
        // (that would require a full lexer). We use a simple heuristic:
        // find the first `//` that is not inside a string.
        let code_part = strip_line_comment(line);

        // ── de-stub-pending (case-insensitive, whole file) ────────────────
        if line.to_ascii_lowercase().contains("de-stub-pending") {
            violations.push(Violation {
                file: path.to_string_lossy().into_owned(),
                line: lineno,
                kind: ViolationKind::DeStubPending,
            });
        }

        // ── todo!( / unimplemented!( — in code part only ──────────────────
        // We strip string literals from code_part before checking, to avoid
        // false positives from strings like `"call todo!()"`.
        let code_no_strings = strip_string_literals(code_part);
        if code_no_strings.contains("todo!(") || code_no_strings.contains("unimplemented!(") {
            violations.push(Violation {
                file: path.to_string_lossy().into_owned(),
                line: lineno,
                kind: ViolationKind::TodoOrUnimplemented,
            });
        }
    }
    violations
}

/// Returns true if the file path has a `tests` component, indicating it is a
/// test-only file and should be excluded from reachable-code scanning.
pub(crate) fn is_test_file(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| s == "tests")
            .unwrap_or(false)
    })
}

/// Strip the `//` line-comment suffix from a line.
///
/// Uses a simple heuristic: scan left-to-right, track whether we're inside a
/// `"…"` string, and cut at the first `//` outside a string.
fn strip_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' if !in_string => in_string = true,
            b'"' if in_string => {
                // Check for escape: if preceded by odd number of backslashes,
                // the quote is escaped. Simple check: count preceding `\`.
                let mut slashes = 0usize;
                let mut j = i;
                while j > 0 && bytes[j - 1] == b'\\' {
                    slashes += 1;
                    j -= 1;
                }
                if slashes.is_multiple_of(2) {
                    in_string = false;
                }
            }
            b'/' if !in_string && i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                return &line[..i];
            }
            _ => {}
        }
        i += 1;
    }
    line
}

/// Replace the contents of string literals in a code line with spaces so
/// macro-invocation patterns inside strings don't trigger false positives.
fn strip_string_literals(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut result = String::with_capacity(line.len());
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        match bytes[i] {
            b'"' if !in_string => {
                in_string = true;
                result.push(ch);
            }
            b'"' if in_string => {
                let mut slashes = 0usize;
                let mut j = i;
                while j > 0 && bytes[j - 1] == b'\\' {
                    slashes += 1;
                    j -= 1;
                }
                if slashes.is_multiple_of(2) {
                    in_string = false;
                }
                result.push(ch);
            }
            _ if in_string => result.push(' '),
            _ => result.push(ch),
        }
        i += 1;
    }
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Directory walker
// ─────────────────────────────────────────────────────────────────────────────

/// Walk `dir` recursively and collect all files matching `predicate`.
fn walk_files(dir: &Path, predicate: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut results = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if entry.file_type().is_file() && predicate(path) {
            results.push(path.to_path_buf());
        }
    }
    results
}

// ─────────────────────────────────────────────────────────────────────────────
// Top-level scan
// ─────────────────────────────────────────────────────────────────────────────

/// Run the full regression-budget scan over the workspace. Returns all
/// violations found.
pub(crate) fn scan_workspace(root: &Path) -> Result<Vec<Violation>, String> {
    let mut all = Vec::new();

    // 1. Golden corpus: scan for `// vox:skip`
    let golden_dir = root.join("examples").join("golden");
    if golden_dir.exists() {
        let vox_files = walk_files(&golden_dir, |p| {
            p.extension().and_then(|e| e.to_str()) == Some("vox")
        });
        for path in &vox_files {
            let src = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            all.extend(scan_vox_golden(path, &src));
        }
    }

    // 2 & 3. Rust sources in vox-compiler and vox-codegen
    for crate_name in ["vox-compiler", "vox-codegen"] {
        let src_dir = root.join("crates").join(crate_name).join("src");
        if !src_dir.exists() {
            continue;
        }
        let rs_files = walk_files(&src_dir, |p| {
            p.extension().and_then(|e| e.to_str()) == Some("rs") && !is_test_file(p)
        });
        for path in &rs_files {
            let src = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            all.extend(scan_rust_source(path, &src));
        }
    }

    // 4. Per-spoke `pass@1` regression ratchet (`AUDIT-04`) when scorecard artifact is present
    let scorecard_path = root
        .join("contracts")
        .join("reports")
        .join("mens-spoke-scorecard.v1.json");
    if scorecard_path.exists() {
        let raw = std::fs::read_to_string(&scorecard_path)
            .map_err(|e| format!("cannot read {}: {e}", scorecard_path.display()))?;
        let val: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("cannot parse {}: {e}", scorecard_path.display()))?;
        let mut observed = std::collections::BTreeMap::new();
        if let Some(spokes) = val.get("spokes").and_then(|v| v.as_array()) {
            for s in spokes {
                if let (Some(name), Some(pass1)) = (
                    s.get("spoke").and_then(|v| v.as_str()),
                    s.get("pass_at_1").and_then(|v| v.as_f64()),
                ) {
                    observed.insert(name.to_string(), pass1);
                }
            }
        }
        let ratchet_path = root
            .join("contracts")
            .join("eval")
            .join("spoke-regression-ratchet.v1.json");
        let ratchet = if ratchet_path.exists() {
            let r_raw = std::fs::read_to_string(&ratchet_path)
                .map_err(|e| format!("cannot read {}: {e}", ratchet_path.display()))?;
            serde_json::from_str::<SpokeRegressionRatchetV1>(&r_raw)
                .map_err(|e| format!("cannot parse {}: {e}", ratchet_path.display()))?
        } else {
            SpokeRegressionRatchetV1::canonical_five_spoke_floors(1.0)
        };
        if verify_spoke_regression_ratchet(&ratchet, &observed).is_err() {
            all.push(Violation {
                file: scorecard_path.to_string_lossy().into_owned(),
                line: 1,
                kind: ViolationKind::SpokeRatchetRegression,
            });
        }
    }

    // 5. Organic search-to-compilation ratchet (`PERSIST-BENCH-03`) when organic scorecard is present
    let organic_path = root
        .join("contracts")
        .join("reports")
        .join("mens-organic-search-spoke-scorecard.v1.json");
    if organic_path.exists() {
        let raw = std::fs::read_to_string(&organic_path)
            .map_err(|e| format!("cannot read {}: {e}", organic_path.display()))?;
        let val: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("cannot parse {}: {e}", organic_path.display()))?;
        let ratchet_path = root
            .join("contracts")
            .join("eval")
            .join("spoke-regression-ratchet.v1.json");
        let ratchet = if ratchet_path.exists() {
            let r_raw = std::fs::read_to_string(&ratchet_path)
                .map_err(|e| format!("cannot read {}: {e}", ratchet_path.display()))?;
            serde_json::from_str::<SpokeRegressionRatchetV1>(&r_raw)
                .map_err(|e| format!("cannot parse {}: {e}", ratchet_path.display()))?
        } else {
            SpokeRegressionRatchetV1::canonical_five_spoke_floors(1.0)
        };
        if verify_organic_search_ratchet(&ratchet, &val).is_err() {
            all.push(Violation {
                file: organic_path.to_string_lossy().into_owned(),
                line: 1,
                kind: ViolationKind::SpokeRatchetRegression,
            });
        }
    }

    Ok(all)
}

// ─────────────────────────────────────────────────────────────────────────────
// Subcommand impl
// ─────────────────────────────────────────────────────────────────────────────

impl Subcommand for RegressionBudgetSubcommand {
    fn gate(&self) -> CrlGate {
        CrlGate::F6RegressionBudget
    }

    fn description(&self) -> &'static str {
        "CR-F6: regression budget — zero todo!/unimplemented!/vox:skip/de-stub-pending \
         in foundation crates and golden corpus."
    }

    fn run(&self, args: &CommonArgs) -> RunOutcome {
        let thing = CrlGate::F6RegressionBudget.thing_name();

        if args.dry_run {
            return RunOutcome {
                report: AuditReport::complete(
                    thing,
                    "dry-run",
                    0,
                    Results {
                        overall_pass_rate: 1.0,
                        median_pass_rate: None,
                        per_llm: Vec::new(),
                    },
                ),
                exit_code: ExitCode::Ok,
            };
        }

        let root = workspace_root();
        let violations = match scan_workspace(&root) {
            Ok(v) => v,
            Err(msg) => {
                return RunOutcome {
                    report: AuditReport::infra_error(thing, msg),
                    exit_code: ExitCode::InfrastructureError,
                };
            }
        };

        let count = violations.len() as u32;
        let met = count == 0;

        // Encode violations as the corpus_hash / note so consumers can see
        // what was found without loading a separate artifact.
        let corpus_hash = format!("violation-count:{count}");

        let mut report = AuditReport::complete(
            thing,
            corpus_hash,
            count,
            Results {
                overall_pass_rate: if met { 1.0 } else { 0.0 },
                median_pass_rate: None,
                per_llm: Vec::new(),
            },
        );
        report.threshold = Some(Threshold {
            target: args.threshold.unwrap_or(0.0), // target = 0 violations
            met,
        });
        if !met {
            let details: Vec<String> = violations
                .iter()
                .map(|v| format!("{}:{} ({:?})", v.file, v.line, v.kind))
                .collect();
            report.note = Some(format!(
                "{count} regression marker(s) found: {}",
                details.join("; ")
            ));
        }

        RunOutcome {
            report,
            exit_code: if met {
                ExitCode::Ok
            } else {
                ExitCode::BarMissed
            },
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests (TDD — written before implementation, then made green)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn write(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    // ── vox golden scanner ───────────────────────────────────────────────────

    #[test]
    fn vox_skip_in_golden_detected() {
        let dir = tmp();
        let path = write(dir.path(), "foo.vox", "fn main() {}\n// vox:skip\n");
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_vox_golden(&path, &src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 2);
        assert_eq!(v[0].kind, ViolationKind::VoxSkipInGolden);
    }

    #[test]
    fn vox_skip_in_golden_multiple_lines() {
        let dir = tmp();
        let path = write(
            dir.path(),
            "bar.vox",
            "// vox:skip\nfn main() {}\n// vox:skip\n",
        );
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_vox_golden(&path, &src);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn clean_vox_golden_no_violations() {
        let dir = tmp();
        let path = write(dir.path(), "clean.vox", "fn main() { print(42) }\n");
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_vox_golden(&path, &src);
        assert!(v.is_empty());
    }

    // ── Rust source scanner — reachable todo!/unimplemented! ─────────────────

    #[test]
    fn reachable_todo_detected() {
        let dir = tmp();
        let path = write(
            dir.path(),
            "lib.rs",
            "pub fn foo() {\n    todo!(\"later\")\n}\n",
        );
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, ViolationKind::TodoOrUnimplemented);
        assert_eq!(v[0].line, 2);
    }

    #[test]
    fn reachable_unimplemented_detected() {
        let dir = tmp();
        let path = write(
            dir.path(),
            "lib.rs",
            "pub fn bar() {\n    unimplemented!()\n}\n",
        );
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, ViolationKind::TodoOrUnimplemented);
    }

    #[test]
    fn todo_in_line_comment_not_detected() {
        let dir = tmp();
        // `todo!(` only appears in a `//` comment — must NOT be a violation.
        let path = write(
            dir.path(),
            "lib.rs",
            "// TODO: call todo!() here later\npub fn ok() {}\n",
        );
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert!(v.is_empty(), "comment-only todo should not be flagged");
    }

    #[test]
    fn todo_in_string_literal_not_detected() {
        let dir = tmp();
        // `todo!(` inside a string literal — must NOT be a violation.
        let path = write(
            dir.path(),
            "lib.rs",
            r#"pub fn msg() -> &'static str { "call todo!() here" }"#,
        );
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert!(v.is_empty(), "string-literal todo should not be flagged");
    }

    #[test]
    fn todo_inside_cfg_test_not_detected() {
        let dir = tmp();
        let src = "\
pub fn real_fn() {}\n\
#[cfg(test)]\nmod tests {\n    #[test]\n    fn it() {\n        todo!(\"test stub\")\n    }\n}\n";
        let path = write(dir.path(), "lib.rs", src);
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert!(
            v.is_empty(),
            "#[cfg(test)] todo should not be flagged; got: {v:?}"
        );
    }

    #[test]
    fn todo_inside_mod_tests_not_detected() {
        let dir = tmp();
        let src = "pub fn real() {}\nmod tests {\n    fn helper() { todo!() }\n}\n";
        let path = write(dir.path(), "lib.rs", src);
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert!(
            v.is_empty(),
            "mod tests todo should not be flagged; got: {v:?}"
        );
    }

    // ── de-stub-pending ───────────────────────────────────────────────────────

    #[test]
    fn de_stub_pending_detected() {
        let dir = tmp();
        let path = write(
            dir.path(),
            "lib.rs",
            "// de-stub-pending: wire this up\npub fn f() {}\n",
        );
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, ViolationKind::DeStubPending);
    }

    #[test]
    fn de_stub_pending_case_insensitive() {
        let dir = tmp();
        let path = write(dir.path(), "lib.rs", "// DE-STUB-PENDING\npub fn f() {}\n");
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, ViolationKind::DeStubPending);
    }

    // ── false-positive guards ─────────────────────────────────────────────────

    #[test]
    fn fail_placeholder_not_detected() {
        // `FAIL_PLACEHOLDER` is a graceful-degradation constant name, not a stub.
        let dir = tmp();
        let path = write(
            dir.path(),
            "lib.rs",
            "const FAIL_PLACEHOLDER: &str = \"error\";\npub fn ok() {}\n",
        );
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert!(
            v.is_empty(),
            "FAIL_PLACEHOLDER constant should not be flagged; got {v:?}"
        );
    }

    #[test]
    fn doc_comment_vox_skip_prose_not_detected_by_rust_scanner() {
        // In Rust doc comments (`//!` / `///`), `// vox:skip` is prose/illustration.
        // The Rust scanner only runs on .rs files; `.vox` scanner handles .vox files.
        // A `//!` line is a line comment — strip_line_comment handles it the same way,
        // but doc comments don't contain `todo!(` so this is a direct check that
        // the scanner doesn't crash or misbehave on doc comment content.
        let dir = tmp();
        let path = write(
            dir.path(),
            "fragment_emit.rs",
            "//! Example: `// vox:skip — illustrative`\npub fn f() {}\n",
        );
        let src = fs::read_to_string(&path).unwrap();
        let v = scan_rust_source(&path, &src);
        assert!(v.is_empty(), "doc comment prose should not be flagged");
    }

    #[test]
    fn is_test_file_detects_tests_component() {
        assert!(is_test_file(Path::new("crates/vox-compiler/tests/foo.rs")));
        assert!(is_test_file(Path::new("src/tests/bar.rs")));
        assert!(!is_test_file(Path::new("src/codegen/emit.rs")));
        assert!(!is_test_file(Path::new("tests_util.rs"))); // not a path component
    }

    // ── full workspace scan on a synthetic tree ───────────────────────────────

    #[test]
    fn workspace_scan_detects_all_violation_types() {
        let root = tmp();
        let r = root.path();

        // Golden with vox:skip
        write(r, "examples/golden/bad.vox", "fn main() {}\n// vox:skip\n");

        // vox-compiler with todo!
        write(
            r,
            "crates/vox-compiler/src/passes/lower.rs",
            "pub fn lower() { todo!(\"lower\") }\n",
        );

        // vox-codegen with de-stub-pending
        write(
            r,
            "crates/vox-codegen/src/emit.rs",
            "// de-stub-pending: wire codegen\npub fn emit() {}\n",
        );

        let violations = scan_workspace(r).unwrap();
        let kinds: Vec<&ViolationKind> = violations.iter().map(|v| &v.kind).collect();
        assert!(
            kinds.contains(&&ViolationKind::VoxSkipInGolden),
            "expected VoxSkipInGolden; got {kinds:?}"
        );
        assert!(
            kinds.contains(&&ViolationKind::TodoOrUnimplemented),
            "expected TodoOrUnimplemented; got {kinds:?}"
        );
        assert!(
            kinds.contains(&&ViolationKind::DeStubPending),
            "expected DeStubPending; got {kinds:?}"
        );
    }

    #[test]
    fn workspace_scan_clean_tree_zero_violations() {
        let root = tmp();
        let r = root.path();

        // Clean golden
        write(r, "examples/golden/clean.vox", "fn main() { print(1) }\n");

        // Clean compiler source
        write(
            r,
            "crates/vox-compiler/src/lib.rs",
            "pub fn parse() -> bool { true }\n",
        );

        // false-positive: FAIL_PLACEHOLDER constant
        write(
            r,
            "crates/vox-codegen/src/reactive.rs",
            "const FAIL_PLACEHOLDER: &str = \"fallback\";\npub fn codegen() {}\n",
        );

        // false-positive: todo!() only inside #[cfg(test)]
        write(
            r,
            "crates/vox-codegen/src/emit.rs",
            "#[cfg(test)]\nmod tests {\n    fn stub() { todo!() }\n}\npub fn emit() {}\n",
        );

        let violations = scan_workspace(r).unwrap();
        assert!(
            violations.is_empty(),
            "clean tree must have 0 violations; got: {violations:?}"
        );
    }

    #[test]
    fn workspace_scan_ignores_test_files() {
        let root = tmp();
        let r = root.path();

        // todo! in a tests/ subdirectory file — must be ignored
        write(
            r,
            "crates/vox-compiler/src/tests/integration.rs",
            "pub fn helper() { todo!(\"test infra\") }\n",
        );

        let violations = scan_workspace(r).unwrap();
        assert!(
            violations.is_empty(),
            "test files must be excluded; got: {violations:?}"
        );
    }

    // ── subcommand trait smoke test ───────────────────────────────────────────

    #[test]
    fn subcommand_gate_and_description() {
        assert_eq!(
            RegressionBudgetSubcommand.gate(),
            CrlGate::F6RegressionBudget
        );
        assert!(RegressionBudgetSubcommand.description().contains("CR-F6"));
    }

    #[test]
    fn subcommand_run_on_real_workspace_passes() {
        // The real workspace must report 0 violations and exit Ok.
        let args = CommonArgs {
            write_canonical_report: false,
            ..CommonArgs::default()
        };
        let outcome = RegressionBudgetSubcommand.run(&args);
        assert_eq!(
            outcome.report.thing, "regression-budget",
            "thing name mismatch"
        );
        assert_eq!(
            outcome.exit_code,
            ExitCode::Ok,
            "real workspace must have 0 violations; note: {:?}",
            outcome.report.note
        );
        assert_eq!(
            outcome.report.corpus_size, 0,
            "corpus_size = violation count; must be 0"
        );
    }

    #[test]
    fn audit_04_spoke_regression_ratchet_enforces_monotonic_floors() {
        let mut floors = std::collections::BTreeMap::new();
        floors.insert("vox_lang".to_string(), 0.80);
        floors.insert("rust_workspace".to_string(), 0.75);
        let ratchet = SpokeRegressionRatchetV1 {
            schema_version: "vox.spoke_regression_ratchet.v1".to_string(),
            spoke_pass_at_1_floors: floors,
            organic_search_recall_at_5_floor: 1.0,
            organic_mrr_floor: 0.95,
            organic_compilation_pass_rate_floor: 1.0,
        };

        let mut passing_observed = std::collections::BTreeMap::new();
        passing_observed.insert("vox_lang".to_string(), 0.85);
        passing_observed.insert("rust_workspace".to_string(), 0.75);
        assert!(verify_spoke_regression_ratchet(&ratchet, &passing_observed).is_ok());

        let mut regressed_observed = std::collections::BTreeMap::new();
        regressed_observed.insert("vox_lang".to_string(), 0.79);
        regressed_observed.insert("rust_workspace".to_string(), 0.80);
        assert!(verify_spoke_regression_ratchet(&ratchet, &regressed_observed).is_err());
    }

    #[test]
    fn committed_spoke_ratchet_and_scorecard_contracts_pass_verification() {
        let root = workspace_root();
        let ratchet_path = root
            .join("contracts")
            .join("eval")
            .join("spoke-regression-ratchet.v1.json");
        let scorecard_path = root
            .join("contracts")
            .join("reports")
            .join("mens-spoke-scorecard.v1.json");
        let organic_path = root
            .join("contracts")
            .join("reports")
            .join("mens-organic-search-spoke-scorecard.v1.json");
        let ratchet_raw =
            std::fs::read_to_string(&ratchet_path).expect("read spoke-regression-ratchet.v1.json");
        let ratchet: SpokeRegressionRatchetV1 =
            serde_json::from_str(&ratchet_raw).expect("parse spoke-regression-ratchet.v1.json");
        assert_eq!(ratchet.schema_version, "vox.spoke_regression_ratchet.v1");
        assert_eq!(ratchet.spoke_pass_at_1_floors.len(), 5);
        assert!((ratchet.organic_search_recall_at_5_floor - 1.0).abs() < 1e-9);
        assert!(ratchet.organic_mrr_floor >= 0.95);
        assert!((ratchet.organic_compilation_pass_rate_floor - 1.0).abs() < 1e-9);

        let scorecard_raw =
            std::fs::read_to_string(&scorecard_path).expect("read mens-spoke-scorecard.v1.json");
        let val: serde_json::Value =
            serde_json::from_str(&scorecard_raw).expect("parse mens-spoke-scorecard.v1.json");
        let mut observed = std::collections::BTreeMap::new();
        for s in val
            .get("spokes")
            .and_then(|v| v.as_array())
            .expect("spokes array")
        {
            let name = s.get("spoke").and_then(|v| v.as_str()).expect("spoke name");
            let pass1 = s
                .get("pass_at_1")
                .and_then(|v| v.as_f64())
                .expect("pass_at_1");
            observed.insert(name.to_string(), pass1);
        }
        assert!(verify_spoke_regression_ratchet(&ratchet, &observed).is_ok());

        let organic_raw = std::fs::read_to_string(&organic_path)
            .expect("read mens-organic-search-spoke-scorecard.v1.json");
        let organic_val: serde_json::Value = serde_json::from_str(&organic_raw)
            .expect("parse mens-organic-search-spoke-scorecard.v1.json");
        assert!(verify_organic_search_ratchet(&ratchet, &organic_val).is_ok());
    }
}

fn default_one() -> f64 {
    1.0
}

fn default_mrr_floor() -> f64 {
    0.95
}

/// Per-spoke non-decreasing `pass@1` and organic search-to-compilation regression ratchet (`AUDIT-04`, `PERSIST-BENCH-03`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpokeRegressionRatchetV1 {
    /// Schema discriminator (`"vox.spoke_regression_ratchet.v1"`).
    pub schema_version: String,
    /// Minimum required `pass@1` floor per spoke.
    pub spoke_pass_at_1_floors: std::collections::BTreeMap<String, f64>,
    /// Minimum required organic search `Recall@5` floor (`1.0`).
    #[serde(default = "default_one")]
    pub organic_search_recall_at_5_floor: f64,
    /// Minimum required organic search `MRR` floor (`0.95`).
    #[serde(default = "default_mrr_floor")]
    pub organic_mrr_floor: f64,
    /// Minimum required organic compilation pass rate floor (`1.0`).
    #[serde(default = "default_one")]
    pub organic_compilation_pass_rate_floor: f64,
}

impl SpokeRegressionRatchetV1 {
    /// Construct a ratchet seeded with `floor` across all 5 canonical [`crate::subcommands::corpus_feedback::AuditSpokeKind`] spokes.
    #[must_use]
    pub fn canonical_five_spoke_floors(floor: f64) -> Self {
        let mut spoke_pass_at_1_floors = std::collections::BTreeMap::new();
        for spoke in crate::subcommands::corpus_feedback::AuditSpokeKind::ALL {
            spoke_pass_at_1_floors.insert(spoke.as_str().to_string(), floor);
        }
        Self {
            schema_version: "vox.spoke_regression_ratchet.v1".to_string(),
            spoke_pass_at_1_floors,
            organic_search_recall_at_5_floor: 1.0,
            organic_mrr_floor: 0.95,
            organic_compilation_pass_rate_floor: 1.0,
        }
    }
}

/// Verify that an organic search-to-compilation scorecard JSON satisfies the ratchet floors (`PERSIST-BENCH-03`).
pub fn verify_organic_search_ratchet(
    ratchet: &SpokeRegressionRatchetV1,
    organic_scorecard_json: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();
    let recall = organic_scorecard_json
        .get("overall_search_recall_at_5")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let mrr = organic_scorecard_json
        .get("overall_mrr")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let comp = organic_scorecard_json
        .get("overall_compilation_pass_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let e2e = organic_scorecard_json
        .get("overall_end_to_end_pass_at_1")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    if recall + 1e-9 < ratchet.organic_search_recall_at_5_floor {
        violations.push(format!(
            "organic_search_recall_at_5 regressed: {recall:.4} < floor {:.4}",
            ratchet.organic_search_recall_at_5_floor
        ));
    }
    if mrr + 1e-9 < ratchet.organic_mrr_floor {
        violations.push(format!(
            "organic_mrr regressed: {mrr:.4} < floor {:.4}",
            ratchet.organic_mrr_floor
        ));
    }
    if comp + 1e-9 < ratchet.organic_compilation_pass_rate_floor {
        violations.push(format!(
            "organic_compilation_pass_rate regressed: {comp:.4} < floor {:.4}",
            ratchet.organic_compilation_pass_rate_floor
        ));
    }
    if e2e + 1e-9 < ratchet.organic_compilation_pass_rate_floor {
        violations.push(format!(
            "organic_end_to_end_pass_at_1 regressed: {e2e:.4} < floor {:.4}",
            ratchet.organic_compilation_pass_rate_floor
        ));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Verify that every spoke in `observed_pass_at_1` meets or exceeds the ratchet floor (`AUDIT-04`).
pub fn verify_spoke_regression_ratchet(
    ratchet: &SpokeRegressionRatchetV1,
    observed_pass_at_1: &std::collections::BTreeMap<String, f64>,
) -> Result<SpokeRegressionRatchetV1, Vec<String>> {
    let mut violations = Vec::new();
    let mut updated = ratchet.spoke_pass_at_1_floors.clone();
    for (spoke, &floor) in &ratchet.spoke_pass_at_1_floors {
        match observed_pass_at_1.get(spoke) {
            Some(&obs) if obs + 1e-9 >= floor => {
                if obs > floor {
                    updated.insert(spoke.clone(), obs);
                }
            }
            Some(&obs) => violations.push(format!(
                "spoke `{spoke}` regressed: observed {obs:.4} < ratchet floor {floor:.4}"
            )),
            None => violations.push(format!("spoke `{spoke}` missing from observed scorecard")),
        }
    }
    if violations.is_empty() {
        Ok(SpokeRegressionRatchetV1 {
            schema_version: ratchet.schema_version.clone(),
            spoke_pass_at_1_floors: updated,
            organic_search_recall_at_5_floor: ratchet.organic_search_recall_at_5_floor,
            organic_mrr_floor: ratchet.organic_mrr_floor,
            organic_compilation_pass_rate_floor: ratchet.organic_compilation_pass_rate_floor,
        })
    } else {
        Err(violations)
    }
}
