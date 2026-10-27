//! Container geometry on real diagrams: frames hug their children (D3) and no edge strikes
//! through a title (D1). Fixtures are the nested golden case and the reference page's stress
//! fence (pinned as golden case 22).

use super::*;
use crate::layout::dagre_impl::DagreEngine;
use crate::layout::verify::crosses;

const NESTED: &str = include_str!("../../../tests/fixtures/svg/cases/21-nested.vdg");
const STRESS: &str = include_str!("../../../tests/fixtures/svg/cases/22-stress-nested.vdg");
const SINGLE: &str = include_str!("../../../tests/fixtures/svg/cases/20-container.vdg");

/// Widest a frame may sit from the box around its children on any side: the 8 px padding plus
/// the title box's own text padding, with room to spare. The pre-fix layout left 70 to 120 px.
const MAX_GAP: Half = 48;

pub(super) fn laid(src: &str) -> (LayoutInput, Placed) {
    let Some(Ir::Graph(g)) = crate::check(src).ir else {
        panic!("not a graph: {src}");
    };
    let Ok(input) = input::build(&g) else {
        panic!("build failed");
    };
    match lay_out(&DagreEngine, &input) {
        Ok(p) => (input, p),
        Err(e) => panic!("lay_out failed: {e}"),
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    let (r, btm) = ((a.x + a.w).max(b.x + b.w), (a.y + a.h).max(b.y + b.h));
    Rect {
        x,
        y,
        w: r - x,
        h: btm - y,
    }
}

/// The title's text block: widest line by 16 px a line, centred in its area.
fn text_box(c: &PlacedContainer) -> Rect {
    let area = c.title;
    let w = c
        .title_lines
        .iter()
        .map(|l| text::measure(l))
        .max()
        .unwrap_or(0);
    let h = 32 * Half::try_from(c.title_lines.len()).unwrap_or(0);
    Rect {
        x: area.x + (area.w - w) / 2,
        y: area.y + (area.h - h) / 2,
        w,
        h,
    }
}

/// The box around everything container `k` holds: its title text, its own nodes, the frames of its
/// child containers and the edge labels drawn inside it.
fn children_box(input: &LayoutInput, p: &Placed, k: usize) -> Rect {
    let nodes = input
        .nodes
        .iter()
        .zip(&p.nodes)
        .filter(|(n, _)| n.container == Some(k))
        .map(|(_, r)| *r);
    let subs = input
        .containers
        .iter()
        .zip(&p.containers)
        .filter(|(c, _)| c.parent == Some(k))
        .map(|(_, c)| c.rect);
    let f = p.containers[k].rect;
    let labels = p
        .edges
        .iter()
        .filter_map(|e| e.label)
        .filter(|l| union(f, *l) == f);
    nodes
        .chain(subs)
        .chain(labels)
        .fold(text_box(&p.containers[k]), union)
}

fn assert_compact(src: &str) {
    let (input, p) = laid(src);
    for (k, c) in p.containers.iter().enumerate() {
        let (f, b) = (c.rect, children_box(&input, &p, k));
        let gaps = [
            b.x - f.x,
            b.y - f.y,
            f.x + f.w - b.x - b.w,
            f.y + f.h - b.y - b.h,
        ];
        assert!(
            gaps.iter().all(|&g| (0..=MAX_GAP).contains(&g)),
            "container {k}: frame {f:?} sits {gaps:?} half-px from its children {b:?}"
        );
        // An empty container is its title and padding only; the gaps above already bound it.
        if b == text_box(c) {
            continue;
        }
        let (fa, ba) = (
            i64::from(f.w) * i64::from(f.h),
            i64::from(b.w) * i64::from(b.h),
        );
        assert!(
            fa <= 2 * ba,
            "container {k}: frame area {fa} over twice {ba}"
        );
    }
}

#[test]
fn nested_frames_hug_their_children() {
    assert_compact(NESTED);
    assert_compact(STRESS);
    assert_compact(SINGLE);
    assert_compact(&leftward(NESTED));
    assert_compact(&leftward(STRESS));
}

fn assert_titles_clear(src: &str) {
    let (_, p) = laid(src);
    for (k, c) in p.containers.iter().enumerate() {
        let t = text_box(c);
        for (e, edge) in p.edges.iter().enumerate() {
            for s in edge.points.windows(2) {
                assert!(
                    !crosses(s[0], s[1], t),
                    "edge {e} segment {s:?} strikes through the title of container {k} at {t:?}"
                );
            }
        }
    }
}

/// `src` drawn right-to-left; the nested fixtures are left-to-right.
fn leftward(src: &str) -> String {
    assert!(src.contains("direction: right"), "{src}");
    src.replace("direction: right", "direction: left")
}

#[test]
fn no_edge_strikes_through_a_container_title() {
    assert_titles_clear(NESTED);
    assert_titles_clear(STRESS);
    assert_titles_clear(SINGLE);
    assert_titles_clear(&leftward(NESTED));
    assert_titles_clear(&leftward(STRESS));
}

/// No regrowth: the pre-fix layout drew 21-nested at 1147 x 1247 px and the stress fence at
/// 1696 x 1482; 07b66a573 drew them at 1805 x 382.5 and 1284.5 x 536, with a title column of its
/// own in every container. Measured now: 1016.5 x 802 and 1392.5 x 446. The ceilings leave 10%.
#[test]
fn nested_figures_stay_compact() {
    for (src, ceiling_px2) in [(NESTED, 897_000), (STRESS, 683_000)] {
        for src in [src.to_owned(), leftward(src)] {
            let (_, p) = laid(&src);
            let area = i64::from(p.width) * i64::from(p.height) / 4;
            assert!(
                area <= ceiling_px2,
                "{} x {} half-px is {area} px2, over {ceiling_px2}",
                p.width,
                p.height
            );
        }
    }
}

/// Corpus 03 imported: three left-to-right container columns joined by one-way edges.
fn river(direction: &str) -> String {
    let mmd =
        include_str!("../../../tests/fixtures/corpus/03-axis-harness-river-design-2026-l83.mmd");
    let Some(src) = crate::mermaid::import(mmd, crate::mermaid::ImportOpts::default()).source
    else {
        panic!("corpus 03 does not import");
    };
    assert!(src.contains("direction: right"), "{src}");
    src.replace("direction: right", &format!("direction: {direction}"))
}

/// The top-level container holding node `n`, if any.
fn top_level(input: &LayoutInput, n: usize) -> Option<usize> {
    let mut k = input.nodes[n].container?;
    while let Some(p) = input.containers[k].parent {
        k = p;
    }
    Some(k)
}

/// Sideways, a container that only feeds another lies wholly before it along the flow. The
/// 07b66a573 layout ranked each title in a column of its own before the content; that column
/// shares the feeding container's last rank, so dagre stacked the two across the flow instead and
/// corpus 03 became a staircase (Controls spanned x 16..450, Store fed by it 280..823).
#[test]
fn sideways_containers_follow_the_flow() {
    for direction in ["right", "left"] {
        let (input, p) = laid(&river(direction));
        let pairs: Vec<(usize, usize)> = input
            .edges
            .iter()
            .filter_map(|e| Some((top_level(&input, e.from)?, top_level(&input, e.to)?)))
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(pairs.len(), 11, "{direction}: {pairs:?}");
        for &(a, b) in &pairs {
            assert!(
                !pairs.contains(&(b, a)),
                "{direction}: {a} and {b} feed each other"
            );
            let (up, down) = (p.containers[a].rect, p.containers[b].rect);
            let before = if direction == "right" {
                up.x + up.w <= down.x
            } else {
                down.x + down.w <= up.x
            };
            assert!(
                before,
                "{direction}: container {a} {up:?} feeds {b} {down:?} but is not before it"
            );
        }
    }
}

/// Corpus 03 sideways: frames hug, no edge crosses a title, and each title tops its frame.
#[test]
fn sideways_river_frames_hug_and_titles_top_them() {
    for direction in ["right", "left"] {
        let src = river(direction);
        assert_compact(&src);
        assert_titles_clear(&src);
        let (_, p) = laid(&src);
        for (k, c) in p.containers.iter().enumerate() {
            assert_eq!(
                c.title.y,
                c.rect.y + finish::CONTAINER_PAD,
                "{direction}: title {k} {:?} is not on top of {:?}",
                c.title,
                c.rect
            );
        }
    }
}

fn title(lines: &[&str]) -> LContainer {
    let lines: Vec<String> = lines.iter().map(|s| (*s).to_owned()).collect();
    let widest = lines.iter().map(|l| text::measure(l)).max().unwrap_or(0);
    LContainer {
        parent: None,
        depth: 1,
        title_w: widest + 40,
        title_h: finish::title_h(lines.len()),
        title_lines: lines,
    }
}

#[test]
fn an_empty_container_is_its_title_box_padded_8px() {
    use crate::layout::dagre_impl::DagreEngine;
    let mut input = one_node();
    input.nodes.clear();
    input.containers = vec![title(&["An empty container title"])];
    let Ok(p) = lay_out(&DagreEngine, &input) else {
        panic!("lay_out failed");
    };
    let (c, t) = (&p.containers[0], p.containers[0].title);
    assert_eq!(
        (t.w, t.h),
        (input.containers[0].title_w, finish::title_h(1))
    );
    assert_eq!(
        c.rect,
        Rect {
            x: t.x - 16,
            y: t.y - 16,
            w: t.w + 32,
            h: t.h + 32
        }
    );
}

fn at(cx: f64, cy: f64, w: f64, h: f64) -> RawBox {
    RawBox {
        cx: Some(cx),
        cy: Some(cy),
        w: Some(w),
        h: Some(h),
    }
}

/// A node on top of the title box on every rung.
struct TitleAlwaysCovered(Cell<usize>);

impl Engine for TitleAlwaysCovered {
    fn solve(&self, _: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.0.set(self.0.get() + 1);
        Ok(RawLayout {
            nodes: vec![at(100.0, 100.0, 64.0, 38.0)],
            titles: vec![at(100.0, 100.0, 20.0, 18.0)],
            edges: vec![],
        })
    }
}

#[test]
fn a_node_on_a_title_on_every_rung_is_a_geometry_error() {
    let mut input = one_node();
    input.nodes[0].container = Some(0);
    input.containers = vec![title(&["T"])];
    let e = TitleAlwaysCovered(Cell::new(0));
    let r = lay_out(&e, &input);
    assert!(
        matches!(&r, Err(RenderError::Geometry(m)) if m.contains("TitleBand")),
        "{r:?}"
    );
    assert_eq!(e.0.get(), 3);
}

/// Two nodes and an empty container's title between them; the edge runs through the title on the
/// first `crossed` rungs and around it after.
struct EdgeThroughTitle {
    crossed: usize,
    calls: Cell<usize>,
}

impl Engine for EdgeThroughTitle {
    fn solve(&self, _: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.calls.set(self.calls.get() + 1);
        let via = if self.calls.get() <= self.crossed {
            100.0
        } else {
            160.0
        };
        Ok(RawLayout {
            nodes: vec![at(100.0, 30.0, 64.0, 38.0), at(100.0, 170.0, 64.0, 38.0)],
            titles: vec![at(100.0, 100.0, 40.0, 18.0)],
            edges: vec![RawEdge {
                points: vec![(100.0, 49.0), (via, 100.0), (100.0, 151.0)],
                label: None,
            }],
        })
    }
}

#[test]
fn an_edge_through_a_title_escalates_the_rung_and_the_last_resort_is_the_first_rung() {
    let mut input = two_nodes();
    input.edges = vec![LEdge {
        from: 0,
        to: 1,
        label: None,
    }];
    input.containers = vec![title(&["T"])];
    let once = EdgeThroughTitle {
        crossed: 1,
        calls: Cell::new(0),
    };
    let Ok(p) = lay_out(&once, &input) else {
        panic!("lay_out failed");
    };
    assert_eq!(once.calls.get(), 2);
    assert_eq!(verify::title_crossing(&p), Ok(()));
    let always = EdgeThroughTitle {
        crossed: 3,
        calls: Cell::new(0),
    };
    let Ok(p) = lay_out(&always, &input) else {
        panic!("an edge through a title must not reject the diagram");
    };
    assert_eq!(always.calls.get(), 3);
    assert_eq!(
        verify::title_crossing(&p),
        Err(verify::Violation::EdgeThroughTitle(0, 0))
    );
}

#[test]
fn the_stress_fixture_is_the_reference_page_fence() {
    let page = include_str!("../../../../../docs/src/reference/vox-diagram.md");
    assert!(
        page.contains(&format!("```vox-diagram\n{STRESS}```")),
        "docs/src/reference/vox-diagram.md no longer carries 22-stress-nested.vdg verbatim"
    );
}
