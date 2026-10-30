//! Helpers shared by the Mermaid-ism suites.
#![allow(dead_code)]

use vox_diagram::ir::{GraphIr, Ir};
use vox_diagram::{check, repair};

pub fn wrap(body: &str) -> String {
    format!("graph {{\n    node a: \"A\"\n    node b: \"B\"\n{body}}}\n")
}

/// `(code, [(original text at the fix span, replacement)])` for every diagnostic.
pub fn fixes_of(src: &str) -> Vec<(&'static str, Vec<(String, String)>)> {
    check(src)
        .diagnostics
        .iter()
        .map(|d| {
            let edits = d.fixes.iter().map(|f| {
                let orig = src.get(f.span.start as usize..f.span.end as usize);
                (
                    orig.unwrap_or("<bad span>").to_string(),
                    f.replacement.clone(),
                )
            });
            (d.code, edits.collect())
        })
        .collect()
}

pub fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_string(), b.to_string())
}

/// Fix edits of the diagnostics with `code` (diagnostic order).
pub fn edits(src: &str, code: &str) -> Vec<Vec<(String, String)>> {
    let all = fixes_of(src);
    all.into_iter()
        .filter(|(c, _)| *c == code)
        .map(|(_, e)| e)
        .collect()
}

/// Parse `src` into the IR of a repaired document.
pub fn repaired_graph(src: &str) -> GraphIr {
    let r = repair(src, 5);
    assert!(
        r.report.valid,
        "{src:?} -> {}\n{:?}",
        r.text, r.report.diagnostics
    );
    match r.report.ir {
        Some(Ir::Graph(g)) => g,
        other => panic!("no graph ir: {other:?}"),
    }
}
