//! Sequence renderer safety: render caps, IR validation, hostile text in every text slot and
//! byte determinism. The oracle is `support::svgcheck`, a quick-xml allowlist checker independent
//! of the writer, plus `sanitize`/`squeeze` restated here (not read from `src`).

mod support;

use std::collections::BTreeSet;

use proptest::prelude::*;
use support::hostile::FRAGMENTS;
use support::roundtrip::{fixtures, stems};
use support::svgcheck::seq::sequence_ir;
use support::svgcheck::{self, Svg};
use vox_diagram::check;
use vox_diagram::ir::Ir;
use vox_diagram::limits::{MAX_LABEL_CHARS, MAX_RENDER_ACTORS, MAX_RENDER_EVENTS};
use vox_diagram::sequence_ir::{Actor, SeqEvent, SequenceIr};
use vox_diagram::svg::{RenderError, SvgOpts, render};

fn actor(id: &str, label: &str) -> Actor {
    Actor {
        id: id.into(),
        label: label.into(),
        span: Default::default(),
    }
}

fn message(from: &str, to: &str, text: &str) -> SeqEvent {
    SeqEvent::Message {
        from: from.into(),
        to: to.into(),
        text: text.into(),
        dashed: false,
        span: Default::default(),
    }
}

fn note(over: &[&str], text: &str) -> SeqEvent {
    SeqEvent::Note {
        over: over.iter().map(|s| (*s).into()).collect(),
        text: text.into(),
        span: Default::default(),
    }
}

fn seq(ids: &[&str], events: Vec<SeqEvent>) -> SequenceIr {
    SequenceIr {
        actors: ids.iter().map(|i| actor(i, &i.to_uppercase())).collect(),
        events,
    }
}

fn run(s: &SequenceIr) -> Result<String, RenderError> {
    render(&Ir::Sequence(s.clone()), &SvgOpts::default())
}

fn checked(svg: &str, prefix: &str) -> Svg {
    match svgcheck::check(svg, prefix) {
        Ok(read) => read,
        Err(e) => panic!("checker rejected the svg: {e}\n{svg}"),
    }
}

fn is_invalid(s: &SequenceIr) -> bool {
    matches!(run(s), Err(RenderError::InvalidIr(_)))
}

// ---- caps and IR validation --------------------------------------------------------------

#[test]
fn caps_are_checked_before_layout() {
    let n = MAX_RENDER_ACTORS + 1;
    let ids: Vec<String> = (0..n).map(|i| format!("a{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    assert_eq!(
        run(&seq(&refs, Vec::new())),
        Err(RenderError::TooLarge { nodes: n, edges: 0 })
    );
    // Events naming unknown actors still hit the cap first: nothing was placed or resolved.
    let m = MAX_RENDER_EVENTS + 1;
    let events = (0..m).map(|_| message("zz", "yy", "x")).collect();
    assert_eq!(
        run(&seq(&["a", "b"], events)),
        Err(RenderError::TooLarge { nodes: 2, edges: m })
    );
}

#[test]
fn a_diagram_at_the_caps_renders() {
    let ids: Vec<String> = (0..MAX_RENDER_ACTORS).map(|i| format!("a{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let events = (0..MAX_RENDER_EVENTS)
        .map(|i| match i % 3 {
            0 => message(refs[i % refs.len()], refs[(i * 7 + 1) % refs.len()], "go"),
            1 => message(
                refs[(i * 5) % refs.len()],
                refs[(i * 5) % refs.len()],
                "self",
            ),
            _ => note(&[refs[i % refs.len()]], "n"),
        })
        .collect();
    let svg = match run(&seq(&refs, events)) {
        Ok(s) => s,
        Err(e) => panic!("render at the caps failed: {e}"),
    };
    let read = checked(&svg, "vd");
    assert_eq!(read.nodes.len(), MAX_RENDER_ACTORS);
}

#[test]
fn duplicate_actor_is_invalid_ir() {
    let r = run(&seq(&["a", "b", "a"], Vec::new()));
    assert_eq!(
        r,
        Err(RenderError::InvalidIr("actor 2: duplicate id".into()))
    );
}

#[test]
fn overlong_labels_are_invalid_ir() {
    let (ok, long) = ("w".repeat(MAX_LABEL_CHARS), "w".repeat(MAX_LABEL_CHARS + 1));
    let mut fine = seq(&["a", "b"], vec![message("a", "b", &ok), note(&["a"], &ok)]);
    fine.actors[0].label = ok.clone();
    assert!(run(&fine).is_ok());
    let mut actor_long = fine.clone();
    actor_long.actors[1].label = long.clone();
    let msg_long = seq(&["a", "b"], vec![message("a", "b", &long)]);
    let note_long = seq(&["a", "b"], vec![note(&["a"], &long)]);
    assert_eq!(
        run(&actor_long),
        Err(RenderError::InvalidIr("actor 1: label too long".into()))
    );
    assert_eq!(
        run(&msg_long),
        Err(RenderError::InvalidIr("event 0: label too long".into()))
    );
    assert!(is_invalid(&note_long));
}

#[test]
fn note_arity_is_invalid_ir() {
    for over in [&[][..], &["a", "b", "a"][..]] {
        let r = run(&seq(&["a", "b"], vec![note(over, "x")]));
        let want = "event 0: a note covers one or two actors";
        assert_eq!(r, Err(RenderError::InvalidIr(want.into())), "{over:?}");
    }
}

#[test]
fn unknown_actor_is_invalid_ir() {
    for ev in [
        message("a", "zz", "x"),
        message("zz", "a", "x"),
        note(&["zz"], "x"),
    ] {
        let r = run(&seq(&["a"], vec![ev]));
        assert_eq!(
            r,
            Err(RenderError::InvalidIr("event 0: unknown actor".into()))
        );
    }
}

#[test]
fn bad_prefix_is_rejected() {
    let opts = SvgOpts {
        id_prefix: "Bad Prefix".into(),
        ..SvgOpts::default()
    };
    let ir = Ir::Sequence(seq(&["a"], Vec::new()));
    assert_eq!(render(&ir, &opts), Err(RenderError::InvalidIdPrefix));
}

// ---- hostile text ------------------------------------------------------------------------

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

fn text_of(svg: &Svg, class: &str) -> String {
    svg.texts
        .iter()
        .filter(|(c, _)| c == class)
        .map(|(_, t)| t.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `t` as an actor label, a message text, a note text and the title.
fn hostile_sequence(t: &str) -> (Ir, SvgOpts) {
    let mut s = seq(
        &["a", "b"],
        vec![message("a", "b", t), note(&["a", "b"], &format!("n{t}"))],
    );
    s.actors[0].label = t.into();
    s.actors[1].label = "ok".into();
    let opts = SvgOpts {
        title: Some(t.into()),
        ..SvgOpts::default()
    };
    (Ir::Sequence(s), opts)
}

fn hostile_texts_hold(t: &str) -> Result<(), String> {
    let (ir, opts) = hostile_sequence(t);
    let svg = render(&ir, &opts).map_err(|e| format!("render failed for {t:?}: {e}"))?;
    let read = svgcheck::check(&svg, "vd").map_err(|e| format!("{e}\n{svg}"))?;
    let want = squeeze(&sanitize(t));
    let got = |class: &str| squeeze(&text_of(&read, class));
    let checks = [
        ("vd-label", format!("{want}ok")),
        ("vd-edge-label", want.clone()),
        ("vd-note-label", format!("n{want}")),
        ("title", want),
    ];
    for (class, expect) in checks {
        if got(class) != expect {
            return Err(format!("{class} of {t:?}: {:?} != {expect:?}", got(class)));
        }
    }
    Ok(())
}

#[test]
fn hostile_fragments_in_every_text_slot() {
    for f in FRAGMENTS {
        for text in [(*f).to_owned(), format!("x{f}y")] {
            if let Err(e) = hostile_texts_hold(&text) {
                panic!("{e}");
            }
        }
    }
}

fn hostile_text() -> impl Strategy<Value = String> {
    let pick = prop_oneof![
        prop::sample::select(FRAGMENTS.to_vec()).prop_map(str::to_owned),
        any::<char>().prop_map(String::from),
    ];
    prop::collection::vec(pick, 1..=8)
        .prop_map(|v| v.concat().chars().take(MAX_LABEL_CHARS - 1).collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// The checker accepts every render of arbitrary text in all three text slots and the title.
    #[test]
    fn hostile_text_property(t in hostile_text()) {
        let (ir, opts) = hostile_sequence(&t);
        let rendered = render(&ir, &opts);
        prop_assert!(rendered.is_ok(), "{:?}", rendered);
        let svg = rendered.unwrap_or_default();
        let parsed = svgcheck::check(&svg, "vd");
        prop_assert!(parsed.is_ok(), "{:?}\n{}", parsed.as_ref().err(), svg);
    }

    #[test]
    fn generated_sequences_render_identically(s in sequence_ir()) {
        prop_assert_eq!(run(&s), run(&s));
    }
}

const MARK: &str = "QzQ7";

#[test]
fn no_user_text_in_attributes() {
    let mut s = seq(
        &["a", "b"],
        vec![message("a", "b", MARK), note(&["a"], MARK)],
    );
    s.actors[0].label = MARK.into();
    let opts = SvgOpts {
        title: Some(MARK.into()),
        ..SvgOpts::default()
    };
    let svg = match render(&Ir::Sequence(s), &opts) {
        Ok(svg) => svg,
        Err(e) => panic!("render failed: {e}"),
    };
    let read = checked(&svg, "vd");
    assert!(
        read.attr_values.iter().all(|v| !v.contains(MARK)),
        "{:?}",
        read.attr_values
    );
    for class in ["vd-label", "vd-edge-label", "vd-note-label", "title"] {
        assert!(text_of(&read, class).contains(MARK), "no {class} text");
    }
}

// ---- determinism -------------------------------------------------------------------------

#[test]
fn twenty_fresh_renders_are_identical() {
    let cases = fixtures().join("svg/cases");
    let mut sources = vec![fixtures().join("valid/sequence_all_constructs.vdg")];
    sources.extend(
        stems(&cases, "vdg")
            .into_iter()
            .filter(|s| s.contains("-seq-"))
            .map(|s| cases.join(format!("{s}.vdg"))),
    );
    assert!(sources.len() >= 5, "{sources:?}");
    for path in sources {
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        let Some(ir) = check(&src).ir else {
            panic!("{} does not check", path.display());
        };
        let outputs: BTreeSet<String> = (0..20)
            .map(|_| match render(&ir, &SvgOpts::default()) {
                Ok(s) => s,
                Err(e) => panic!("{}: {e}", path.display()),
            })
            .collect();
        assert_eq!(outputs.len(), 1, "{} is not deterministic", path.display());
    }
}
