//! Phase 90 review WR-01: a container title sits right above its content. The engine left the
//! title a rank (top-down) or a node gap (sideways) above it, a 39 to 70 px hole against the 8 px
//! padding at the bottom, which the compactness oracle could not see because it counts the title
//! as content.

use super::containers::laid;
use super::*;
use crate::layout::verify::{crosses, encloses, overlap};

/// The most a title may sit above its content: [`finish::TITLE_GAP`], unless it cannot move down
/// by one [`finish::SNAP_STEP`] without touching an edge, a label, a node or a foreign frame.
const MAX_GAP: Half = finish::TITLE_GAP;

/// Every imported corpus graph and the container cases, the sideways ones also drawn leftward.
fn sources() -> Vec<(String, String)> {
    // Tests run from the package root; `env!` is off limits under `layout/` (determinism guard).
    let fixtures = std::path::Path::new("tests/fixtures");
    let mut out = Vec::new();
    let mut corpus: Vec<_> = std::fs::read_dir(fixtures.join("corpus"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
        .collect();
    corpus.sort();
    for p in corpus {
        let mmd = std::fs::read_to_string(&p).unwrap_or_default();
        if let Some(src) =
            crate::mermaid::import(&mmd, crate::mermaid::ImportOpts::default()).source
        {
            out.push((p.display().to_string(), src));
        }
    }
    for case in ["20-container", "21-nested", "22-stress-nested"] {
        let path = fixtures.join(format!("svg/cases/{case}.vdg"));
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        if src.contains("direction: right") {
            out.push((
                format!("{case} leftward"),
                src.replace("direction: right", "direction: left"),
            ));
        }
        out.push((case.to_owned(), src));
    }
    out
}

/// Whether `r` touches an edge segment, an edge label, a node or a frame not holding container `k`.
fn blocked(input: &LayoutInput, p: &Placed, k: usize, r: Rect) -> bool {
    p.edges.iter().any(|e| {
        e.label.is_some_and(|l| overlap(l, r))
            || e.points.windows(2).any(|s| crosses(s[0], s[1], r))
    }) || p.nodes.iter().any(|n| overlap(*n, r))
        || p.containers
            .iter()
            .enumerate()
            .any(|(j, c)| !encloses(input, j, Some(k)) && overlap(c.rect, r))
}

#[test]
fn a_title_hugs_its_content() {
    let (mut frames, mut hugging, mut defects) = (0, 0, Vec::new());
    for (name, src) in sources() {
        let Some(Ir::Graph(g)) = crate::check(&src).ir else {
            continue;
        };
        if g.containers.is_empty() {
            continue;
        }
        let (input, p) = laid(&src);
        for (k, c) in p.containers.iter().enumerate() {
            let held = |n: usize| encloses(&input, k, input.nodes[n].container);
            let own = input
                .nodes
                .iter()
                .zip(&p.nodes)
                .filter(|(n, _)| n.container == Some(k))
                .map(|(_, r)| r.y);
            let subs = input
                .containers
                .iter()
                .zip(&p.containers)
                .filter(|(x, _)| x.parent == Some(k))
                .map(|(_, x)| x.rect.y);
            let inner = input
                .edges
                .iter()
                .zip(&p.edges)
                .filter(|(e, _)| held(e.from) && held(e.to))
                .filter_map(|(_, e)| e.label.map(|l| l.y));
            let Some(top) = own.chain(subs).chain(inner).min() else {
                continue;
            };
            frames += 1;
            let gap = top - (c.title.y + c.title.h);
            let lower = Rect {
                y: c.title.y + finish::SNAP_STEP,
                ..c.title
            };
            if gap <= MAX_GAP {
                hugging += 1;
            } else if !blocked(&input, &p, k, lower) {
                defects.push(format!(
                    "{name}: container {k} title sits {gap} half-px above its content"
                ));
            }
        }
    }
    println!("{frames} frames, {hugging} titles hug their content");
    assert!(frames >= 12, "only {frames} container frames read");
    assert!(defects.is_empty(), "{}", defects.join("\n"));
}
