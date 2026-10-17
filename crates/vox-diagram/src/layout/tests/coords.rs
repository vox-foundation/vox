//! Coordinate-bound regressions: huge engine output must fail closed, never overflow drawing.

use super::*;

/// One diamond of `size` px square with a self-loop whose end point sits at `reach` px.
struct BigDiamond {
    size: f64,
    reach: f64,
}

impl Engine for BigDiamond {
    fn solve(&self, _: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        let half = self.size / 2.0;
        Ok(RawLayout {
            nodes: vec![RawBox {
                cx: Some(half),
                cy: Some(half),
                w: Some(self.size),
                h: Some(self.size),
            }],
            containers: vec![],
            edges: vec![RawEdge {
                points: vec![(self.reach, half), (half, half)],
                label: None,
            }],
        })
    }
}

fn diamond_loop() -> Ir {
    let Ir::Graph(mut g) = graph_ir();
    g.nodes[0].shape = Shape::Diamond;
    g.edges.push(crate::ir::IrEdge {
        from: "a".into(),
        to: "a".into(),
        label: None,
        span: Default::default(),
    });
    Ir::Graph(g)
}

#[test]
fn a_huge_engine_coordinate_cannot_overflow_the_diamond_clip() {
    // 5e6 px passed the old 1e7 bound and made `dx * w * h` overflow i64 in `clip_diamond`.
    let huge = BigDiamond {
        size: 5.0e6,
        reach: 9.0e6,
    };
    assert_eq!(
        render_with(&huge, &diamond_loop(), &SvgOpts::default()),
        Err(RenderError::Layout("non-finite coordinate"))
    );
}

#[test]
fn the_largest_accepted_coordinates_still_clip_safely() {
    let near = BigDiamond {
        size: 9.0e4,
        reach: 9.9e4,
    };
    let svg = render_with(&near, &diamond_loop(), &SvgOpts::default());
    assert!(svg.is_ok(), "{svg:?}");
}
