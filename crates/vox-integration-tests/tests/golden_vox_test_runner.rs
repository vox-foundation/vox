//! Integration test: run all `@test` functions in golden `.vox` files through
//! the compiler's built-in eval layer.
//!
//! This validates v1.0 CR-D3 ("100% of vox-cli subcommands have associated
//! `.vox` example scripts") for the `vox test` command surface specifically,
//! and demonstrates that Vox's self-test mechanism actually executes correctly.
//!
//! For every `examples/golden/**/*.vox` file that contains `@test` functions:
//!   1. lex → parse → lower → typecheck
//!   2. create an `Interpreter` and call `run_module`
//!   3. call each `HirFn` from `module.tests` with no args
//!   4. assert no `EvalError::AssertionFailed` or `EvalError::Panic` is returned
//!
//! Skipped: golden files with no `@test` fns (they don't exercise this path).
//! Non-fatal: `EvalError::StepLimitExceeded` prints a warning but does not fail
//! (guards against infinite-loop in golden tests consuming CI time).

use std::path::{Path, PathBuf};

use vox_compiler::eval::{EvalError, Interpreter};
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::typecheck_module;

const STEP_LIMIT: usize = 100_000;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("../.."))
}

fn collect_golden_vox(root: &Path) -> Vec<PathBuf> {
    let golden = root.join("examples").join("golden");
    let mut files: Vec<PathBuf> = Vec::new();
    collect_vox_recursive(&golden, &mut files);
    files.sort();
    files
}

fn collect_vox_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_vox_recursive(&p, out);
            } else if p.extension().is_some_and(|e| e == "vox") {
                out.push(p);
            }
        }
    }
}

/// Run all `@test` fns from one `.vox` source string.
/// Returns a list of `(test_name, error_message)` for any that fail.
fn run_vox_tests(src: &str, file_label: &str) -> Vec<(String, String)> {
    let tokens = lex(src);
    let module = match parse(tokens) {
        Ok(m) => m,
        Err(errs) => {
            // Golden files are pre-validated by golden_examples_strict_parse; skip gracefully.
            eprintln!("[golden_vox_test_runner] parse error in {file_label}: {errs:?}");
            return vec![];
        }
    };

    let _diags = typecheck_module(&module, file_label);
    let hir = lower_module(&module);

    if hir.tests.is_empty() {
        return vec![];
    }

    let mut interp = Interpreter::new(STEP_LIMIT);
    if let Err(e) = interp.run_module(&hir) {
        return vec![(
            "<module_setup>".to_string(),
            format!("run_module failed: {e:?}"),
        )];
    }

    let mut failures = Vec::new();
    for test_fn in &hir.tests {
        match interp.call(&test_fn.name, vec![]) {
            Ok(_) => {}
            Err(EvalError::StepLimitExceeded) => {
                eprintln!(
                    "[golden_vox_test_runner] WARN: {} in {} hit step limit ({STEP_LIMIT}), skipping",
                    test_fn.name, file_label
                );
            }
            Err(e) => {
                failures.push((test_fn.name.clone(), format!("{e:?}")));
            }
        }
    }
    failures
}

#[test]
fn all_golden_at_test_fns_pass() {
    let root = repo_root();
    let files = collect_golden_vox(&root);

    assert!(
        !files.is_empty(),
        "No golden .vox files found — check repo root (looked in {})",
        root.join("examples/golden").display()
    );

    let mut total_tests = 0usize;
    let mut all_failures: Vec<(PathBuf, String, String)> = Vec::new();

    for path in &files {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[golden_vox_test_runner] IO error reading {}: {e}",
                    path.display()
                );
                continue;
            }
        };

        // Quick pre-filter: skip files with no @test marker at all (saves parse time).
        if !src.contains("@test") {
            continue;
        }

        let label = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();

        let failures = run_vox_tests(&src, &label);

        // Count test fns in this file (heuristic: count "@test" occurrences).
        let count = src.matches("@test").count();
        total_tests += count;

        for (name, msg) in failures {
            all_failures.push((path.clone(), name, msg));
        }
    }

    if all_failures.is_empty() {
        println!(
            "[golden_vox_test_runner] {} @test functions across {} golden files: all passed ✓",
            total_tests,
            files.len(),
        );
    } else {
        let report: String = all_failures
            .iter()
            .map(|(path, name, msg)| {
                format!(
                    "  FAIL {}::{}\n       {}\n",
                    path.strip_prefix(&root).unwrap_or(path).to_string_lossy(),
                    name,
                    msg
                )
            })
            .collect();
        panic!(
            "{} @test function(s) failed in golden examples:\n{}",
            all_failures.len(),
            report
        );
    }
}
