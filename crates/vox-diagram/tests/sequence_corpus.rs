//! ROADMAP SC3: the one corpus `sequenceDiagram` block goes Mermaid -> DSL -> IR -> SVG and back
//! to the same Mermaid text. Bless the SVG:
//! `UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_corpus)'`.

mod support;

use std::fs;

use serde_json::to_value;
use support::roundtrip::{assert_golden, fixtures, stems};
use support::strip_spans;
use support::svgcheck::seq::assert_sequence_geometry;
use support::svgcheck::{self};
use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{ImportOpts, ImportReport, export, import};
use vox_diagram::svg::{SvgOpts, render};
use vox_diagram::{check, fmt};

const STEM: &str = "19-scientia-automated-research-historical-extension-research-2026-l112";

fn imported() -> ImportReport {
    let path = fixtures().join(format!("corpus/{STEM}.mmd"));
    let Ok(block) = fs::read_to_string(&path) else {
        panic!("cannot read {}", path.display());
    };
    import(&block, ImportOpts::default())
}

#[test]
fn tracer_corpus_sequence_round_trips() {
    let r = imported();
    assert!(r.unsupported.is_empty(), "{:?}", r.unsupported);
    let (Some(source), Some(seq)) = (r.source, r.sequence) else {
        panic!("sequence block imported without source or IR");
    };
    assert_eq!(fmt(&source).as_deref(), Some(source.as_str()));
    let Some(Ir::Sequence(d)) = check(&source).ir else {
        panic!("imported DSL does not check as a sequence:\n{source}");
    };
    let spans = |s: &_| strip_spans(to_value(s).unwrap_or_default());
    assert_eq!(spans(&d), spans(&seq));
    assert_eq!((d.actors.len(), d.events.len()), (8, 9));

    let ir = Ir::Sequence(d.clone());
    let opts = SvgOpts {
        id_prefix: "vd-seq".into(),
        title: None,
    };
    let svg = match render(&ir, &opts) {
        Ok(s) => s,
        Err(e) => panic!("render failed: {e}"),
    };
    let read = match svgcheck::check(&svg, "vd-seq") {
        Ok(read) => read,
        Err(e) => panic!("svg outside the allowlist: {e}"),
    };
    assert_eq!(assert_sequence_geometry(&read, &d), Ok(()));
    assert_golden(
        &fixtures().join(format!("svg/sequence-corpus/{STEM}.svg")),
        &svg,
    );

    let want_path = fixtures().join(format!("corpus-roundtrip/{STEM}.export.mmd"));
    let Ok(want) = fs::read_to_string(&want_path) else {
        panic!("cannot read {}", want_path.display());
    };
    assert_eq!(export(&ir), want);
}

#[test]
fn imported_source_is_canonical_dsl() {
    let Some(source) = imported().source else {
        panic!("no source");
    };
    assert!(
        source.starts_with("sequence {\n    actor Agent: \"CodingAgent\"\n"),
        "{source}"
    );
    assert!(
        source
            .lines()
            .any(|l| l == "    note over RS, SC: \"FindingCandidate mesh emission not wired\""),
        "{source}"
    );
    let dir = fixtures().join("svg/sequence-corpus");
    assert_eq!(stems(&dir, "svg"), vec![STEM.to_owned()]);
}
