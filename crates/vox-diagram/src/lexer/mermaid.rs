//! Mermaid link tokens: dash, equals and dotted runs with the three label spellings. The
//! whole link, label included, is one token so label text never reaches the identifier rules.

use super::{Lexer, TokenKind};

impl Lexer<'_> {
    /// True when a link (`--`, `==` or `-.-` run) starts at byte `i`.
    pub(super) fn starts_link(&self, i: usize) -> bool {
        let rest = self.src.get(i..).unwrap_or_default();
        rest.starts_with("--") || rest.starts_with("==") || rest.starts_with("-.-")
    }

    /// Consume the link that starts at `start` (its first char is already consumed).
    pub(super) fn link(&mut self, start: usize, end: &mut usize) -> TokenKind {
        let rest = self.src.get(start..).unwrap_or_default();
        let line = rest.split(['\n', '\r']).next().unwrap_or_default();
        let line_end = start + line.len();
        let first = line.chars().next().unwrap_or('-');
        let dotted = line.starts_with("-.");
        let (run_from, run) = if dotted { (2, '-') } else { (0, first) };
        let tail = line.get(run_from..).unwrap_or_default();
        let mut len = run_from + tail.chars().take_while(|&c| c == run).count();
        let (mut exact, mut label) = (false, None);
        let tail = line.get(len..).unwrap_or_default();
        let after = tail.chars().nth(1);
        if tail.starts_with('>') {
            len += 1;
            exact = first == '-' && !dotted;
        } else if first == '-' && !dotted {
            if matches!(tail.chars().next(), Some('x' | 'o'))
                && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                len += 1;
            } else if len == 2 && tail.starts_with(' ') && start >= self.dash_fail_until {
                // `-- text -->`
                match tail.find("-->") {
                    Some(p) => {
                        label = Some(tail.get(..p).unwrap_or_default().trim().to_string());
                        len += p + 3;
                        exact = true;
                    }
                    None => self.dash_fail_until = line_end,
                }
            }
        }
        if label.is_none() {
            label = self.pipe_label(start, line, &mut len);
        }
        *end = start + len;
        while self.it.peek().is_some_and(|&(j, _)| j < *end) {
            self.it.next();
        }
        TokenKind::MArrow { exact, label }
    }

    /// `|text|` after the link (blanks allowed before it); extends `len` through the closing
    /// pipe. A pipe with no closer on the line is left alone.
    fn pipe_label(&mut self, start: usize, line: &str, len: &mut usize) -> Option<String> {
        let after = line.get(*len..).unwrap_or_default();
        let body = after.trim_start_matches([' ', '\t']);
        let inner = body.strip_prefix('|')?;
        if start >= self.pipe_fail_until {
            let Some(p) = inner.find('|') else {
                self.pipe_fail_until = start + line.len();
                return None;
            };
            *len += after.len() - inner.len() + p + 1;
            return Some(inner.get(..p).unwrap_or_default().trim().to_string());
        }
        None
    }
}
