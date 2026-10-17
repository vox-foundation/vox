//! Mermaid import/export tracer and fixture round trips. Rust writes the goldens under
//! `tests/fixtures/import-cases-roundtrip/`; the Node oracle in `docs-astro` reads them.

mod support;

use std::fs;

use support::roundtrip::{assert_golden, canon_graph, fixtures, sets_json, stems};
use vox_diagram::ir::Ir;
use vox_diagram::limits::MAX_SOURCE_BYTES;
use vox_diagram::mermaid::{ImportOpts, Unsupported, export, import};
use vox_diagram::{check, fmt};

fn unsupported(line: u32, construct: &str) -> Unsupported {
    Unsupported {
        line,
        construct: construct.to_owned(),
    }
}

fn imported(block: &str) -> String {
    let r = import(block, ImportOpts::default());
    let Some(src) = r.source else {
        panic!("import refused {block:?}: {:?}", r.unsupported);
    };
    src
}

#[test]
fn tracer_minimal_flowchart_imports_to_canonical_dsl() {
    let r = import("flowchart TD\n    a --> b\n", ImportOpts::default());
    let want = "graph {\n    node a: \"a\"\n    node b: \"b\"\n    a -> b\n}\n";
    assert_eq!(r.source.as_deref(), Some(want));
    assert_eq!(r.sequence, None);
    assert!(r.unsupported.is_empty() && r.dropped.is_empty());
    assert_eq!(fmt(want).as_deref(), Some(want));
    assert!(check(want).valid);
}

#[test]
fn tracer_header_direction_maps() {
    let second = |block: &str| imported(block).lines().nth(1).map(str::to_owned);
    assert_eq!(
        second("flowchart LR\n    a --> b\n").as_deref(),
        Some("    direction: right")
    );
    assert_eq!(
        second("graph BT\n    a --> b\n").as_deref(),
        Some("    direction: up")
    );
    assert_eq!(
        second("flowchart RL\n    a --> b\n").as_deref(),
        Some("    direction: left")
    );
    for header in ["flowchart TB", "graph", "flowchart TD"] {
        let src = imported(&format!("{header}\n    a --> b\n"));
        assert!(!src.contains("direction:"), "{header}: {src}");
    }
}

#[test]
fn tracer_unreadable_line_fails_closed() {
    let r = import(
        "flowchart TD\n    a --> b\n    this is not mermaid\n",
        ImportOpts::default(),
    );
    assert_eq!((r.source, r.sequence), (None, None));
    assert_eq!(r.unsupported, vec![unsupported(3, "statement")]);
}

#[test]
fn tracer_gantt_header_is_unsupported() {
    let r = import("gantt\n    title Plan\n", ImportOpts::default());
    assert_eq!((r.source, r.sequence), (None, None));
    assert_eq!(r.unsupported, vec![unsupported(1, "header:gantt")]);
}

#[test]
fn tracer_oversized_block_is_gated() {
    let big = format!(
        "flowchart TD\n{}",
        "    a --> b\n".repeat(MAX_SOURCE_BYTES / 8)
    );
    assert!(big.len() > MAX_SOURCE_BYTES);
    let r = import(&big, ImportOpts::default());
    assert_eq!(r.source, None);
    assert_eq!(r.unsupported, vec![unsupported(1, "limit-source")]);
}

#[test]
fn tracer_minimal_export_text() {
    let src = imported("flowchart TD\n    a --> b\n");
    let Some(ir) = check(&src).ir else {
        panic!("imported source does not check");
    };
    assert_eq!(
        export(&ir),
        "flowchart TD\n    a[\"a\"]\n    b[\"b\"]\n    a --> b\n"
    );
}

#[test]
fn import_cases_roundtrip() {
    let root = fixtures();
    let inputs = stems(&root.join("import-cases"), "mmd");
    assert!(!inputs.is_empty(), "no import-cases fixtures");
    let out = root.join("import-cases-roundtrip");
    for stem in &inputs {
        let path = root.join(format!("import-cases/{stem}.mmd"));
        let Ok(block) = fs::read_to_string(&path) else {
            panic!("cannot read {}", path.display());
        };
        let report = import(&block, ImportOpts::default());
        assert!(
            report.unsupported.is_empty(),
            "{stem}: {:?}",
            report.unsupported
        );
        let Some(src) = report.source else {
            panic!("{stem}: no source");
        };
        assert_eq!(
            fmt(&src).as_deref(),
            Some(src.as_str()),
            "{stem}: not a fmt fixed point"
        );
        let Some(Ir::Graph(g)) = check(&src).ir else {
            panic!("{stem}: imported source does not check");
        };
        let ir = Ir::Graph(g.clone());
        assert_golden(&out.join(format!("{stem}.vdg")), &src);
        assert_golden(&out.join(format!("{stem}.export.mmd")), &export(&ir));
        assert_golden(
            &out.join(format!("{stem}.sets.json")),
            &sets_json(&canon_graph(&g)),
        );
    }
    // Every golden has a source input.
    for ext in ["vdg", "mmd", "json"] {
        for stem in stems(&out, ext) {
            let base = stem.trim_end_matches(".export").trim_end_matches(".sets");
            assert!(
                inputs.iter().any(|i| i == base),
                "orphan golden {stem}.{ext}"
            );
        }
    }
}
