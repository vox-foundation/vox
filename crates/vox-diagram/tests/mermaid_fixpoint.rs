//! `import(export(ir))` is a set-preserving inverse of `export` (MIG-02, Rust side only; the
//! Node oracle judges the fixed fixtures). Canonical sets come from the IR alone.

mod support;

use proptest::prelude::*;
use support::hostile::FRAGMENTS;
use support::roundtrip::{canon_graph, canon_sequence};
use support::{model_graph, noise, noisy_source};
use vox_diagram::check;
use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{ImportOpts, export, export_sequence, import};
use vox_diagram::sequence_ir::{Actor, SeqEvent, SequenceIr};

/// The DSL lexer's forbidden characters (plus `\r`): the importer refuses them even as numeric
/// entities, so a sequence IR drawn from DSL-legal text never holds one. Generator domain only.
fn legal(c: char) -> bool {
    !matches!(c, '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{61c}'
        | '\u{200e}' | '\u{200f}' | '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}'
        | '\u{2066}'..='\u{2069}')
}

fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            "[ -~]{0,6}",
            prop::sample::select(FRAGMENTS.to_vec()).prop_map(str::to_owned),
        ],
        0..4,
    )
    .prop_map(|v| v.concat().chars().filter(|c| legal(*c)).collect())
}

fn non_blank() -> impl Strategy<Value = String> {
    text().prop_filter("non-blank after trim", |t| !t.trim().is_empty())
}

/// One or two declared actors and events that only mention them.
fn sequence() -> impl Strategy<Value = SequenceIr> {
    (1usize..=2).prop_flat_map(|n| {
        let ids = ["a", "b"];
        let who = move || prop::sample::select(ids[..n].to_vec()).prop_map(str::to_owned);
        let event = prop_oneof![
            (who(), who(), text(), any::<bool>()).prop_map(|(from, to, text, dashed)| {
                SeqEvent::Message {
                    from,
                    to,
                    text,
                    dashed,
                    span: Default::default(),
                }
            }),
            (prop::collection::vec(who(), 1..=2), non_blank()).prop_map(|(over, text)| {
                SeqEvent::Note {
                    over,
                    text,
                    span: Default::default(),
                }
            }),
        ];
        (
            prop::collection::vec(non_blank(), n),
            prop::collection::vec(event, 0..8),
        )
            .prop_map(move |(labels, events)| SequenceIr {
                actors: labels
                    .into_iter()
                    .enumerate()
                    .map(|(i, label)| Actor {
                        id: ids[i].to_owned(),
                        label,
                        span: Default::default(),
                    })
                    .collect(),
                events,
            })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// F1: graph IRs survive export then import with identical canonical sets.
    #[test]
    fn f1_graph_import_inverts_export(m in model_graph(), n in noise()) {
        let src = noisy_source(&m, &n);
        let Some(Ir::Graph(g)) = check(&src).ir else {
            return Err(TestCaseError::fail(format!("model source does not check:\n{src}")));
        };
        prop_assume!(g.nodes.iter().all(|x| !x.label.trim().is_empty()));
        prop_assume!(g.containers.iter().all(|c| !c.label.trim().is_empty()));
        let exported = export(&Ir::Graph(g.clone()));
        let report = import(&exported, ImportOpts::default());
        prop_assert!(report.unsupported.is_empty(), "{:?}\n{exported}", report.unsupported);
        let Some(back) = report.source else {
            return Err(TestCaseError::fail(format!("no source for:\n{exported}")));
        };
        let Some(Ir::Graph(h)) = check(&back).ir else {
            return Err(TestCaseError::fail(format!("imported source does not check:\n{back}")));
        };
        prop_assert_eq!(canon_graph(&h), canon_graph(&g), "{}\n---\n{}", exported, back);
    }

    /// F2: sequence IRs survive export then import with identical canonical sets.
    #[test]
    fn f2_sequence_import_inverts_export(s in sequence()) {
        let exported = export_sequence(&s);
        let report = import(&exported, ImportOpts::default());
        prop_assert!(report.unsupported.is_empty(), "{:?}\n{exported}", report.unsupported);
        let Some(back) = report.sequence else {
            return Err(TestCaseError::fail(format!("no sequence for:\n{exported}")));
        };
        prop_assert_eq!(canon_sequence(&back), canon_sequence(&s), "{}", exported);
    }
}
