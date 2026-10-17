use super::*;

#[test]
fn quantize_rejects_missing_and_non_finite() {
    assert_eq!(quantize(None), Err(LayoutError::Missing));
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0e7, -1.0e7] {
        assert_eq!(quantize(Some(v)), Err(LayoutError::NonFinite), "{v}");
    }
}

#[test]
fn quantize_rounds_to_half_pixels_without_negative_zero() {
    assert_eq!(quantize(Some(1.24)), Ok(2));
    assert_eq!(quantize(Some(1.25)), Ok(3));
    assert_eq!(quantize(Some(-0.2)), Ok(0));
    assert_eq!(quantize(Some(9_999_999.0)), Ok(19_999_998));
}

#[test]
fn guarded_catches_a_panic() {
    let r: Result<(), LayoutError> = guarded(|| panic!("x"));
    assert_eq!(r, Err(LayoutError::EnginePanic));
    assert_eq!(guarded(|| 7), Ok(7));
}

/// Engine double returning a canned result.
struct Canned(Result<RawLayout, LayoutError>);

impl Engine for Canned {
    fn solve(&self, _: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.0.clone()
    }
}

struct Panicky;

impl Engine for Panicky {
    fn solve(&self, _: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        panic!("boom")
    }
}

fn one_node() -> LayoutInput {
    LayoutInput {
        direction: Direction::Down,
        nodes: vec![LNode {
            shape: Shape::Rect,
            lines: vec!["A".into()],
            w: 128,
            h: 76,
        }],
        edges: vec![],
    }
}

fn boxed(cx: Option<f64>) -> RawLayout {
    RawLayout {
        nodes: vec![RawBox {
            cx,
            cy: Some(19.0),
            w: Some(64.0),
            h: Some(38.0),
        }],
        edges: vec![],
    }
}

#[test]
fn lay_out_places_at_the_margin() {
    let engine = Canned(Ok(boxed(Some(100.0))));
    let Ok(p) = lay_out(&engine, &one_node()) else {
        panic!("lay_out failed");
    };
    assert_eq!(
        p.nodes,
        vec![Rect {
            x: 32,
            y: 32,
            w: 128,
            h: 76
        }]
    );
    assert_eq!((p.width, p.height), (192, 140));
}

#[test]
fn lay_out_fails_closed_on_engine_misbehaviour() {
    let nan = Canned(Ok(boxed(Some(f64::NAN))));
    assert_eq!(
        lay_out(&nan, &one_node()),
        Err(RenderError::Layout("non-finite coordinate"))
    );
    let missing = Canned(Ok(boxed(None)));
    assert_eq!(
        lay_out(&missing, &one_node()),
        Err(RenderError::Layout("missing coordinate"))
    );
    assert_eq!(
        lay_out(&Panicky, &one_node()),
        Err(RenderError::Layout("engine panic"))
    );
    let short = Canned(Ok(RawLayout::default()));
    assert_eq!(
        lay_out(&short, &one_node()),
        Err(RenderError::Layout("missing coordinate"))
    );
}
