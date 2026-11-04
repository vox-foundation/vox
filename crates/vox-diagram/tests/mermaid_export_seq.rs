//! `export_sequence`: exact text, the hostile goldens the Node oracle judges
//! (`fixtures/export-hostile/s*`) and a structural property over arbitrary sequence IRs.

mod support;

use std::collections::BTreeSet;

use proptest::prelude::*;
use support::hostile::{FRAGMENTS, payload_problem};
use support::roundtrip::{assert_golden, canon_sequence, fixtures, sets_json, stems};
use vox_diagram::mermaid::export_sequence;
use vox_diagram::sequence_ir::{Actor, SeqEvent, SequenceIr};

fn seq(actors: &[(&str, &str)], events: Vec<SeqEvent>) -> SequenceIr {
    SequenceIr {
        actors: actors
            .iter()
            .map(|(id, label)| Actor {
                id: (*id).into(),
                label: (*label).into(),
                span: Default::default(),
            })
            .collect(),
        events,
    }
}

fn msg(from: &str, to: &str, text: &str, dashed: bool) -> SeqEvent {
    SeqEvent::Message {
        from: from.into(),
        to: to.into(),
        text: text.into(),
        dashed,
        span: Default::default(),
    }
}

fn note(over: &[&str], text: &str) -> SeqEvent {
    SeqEvent::Note {
        over: over.iter().map(|s| (*s).to_owned()).collect(),
        text: text.into(),
        span: Default::default(),
    }
}

fn basic() -> SequenceIr {
    seq(
        &[("a", "Alice"), ("b", "Bob")],
        vec![
            msg("a", "b", "hello", false),
            msg("b", "a", "hi", true),
            note(&["a", "b"], "both"),
            note(&["a"], "one"),
        ],
    )
}

#[test]
fn export_sequence_basic() {
    let want = "sequenceDiagram\n    participant p0 as Alice\n    participant p1 as Bob\n    p0->>p1: hello\n    p1-->>p0: hi\n    Note over p0,p1: both\n    Note over p0: one\n";
    assert_eq!(export_sequence(&basic()), want);
}

#[test]
fn undeclared_ids_become_participants_in_first_appearance_order() {
    let s = seq(
        &[("a", "A")],
        vec![msg("z", "a", "x", false), note(&["y", "z"], "n")],
    );
    let want = "sequenceDiagram\n    participant p0 as A\n    participant p1 as z\n    participant p2 as y\n    p1->>p0: x\n    Note over p2,p1: n\n";
    assert_eq!(export_sequence(&s), want);
}

#[test]
fn blank_texts_print_the_forms_the_parser_accepts() {
    let s = seq(
        &[("a", ""), ("b", "B")],
        vec![
            msg("a", "b", "", false),
            msg("b", "a", "  ", true),
            note(&["a"], ""),
        ],
    );
    let want = "sequenceDiagram\n    participant p0 as #32;\n    participant p1 as B\n    p0->>p1:\n    p1-->>p0:\n    Note over p0: #32;\n";
    assert_eq!(export_sequence(&s), want);
}

#[test]
fn degenerate_notes_keep_at_most_two_ids() {
    let s = seq(
        &[("a", "A"), ("b", "B"), ("c", "C")],
        vec![note(&["a", "b", "c"], "three"), note(&[], "none")],
    );
    let out = export_sequence(&s);
    assert!(out.contains("    Note over p0,p1: three\n"), "{out}");
    assert_eq!(out.lines().count(), 1 + 3 + 2, "{out}");
}

fn s01_text() -> SequenceIr {
    let texts = [
        "a; b",
        "# hash",
        "wrap: x",
        "nowrap: x",
        ":wrap: x",
        ":nowrap:x",
        "literal <br> tag",
        "real\nnewline",
        "say \"hi\" & bye",
        "x as y",
        "a ->> b",
        "Note over a: z",
        "%%{init: {}}%%",
        "`tick",
        "style: #quot;",
    ];
    let mut events: Vec<SeqEvent> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| msg(["a", "b"][i % 2], ["b", "a"][i % 2], t, i % 3 == 0))
        .collect();
    events.push(note(&["a"], "semi; and # and wrap: inside"));
    events.push(note(&["a", "b"], "wrap: whole"));
    events.push(note(&["b"], ":nowrap: lead"));
    seq(&[("a", "Alice; the first"), ("b", "Bob <b> & co")], events)
}

fn s02_ids() -> SequenceIr {
    let ids = ["end", "opt", "loop", "alt", "participant", "Note", "a-b"];
    let labels: Vec<String> = ids.iter().map(|i| format!("Actor {i}")).collect();
    let actors: Vec<(&str, &str)> = ids
        .iter()
        .zip(&labels)
        .map(|(i, l)| (*i, l.as_str()))
        .collect();
    seq(
        &actors,
        vec![
            msg("end", "end", "self", false),
            msg("loop", "loop", "dashed self", true),
            msg("opt", "alt", "forward", false),
            note(&["participant", "Note"], "two actors"),
            note(&["a-b"], "one actor"),
            msg("alt", "ghost", "to nowhere", false),
            note(&["phantom"], "over nowhere"),
        ],
    )
}

fn s03_empty() -> SequenceIr {
    seq(
        &[("a", ""), ("b", "   "), ("c", "C")],
        vec![
            msg("a", "b", "", false),
            msg("b", "c", "   ", true),
            note(&["c"], ""),
            note(&["a", "b"], "  "),
        ],
    )
}

fn cases() -> Vec<(&'static str, SequenceIr)> {
    vec![
        ("s00-basic", basic()),
        ("s01-text", s01_text()),
        ("s02-ids", s02_ids()),
        ("s03-empty", s03_empty()),
    ]
}

#[test]
fn export_sequence_hostile_goldens() {
    let dir = fixtures().join("export-hostile");
    let table = cases();
    for (stem, ir) in &table {
        assert_golden(
            &dir.join(format!("{stem}.export.mmd")),
            &export_sequence(ir),
        );
        assert_golden(
            &dir.join(format!("{stem}.sets.json")),
            &sets_json(&canon_sequence(ir)),
        );
    }
    let known: BTreeSet<&str> = table.iter().map(|(s, _)| *s).collect();
    for ext in ["mmd", "json"] {
        for stem in stems(&dir, ext) {
            let base = stem.trim_end_matches(".export").trim_end_matches(".sets");
            assert!(
                !base.starts_with('s') || known.contains(base),
                "orphan golden {stem}.{ext}"
            );
        }
    }
}

const POOL: [&str; 8] = ["a", "b", "end", "Note", "a-b", "ghost", "", "loop"];

fn id() -> impl Strategy<Value = String> {
    prop::sample::select(POOL.to_vec()).prop_map(str::to_owned)
}

fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            "\\PC{0,6}",
            prop::sample::select(FRAGMENTS.to_vec()).prop_map(str::to_owned),
            prop::sample::select(vec!["wrap:", "nowrap:", ":wrap:", ":nowrap:", ";", " "])
                .prop_map(str::to_owned),
        ],
        0..4,
    )
    .prop_map(|v| v.concat())
}

fn arbitrary_sequence() -> impl Strategy<Value = SequenceIr> {
    let actor = (id(), text()).prop_map(|(id, label)| Actor {
        id,
        label,
        span: Default::default(),
    });
    let event = prop_oneof![
        (id(), id(), text(), any::<bool>()).prop_map(|(f, t, x, d)| msg(&f, &t, &x, d)),
        (prop::collection::vec(id(), 0..4), text()).prop_map(|(o, x)| SeqEvent::Note {
            over: o,
            text: x,
            span: Default::default()
        }),
    ];
    (
        prop::collection::vec(actor, 0..4),
        prop::collection::vec(event, 0..8),
    )
        .prop_map(|(actors, events)| SequenceIr { actors, events })
}

fn undeclared(s: &SequenceIr) -> usize {
    let declared: BTreeSet<&str> = s.actors.iter().map(|a| a.id.as_str()).collect();
    let mut seen = BTreeSet::new();
    for e in &s.events {
        match e {
            SeqEvent::Message { from, to, .. } => {
                seen.extend([from.as_str(), to.as_str()]);
            }
            SeqEvent::Note { over, .. } => seen.extend(over.iter().take(2).map(String::as_str)),
        }
    }
    seen.difference(&declared).count()
}

fn check_text(t: &str) -> Result<(), TestCaseError> {
    if let Some(why) = payload_problem(t, true) {
        return Err(TestCaseError::fail(why));
    }
    let lead = t.trim_start();
    for p in ["wrap:", "nowrap:", ":wrap:", ":nowrap:"] {
        prop_assert!(!lead.starts_with(p), "wrap prefix survives in {t:?}");
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn export_sequence_is_structurally_safe(s in arbitrary_sequence()) {
        let out = export_sequence(&s);
        let lines: Vec<&str> = out.lines().collect();
        prop_assert_eq!(
            lines.len(),
            1 + s.actors.len() + undeclared(&s) + s.events.len(),
            "{}",
            out
        );
        prop_assert_eq!(lines[0], "sequenceDiagram");
        let participants = s.actors.len() + undeclared(&s);
        for (i, line) in lines[1..].iter().enumerate() {
            let t = line.trim_start();
            if i < participants {
                let Some((_, text)) = t.split_once(" as ") else {
                    return Err(TestCaseError::fail(format!("participant line {line:?}")));
                };
                prop_assert!(t.starts_with("participant p"), "{line:?}");
                check_text(text)?;
            } else {
                let digits = |r: &str| r.bytes().take_while(u8::is_ascii_digit).count();
                let head_ok = t.strip_prefix("p").is_some_and(|r| {
                    let n = digits(r);
                    n > 0 && (r[n..].starts_with("->>") || r[n..].starts_with("-->>"))
                });
                prop_assert!(head_ok || t.starts_with("Note over "), "{line:?}");
                let text = t.split_once(": ").map_or("", |(_, x)| x);
                check_text(text)?;
            }
        }
    }
}
