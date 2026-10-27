//! Phase 85 go/no-go input: every importable corpus graph renders, the ten largest are pinned as
//! goldens, and an ignored generator writes the light/dark contact sheet.
//! Bless: `UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram -E 'binary(svg_corpus) & test(top10_goldens)'`.

mod support;

use std::fmt::Write as _;
use std::fs;

use support::clearance::clearance;
use support::roundtrip::{assert_golden, fixtures, stems};
use support::svgcheck::check as check_svg;
use support::theme::{Theme, pane_css};
use vox_diagram::check;
use vox_diagram::ir::{GraphIr, Ir};
use vox_diagram::mermaid::{ImportOpts, import};
use vox_diagram::svg::{SvgOpts, render};

/// Frozen top 10 by `nodes + edges` descending, stem ascending as tiebreak.
const TOP10: [&str; 10] = [
    "13-deep-research-waves-and-competitive-analysis-research-2026-l110",
    "04-deep-research-full-surface-audit-and-roadmap-2026-l46",
    "21-unified-workstream-execution-plan-2026-l45",
    "12-deep-research-waves-and-competitive-analysis-research-2026-l76",
    "26-vox-react-backend-interop-audit-2026-l51",
    "03-axis-harness-river-design-2026-l83",
    "01-ai-first-fixtures-research-2026-l147",
    "11-deep-research-waves-and-competitive-analysis-research-2026-l45",
    "20-tauri-convergence-migration-plan-2026-l117",
    "06-deep-research-full-surface-audit-and-roadmap-2026-l238",
];

/// `(stem, ir)` of every corpus block that imports to a graph, in stem order. Also asserts the
/// split of the 33 blocks: 30 graphs, one sequence, two refusals.
fn corpus_graphs() -> Vec<(String, Ir)> {
    let dir = fixtures().join("corpus");
    let all = stems(&dir, "mmd");
    assert_eq!(all.len(), 33, "frozen corpus size");
    let (mut graphs, mut sequences, mut refused) = (Vec::new(), 0, 0);
    for stem in all {
        let path = dir.join(format!("{stem}.mmd"));
        let Ok(block) = fs::read_to_string(&path) else {
            panic!("cannot read {}", path.display());
        };
        let r = import(&block, ImportOpts::default());
        if !r.unsupported.is_empty() {
            refused += 1;
        } else if r.sequence.is_some() {
            sequences += 1;
        } else if let Some(src) = r.source {
            let Some(ir) = check(&src).ir else {
                panic!("{stem}: imported source does not check");
            };
            graphs.push((stem, ir));
        } else {
            panic!("{stem}: no source, no sequence, nothing refused");
        }
    }
    assert_eq!((graphs.len(), sequences, refused), (30, 1, 2));
    graphs
}

fn graph(ir: &Ir) -> &GraphIr {
    let Ir::Graph(g) = ir else {
        panic!("expected a graph ir");
    };
    g
}

fn render_ok(stem: &str, ir: &Ir, prefix: &str) -> String {
    let opts = SvgOpts {
        id_prefix: prefix.to_owned(),
        title: None,
    };
    match render(ir, &opts) {
        Ok(s) => s,
        Err(e) => panic!("{stem}: render failed: {e}"),
    }
}

/// `(score, stem, ir)` ranked by `nodes + edges` descending, stem ascending.
fn ranked() -> Vec<(usize, String, Ir)> {
    let mut v: Vec<(usize, String, Ir)> = corpus_graphs()
        .into_iter()
        .map(|(stem, ir)| (graph(&ir).nodes.len() + graph(&ir).edges.len(), stem, ir))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    v
}

#[test]
fn corpus_graphs_all_render() {
    for (stem, ir) in corpus_graphs() {
        let svg = render_ok(&stem, &ir, "vd");
        if let Err(e) = check_svg(&svg, "vd") {
            panic!("{stem}: svg outside the allowlist: {e}");
        }
    }
}

/// Phase 90 review (batch 2): no node label word of up to 48 cells is split across lines; a long
/// identifier widens its box instead (Mermaid does the same).
#[test]
fn corpus_words_are_never_split() {
    let mut longest = 0;
    for (stem, ir) in corpus_graphs() {
        let svg = render_ok(&stem, &ir, "vd");
        let Ok(parsed) = check_svg(&svg, "vd") else {
            panic!("{stem}: svg outside the allowlist");
        };
        let shown: Vec<&str> = parsed
            .texts
            .iter()
            .flat_map(|(_, t)| t.split('\n'))
            .flat_map(str::split_whitespace)
            .collect();
        for node in &graph(&ir).nodes {
            for word in node
                .label
                .split_whitespace()
                .filter(|w| w.chars().count() <= 48)
            {
                longest = longest.max(word.chars().count());
                assert!(
                    shown.contains(&word),
                    "{stem}: word {word:?} of {:?} was split",
                    node.label
                );
            }
        }
    }
    assert!(
        longest > 28,
        "the corpus must exercise a word longer than one line"
    );
}

/// `(stem, ir)` of every graph case under `svg/cases` (sequence cases are skipped).
fn case_graphs() -> Vec<(String, Ir)> {
    let dir = fixtures().join("svg/cases");
    stems(&dir, "vdg")
        .into_iter()
        .filter_map(|stem| {
            let src = fs::read_to_string(dir.join(format!("{stem}.vdg"))).ok()?;
            let ir = check(&src).ir?;
            matches!(ir, Ir::Graph(_)).then_some((stem, ir))
        })
        .collect()
}

/// Phase 90 review CR-01: no edge runs through a node it does not join and none ends on a corner
/// of its own node. Corpus 18 drew the "1960s-80s" edge out of a box side, through the corner of
/// the "2018-2023" box beside it, so it read as leaving that box.
#[test]
fn corpus_edges_clear_foreign_nodes() {
    let graphs = corpus_graphs().into_iter().chain(case_graphs());
    let (mut defects, mut grazing, mut drawn) = (Vec::new(), 0, 0);
    for (stem, ir) in graphs {
        let svg = render_ok(&stem, &ir, "vd");
        let Ok(parsed) = check_svg(&svg, "vd") else {
            panic!("{stem}: svg outside the allowlist");
        };
        let c = clearance(graph(&ir), &parsed);
        drawn += 1;
        grazing += c.grazing.len();
        if !c.through.is_empty() || !c.corners.is_empty() {
            defects.push(format!(
                "{stem}: through {:?}, at a corner {:?}",
                c.through, c.corners
            ));
        }
    }
    println!(
        "{drawn} graphs: {} with a defect, {grazing} edges grazing a node",
        defects.len()
    );
    assert!(drawn > 40, "only {drawn} graphs read");
    assert!(defects.is_empty(), "{}", defects.join("\n"));
}

#[test]
fn corpus_top10_ranking() {
    let r = ranked();
    let got: Vec<&str> = r.iter().take(10).map(|(_, s, _)| s.as_str()).collect();
    assert_eq!(got, TOP10);
    assert_eq!((r[9].0, r[10].0), (20, 19), "tenth and eleventh scores");
}

#[test]
fn corpus_top10_goldens() {
    let dir = fixtures().join("svg/corpus");
    let ranked = ranked();
    for stem in TOP10 {
        let Some((_, _, ir)) = ranked.iter().find(|(_, s, _)| s == stem) else {
            panic!("{stem} is not an importable corpus graph");
        };
        assert_golden(&dir.join(format!("{stem}.svg")), &render_ok(stem, ir, "vd"));
    }
    let have = stems(&dir, "svg");
    let mut want: Vec<String> = TOP10.iter().map(|s| (*s).to_owned()).collect();
    want.sort();
    assert_eq!(have, want, "golden directory holds exactly the top 10");
}

/// `12.5` for 25 half-pixels, `12` for 24.
fn px(half: i64) -> String {
    if half % 2 == 0 {
        (half / 2).to_string()
    } else {
        format!("{}.5", half / 2)
    }
}

/// Writes the static contact sheet to `$CONTACT_SHEET_OUT`: each of the ten diagrams in a light
/// and a dark pane that differ only in the `--sl-*` token values.
#[test]
#[ignore = "one-shot generator: set CONTACT_SHEET_OUT"]
fn contact_sheet() {
    let Ok(out) = std::env::var("CONTACT_SHEET_OUT") else {
        panic!(
            "set CONTACT_SHEET_OUT: CONTACT_SHEET_OUT=<path>.html cargo nextest run -p vox-diagram --run-ignored only -E 'binary(svg_corpus) & test(contact_sheet)'"
        );
    };
    let mut html = String::from(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>VoxDiagram contact sheet: the 10 largest corpus graphs</title>\n<style>\n\
         body { font-family: system-ui, sans-serif; margin: 16px; }\n\
         .facts { color: #555; font-size: 13px; margin: 0 0 6px; }\n\
         .row { display: flex; gap: 12px; margin-bottom: 28px; }\n\
         .vd-light, .vd-dark { flex: 1 1 0; min-width: 0; padding: 12px; overflow: auto;\n\
         \x20 background: var(--sl-color-black); color: var(--sl-color-white); border: 1px solid #888; }\n\
         .vd-light svg, .vd-dark svg { max-width: 760px; height: auto; }\n",
    );
    html.push_str(&pane_css(Theme::Light).replace(".pane-light", ".vd-light"));
    html.push_str(&pane_css(Theme::Dark).replace(".pane-dark", ".vd-dark"));
    html.push_str("</style>\n</head>\n<body>\n");
    for (i, (score, stem, ir)) in ranked().into_iter().take(10).enumerate() {
        let (n, ln) = (i + 1, format!("s{:02}l", i + 1));
        let dn = format!("s{:02}d", i + 1);
        let light = render_ok(&stem, &ir, &ln);
        let dark = render_ok(&stem, &ir, &dn);
        assert_eq!(
            dark.replace(&format!("{dn}-"), &format!("{ln}-")),
            light,
            "{stem}: panes must differ only in the id prefix"
        );
        let Ok(svg) = check_svg(&light, &ln) else {
            panic!("{stem}: light svg fails the checker");
        };
        let g = graph(&ir);
        let scale = 152_000 / svg.width.max(1);
        let _ = write!(
            html,
            "<h2>{n}. {stem}</h2>\n<p class=\"facts\">{} nodes, {} edges, {} containers, \
             direction {}, viewBox {} x {} px, scale at 760 px column {}% (nodes+edges {score})</p>\n\
             <div class=\"row\">\n<div class=\"vd-light\">\n{light}</div>\n\
             <div class=\"vd-dark\">\n{dark}</div>\n</div>\n",
            g.nodes.len(),
            g.edges.len(),
            g.containers.len(),
            serde_json::to_value(g.direction)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            px(svg.width),
            px(svg.height),
            scale.min(100),
        );
    }
    html.push_str("</body>\n</html>\n");
    if let Err(e) = fs::write(&out, html) {
        panic!("cannot write {out}: {e}");
    }
}
