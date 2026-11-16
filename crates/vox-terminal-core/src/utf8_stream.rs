//! Streaming UTF-8 tail reassembler and carriage-return (`\r` / `CSI K`) line normalizer.
//!
//! PTY `read()` calls slice output on arbitrary byte boundaries (commonly `4096` bytes, or any
//! kernel buffer boundary), frequently splitting 2..=4-byte UTF-8 sequences (box-drawing glyphs,
//! emojis) across consecutive reads. [`Utf8StreamDecoder`] retains `1..=3` trailing incomplete
//! UTF-8 bytes until the continuation arrives so valid UTF-8 streams never emit `U+FFFD`.
//!
//! [`normalize_cr_overwrites`] collapses in-place terminal progress bars and spinners (`\r`
//! without `\n`, `\x1b[K` / `\x1b[0K` erase-to-EOL, and `\x1b[2K` erase-line) while preserving
//! standard `\r\n` newlines even when `\r` and `\n` arrived in separate chunks.

/// Stateful UTF-8 byte-stream decoder holding `0..=3` trailing incomplete UTF-8 bytes.
#[derive(Debug, Clone, Default)]
pub struct Utf8StreamDecoder {
    tail: [u8; 3],
    tail_len: usize,
}

impl Utf8StreamDecoder {
    /// Create a new decoder with an empty trailing byte buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when `1..=3` incomplete UTF-8 bytes are buffered awaiting continuation.
    pub fn has_pending(&self) -> bool {
        self.tail_len > 0
    }

    /// Decode `bytes` combined with any buffered trailing bytes, retaining `1..=3` incomplete
    /// trailing UTF-8 bytes for the next call and replacing genuinely invalid sequences with
    /// `U+FFFD`.
    pub fn push(&mut self, bytes: &[u8]) -> String {
        if self.tail_len == 0 && bytes.is_empty() {
            return String::new();
        }

        let mut combined = Vec::with_capacity(self.tail_len + bytes.len());
        combined.extend_from_slice(&self.tail[..self.tail_len]);
        combined.extend_from_slice(bytes);
        self.tail_len = 0;

        let mut out = String::with_capacity(combined.len());
        let mut rest: &[u8] = &combined;
        while !rest.is_empty() {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    out.push_str(valid);
                    break;
                }
                Err(err) => {
                    let valid_len = err.valid_up_to();
                    if valid_len > 0 {
                        out.push_str(std::str::from_utf8(&rest[..valid_len]).unwrap_or_default());
                    }
                    rest = &rest[valid_len..];
                    match err.error_len() {
                        None => {
                            // 1..=3 incomplete UTF-8 bytes at the end of this chunk.
                            self.tail_len = rest.len();
                            self.tail[..self.tail_len].copy_from_slice(rest);
                            break;
                        }
                        Some(bad_len) => {
                            out.push('\u{FFFD}');
                            rest = &rest[bad_len..];
                        }
                    }
                }
            }
        }
        out
    }

    /// Flush any buffered incomplete trailing UTF-8 sequence as `U+FFFD` (used at stream EOF or
    /// before an escape sequence boundary).
    pub fn flush_lossy(&mut self) -> String {
        if self.tail_len == 0 {
            String::new()
        } else {
            self.tail_len = 0;
            "\u{FFFD}".to_string()
        }
    }
}

#[derive(Default)]
struct LineCell {
    prefix: String,
    ch: char,
}

fn flush_cells(
    out: &mut String,
    cells: &[LineCell],
    pending_esc: &str,
    eol_esc: &str,
    with_newline: bool,
) {
    for cell in cells {
        out.push_str(&cell.prefix);
        out.push(cell.ch);
    }
    out.push_str(pending_esc);
    out.push_str(eol_esc);
    if with_newline {
        out.push('\n');
    }
}

/// Normalize carriage-return (`\r`) line overwrites and CSI erase-in-line (`\x1b[K`, `\x1b[0K`,
/// `\x1b[1K`, `\x1b[2K`) sequences while converting `\r\n` to `\n`.
pub fn normalize_cr_overwrites(raw: &str) -> String {
    if !raw.contains('\r') && !raw.contains("\x1b[") {
        return raw.to_string();
    }

    let mut out = String::with_capacity(raw.len());
    let mut cells: Vec<LineCell> = Vec::new();
    let mut col: usize = 0;
    let mut pending_esc = String::new();
    let mut eol_esc = String::new();

    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.peek().copied() {
                Some('[') => {
                    chars.next();
                    let mut params = String::new();
                    let mut final_byte: Option<char> = None;
                    for ch in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&ch) {
                            final_byte = Some(ch);
                            break;
                        }
                        params.push(ch);
                    }
                    if final_byte == Some('K') {
                        match params.trim() {
                            "" | "0" => {
                                if col < cells.len() {
                                    cells.truncate(col);
                                }
                                eol_esc.clear();
                            }
                            "1" => {
                                let end = (col + 1).min(cells.len());
                                for cell in &mut cells[..end] {
                                    cell.prefix.clear();
                                    cell.ch = ' ';
                                }
                            }
                            "2" => {
                                cells.clear();
                                col = 0;
                                pending_esc.clear();
                                eol_esc.clear();
                            }
                            _ => {
                                pending_esc.push_str("\x1b[");
                                pending_esc.push_str(&params);
                                pending_esc.push('K');
                            }
                        }
                    } else {
                        pending_esc.push_str("\x1b[");
                        pending_esc.push_str(&params);
                        if let Some(fb) = final_byte {
                            pending_esc.push(fb);
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    pending_esc.push_str("\x1b]");
                    while let Some(ch) = chars.next() {
                        pending_esc.push(ch);
                        if ch == '\x07' {
                            break;
                        }
                        if ch == '\x1b' {
                            if chars.peek() == Some(&'\\') {
                                pending_esc.push('\\');
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {
                    pending_esc.push('\x1b');
                }
            },
            '\r' => {
                if !pending_esc.is_empty() {
                    eol_esc.push_str(&pending_esc);
                    pending_esc.clear();
                }
                col = 0;
            }
            '\n' => {
                flush_cells(&mut out, &cells, &pending_esc, &eol_esc, true);
                cells.clear();
                pending_esc.clear();
                eol_esc.clear();
                col = 0;
            }
            ch => {
                let prefix = std::mem::take(&mut pending_esc);
                if col < cells.len() {
                    cells[col] = LineCell { prefix, ch };
                } else {
                    while cells.len() < col {
                        cells.push(LineCell {
                            prefix: String::new(),
                            ch: ' ',
                        });
                    }
                    cells.push(LineCell { prefix, ch });
                }
                col += 1;
            }
        }
    }

    if !cells.is_empty() || !pending_esc.is_empty() {
        flush_cells(&mut out, &cells, &pending_esc, &eol_esc, false);
    }

    out
}
