//! Totality of `mermaid::import`: on arbitrary text and on Mermaid-fragment soup it never
//! panics, keeps the `ImportReport` invariant, lists entries in order without duplicates and
//! never reports `internal` (that would mean the importer emitted DSL that `fmt` refused).

mod support;

use proptest::prelude::*;
use serde_json::to_value;
use support::strip_spans;
use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{ImportOpts, import};
use vox_diagram::sequence_ir::{SeqEvent, SequenceIr};
use vox_diagram::{check, fmt};

const FRAGMENTS: [&str; 80] = [
    "flowchart TD\n",
    "graph LR\n",
    "stateDiagram-v2\n",
    "sequenceDiagram\n",
    "[*] --> A",
    "A --> [*]",
    "A --> B: t",
    "A : d",
    "state X {",
    "<<fork>>",
    "participant A as Alice",
    "participant B",
    "A->>B: hi",
    "B-->>A: ",
    "Note over A,B: n",
    "Note left of A: n",
    "loop x",
    "activate A",
    "A-)B: x",
    "accTitle: t",
    "wrap: x",
    "#59;",
    "subgraph s",
    "subgraph t [Title]",
    "end",
    "direction LR",
    "A",
    "B[x]",
    "C(y)",
    "D{z}",
    "E[(db)]",
    "F((o))",
    "G(((e)))",
    "H[\"q\"]",
    "[\"`md`\"]",
    "[a <b> c]",
    "[R&amp;D]",
    "[\"#bogus;\"]",
    "[\"#13;\"]",
    "[\"#35;quot;\"]",
    "[a<br/>b]",
    "-->",
    "-->|t|",
    "-->|\"a|b\"|",
    "-- t -->",
    "-- \"t\" -->",
    "---",
    "==>",
    "-.->",
    "--o",
    "<-->",
    "~~~",
    " & ",
    ":::c",
    "%%",
    "%%{init}",
    ";",
    "style a fill:red",
    "class a b",
    "classDef x y",
    "click a b",
    "([",
    "[[",
    "{{",
    ">",
    "@{",
    "|",
    "\"",
    "\\",
    "\n",
    "\r\n",
    " ",
    "\u{e9}",
    "\u{202e}",
    "&#65;",
    "#",
    "-",
    "(",
    ")",
    "]",
];

fn soup() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(FRAGMENTS.to_vec()), 0..80).prop_map(|v| v.concat())
}

fn check_report(block: &str, opts: ImportOpts) -> Result<(), TestCaseError> {
    let r = import(block, opts);
    let lines = block.lines().count().max(1) as u32;
    prop_assert_eq!(r.unsupported.is_empty(), r.source.is_some(), "{:?}", r);
    if let Some(seq) = &r.sequence {
        let ids: Vec<&str> = seq.actors.iter().map(|a| a.id.as_str()).collect();
        let mut uniq = ids.clone();
        uniq.sort_unstable();
        uniq.dedup();
        prop_assert_eq!(uniq.len(), ids.len(), "duplicate actors {:?}", ids);
        for e in &seq.events {
            let (refs, span) = match e {
                SeqEvent::Message { from, to, span, .. } => (vec![from, to], span),
                SeqEvent::Note { over, span, .. } => (over.iter().collect(), span),
            };
            prop_assert!(
                refs.iter().all(|r| ids.contains(&r.as_str())),
                "undeclared actor in {:?}",
                e
            );
            prop_assert!(span.start <= span.end && span.end as usize <= block.len());
            prop_assert!((1..=lines).contains(&span.line), "{:?}", span);
        }
    }
    for list in [&r.unsupported, &r.dropped] {
        let keys: Vec<(u32, &str)> = list
            .iter()
            .map(|u| (u.line, u.construct.as_str()))
            .collect();
        prop_assert!(
            keys.windows(2).all(|w| w[0].0 <= w[1].0),
            "unsorted {keys:?}"
        );
        let mut uniq = keys.clone();
        uniq.sort_unstable();
        uniq.dedup();
        prop_assert_eq!(uniq.len(), keys.len(), "duplicates {:?}", keys);
        prop_assert!(
            keys.iter().all(|(l, _)| (1..=lines).contains(l)),
            "{keys:?}"
        );
    }
    prop_assert!(
        r.unsupported.iter().all(|u| u.construct != "internal"),
        "internal for {block:?}"
    );
    if let Some(src) = &r.source {
        let again = fmt(src);
        prop_assert_eq!(again.as_deref(), Some(src.as_str()), "{:?}", block);
        prop_assert!(check(src).valid, "{block:?}\n=>\n{src}");
        if let Some(seq) = &r.sequence {
            let Some(Ir::Sequence(d)) = check(src).ir else {
                return Err(TestCaseError::fail(format!("not a sequence: {src}")));
            };
            let bare = |s: &SequenceIr| strip_spans(to_value(s).unwrap_or_default());
            prop_assert_eq!(bare(&d), bare(seq), "{}", src);
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn import_is_total_on_arbitrary_text(s in "\\PC{0,400}") {
        check_report(&s, ImportOpts::default())?;
        check_report(&format!("flowchart TD\n{s}"), ImportOpts { drop_styling: true })?;
        for head in ["stateDiagram-v2", "sequenceDiagram"] {
            check_report(&format!("{head}\n{s}"), ImportOpts::default())?;
            check_report(&format!("{head}\n{s}"), ImportOpts { drop_styling: true })?;
        }
    }

    #[test]
    fn import_is_total_on_mermaid_fragment_soup(s in soup()) {
        check_report(&format!("flowchart LR\n{s}"), ImportOpts::default())?;
        check_report(&format!("stateDiagram-v2\n{s}"), ImportOpts::default())?;
        check_report(&format!("sequenceDiagram\n{s}"), ImportOpts::default())?;
        check_report(&s, ImportOpts { drop_styling: true })?;
    }
}
