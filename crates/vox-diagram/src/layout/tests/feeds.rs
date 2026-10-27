//! A container fed from outside lies wholly after its source along the flow. Corpus 18 (two
//! containers, edges only between them) drew the fed container's title in the rank of its source
//! nodes, so the frames overlapped across the flow: a staircase with seven edges bundled through
//! one channel.

use super::containers::laid;
use super::*;

const SCIENTIA: &str = include_str!(
    "../../../tests/fixtures/corpus-roundtrip/18-scientia-automated-research-historical-extension-research-2026-l50.vdg"
);

/// Container 0 (eras) feeds container 1 (vox); `direction` is `down` or `up`.
fn frames(direction: &str) -> (Rect, Rect) {
    let src = SCIENTIA.replacen(
        "graph {",
        &format!("graph {{\n    direction: {direction}"),
        1,
    );
    let (_, p) = laid(&src);
    (p.containers[0].rect, p.containers[1].rect)
}

#[test]
fn a_fed_container_lies_wholly_after_its_source_container() {
    let (eras, vox) = frames("down");
    assert!(eras.y + eras.h <= vox.y, "down: {eras:?} feeds {vox:?}");
    let (eras, vox) = frames("up");
    assert!(vox.y + vox.h <= eras.y, "up: {eras:?} feeds {vox:?}");
}

/// Phase 90 review WR-02: the outside edge enters through a nested container, so the outer
/// container's title had no feeder link, shared the feeder's rank and set the two frames side by
/// side (outer platform at x 340.5 beside the source stage, overlapping it along the flow).
#[test]
fn a_container_fed_through_a_nested_child_lies_after_its_source() {
    let src = r#"graph {
    container src: "Source stage" {
        node A: "Alpha source node"
        node B: "Beta source node"
    }
    container outer: "Outer platform" {
        container inner: "Inner services" {
            node C: "Gamma service"
            node D: "Delta service"
        }
    }
    src.A -> src.B
    src.B -> outer.inner.C
    src.A -> outer.inner.D
}"#;
    for direction in ["down", "up"] {
        let (_, p) = laid(&src.replacen(
            "graph {",
            &format!("graph {{\n    direction: {direction}"),
            1,
        ));
        let (from, to) = (p.containers[0].rect, p.containers[1].rect);
        let after = if direction == "down" {
            from.y + from.h <= to.y
        } else {
            to.y + to.h <= from.y
        };
        assert!(after, "{direction}: {from:?} feeds {to:?}");
    }
}
