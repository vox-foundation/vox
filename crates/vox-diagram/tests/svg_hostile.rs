//! REND-03: hostile text can neither escape `<text>`/`<title>`/`<desc>` nor reach an attribute.
//! The oracle is `support::svgcheck`, a quick-xml allowlist checker independent of the writer,
//! plus a `sanitize` restated here (not read from `src`). The per-arm mutation proof of `esc`
//! is recorded in the 85-04 SUMMARY.

mod support;

use proptest::prelude::*;
use support::hostile::{FRAGMENTS, build, graph_cases};
use support::roundtrip::fixtures;
use support::svgcheck::{Svg, check};
use vox_diagram::ir::{Ir, Shape};
use vox_diagram::svg::{SvgOpts, render};

/// What the writer must turn a char into: the XML-illegal, C1, bidi and separator set.
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{0}'..='\u{8}'
            | '\u{b}'
            | '\u{c}'
            | '\u{e}'..='\u{1f}'
            | '\u{7f}'..='\u{9f}'
            | '\u{fffe}'
            | '\u{ffff}'
            | '\u{61c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
            | '\u{2028}'
            | '\u{2029}' => '\u{fffd}',
            c => c,
        })
        .collect()
}

/// Wrapping and XML line-end handling may move or rewrite these, nothing else.
fn squeeze(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
        .collect()
}

fn two_nodes(label: &str, edge_label: &str) -> Ir {
    let nodes = [
        ("a", label.to_owned(), Shape::Rect, None),
        ("b", "ok".to_owned(), Shape::Rect, None),
    ];
    Ir::Graph(build(&[], &nodes, &[(0, 1, Some(edge_label.to_owned()))]))
}

fn render_checked(ir: &Ir, opts: &SvgOpts) -> Svg {
    let svg = match render(ir, opts) {
        Ok(s) => s,
        Err(e) => panic!("render failed for {opts:?}: {e}"),
    };
    match check(&svg, &opts.id_prefix) {
        Ok(s) => s,
        Err(e) => panic!("{e}\n{svg}"),
    }
}

/// All decoded text of elements of `class`, joined.
fn text_of(svg: &Svg, class: &str) -> String {
    svg.texts
        .iter()
        .filter(|(c, _)| c == class)
        .map(|(_, t)| t.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_label_texts(svg: &Svg, f: &str, what: &str) {
    let want = squeeze(&sanitize(f));
    assert_eq!(
        squeeze(&text_of(svg, "vd-label")),
        format!("{want}ok"),
        "node label of {what}"
    );
    assert_eq!(
        squeeze(&text_of(svg, "vd-edge-label")),
        want,
        "edge label of {what}"
    );
    assert_eq!(squeeze(&text_of(svg, "title")), want, "title of {what}");
}

fn hostile_opts(f: &str) -> SvgOpts {
    SvgOpts {
        title: Some(f.to_owned()),
        ..SvgOpts::default()
    }
}

// ---- the checker itself ----------------------------------------------------------------

const BASE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\" role=\"img\" \
aria-labelledby=\"vd-title vd-desc\" class=\"vd-diagram\">\n<title id=\"vd-title\">T</title>\n\
<desc id=\"vd-desc\">D</desc>\n@@</svg>\n";

fn with(body: &str) -> String {
    BASE.replace("@@", body)
}

#[test]
fn checker_accepts_the_minimal_golden_and_the_base() {
    let path = fixtures().join("svg/cases/00-minimal.svg");
    let Ok(golden) = std::fs::read_to_string(&path) else {
        panic!("missing golden {}", path.display());
    };
    assert!(check(&golden, "vd").is_ok(), "{:?}", check(&golden, "vd"));
    assert!(check(&with(""), "vd").is_ok());
    assert!(check(&golden, "other").is_err(), "wrong prefix is rejected");
    let two_roots = format!("{}<g/>", with(""));
    match check(&two_roots, "vd") {
        Ok(_) => panic!("accepted a second root"),
        Err(e) => assert!(e.contains("second root"), "{e}"),
    }
}

#[test]
fn checker_rejects_each_class_of_unsafe_markup() {
    let cases: &[(&str, &str)] = &[
        ("<script>x</script>", "script"),
        ("<foreignObject/>", "foreignObject"),
        ("<a href=\"x\"/>", "element"),
        ("<g onload=\"x\"/>", "attribute"),
        ("<!-- c -->", "comment"),
        ("<![CDATA[x]]>", "CDATA"),
        ("<?pi x?>", "processing"),
        ("<g id=\"vd-title\"/>", "duplicate id"),
        ("<g id=\"other\"/>", "prefix"),
        (
            "<path d=\"M0 0L1 1\" marker-end=\"url(#vd-nope)\"/>",
            "unresolved",
        ),
        ("<text><tspan>&bogus;</tspan></text>", "entity"),
        ("<text><tspan>&#60;</tspan></text>", "entity"),
        ("<text><tspan>a>b</tspan></text>", "raw >"),
        ("<text><tspan>a]]>b</tspan></text>", "raw >"),
        ("<text><tspan>a\u{1}b</tspan></text>", "control"),
        ("<g>stray</g>", "text outside"),
        ("<g style=\"fill:url(#x)\"/>", "url("),
        ("<g style=\"behavior:x\"/>", "style property"),
        ("<g class=\"evil\"/>", "class"),
        ("<g></h>", "ill-formed"),
    ];
    for (body, needle) in cases {
        match check(&with(body), "vd") {
            Ok(_) => panic!("accepted {body:?}"),
            Err(e) => assert!(e.contains(needle), "{body:?}: {e:?} lacks {needle:?}"),
        }
    }
}

// ---- rendered output ---------------------------------------------------------------------

#[test]
fn hostile_fragments() {
    for f in FRAGMENTS {
        for text in [(*f).to_owned(), format!("x{f}y")] {
            let svg = render_checked(&two_nodes(&text, &text), &hostile_opts(&text));
            assert_label_texts(&svg, &text, &format!("{text:?}"));
        }
    }
}

#[test]
fn hostile_graph_cases() {
    for (name, g) in graph_cases() {
        let ir = Ir::Graph(g);
        render_checked(&ir, &SvgOpts::default());
        let opts = SvgOpts {
            id_prefix: "h-1".into(),
            ..SvgOpts::default()
        };
        let svg = render_checked(&ir, &opts);
        assert!(svg.ids.iter().all(|i| i.starts_with("h-1-")), "{name}");
    }
}

fn hostile_text() -> impl Strategy<Value = String> {
    let pick = prop_oneof![
        prop::sample::select(FRAGMENTS.to_vec()).prop_map(str::to_owned),
        any::<char>().prop_map(String::from),
    ];
    prop::collection::vec(pick, 1..=8).prop_map(|v| v.concat().chars().take(200).collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// A node, a labelled self-loop and the title carry the same hostile text.
    #[test]
    fn hostile_property(t in hostile_text()) {
        let nodes = [("a", t.clone(), Shape::Rect, None), ("b", "ok".to_owned(), Shape::Rect, None)];
        let ir = Ir::Graph(build(&[], &nodes, &[(0, 0, Some(t.clone()))]));
        let rendered = render(&ir, &hostile_opts(&t));
        prop_assert!(rendered.is_ok(), "{:?}", rendered);
        let svg = rendered.unwrap_or_default();
        let parsed = check(&svg, "vd");
        prop_assert!(parsed.is_ok(), "{:?}\n{}", parsed.as_ref().err(), svg);
        if let Ok(p) = parsed {
            let want = squeeze(&sanitize(&t));
            prop_assert_eq!(squeeze(&text_of(&p, "vd-label")), format!("{want}ok"));
            prop_assert_eq!(squeeze(&text_of(&p, "vd-edge-label")), want.clone());
            prop_assert_eq!(squeeze(&text_of(&p, "title")), want);
        }
    }
}

const MARK: &str = "QzQ7";

fn marked() -> Ir {
    let boxes = [("grp", MARK, None)];
    let nodes = [
        ("a", MARK.to_owned(), Shape::Rect, Some(0)),
        ("b", MARK.to_owned(), Shape::Diamond, Some(0)),
        ("c", MARK.to_owned(), Shape::Cylinder, None),
    ];
    let edges = [(0, 1, Some(MARK.to_owned())), (1, 2, Some(MARK.to_owned()))];
    Ir::Graph(build(&boxes, &nodes, &edges))
}

#[test]
fn no_user_text_in_attributes() {
    let opts = SvgOpts {
        title: Some(MARK.into()),
        ..SvgOpts::default()
    };
    let svg = render_checked(&marked(), &opts);
    assert!(
        svg.attr_values.iter().all(|v| !v.contains(MARK)),
        "{:?}",
        svg.attr_values
    );
    for class in ["vd-label", "vd-edge-label", "vd-container-label", "title"] {
        assert!(text_of(&svg, class).contains(MARK), "no {class} text");
    }
}

#[test]
fn structure_contract() {
    for prefix in ["vd", "h-1"] {
        let opts = SvgOpts {
            id_prefix: prefix.into(),
            title: Some(MARK.into()),
        };
        let svg = render_checked(&marked(), &opts);
        assert_eq!(&svg.root_children[..2], ["title", "desc"], "{prefix}");
        let mut ids = svg.ids.clone();
        ids.sort();
        let mut want: Vec<String> = ["arrow", "desc", "title"]
            .iter()
            .map(|s| format!("{prefix}-{s}"))
            .collect();
        want.sort();
        assert_eq!(ids, want, "{prefix}");
    }
}
