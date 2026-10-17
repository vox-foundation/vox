//! Mermaid import/export tracer and fixture round trips. Rust writes the goldens under
//! `tests/fixtures/import-cases-roundtrip/`; the Node oracle in `docs-astro` reads them.

mod support;

use std::fs;

use std::path::Path;
use support::roundtrip::{assert_golden, canon_graph, canon_sequence, fixtures, sets_json, stems};
use vox_diagram::ir::Ir;
use vox_diagram::limits::MAX_SOURCE_BYTES;

use vox_diagram::mermaid::{ImportOpts, Unsupported, export, export_sequence, import};
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

/// Outcome of one `roundtrip_dir` pass.
struct Round {
    /// Stems that imported and have goldens.
    done: Vec<String>,
    /// Stems that wrote a `.vdg` (graph and state blocks, not sequences).
    with_vdg: usize,
    /// `(stem, unsupported)` for every refused block.
    refused: Vec<(String, Vec<Unsupported>)>,
}

/// Import every `<input>/*.mmd`; assert the goldens under `output` (`.vdg`, `.export.mmd`,
/// `.sets.json`, and `unsupported.json` only when something was refused) and reject orphans.
fn roundtrip_dir(input: &Path, output: &Path) -> Round {
    let inputs = stems(input, "mmd");
    assert!(!inputs.is_empty(), "no fixtures in {}", input.display());
    let mut round = Round {
        done: Vec::new(),
        with_vdg: 0,
        refused: Vec::new(),
    };
    for stem in &inputs {
        let path = input.join(format!("{stem}.mmd"));
        let Ok(block) = fs::read_to_string(&path) else {
            panic!("cannot read {}", path.display());
        };
        let report = import(&block, ImportOpts::default());
        if !report.unsupported.is_empty() {
            round.refused.push((stem.clone(), report.unsupported));
            continue;
        }
        let out = |ext: &str| output.join(format!("{stem}.{ext}"));
        if let Some(seq) = report.sequence {
            assert_golden(&out("export.mmd"), &export_sequence(&seq));
            assert_golden(&out("sets.json"), &sets_json(&canon_sequence(&seq)));
        } else {
            let Some(src) = report.source else {
                panic!("{stem}: no source and no sequence");
            };
            assert_eq!(
                fmt(&src).as_deref(),
                Some(src.as_str()),
                "{stem}: not a fmt fixed point"
            );
            let Some(Ir::Graph(g)) = check(&src).ir else {
                panic!("{stem}: imported source does not check");
            };
            assert_golden(&out("vdg"), &src);
            assert_golden(&out("export.mmd"), &export(&Ir::Graph(g.clone())));
            assert_golden(&out("sets.json"), &sets_json(&canon_graph(&g)));
            round.with_vdg += 1;
        }
        round.done.push(stem.clone());
    }
    let list = output.join("unsupported.json");
    if round.refused.is_empty() {
        assert!(!list.exists(), "orphan {}", list.display());
    } else {
        let entries: Vec<serde_json::Value> = round
            .refused
            .iter()
            .map(|(file, u)| serde_json::json!({"file": file, "unsupported": u}))
            .collect();
        assert_golden(&list, &sets_json(&serde_json::Value::Array(entries)));
    }
    for ext in ["vdg", "mmd", "json"] {
        for stem in stems(output, ext) {
            let base = stem.trim_end_matches(".export").trim_end_matches(".sets");
            let ok =
                round.done.iter().any(|d| d == base) || (stem == "unsupported" && ext == "json");
            assert!(ok, "orphan golden {stem}.{ext}");
        }
    }
    round
}

#[test]
fn import_cases_roundtrip() {
    let root = fixtures();
    let round = roundtrip_dir(
        &root.join("import-cases"),
        &root.join("import-cases-roundtrip"),
    );
    assert!(round.refused.is_empty(), "refused: {:?}", round.refused);
}

/// ROADMAP SC3: 31 of the 33 frozen blocks round-trip; the gantt block and the markdown-string
/// block are the only refusals.
#[test]
fn corpus_roundtrip() {
    let root = fixtures();
    let round = roundtrip_dir(&root.join("corpus"), &root.join("corpus-roundtrip"));
    assert_eq!(round.done.len(), 31, "round-tripped: {:?}", round.done);
    assert_eq!(round.with_vdg, 30, "sequence blocks have no .vdg");
    let manifest = fs::read_to_string(root.join("corpus/MANIFEST.json")).unwrap_or_default();
    let Ok(serde_json::Value::Array(entries)) = serde_json::from_str(&manifest) else {
        panic!("corpus/MANIFEST.json unreadable");
    };
    let key = |stem: &str| -> (String, u64) {
        let want = format!("{stem}.mmd");
        let e = entries.iter().find(|e| e["fixture"] == want.as_str());
        let Some(e) = e else {
            panic!("{stem} not in the manifest");
        };
        (
            e["source"].as_str().unwrap_or_default().to_owned(),
            e["open_line"].as_u64().unwrap_or(0),
        )
    };
    // One construct name per refused block (the markdown-string block lists three lines).
    let mut got: Vec<((String, u64), &str)> = round
        .refused
        .iter()
        .map(|(stem, u)| {
            let first = u[0].construct.as_str();
            assert!(u.iter().all(|x| x.construct == first), "{stem}: {u:?}");
            (key(stem), first)
        })
        .collect();
    got.sort();
    let gantt = "docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md";
    let md = "docs/src/how-to/how-to-rust-crate-imports.md";
    let want = vec![
        ((gantt.to_owned(), 485), "header:gantt"),
        ((md.to_owned(), 53), "markdown-string"),
    ];
    assert_eq!(got, want);
}
