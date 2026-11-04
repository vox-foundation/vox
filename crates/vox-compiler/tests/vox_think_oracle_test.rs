//! Oracle for the held-out `vox_think` slice (BENCH-02).
//!
//! Every row carries a Vox `program`. Its gold `answer` must be the integer the
//! interpreter returns from `main`, not a hand-computed number. This test runs the
//! full lex -> parse -> lower -> eval pipeline on each program and compares.

use std::path::{Path, PathBuf};

use vox_compiler::eval::value::VoxValue;
use vox_compiler::{eval, hir, lexer, parser};
use vox_eval::reasoning_bench::{ReasoningItem, SLICE_SIZE, load_slice};

fn slice_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mens/data/vox_think/reasoning.jsonl")
}

fn run_probe(source: &str) -> Result<VoxValue, String> {
    let tokens = lexer::lex(source);
    let module = parser::descent::parse(tokens).map_err(|e| format!("parse: {e:?}"))?;
    let lowered = hir::lower::lower_module(&module);
    let mut interp = eval::Interpreter::new(1_000_000);
    interp
        .run_module(&lowered)
        .map_err(|e| format!("run: {e:?}"))?;
    interp
        .call("main", vec![])
        .map_err(|e| format!("call: {e:?}"))
}

/// Items from the shared loader paired with each row's `program` key.
fn rows() -> Vec<(ReasoningItem, String)> {
    let path = slice_path();
    let items = load_slice(&path, SLICE_SIZE).expect("vox_think slice loads");
    let text = std::fs::read_to_string(&path).expect("read vox_think");
    let programs: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).expect("json row");
            v["program"]
                .as_str()
                .expect("every vox_think row has a program")
                .to_string()
        })
        .collect();
    assert_eq!(items.len(), programs.len());
    items.into_iter().zip(programs).collect()
}

#[test]
fn every_vox_think_program_returns_its_gold() {
    let mut failures = Vec::new();
    for (item, program) in rows() {
        match run_probe(&program) {
            Ok(VoxValue::Int(n)) if n.to_string() == item.answer => {}
            other => failures.push(format!(
                "{}: expected answer {} but main returned {:?}",
                item.id, item.answer, other
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "gold mismatch:\n{}",
        failures.join("\n")
    );
}

#[test]
fn question_embeds_its_program() {
    for (item, program) in rows() {
        assert!(
            item.question.contains(program.trim()),
            "{}: question does not embed its program verbatim",
            item.id
        );
    }
}

#[test]
fn programs_are_pure_and_nontrivial() {
    const DENY: [&str; 9] = [
        "fs.", "http", "process.", "env.", "net.", "random", "time.", "shell", "@uses",
    ];
    for (item, program) in rows() {
        for bad in DENY {
            assert!(
                !program.contains(bad),
                "{}: program uses forbidden identifier {bad:?}",
                item.id
            );
        }
        let has_logic = ["for ", "while ", "match ", "if "]
            .iter()
            .any(|k| program.contains(k))
            || program.matches("fn ").count() > 1;
        assert!(
            has_logic,
            "{}: program has no control flow or helper fn",
            item.id
        );
    }
}
