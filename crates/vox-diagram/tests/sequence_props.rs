//! Properties of `check`, `fmt` and `repair` over generated sequence documents (S1-S4): the
//! sequence contract matches the graph contract. A counterexample is a real defect.

mod support;

use proptest::prelude::*;
use serde_json::{Value, to_value};
use support::seq::{
    SeqItem, SeqModel, actor_id, model_seq_ir_json, model_sequence, noisy_seq_source,
};
use support::{noise, strip_spans};
use vox_diagram::ast::{Entry, Item, ItemKind, Label};
use vox_diagram::diag::CheckReport;
use vox_diagram::{check, fmt, parse, repair};

fn ir(src: &str) -> Value {
    strip_spans(to_value(check(src).ir).unwrap_or(Value::Null))
}

fn ast(src: &str) -> Value {
    strip_spans(to_value(parse(src).ast).unwrap_or(Value::Null))
}

fn report_ir(report: &CheckReport) -> Value {
    strip_spans(to_value(&report.ir).unwrap_or(Value::Null))
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

/// One byte-range edit: `(start, end, replacement)`.
type Edit = (usize, usize, String);

fn items(src: &str) -> Vec<Item> {
    let Some(doc) = parse(src).ast else {
        return Vec::new();
    };
    doc.body
        .entries
        .into_iter()
        .filter_map(|e| match e {
            Entry::Item(i) => Some(i),
            Entry::Comment(_) => None,
        })
        .collect()
}

/// The sites of corruption `kind`; each site is the edits that apply together. Located through
/// the AST, never by searching the text.
/// 0 `->` to `->>`; 1 drop `reply ` and `->` to `-->>`; 2 `actor` to `participant ... as`;
/// 3 `note` to `Note`; 4 a message label left unquoted; 5 delete the final `}`.
fn sites(src: &str, kind: usize) -> Vec<Vec<Edit>> {
    let at = |s: u32| s as usize;
    if kind == 5 {
        let close = parse(src).ast.and_then(|d| d.body.braces.map(|b| b.1));
        return close
            .map(|c| vec![(at(c), at(c) + 1, String::new())])
            .into_iter()
            .collect();
    }
    let mut out = Vec::new();
    for item in items(src) {
        match (&item.kind, kind) {
            (ItemKind::Message(m), 0 | 1) if m.reply == (kind == 1) => {
                let gap = src.get(at(m.from.span.end)..at(m.to.span.start));
                let Some(arrow) = gap.and_then(|g| g.find("->")) else {
                    continue;
                };
                let a = at(m.from.span.end) + arrow;
                if kind == 0 {
                    out.push(vec![(a, a + 2, "->>".to_string())]);
                } else {
                    let drop = (at(item.span.start), at(m.from.span.start), String::new());
                    out.push(vec![drop, (a, a + 2, "-->>".to_string())]);
                }
            }
            (ItemKind::Actor(a), 2) => {
                if item.span.end == a.label.span.end && plain(&a.label.value) {
                    let text = format!("participant {} as {}", a.id.name, a.label.value);
                    out.push(vec![(at(item.span.start), at(item.span.end), text)]);
                }
            }
            (ItemKind::Note(_), 3) => {
                let s = at(item.span.start);
                if src.get(s..s + 4) == Some("note") {
                    out.push(vec![(s, s + 4, "Note".to_string())]);
                }
            }
            (ItemKind::Message(m), 4) => {
                let ok = |l: &vox_diagram::ast::Label| {
                    plain(&l.value) && ends_line(src, l.span.end) && item.trailing.is_none()
                };
                if let Some(l) = m.label.as_ref().filter(|l| ok(l)) {
                    out.push(vec![(at(l.span.start), at(l.span.end), l.value.clone())]);
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
        found = sites(src, 5);
    }
    let Some(site) = found.get(pick % found.len().max(1)) else {
        return src.to_string();
    };
    apply(src, site)
}

fn apply(src: &str, site: &[Edit]) -> String {
    let mut edits = site.to_vec();
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = src.to_string();
    for (start, end, text) in edits {
        out.replace_range(start..end, &text);
    }
    out
}

/// A model every event of which has a plain label (`word<i> beta`, or `word<i>\nbeta` when
/// `multiline`) and whose last actor is labelled with its own id and mentioned by an event: the
/// shape the [`sites2`] corruptions need to repair back to an identical IR.
fn tidy(mut m: SeqModel, multiline: bool) -> SeqModel {
    if m.actors.is_empty() {
        m.actors.push((7, String::new()));
    }
    let last = m.actors.len() - 1;
    m.actors[last].1 = actor_id(m.actors[last].0, last);
    for _ in 0..2 {
        m.events.push(SeqItem::Message {
            from: last,
            to: last,
            label: None,
            reply: false,
        });
    }
    let sep = if multiline { '\n' } else { ' ' };
    for (i, e) in m.events.iter_mut().enumerate() {
        let text = format!("word{i}{sep}beta");
        match e {
            SeqItem::Message { label, .. } => *label = Some(text),
            SeqItem::Note { label, .. } => *label = text,
        }
    }
    m
}

fn label_of(kind: &ItemKind) -> Option<&Label> {
    match kind {
        ItemKind::Message(m) => m.label.as_ref(),
        ItemKind::Note(n) => Some(&n.label),
        _ => None,
    }
}

/// Review-driven corruptions, each a single-site edit set. 0 a message or note label left
/// unquoted and joined to the next statement with `;`; 1 an unquoted label ended by a trailing
/// `;`; 2 a quoted `\n` written as an unquoted `<br>` / `<br/>` / `<BR>`; 3 the last actor's
/// declaration deleted while an event still mentions it.
fn sites2(src: &str, kind: usize, pick: usize) -> Vec<Vec<Edit>> {
    let at = |s: u32| s as usize;
    let all = items(src);
    if kind == 3 {
        let last = all
            .iter()
            .rev()
            .find(|i| matches!(i.kind, ItemKind::Actor(_)));
        return last
            .map(|i| vec![(at(i.span.start), at(i.span.end), String::new())])
            .into_iter()
            .collect();
    }
    let br = ["<br>", "<br/>", "<BR>"][pick % 3];
    let mut out = Vec::new();
    for (n, item) in all.iter().enumerate() {
        let Some(l) = label_of(&item.kind) else {
            continue;
        };
        if !ends_line(src, l.span.end) || item.trailing.is_some() {
            continue;
        }
        let (start, end) = (at(l.span.start), at(l.span.end));
        let bare = (start, end, l.value.replace('\n', br));
        match kind {
            0 => {
                let next = all.get(n + 1).map_or(end, |i| at(i.span.start));
                let gap = src.get(end..next).and_then(|g| g.strip_prefix('\n'));
                if gap.is_some_and(|g| g.chars().all(|c| matches!(c, ' ' | '\t'))) {
                    out.push(vec![bare, (end, next, "; ".to_string())]);
                }
            }
            1 => out.push(vec![bare, (end, end, ";".to_string())]),
            _ => out.push(vec![bare]),
        }
    }
    out
}

fn canonical(m: &SeqModel, n: &support::Noise) -> String {
    fmt(&noisy_seq_source(m, n)).unwrap_or_default()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// S1: the checker agrees with an oracle built from the model alone.
    #[test]
    fn s1_check_matches_the_model_oracle(m in model_sequence(), n in noise()) {
        let src = noisy_seq_source(&m, &n);
        let report = check(&src);
        prop_assert!(report.diagnostics.is_empty(), "{:?}\n{src}", report.diagnostics);
        prop_assert!(report.valid);
        prop_assert_eq!(ir(&src), model_seq_ir_json(&m), "{}", src);
    }

    /// S2: formatting is idempotent and canonical in shape.
    #[test]
    fn s2_fmt_is_idempotent(m in model_sequence(), n in noise()) {
        let src = noisy_seq_source(&m, &n);
        let once = fmt(&src);
        prop_assert!(once.is_some(), "valid source must format:\n{src}");
        let once = once.unwrap_or_default();
        let twice = fmt(&once);
        prop_assert_eq!(twice.as_deref(), Some(once.as_str()));
        prop_assert!(!once.contains('\r') && once.ends_with('\n') && !once.ends_with("\n\n"));
        prop_assert!(once.lines().all(|l| l == l.trim_end()), "trailing whitespace:\n{once}");
    }

    /// S3: formatting never changes the AST (comments and blank lines included) or the IR.
    #[test]
    fn s3_fmt_preserves_ast_and_ir(m in model_sequence(), n in noise()) {
        let src = noisy_seq_source(&m, &n);
        let out = fmt(&src).unwrap_or_default();
        prop_assert_eq!(ast(&out), ast(&src), "{}\n---\n{}", src, out);
        prop_assert_eq!(ir(&out), ir(&src));
    }

    /// S4: one Mermaid-ism whose inverse is exact is applied to a canonical document, and
    /// `repair` brings back a valid document with the original IR.
    #[test]
    fn s4_mermaid_corruptions_repair(
        m in model_sequence(), n in noise(), kind in 0usize..5, pick in 0usize..64,
    ) {
        let src = canonical(&m, &n);
        let original = check(&src);
        prop_assert!(original.valid, "{:?}\n{src}", original.diagnostics);
        let bad = corrupt(&src, kind, pick);
        prop_assert_ne!(&bad, &src, "the corruption changed nothing");
        let fixed = repair(&bad, 5);
        prop_assert!(fixed.report.valid, "{:?}\n{bad}\n=>\n{}", fixed.report.diagnostics, fixed.text);
        prop_assert_eq!(
            report_ir(&fixed.report),
            report_ir(&original),
            "{}\n=>\n{}\n=>\n{}", src, bad, fixed.text
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// S5: `;` statement terminators after unquoted labels, Mermaid `<br>` in unquoted labels
    /// and a deleted actor declaration all repair back to the original IR.
    #[test]
    fn s5_semicolons_br_and_undeclared_actors_repair(
        m in model_sequence(), n in noise(), kind in 0usize..4, pick in 0usize..64,
    ) {
        let src = canonical(&tidy(m, kind == 2), &n);
        let original = check(&src);
        prop_assert!(original.valid, "{:?}\n{src}", original.diagnostics);
        let mut found = sites2(&src, kind, pick);
        if found.is_empty() {
            found = sites2(&src, 1, pick);
        }
        prop_assume!(!found.is_empty());
        let bad = apply(&src, &found[pick % found.len()]);
        prop_assert_ne!(&bad, &src, "the corruption changed nothing");
        prop_assert!(!check(&bad).valid, "stayed valid:\n{bad}");
        let fixed = repair(&bad, 5);
        prop_assert!(fixed.report.valid, "{:?}\n{bad}\n=>\n{}", fixed.report.diagnostics, fixed.text);
        prop_assert_eq!(
            report_ir(&fixed.report),
            report_ir(&original),
            "{}\n=>\n{}\n=>\n{}", src, bad, fixed.text
        );
    }
}

/// Every corruption kind has a real site in a plain document, changes the text, invalidates
/// it and repairs back, so S4 can never pass by always falling back to the same corruption.
#[test]
fn every_corruption_kind_applies_and_repairs() {
    let src = concat!(
        "sequence {\n",
        "    actor a: \"Alpha one\"\n",
        "    actor b: \"Beta\"\n",
        "    a -> b: \"go now\"\n",
        "    reply b -> a: \"back\"\n",
        "    note over a, b: \"both\"\n",
        "}\n",
    );
    let original = check(src);
    assert!(original.valid, "{:?}", original.diagnostics);
    for kind in 0..6 {
        assert!(!sites(src, kind).is_empty(), "kind {kind} has no site");
        for pick in 0..2 {
            let bad = corrupt(src, kind, pick);
            assert_ne!(bad, src, "kind {kind} pick {pick}");
            assert!(
                !check(&bad).valid,
                "kind {kind} pick {pick} stayed valid:\n{bad}"
            );
            let fixed = repair(&bad, 5);
            assert!(
                fixed.report.valid,
                "kind {kind} pick {pick}:\n{bad}\n=>\n{}",
                fixed.text
            );
            assert_eq!(
                report_ir(&fixed.report),
                report_ir(&original),
                "kind {kind} pick {pick}"
            );
        }
    }
}

/// The review corruptions have a site in a plain document and repair back to its IR.
#[test]
fn every_review_corruption_applies_and_repairs() {
    let src = concat!(
        "sequence {\n",
        "    actor a: \"a\"\n",
        "    actor b: \"b\"\n",
        "    a -> b: \"go\\nnow\"\n",
        "    b -> a: \"back\"\n",
        "    note over a, b: \"both\"\n",
        "}\n",
    );
    let original = check(src);
    assert!(original.valid, "{:?}", original.diagnostics);
    for kind in 0..4 {
        for pick in 0..3 {
            let found = sites2(src, kind, pick);
            assert!(!found.is_empty(), "kind {kind} has no site");
            let bad = apply(src, &found[pick % found.len()]);
            assert!(
                !check(&bad).valid,
                "kind {kind} pick {pick} stayed valid:\n{bad}"
            );
            let fixed = repair(&bad, 5);
            assert!(
                fixed.report.valid,
                "kind {kind} pick {pick}:\n{bad}\n=>\n{}",
                fixed.text
            );
            assert_eq!(
                report_ir(&fixed.report),
                report_ir(&original),
                "kind {kind} pick {pick}\n{bad}"
            );
        }
    }
}
