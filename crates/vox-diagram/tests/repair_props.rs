//! S4 mutate-and-repair: one corruption whose inverse is exact is applied to a valid generated
//! document, and `repair` must bring back a valid document with the original IR. Any
//! counterexample is a real fix bug.

mod support;

use proptest::prelude::*;
use serde_json::{Value, to_value};
use support::{model_graph, noise, noisy_source, strip_spans};
use vox_diagram::ast::{Block, Entry, Item, ItemKind};
use vox_diagram::diag::CheckReport;
use vox_diagram::parse;
use vox_diagram::{check, repair};

/// Every statement of the block, containers included.
fn items<'a>(b: &'a Block, out: &mut Vec<&'a Item>) {
    for e in &b.entries {
        if let Entry::Item(i) = e {
            out.push(i);
            if let ItemKind::Container(c) = &i.kind {
                items(&c.body, out);
            }
        }
    }
}

/// Words that survive as unquoted text: letters, digits and `_`, single-spaced, none starting
/// with a digit (a digit start is an invalid identifier).
fn plain(text: &str) -> bool {
    let word = |w: &str| {
        w.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && w.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    !text.is_empty() && text.split(' ').all(word)
}

/// Only blanks between `pos` and the end of the line.
fn ends_line(src: &str, pos: u32) -> bool {
    let rest = src.get(pos as usize..).unwrap_or_default();
    let rest = rest.trim_start_matches([' ', '\t']);
    rest.is_empty() || rest.starts_with(['\n', '\r'])
}

type Edit = (usize, usize, String);

/// The sites of corruption `kind` (0 arrow, 1 unquote, 2 final brace, 3 quote, 4 bracket form).
fn sites(src: &str, kind: usize) -> Vec<Edit> {
    let Some(doc) = parse(src).ast else {
        return Vec::new();
    };
    if kind == 2 {
        let close = doc.body.braces.map(|(_, c)| c as usize);
        return close
            .map(|c| (c, c + 1, String::new()))
            .into_iter()
            .collect();
    }
    let mut all = Vec::new();
    items(&doc.body, &mut all);
    let mut out = Vec::new();
    for item in all {
        match (&item.kind, kind) {
            (ItemKind::Edge(e), 0) => {
                let gap = src.get(e.from.span.end as usize..e.to.span.start as usize);
                if let Some(at) = gap.and_then(|g| g.find("->")) {
                    let at = e.from.span.end as usize + at;
                    out.push((at, at + 2, "-->".to_string()));
                }
            }
            (ItemKind::Edge(e), 1) => {
                let l = e.label.as_ref();
                let ok =
                    |l: &vox_diagram::ast::Label| plain(&l.value) && ends_line(src, l.span.end);
                if let Some(l) = l.filter(|l| ok(l) && item.trailing.is_none()) {
                    out.push((l.span.start as usize, l.span.end as usize, l.value.clone()));
                }
            }
            (ItemKind::Edge(_) | ItemKind::Node(_), 3) => {
                let label = match &item.kind {
                    ItemKind::Edge(e) => e.label.as_ref(),
                    ItemKind::Node(n) => Some(&n.label),
                    _ => None,
                };
                let tidy = |v: &str| !v.is_empty() && !v.ends_with(char::is_whitespace);
                if let Some(l) = label.filter(|l| tidy(&l.value) && ends_line(src, l.span.end)) {
                    let at = l.span.end as usize - 1;
                    out.push((at, at + 1, String::new()));
                }
            }
            (ItemKind::Node(n), 4) => {
                let rect = n.shape == vox_diagram::ir::Shape::Rect;
                if rect && item.span.end == n.label.span.end && plain(&n.label.value) {
                    let text = format!("{}[{}]", n.id.name, n.label.value);
                    out.push((item.span.start as usize, item.span.end as usize, text));
                }
            }
            _ => {}
        }
    }
    out
}

/// Apply one corruption of `kind` at the `pick`th site; with no site, delete the final `}`.
fn corrupt(src: &str, kind: usize, pick: usize) -> String {
    let mut found = sites(src, kind);
    if found.is_empty() {
        found = sites(src, 2);
    }
    let Some((start, end, text)) = found.get(pick % found.len().max(1)) else {
        return src.to_string();
    };
    let mut out = src.to_string();
    out.replace_range(*start..*end, text);
    out
}

fn ir(report: &CheckReport) -> Value {
    strip_spans(to_value(&report.ir).unwrap_or(Value::Null))
}

/// Every corruption kind has a real site in a plain document and repairs back (so the property
/// below can never pass by always falling back to the same corruption).
#[test]
fn every_corruption_kind_applies_and_repairs() {
    let src =
        "graph {\n    node a: \"Alpha one\"\n    node b: \"Beta\"\n    a -> b: \"go now\"\n}\n";
    let original = check(src);
    for kind in 0..5 {
        assert!(!sites(src, kind).is_empty(), "kind {kind} has no site");
        for pick in 0..3 {
            let bad = corrupt(src, kind, pick);
            assert_ne!(bad, src, "kind {kind} pick {pick}");
            let fixed = repair(&bad, 5);
            assert!(fixed.report.valid, "kind {kind} pick {pick}: {bad}");
            assert_eq!(ir(&fixed.report), ir(&original), "kind {kind} pick {pick}");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn mutate_and_repair(m in model_graph(), n in noise(), kind in 0usize..5, pick in 0usize..64) {
        let src = noisy_source(&m, &n);
        let original = check(&src);
        prop_assert!(original.valid, "{:?}\n{src}", original.diagnostics);
        let bad = corrupt(&src, kind, pick);
        prop_assert_ne!(&bad, &src, "the corruption changed nothing");
        let fixed = repair(&bad, 5);
        prop_assert!(fixed.report.valid, "{:?}\n{bad}\n=>\n{}", fixed.report.diagnostics, fixed.text);
        prop_assert_eq!(ir(&fixed.report), ir(&original), "{}\n=>\n{}\n=>\n{}", src, bad, fixed.text);
    }
}
