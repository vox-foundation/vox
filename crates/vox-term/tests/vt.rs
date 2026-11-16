/// Task 2.2 — VT-grid renderer: alacritty_terminal wrapping.
use vox_term::vt::VtGrid;

#[test]
fn ansi_color_produces_styled_span() {
    let mut grid = VtGrid::new(80, 24);
    grid.feed(b"\x1b[31mhi\x1b[0m");
    let lines = grid.render_lines();
    // At least one span should contain "hi"
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.content.as_ref())
        .collect();
    assert!(text.contains("hi"), "rendered text: {text:?}");
    // The span containing "hi" should be styled red
    let red_span = lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .find(|s| s.content.contains("hi"));
    assert!(red_span.is_some(), "no span contains 'hi'");
    use ratatui::style::Color;
    let style = red_span.unwrap().style;
    assert_eq!(style.fg, Some(Color::Red), "expected red fg, got {style:?}");
}

#[test]
fn alternate_screen_does_not_panic() {
    let mut grid = VtGrid::new(80, 24);
    // Alternate screen enter + exit should not panic
    grid.feed(b"\x1b[?1049h");
    grid.feed(b"hello\r\n");
    grid.feed(b"\x1b[?1049l");
    let _ = grid.render_lines();
}

#[test]
fn legacy_alt_screen_modes_47_1047_and_wide_char_spacer_width() {
    let mut grid = VtGrid::new(8, 2);
    assert!(!grid.is_alt_screen());

    grid.feed(b"\x1b[?47hALT47");
    assert!(grid.is_alt_screen(), "?47h must enter alt screen on VtGrid");
    grid.feed(b"\x1b[?47l");
    assert!(!grid.is_alt_screen(), "?47l must exit alt screen on VtGrid");

    grid.feed(b"\x1b[?1047hALT1047");
    assert!(
        grid.is_alt_screen(),
        "?1047h must enter alt screen on VtGrid"
    );
    grid.feed(b"\x1b[?1047l");
    assert!(
        !grid.is_alt_screen(),
        "?1047l must exit alt screen on VtGrid"
    );

    // Wide glyph ('🦀' = 2 columns) + "ok" (2 columns) + 4 trailing blanks = 8 columns total.
    // Skipping Flags::WIDE_CHAR_SPACER ensures we emit 7 chars ("🦀ok    "), not 8 chars ("🦀 ok    ").
    grid.reset();
    grid.feed("🦀ok".as_bytes());
    let lines = grid.render_lines();
    let row0: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(row0, "🦀ok    ");
}
