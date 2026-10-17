use super::*;
use crate::layout::MARGIN;
use crate::sequence_ir::Actor;

fn actor(id: &str) -> Actor {
    Actor {
        id: id.into(),
        label: id.to_uppercase(),
        span: Default::default(),
    }
}

fn msg(from: &str, to: &str, text: &str) -> SeqEvent {
    SeqEvent::Message {
        from: from.into(),
        to: to.into(),
        text: text.into(),
        dashed: false,
        span: Default::default(),
    }
}

fn seq(ids: &[&str], events: Vec<SeqEvent>) -> SequenceIr {
    SequenceIr {
        actors: ids.iter().map(|i| actor(i)).collect(),
        events,
    }
}

fn placed(s: &SequenceIr) -> SeqPlaced {
    match place(s) {
        Ok(p) => p,
        Err(e) => panic!("place failed: {e}"),
    }
}

#[test]
fn a_gap_fits_its_crossing_label_with_clearance() {
    let text = "a fairly long message label that needs room";
    let lbl = label_box(text).map_or(0, |l| l.w);
    let p = placed(&seq(&["a", "b"], vec![msg("a", "b", text)]));
    let gap = W::from(p.lifelines[1].0 - p.lifelines[0].0);
    assert!(gap >= lbl + 2 * LANE_CLEAR, "{gap} {lbl}");
}

#[test]
fn a_label_across_two_lanes_widens_both_gaps() {
    let text = "x".repeat(70);
    let lbl = label_box(&text).map_or(0, |l| l.w);
    let p = placed(&seq(&["a", "b", "c"], vec![msg("a", "c", &text)]));
    let xs: Vec<W> = p.lifelines.iter().map(|l| W::from(l.0)).collect();
    assert!(xs[1] - xs[0] >= lbl + 2 * LANE_CLEAR);
    assert!(xs[2] - xs[1] >= lbl + 2 * LANE_CLEAR);
}

#[test]
fn everything_placed_lies_inside_the_margins() {
    let ir = seq(
        &["a", "b"],
        vec![
            msg("a", "b", "hello there"),
            msg("b", "b", "retry"),
            msg("b", "a", ""),
            SeqEvent::Note {
                over: vec!["b".into(), "a".into()],
                text: "spans both".into(),
                span: Default::default(),
            },
        ],
    );
    let p = placed(&ir);
    let (lo, hi_x, hi_y) = (MARGIN, p.width - MARGIN, p.height - MARGIN);
    let inside = |r: Rect| r.x >= lo && r.y >= lo && r.x + r.w <= hi_x && r.y + r.h <= hi_y;
    for (_, r) in &p.actors {
        assert!(inside(*r), "{r:?}");
    }
    for &(x, top, bottom) in &p.lifelines {
        assert!(x >= lo && x <= hi_x && top >= lo && bottom <= hi_y);
    }
    for e in &p.events {
        match e {
            SeqDrawn::Message { points, label, .. } => {
                assert!(
                    points
                        .iter()
                        .all(|&(x, y)| x >= lo && x <= hi_x && y >= lo && y <= hi_y)
                );
                assert!(label.as_ref().is_none_or(|(_, r)| inside(*r)));
            }
            SeqDrawn::Note { rect, .. } => assert!(inside(*rect)),
        }
    }
    // The tightest element sits exactly on the margin.
    assert_eq!(p.actors.iter().map(|(_, r)| r.y).min(), Some(lo));
}

#[test]
fn an_empty_diagram_is_a_bare_canvas() {
    let p = placed(&seq(&[], Vec::new()));
    assert_eq!((p.width, p.height), (2 * MARGIN, 2 * MARGIN));
}

#[test]
fn an_unknown_actor_is_invalid_ir() {
    let r = place(&seq(&["a"], vec![msg("a", "zz", "x")]));
    assert!(matches!(r, Err(RenderError::InvalidIr(m)) if m == "event 0: unknown actor"));
}

#[test]
fn a_huge_canvas_is_rejected_not_wrapped() {
    let wide = "w".repeat(2_000_000);
    let r = place(&seq(&["a", "b"], vec![msg("a", "b", &wide)]));
    assert!(matches!(r, Err(RenderError::Geometry(_))), "{r:?}");
}
