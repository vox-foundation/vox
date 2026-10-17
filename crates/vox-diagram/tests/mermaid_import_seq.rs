//! sequenceDiagram import into `SequenceIr`: the accepted subset, text rules, participant
//! ordering rules and the fail-closed catalogue. `source` stays `None` for sequence blocks.

use vox_diagram::mermaid::{ImportOpts, import};
use vox_diagram::sequence_ir::{SeqEvent, SequenceIr};

fn seq(block: &str) -> SequenceIr {
    let r = import(block, ImportOpts::default());
    assert!(r.unsupported.is_empty(), "{r:?}");
    assert_eq!(r.source, None);
    let Some(s) = r.sequence else {
        panic!("not imported: {r:?}");
    };
    s
}

/// `(line, construct)` pairs of a refused block; asserts nothing was imported.
fn refused(block: &str) -> Vec<(u32, String)> {
    let r = import(block, ImportOpts::default());
    assert_eq!((&r.source, &r.sequence), (&None, &None), "{block}");
    r.unsupported
        .into_iter()
        .map(|u| (u.line, u.construct))
        .collect()
}

/// A `sequenceDiagram` whose body lines start at line 2, each refused with its construct.
fn each<L: AsRef<str>, C: AsRef<str>>(lines: &[L], constructs: &[C]) {
    assert_eq!(lines.len(), constructs.len());
    let body: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    let want: Vec<(u32, String)> = (2u32..)
        .zip(constructs.iter().map(|c| c.as_ref().to_owned()))
        .collect();
    assert_eq!(
        refused(&format!("sequenceDiagram\n{}\n", body.join("\n"))),
        want
    );
}

fn one(line: &str) -> String {
    let r = refused(&format!("sequenceDiagram\n{line}\n"));
    assert_eq!(r.len(), 1, "{line}: {r:?}");
    assert_eq!(r[0].0, 2, "{line}");
    r[0].1.clone()
}

#[test]
fn seq_basic() {
    let block = "sequenceDiagram\n    participant A as Alice\n    participant B\n    A->>B: hello\n    B-->>A: hi\n    Note over A,B: both\n    A->>C: implicit\n";
    let s = seq(block);
    let actors: Vec<(&str, &str, u32)> = s
        .actors
        .iter()
        .map(|a| (a.id.as_str(), a.label.as_str(), a.span.line))
        .collect();
    assert_eq!(
        actors,
        vec![("A", "Alice", 2), ("B", "B", 3), ("C", "C", 7)]
    );
    let lines: Vec<u32> = s
        .events
        .iter()
        .map(|e| match e {
            SeqEvent::Message { span, .. } | SeqEvent::Note { span, .. } => span.line,
        })
        .collect();
    assert_eq!(lines, vec![4, 5, 6, 7]);
    assert!(
        matches!(&s.events[0], SeqEvent::Message { from, to, text, dashed: false, .. }
        if (from.as_str(), to.as_str(), text.as_str()) == ("A", "B", "hello"))
    );
    assert!(
        matches!(&s.events[1], SeqEvent::Message { from, to, text, dashed: true, .. }
        if (from.as_str(), to.as_str(), text.as_str()) == ("B", "A", "hi"))
    );
    assert!(matches!(&s.events[2], SeqEvent::Note { over, text, .. }
        if over.as_slice() == ["A", "B"] && text == "both"));
    // The span is the statement's trimmed byte range: `A->>B: hello` starts at column 5.
    let SeqEvent::Message { span, .. } = &s.events[0] else {
        panic!("not a message");
    };
    assert_eq!(
        &block[span.start as usize..span.end as usize],
        "A->>B: hello"
    );
    assert_eq!((span.col, span.end_col), (5, 17));
}

#[test]
fn seq_arrow_spacing_note_case_and_single_actor_note() {
    let s = seq("sequenceDiagram\n  A ->> B : x\n  A-->>B:y\n  note over A: n\n");
    assert_eq!(s.events.len(), 3);
    assert!(matches!(&s.events[0], SeqEvent::Message { text, dashed: false, .. } if text == "x"));
    assert!(matches!(&s.events[2], SeqEvent::Note { over, .. } if over.as_slice() == ["A"]));
}

#[test]
fn seq_text_rules() {
    let t = |line: &str| match seq(&format!("sequenceDiagram\n{line}\n")).events.remove(0) {
        SeqEvent::Message { text, .. } | SeqEvent::Note { text, .. } => text,
    };
    assert_eq!(t("A->>B:"), "");
    assert_eq!(t("Note over A:"), "");
    assert_eq!(t("A->>B: say #quot;hi#quot;"), "say \"hi\"");
    assert_eq!(t("A->>B: a #59; b #119;rap: #96;x#96;"), "a ; b wrap: `x`");
    assert_eq!(
        t("A->>B: planner + synthesize (a: b)"),
        "planner + synthesize (a: b)"
    );
    assert_eq!(t("A->>B: a<br/>b"), "a\nb");
    assert_eq!(one("A->>B: wrap: x"), "wrap");
    assert_eq!(one("A->>B: :nowrap: x"), "wrap");
    assert_eq!(one("Note over A: wrap:x"), "wrap");
    assert_eq!(one("A->>B: a; b"), "semicolon");
    assert_eq!(one("A->>B"), "message-syntax");
    assert_eq!(one("A->>B C: x"), "message-syntax");
    assert_eq!(one("A->>B: a %% b"), "comment");
    assert_eq!(one("A->>B: <b>x"), "html-label");
    // An HTML entity always carries a raw `;`, which is reported first in free text.
    assert_eq!(one("A->>B: R&amp;D"), "semicolon");
    assert_eq!(one("A->>B: #bogus;"), "entity");
    assert_eq!(one("A->>B: a#13;b"), "forbidden-char");
    assert_eq!(
        one(&format!("A->>B: {}", "x".repeat(201))),
        "label-too-long"
    );
    assert_eq!(one("participant A as "), "empty-label");
    assert_eq!(one("participant A as wrap: x"), "wrap");
}

#[test]
fn seq_participant_order_rules() {
    assert_eq!(
        refused("sequenceDiagram\nA->>B: x\nparticipant A\n"),
        vec![(3, "participant-after-use".to_owned())]
    );
    assert_eq!(
        refused("sequenceDiagram\nparticipant A\nparticipant A as Again\n"),
        vec![(3, "redeclared-participant".to_owned())]
    );
    assert_eq!(
        refused("sequenceDiagram\nNote over A: n\nparticipant A\n"),
        vec![(3, "participant-after-use".to_owned())]
    );
    assert_eq!(one("participant 1a"), "invalid-id");
    assert_eq!(one("participant node"), "reserved-id");
    assert_eq!(one("A->>end: x"), "reserved-id");
    assert_eq!(one("participant"), "statement");
}

#[test]
fn seq_unsupported_catalogue() {
    each(
        &[
            "actor A",
            "create participant B",
            "destroy B",
            "activate A",
            "deactivate A",
            "A->>+B: x",
            "A->>-B: x",
            "A->B: x",
            "A-->B: x",
            "A -x B: x",
            "A --x B: x",
            "A-)B: x",
            "A--)B: x",
            "A<<->>B: x",
            "A<<-->>B: x",
        ],
        &[
            "actor",
            "lifecycle",
            "lifecycle",
            "activation",
            "activation",
            "activation",
            "activation",
            "arrow:->",
            "arrow:-->",
            "arrow:-x",
            "arrow:--x",
            "arrow:-)",
            "arrow:--)",
            "arrow:<<->>",
            "arrow:<<-->>",
        ],
    );
    let blocks = [
        "loop", "alt", "else", "opt", "par", "and", "critical", "option", "break", "rect", "end",
    ];
    let lines: Vec<String> = blocks.iter().map(|b| format!("{b} some text")).collect();
    let want: Vec<String> = blocks.iter().map(|b| format!("block:{b}")).collect();
    each(&lines, &want);
    each(
        &[
            "autonumber",
            "title Plan",
            "box Aqua group",
            "link A: Docs @ https://x",
            "links A: {}",
            "properties A: {}",
            "details A: {}",
            "Note left of A: n",
            "Note right of A: n",
            "Note over A,B,C: n",
            "%% comment",
            "%%{init: {}}%%",
            "accTitle: t",
            "accDescr: d",
            "what is this",
            "A",
        ],
        &[
            "autonumber",
            "title",
            "box",
            "links",
            "links",
            "links",
            "links",
            "note-placement",
            "note-placement",
            "note-actors",
            "comment",
            "directive",
            "styling:accTitle",
            "styling:accDescr",
            "statement",
            "statement",
        ],
    );
}

#[test]
fn seq_reports_every_bad_line_and_dedups() {
    let r = refused("sequenceDiagram\nactor A\nA->>B: ok\nloop x\nA->B: y\n");
    let want: Vec<(u32, String)> = [(2, "actor"), (4, "block:loop"), (5, "arrow:->")]
        .into_iter()
        .map(|(l, c)| (l, c.to_owned()))
        .collect();
    assert_eq!(r, want);
}

#[test]
fn seq_caps() {
    let many: String = (0..=500).map(|i| format!("participant P{i}\n")).collect();
    let got = refused(&format!("sequenceDiagram\n{many}"));
    assert_eq!(got, vec![(502, "limit-nodes".to_owned())]);
    let msgs = "A->>B: x\n".repeat(1_001);
    let got = refused(&format!("sequenceDiagram\n{msgs}"));
    assert_eq!(got, vec![(1_002, "limit-edges".to_owned())]);
    assert_eq!(
        seq(&format!("sequenceDiagram\n{}", "A->>B: x\n".repeat(1_000)))
            .events
            .len(),
        1_000
    );
}

#[test]
fn seq_header_forms() {
    assert_eq!(
        refused("sequenceDiagram;\nA->>B: x\n"),
        vec![(1, "semicolon".to_owned())]
    );
    assert_eq!(
        refused("sequenceDiagram TB\n"),
        vec![(1, "header:sequenceDiagram".to_owned())]
    );
    assert!(seq("sequenceDiagram\n").actors.is_empty());
}

#[test]
fn drop_styling_in_sequence() {
    let opts = ImportOpts { drop_styling: true };
    let r = import(
        "sequenceDiagram\naccTitle: t\nA->>B: x\naccDescr: d\n",
        opts,
    );
    assert!(r.unsupported.is_empty(), "{r:?}");
    assert!(r.sequence.is_some());
    let d: Vec<(u32, &str)> = r
        .dropped
        .iter()
        .map(|u| (u.line, u.construct.as_str()))
        .collect();
    assert_eq!(d, vec![(2, "styling:accTitle"), (4, "styling:accDescr")]);
}
