//! DGM-04 for `sequence`: Mermaid-isms parse leniently into the canonical AST, report
//! `diagram/mermaid-syntax` and carry the exact canonical rewrite as fixes. One named test per
//! catalogue row, plus the frozen corpus block compared with the strict importer.

mod support;

use std::fs;
use std::path::PathBuf;

use support::mermaid::{fixes_of, pair};
use support::strip_spans;

use vox_diagram::ast::{Entry, ItemKind};
use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{ImportOpts, import};
use vox_diagram::{check, codes, parse, repair};

const MS: &str = codes::MERMAID_SYNTAX;
const EQL: &str = codes::EXPECTED_QUOTED_LABEL;

/// A sequence with actors `a` and `b` declared and `body` as its statements.
fn wrap(body: &str) -> String {
    format!("sequence {{\n    actor a: \"A\"\n    actor b: \"B\"\n{body}}}\n")
}

fn just(code: &'static str, edits: &[(&str, &str)]) -> Vec<(&'static str, Vec<(String, String)>)> {
    vec![(code, edits.iter().map(|(a, b)| pair(a, b)).collect())]
}

/// Messages of the lenient AST as `(from, to, reply)`.
fn messages(src: &str) -> Vec<(String, String, bool)> {
    let Some(doc) = parse(src).ast else {
        panic!("no ast");
    };
    doc.body
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::Item(i) => match &i.kind {
                ItemKind::Message(m) => Some((m.from.name.clone(), m.to.name.clone(), m.reply)),
                _ => None,
            },
            Entry::Comment(_) => None,
        })
        .collect()
}

#[test]
fn header() {
    let src = "sequenceDiagram\n    actor a: \"A\"\n    a -> b\n";
    let got = fixes_of(src);
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(
        got[0],
        (
            MS,
            vec![pair("sequenceDiagram", "sequence {"), pair("", "}\n")]
        )
    );
    // The body is checked in the same call, as a sequence.
    assert_eq!(got[1].0, "diagram/undeclared-actor");
    let bare = "sequenceDiagram\n    actor a: \"A\"";
    assert_eq!(
        fixes_of(bare),
        just(MS, &[("sequenceDiagram", "sequence {"), ("", "\n}\n")])
    );
    let fixed = repair(src, 5);
    assert!(fixed.report.valid, "{}\n{:?}", fixed.text, fixed.report);
    assert!(fixed.text.starts_with("sequence {\n"), "{}", fixed.text);
    let brace = check("sequenceDiagram\n{\n}\n");
    let kinds: Vec<&str> = brace.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(kinds, ["diagram/unknown-diagram-kind"]);
}

#[test]
fn participant_forms() {
    let rows = [
        ("participant A", "actor A: \"A\""),
        ("participant A as User (web)", "actor A: \"User (web)\""),
        ("participant S as Server: core", "actor S: \"Server: core\""),
        ("participant B as two<br/>lines", "actor B: \"two\\nlines\""),
    ];
    for (line, want) in rows {
        let src = wrap(&format!("    {line}\n"));
        assert_eq!(fixes_of(&src), just(MS, &[(line, want)]), "{line}");
    }
    // The participant is declared, so a later message does not cascade.
    let src = wrap("    participant c as See\n    c -> a\n");
    assert_eq!(
        fixes_of(&src),
        just(MS, &[("participant c as See", "actor c: \"See\"")])
    );
}

#[test]
fn actor_forms() {
    let src = wrap("    actor c\n");
    assert_eq!(fixes_of(&src), just(MS, &[("actor c", "actor c: \"c\"")]));
    let src = wrap("    actor c as Alice\n");
    assert_eq!(
        fixes_of(&src),
        just(MS, &[("actor c as Alice", "actor c: \"Alice\"")])
    );
    assert!(check(&wrap("    actor c: \"Alice\"\n")).valid);
}

#[test]
fn arrow_solid() {
    let src = wrap("    a ->> b: \"x\"\n");
    assert_eq!(fixes_of(&src), just(MS, &[("->>", "->")]));
    assert_eq!(messages(&src), [("a".into(), "b".into(), false)]);
}

#[test]
fn arrow_dashed() {
    let src = wrap("    b -->> a: \"x\"\n");
    assert_eq!(fixes_of(&src), just(MS, &[("", "reply "), ("-->>", "->")]));
    assert_eq!(messages(&src), [("b".into(), "a".into(), true)]);
    let src = wrap("    reply b -->> a\n");
    assert_eq!(fixes_of(&src), just(MS, &[("-->>", "->")]));
    let src = wrap("    b --> a\n");
    assert_eq!(fixes_of(&src), just(MS, &[("", "reply "), ("-->", "->")]));
}

#[test]
fn other_links_have_no_fix() {
    for link in ["--x", "==>", "-.->", "-->|t|"] {
        let src = wrap(&format!("    a {link} b\n"));
        assert_eq!(fixes_of(&src), just(MS, &[]), "{link}");
        assert_eq!(messages(&src).len(), 1, "{link}");
    }
}

#[test]
fn note_forms() {
    let src = wrap("    Note over a: hi\n");
    assert_eq!(
        fixes_of(&src),
        [
            (MS, vec![pair("Note", "note")]),
            (EQL, vec![pair("hi", "\"hi\"")])
        ]
    );
    let src = wrap("    NOTE OVER a,b: \"x\"\n");
    assert_eq!(
        fixes_of(&src),
        just(MS, &[("NOTE", "note"), ("OVER", "over")])
    );
    for line in ["Note right of a: x", "note left of a: x"] {
        let src = wrap(&format!("    {line}\n"));
        assert_eq!(fixes_of(&src), just(MS, &[]), "{line}");
    }
}

#[test]
fn unquoted_message_text() {
    let src = wrap("    a->>b: vox research run\n");
    assert_eq!(
        fixes_of(&src),
        [
            (MS, vec![pair("->>", "->")]),
            (EQL, vec![pair("vox research run", "\"vox research run\"")])
        ]
    );
}

#[test]
fn contextual_words_stay_plain_ids() {
    let src = "sequence {\n    actor participant: \"P\"\n    actor reply: \"R\"\n    actor b: \"B\"\n    participant -> b\n}\n";
    assert!(check(src).valid);
    let src = "sequence {\n    actor reply: \"R\"\n    actor b: \"B\"\n    reply -->> b\n}\n";
    assert_eq!(fixes_of(src), just(MS, &[("", "reply "), ("-->>", "->")]));
}

#[test]
fn corpus_block_repairs_to_the_imported_ir() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "tests/fixtures/corpus/19-scientia-automated-research-historical-extension-research-2026-l112.mmd",
    );
    let Ok(block) = fs::read_to_string(&path) else {
        panic!("cannot read {path:?}");
    };
    assert!(!check(&block).valid, "the Mermaid block is not DSL");
    let r = repair(&block, 5);
    assert!(r.report.valid, "{}\n{:?}", r.text, r.report.diagnostics);
    assert!(r.rounds <= 3, "{} rounds", r.rounds);
    assert_eq!(repair(&r.text, 5).rounds, 0, "{}", r.text);
    let Some(ir) = r.report.ir else {
        panic!("no ir");
    };
    let Some(imported) = import(&block, ImportOpts::default()).sequence else {
        panic!("the strict importer refused the block");
    };
    let (Ok(a), Ok(b)) = (
        serde_json::to_value(&ir),
        serde_json::to_value(Ir::Sequence(imported)),
    ) else {
        panic!("ir does not serialise");
    };
    assert_eq!(strip_spans(a), strip_spans(b), "{}", r.text);
}
