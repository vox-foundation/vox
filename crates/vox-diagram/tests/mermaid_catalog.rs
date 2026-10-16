//! DGM-04: Mermaid-isms parse leniently into the canonical AST, report `diagram/mermaid-syntax`
//! (an Error, so the IR is withheld) and carry the exact canonical rewrite as `fixes[]` where one
//! exists. One named test per catalogue row (M1-M10).

use vox_diagram::ast::{Entry, ItemKind};
use vox_diagram::diag::Severity;
use vox_diagram::{check, codes, parse, repair};

const MS: &str = codes::MERMAID_SYNTAX;

fn wrap(body: &str) -> String {
    format!("graph {{\n    node a: \"A\"\n    node b: \"B\"\n{body}}}\n")
}

/// `(code, [(original text at the fix span, replacement)])` for every diagnostic.
fn fixes_of(src: &str) -> Vec<(&'static str, Vec<(String, String)>)> {
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

fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_string(), b.to_string())
}

#[test]
fn mermaid_m1_long_arrow_fix() {
    let src = wrap("    a --> b\n");
    let report = check(&src);
    assert!(!report.valid && report.ir.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].severity, Severity::Error);
    assert_eq!(fixes_of(&src), vec![(MS, vec![pair("-->", "->")])]);
    let long = wrap("    a -----> b\n");
    assert_eq!(fixes_of(&long), vec![(MS, vec![pair("----->", "->")])]);
    // The lenient AST holds the edge exactly as the canonical form would.
    let ast = parse(&src).ast.unwrap_or_else(|| panic!("no ast"));
    let edges = ast.body.entries.iter().filter(|e| {
        matches!(e, Entry::Item(i) if matches!(&i.kind, ItemKind::Edge(x)
            if x.from.segments[0].name == "a" && x.to.segments[0].name == "b"))
    });
    assert_eq!(edges.count(), 1);
}

#[test]
fn mermaid_m1_converges_with_declare_fixes() {
    let src = "graph {\n    A --> B\n}\n";
    let r = repair(src, 5);
    assert!(r.report.valid, "{}", r.text);
    // The arrow and both declarations are independent fixes, so one round may suffice.
    assert!(r.rounds <= 2, "{} rounds", r.rounds);
    for want in ["node A: \"A\"", "node B: \"B\"", "A -> B"] {
        assert!(r.text.contains(want), "{want} missing from\n{}", r.text);
    }
}

#[test]
fn mermaid_m1_styled_arrows_have_no_fix() {
    for arrow in ["==>", "-.->", "---", "--x", "--o"] {
        let src = wrap(&format!("    a {arrow} b\n"));
        assert_eq!(fixes_of(&src), vec![(MS, vec![])], "{arrow}");
        assert!(!check(&src).valid, "{arrow}");
    }
}
