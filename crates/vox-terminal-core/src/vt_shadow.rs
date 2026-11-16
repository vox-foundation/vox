//! Shadow VT100/xterm state machine tracking `TermMode::ALT_SCREEN` transitions.
//!
//! Wraps `alacritty_terminal::Term` with `scrolling_history: 0` (`T-441-02`) and clamped
//! non-zero dimensions so `vox-terminal-core` authoritatively tracks `?1049h`/`?1049l`,
//! `?1047h`/`?1047l`, `?47h`/`?47l`, compound DECSET sequences (`\x1b[?1049;2004h`), and
//! full terminal resets (`\x1bc` RIS) even when zero UI front-ends are attached.

use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    term::{Config, TermMode},
    vte::ansi::{Handler, NamedPrivateMode, PrivateMode, Processor},
};

struct NoopListener;

impl EventListener for NoopListener {
    fn send_event(&self, _: Event) {}
}

struct ShadowSize {
    cols: usize,
    lines: usize,
}

impl Dimensions for ShadowSize {
    fn columns(&self) -> usize {
        self.cols
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn total_lines(&self) -> usize {
        self.lines
    }
}

/// Bridges legacy xterm alternate-screen DECSET/DECRST modes (`?47` and `?1047`, which `vte 0.15`
/// surfaces as `PrivateMode::Unknown`) onto `Term`'s `TermMode::ALT_SCREEN` state.
pub struct LegacyAltSync<'a, T: EventListener>(pub &'a mut Term<T>);

impl<T: EventListener> Handler for LegacyAltSync<'_, T> {
    fn set_private_mode(&mut self, mode: PrivateMode) {
        if matches!(mode, PrivateMode::Unknown(47 | 1047)) {
            Handler::set_private_mode(
                self.0,
                PrivateMode::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor),
            );
        }
    }

    fn unset_private_mode(&mut self, mode: PrivateMode) {
        if matches!(mode, PrivateMode::Unknown(47 | 1047)) {
            Handler::unset_private_mode(
                self.0,
                PrivateMode::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor),
            );
        }
    }
}

/// Advance both the primary `vte::Processor` and a `legacy_parser` (mapping `?47` and `?1047`
/// onto `TermMode::ALT_SCREEN`) segmented on `ESC` and DECSET/DECRST final bytes (`h`/`l`) so
/// mode switches take effect immediately before subsequent draw bytes in the same chunk.
pub fn advance_term_with_legacy_alt<T: EventListener>(
    term: &mut Term<T>,
    parser: &mut Processor,
    legacy_parser: &mut Processor,
    bytes: &[u8],
) {
    if bytes.is_empty() {
        return;
    }
    let mut start = 0;
    for idx in 0..bytes.len() {
        if idx > start && bytes[idx] == b'\x1b' {
            let seg = &bytes[start..idx];
            parser.advance(term, seg);
            legacy_parser.advance(&mut LegacyAltSync(term), seg);
            start = idx;
        }
        if matches!(bytes[idx], b'h' | b'l') {
            let seg = &bytes[start..=idx];
            parser.advance(term, seg);
            legacy_parser.advance(&mut LegacyAltSync(term), seg);
            start = idx + 1;
        }
    }
    if start < bytes.len() {
        let tail = &bytes[start..];
        parser.advance(term, tail);
        legacy_parser.advance(&mut LegacyAltSync(term), tail);
    }
}

/// Zero-scrollback `alacritty_terminal::Term` shadow tracker for `ALT_SCREEN` mode transitions.
pub struct VtShadow {
    term: Term<NoopListener>,
    parser: Processor,
    legacy_parser: Processor,
    alt_screen: bool,
}

impl VtShadow {
    /// Create a shadow VT state machine clamped to at least `1×1` cells with zero scrollback.
    pub fn new(cols: u16, rows: u16) -> Self {
        let size = ShadowSize {
            cols: cols.max(1) as usize,
            lines: rows.max(1) as usize,
        };
        let cfg = Config {
            scrolling_history: 0,
            ..Config::default()
        };
        Self {
            term: Term::new(cfg, &size, NoopListener),
            parser: Processor::new(),
            legacy_parser: Processor::new(),
            alt_screen: false,
        }
    }

    /// Feed raw PTY bytes into the VT parser and return `Some(active)` iff `TermMode::ALT_SCREEN`
    /// changed state at the end of `bytes`.
    pub fn feed(&mut self, bytes: &[u8]) -> Option<bool> {
        if bytes.is_empty() {
            return None;
        }
        advance_term_with_legacy_alt(
            &mut self.term,
            &mut self.parser,
            &mut self.legacy_parser,
            bytes,
        );

        let now = self.term.mode().contains(TermMode::ALT_SCREEN);
        if now != self.alt_screen {
            self.alt_screen = now;
            Some(now)
        } else {
            None
        }
    }

    /// Feed `text` segmented on `ESC` (`0x1b`) and DECSET/DECRST/RIS final bytes (`h`/`l`/`c`)
    /// so back-to-back enter/exit sequences emit every `ALT_SCREEN` transition in order and only
    /// primary-screen slices (`!was_alt && !now_alt`) are returned for `Block` output capture.
    pub(crate) fn feed_extracting_primary(
        &mut self,
        text: &str,
        mut on_toggle: impl FnMut(bool),
    ) -> String {
        if text.is_empty() {
            return String::new();
        }
        let mut primary = String::new();
        let bytes = text.as_bytes();
        let mut start = 0;

        let mut push_seg = |seg: &str| {
            if seg.is_empty() {
                return;
            }
            let was_alt = self.alt_screen;
            if let Some(active) = self.feed(seg.as_bytes()) {
                on_toggle(active);
            }
            if !was_alt && !self.alt_screen {
                primary.push_str(seg);
            }
        };

        for idx in 0..bytes.len() {
            if idx > start && bytes[idx] == b'\x1b' {
                push_seg(&text[start..idx]);
                start = idx;
            }
            if matches!(bytes[idx], b'h' | b'l' | b'c') {
                push_seg(&text[start..=idx]);
                start = idx + 1;
            }
        }
        if start < bytes.len() {
            push_seg(&text[start..]);
        }
        primary
    }

    /// Resize the shadow terminal grid, clamping dimensions to at least `1×1`.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.term.resize(ShadowSize {
            cols: cols.max(1) as usize,
            lines: rows.max(1) as usize,
        });
    }

    /// Returns `true` when the terminal is currently in alternate-screen mode (`TermMode::ALT_SCREEN`).
    pub fn is_alt_screen(&self) -> bool {
        self.alt_screen
    }
}
