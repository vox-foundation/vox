//! ROADMAP SC3: the one corpus `sequenceDiagram` block goes Mermaid -> DSL -> IR -> SVG and back
//! to the same Mermaid text. Bless the SVG:
//! `UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_corpus)'`.

mod support;

use std::fmt::Write as _;
use std::fs;

use serde_json::to_value;
use support::roundtrip::{assert_golden, fixtures, stems};
use support::strip_spans;
use support::svgcheck::seq::assert_sequence_geometry;
use support::svgcheck::{self};
use support::theme::{Theme, pane_css};
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

/// Writes the light/dark contact sheet of the corpus sequence and the four `svg/cases/3x-seq-*`
/// goldens to `$CONTACT_SHEET_OUT` (panes differ only in the `--sl-*` token values).
#[test]
#[ignore = "one-shot generator: set CONTACT_SHEET_OUT"]
fn contact_sheet() {
    let Ok(out) = std::env::var("CONTACT_SHEET_OUT") else {
        panic!("set CONTACT_SHEET_OUT=<path>.html and run with --run-ignored only");
    };
    let mut cases = vec![(STEM.to_owned(), imported().source.unwrap_or_default())];
    for stem in stems(&fixtures().join("svg/cases"), "vdg") {
        if stem.contains("-seq-") {
            let path = fixtures().join(format!("svg/cases/{stem}.vdg"));
            cases.push((stem, fs::read_to_string(path).unwrap_or_default()));
        }
    }
    let mut html = String::from(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>VoxDiagram contact sheet: sequence diagrams</title>\n<style>\n\
         body { font-family: system-ui, sans-serif; margin: 16px; }\n\
         .row { display: flex; gap: 12px; margin-bottom: 28px; }\n\
         .vd-light, .vd-dark { flex: 1 1 0; min-width: 0; padding: 12px; overflow: auto;\n\
         \x20 background: var(--sl-color-black); color: var(--sl-color-white); border: 1px solid #888; }\n\
         .vd-light svg, .vd-dark svg { max-width: 900px; height: auto; }\n",
    );
    html.push_str(&pane_css(Theme::Light).replace(".pane-light", ".vd-light"));
    html.push_str(&pane_css(Theme::Dark).replace(".pane-dark", ".vd-dark"));
    html.push_str("</style>\n</head>\n<body>\n");
    for (i, (stem, src)) in cases.iter().enumerate() {
        let Some(ir) = check(src).ir else {
            panic!("{stem}: does not check");
        };
        let draw = |prefix: String| match render(
            &ir,
            &SvgOpts {
                id_prefix: prefix,
                title: None,
            },
        ) {
            Ok(svg) => svg,
            Err(e) => panic!("{stem}: {e}"),
        };
        let (light, dark) = (draw(format!("s{i}l")), draw(format!("s{i}d")));
        let _ = write!(
            html,
            "<h2>{stem}</h2>\n<div class=\"row\">\n<div class=\"vd-light\">\n{light}</div>\n\
             <div class=\"vd-dark\">\n{dark}</div>\n</div>\n"
        );
    }
    html.push_str("</body>\n</html>\n");
    if let Err(e) = fs::write(&out, html) {
        panic!("cannot write {out}: {e}");
    }
}
