//! The single SVG output path: one escaper, one number formatter, one ordered-attribute writer.
//! Nothing else in `svg/` may build markup or escape text.

use crate::layout::Half;

/// Escape `s` for XML text content (`attr == false`) or a double-quoted attribute value.
///
/// `& < >` are always escaped (so `]]>` can never occur and CDATA is never used); `"` and `'`
/// only in attributes. XML-illegal controls, C1 controls, bidi controls and line/paragraph
/// separators become U+FFFD.
pub(crate) fn esc(s: &str, attr: bool) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' if attr => o.push_str("&quot;"),
            '\'' if attr => o.push_str("&#39;"),
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
            | '\u{2029}' => o.push('\u{fffd}'),
            c => o.push(c),
        }
    }
    o
}

/// Print a half-pixel value as px: no exponent, no trailing `.0`, never `-0`.
pub(crate) fn num(h: Half) -> String {
    let a = h.unsigned_abs();
    let body = if a.is_multiple_of(2) {
        format!("{}", a / 2)
    } else {
        format!("{}.5", a / 2)
    };
    if h < 0 { format!("-{body}") } else { body }
}

/// Ordered attributes: a slice, never a map, so output order is the call-site order.
pub(crate) type Attrs<'a> = &'a [(&'static str, &'a str)];

/// Markup builder. Every attribute value goes through `esc(.., true)` and every text through
/// `esc(.., false)`; tag and attribute names are `'static`, so they cannot carry user text.
#[derive(Debug, Default)]
pub(crate) struct Writer {
    out: String,
}

impl Writer {
    /// An empty document.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn tag(&mut self, name: &'static str, attrs: Attrs<'_>, close: &str) {
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            self.out.push(' ');
            self.out.push_str(k);
            self.out.push_str("=\"");
            self.out.push_str(&esc(v, true));
            self.out.push('"');
        }
        self.out.push_str(close);
    }

    /// `<name attrs>` without a line break (use with [`Writer::end`] for inline children).
    pub(crate) fn start(&mut self, name: &'static str, attrs: Attrs<'_>) {
        self.tag(name, attrs, ">");
    }

    /// `</name>` without a line break.
    pub(crate) fn end(&mut self, name: &'static str) {
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push('>');
    }

    /// `<name attrs>text</name>` without a line break; `text` is escaped as content.
    pub(crate) fn leaf(&mut self, name: &'static str, attrs: Attrs<'_>, text: &str) {
        self.start(name, attrs);
        self.out.push_str(&esc(text, false));
        self.end(name);
    }

    /// A line break.
    pub(crate) fn nl(&mut self) {
        self.out.push('\n');
    }

    /// `<name attrs>` on its own line.
    pub(crate) fn open(&mut self, name: &'static str, attrs: Attrs<'_>) {
        self.start(name, attrs);
        self.nl();
    }

    /// `</name>` on its own line.
    pub(crate) fn close(&mut self, name: &'static str) {
        self.end(name);
        self.nl();
    }

    /// `<name attrs/>` on its own line.
    pub(crate) fn empty(&mut self, name: &'static str, attrs: Attrs<'_>) {
        self.tag(name, attrs, "/>");
        self.nl();
    }

    /// `<name attrs>text</name>` on its own line.
    pub(crate) fn text(&mut self, name: &'static str, attrs: Attrs<'_>, text: &str) {
        self.leaf(name, attrs, text);
        self.nl();
    }

    /// The finished document.
    pub(crate) fn finish(self) -> String {
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_characters() {
        assert_eq!(esc("&", false), "&amp;");
        assert_eq!(esc("<", false), "&lt;");
        assert_eq!(esc(">", false), "&gt;");
        assert_eq!(esc("&<>", true), "&amp;&lt;&gt;");
        assert_eq!(esc("]]>", false), "]]&gt;");
    }

    #[test]
    fn quotes_only_in_attributes() {
        assert_eq!(esc("\"'", false), "\"'");
        assert_eq!(esc("\"", true), "&quot;");
        assert_eq!(esc("'", true), "&#39;");
    }

    #[test]
    fn controls_and_bidi_become_replacement() {
        let hit = [
            '\u{0}', '\u{8}', '\u{b}', '\u{c}', '\u{e}', '\u{1f}', '\u{7f}', '\u{80}', '\u{9f}',
            '\u{fffe}', '\u{ffff}', '\u{61c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202e}',
            '\u{2066}', '\u{2069}', '\u{2028}', '\u{2029}',
        ];
        for c in hit {
            assert_eq!(
                esc(&c.to_string(), false),
                "\u{fffd}",
                "U+{:04X}",
                u32::from(c)
            );
            assert_eq!(
                esc(&c.to_string(), true),
                "\u{fffd}",
                "U+{:04X}",
                u32::from(c)
            );
        }
    }

    #[test]
    fn newline_tab_and_plain_text_pass() {
        assert_eq!(
            esc("a\nb\tc \u{a0}\u{e9}\u{65e5}", true),
            "a\nb\tc \u{a0}\u{e9}\u{65e5}"
        );
    }

    #[test]
    fn numbers_are_half_pixel_decimals() {
        let cases = [
            (0, "0"),
            (1, "0.5"),
            (2, "1"),
            (-1, "-0.5"),
            (-2, "-1"),
            (2001, "1000.5"),
        ];
        for (h, want) in cases {
            assert_eq!(num(h), want, "{h}");
        }
        assert_eq!(num(i32::MIN), "-1073741824");
    }

    #[test]
    fn writer_keeps_attribute_order_and_escapes() {
        let mut w = Writer::new();
        w.open("g", &[("z", "1"), ("a", "2")]);
        w.text("title", &[("id", "t\"x")], "a<b");
        w.empty("path", &[("d", "M0 0")]);
        w.close("g");
        assert_eq!(
            w.finish(),
            "<g z=\"1\" a=\"2\">\n<title id=\"t&quot;x\">a&lt;b</title>\n<path d=\"M0 0\"/>\n</g>\n"
        );
    }

    #[test]
    fn inline_children_stay_on_one_line() {
        let mut w = Writer::new();
        w.start("text", &[]);
        w.leaf("tspan", &[("x", "1")], "A");
        w.leaf("tspan", &[("x", "1")], "B");
        w.end("text");
        w.nl();
        assert_eq!(
            w.finish(),
            "<text><tspan x=\"1\">A</tspan><tspan x=\"1\">B</tspan></text>\n"
        );
    }
}
