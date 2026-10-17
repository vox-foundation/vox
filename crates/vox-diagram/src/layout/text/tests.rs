use proptest::prelude::*;

use super::*;

fn owned(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn table_has_the_arimo_anchors() {
    assert_eq!(ADV.len(), 95);
    assert_eq!((ADV[0], ADV[b'A' as usize - 32], ADV[94]), (278, 667, 584));
    assert_eq!(ADV[b'm' as usize - 32], 833);
    assert_eq!(measure(""), 0);
}

#[test]
fn wrap_keeps_explicit_newlines() {
    assert_eq!(wrap("a\nb"), owned(&["a", "b"]));
}

#[test]
fn wrap_breaks_a_sentence_at_spaces() {
    let s = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu";
    let lines = wrap(s);
    assert!(lines.len() > 1, "{lines:?}");
    assert!(lines.iter().all(|l| cells(l) <= WRAP_CELLS), "{lines:?}");
    assert_eq!(lines.join(" "), s);
}

#[test]
fn wrap_hard_breaks_only_an_overlong_word() {
    let w = "x".repeat(40);
    assert_eq!(wrap(&w), vec!["x".repeat(28), "x".repeat(12)]);
    let fits = "y".repeat(28);
    assert_eq!(wrap(&fits), vec![fits]);
}

#[test]
fn wrap_separators_are_exactly_space_tab_cr() {
    assert_eq!(wrap("a \t\r  b"), owned(&["a b"]));
    assert_eq!(wrap("  a  "), owned(&["a"]));
    assert_eq!(wrap("a\u{a0}b"), owned(&["a\u{a0}b"]));
    assert_eq!(wrap("a\u{2028}b"), owned(&["a\u{2028}b"]));
}

#[test]
fn wrap_drops_blank_lines() {
    assert_eq!(wrap("\n\nx"), owned(&["x"]));
    assert_eq!(wrap(""), Vec::<String>::new());
    assert_eq!(wrap(" \n\t\n"), Vec::<String>::new());
}

#[test]
fn wrap_counts_cjk_as_two_cells() {
    let s = "\u{65e5}".repeat(15);
    assert_eq!(wrap(&s), vec!["\u{65e5}".repeat(14), "\u{65e5}".to_owned()]);
}

#[test]
fn wrap_keeps_a_zero_width_follower_with_its_base() {
    let s = format!("{}e\u{301}x", "a".repeat(27));
    let lines = wrap(&s);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].ends_with("e\u{301}"), "{lines:?}");
    assert_eq!(lines[1], "x");
}

#[test]
fn measure_follows_the_width_model() {
    assert_eq!(measure("A"), 21);
    assert_eq!(measure("\u{65e5}"), 31);
    assert_eq!(measure("\u{1f680}"), 40);
    assert_eq!(measure("\u{26a0}\u{fe0f}"), 40);
    assert_eq!(measure("\u{301}"), 0);
    assert_eq!(measure("\t"), measure(" "));
    assert_eq!(measure("\u{e9}"), 19);
}

const SHAPES: [Shape; 6] = [
    Shape::Rect,
    Shape::Rounded,
    Shape::Diamond,
    Shape::Cylinder,
    Shape::Start,
    Shape::End,
];

#[test]
fn node_size_minimums() {
    let one = owned(&["A"]);
    let want = [
        (Shape::Rect, (128, 76)),
        (Shape::Rounded, (128, 76)),
        (Shape::Diamond, (192, 128)),
        (Shape::Cylinder, (128, 108)),
        (Shape::Start, (144, 76)),
        (Shape::End, (144, 76)),
    ];
    for (shape, size) in want {
        assert_eq!(node_size(shape, &one), size, "{shape:?}");
    }
}

#[test]
fn node_size_formulas_for_a_wide_label() {
    let wide = owned(&["W".repeat(40).as_str()]);
    let tw = measure(&wide[0]);
    assert_eq!(node_size(Shape::Rect, &wide), (tw + 56, 76));
    assert_eq!(node_size(Shape::Rounded, &wide), (tw + 56, 76));
    assert_eq!(node_size(Shape::Diamond, &wide), (2 * (tw + 28), 128));
    assert_eq!(node_size(Shape::Cylinder, &wide), (tw + 56, 108));
    assert_eq!(node_size(Shape::Start, &wide), (tw + 80, 76));
    assert_eq!(node_size(Shape::End, &wide), (tw + 80, 76));
}

fn label() -> impl Strategy<Value = String> {
    let ch = prop_oneof![
        4 => any::<char>(),
        3 => Just(' '),
        2 => Just('\n'),
        1 => Just('\t'),
        2 => Just('\u{65e5}'),
        2 => Just('\u{1f680}'),
        1 => Just('\u{301}'),
        1 => Just('\u{200d}'),
        1 => Just('\u{fe0f}'),
        2 => prop::char::range('a', 'z'),
    ];
    prop::collection::vec(ch, 0..160).prop_map(|v| v.into_iter().collect())
}

fn payload(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn wrapped_lines_are_short_clean_and_lossless(s in label()) {
        let lines = wrap(&s);
        for l in &lines {
            prop_assert!(cells(l) <= 28, "{l:?}");
            prop_assert!(!l.is_empty() && !l.starts_with(' ') && !l.ends_with(' ') && !l.contains("  "), "{l:?}");
        }
        prop_assert_eq!(payload(&lines.concat()), payload(&s));
    }

    #[test]
    fn node_size_contains_the_text_block(s in label()) {
        let lines = wrap(&s);
        let tw = lines.iter().map(|l| measure(l)).max().unwrap_or(0);
        let th = i32::try_from(lines.len()).unwrap_or(0) * 36;
        for shape in SHAPES {
            let (w, h) = node_size(shape, &lines);
            if shape == Shape::Diamond {
                prop_assert!(i64::from(tw) * i64::from(h) + i64::from(th) * i64::from(w) <= i64::from(w) * i64::from(h), "{w}x{h} for {tw}x{th}");
            } else {
                prop_assert!(w >= tw + 56 && h >= th + 40, "{shape:?} {w}x{h} for {tw}x{th}");
            }
        }
    }
}
