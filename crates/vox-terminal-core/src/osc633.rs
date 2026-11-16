//! OSC-633 & OSC-7 block/cwd parser — Rust port and extension of `osc633.ts`.
//!
//! Scans raw PTY byte streams for `ESC ] 633 ; ... BEL/ST` and `ESC ] 7 ; ... BEL/ST`
//! sequences while streaming output chunks live and capping incomplete OSC buffering
//! at `16 KiB` (`T-441-03`).

/// Maximum bytes retained for an unterminated OSC sequence before flushing as plain output.
pub const MAX_INCOMPLETE_OSC_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Osc633Event {
    PromptStart,
    PromptEnd,
    CommandLine(String),
    PreExec,
    Exit(i32),
    Property { key: String, value: String },
    Osc7Cwd(String),
    Output(String),
}

/// General `\xNN` hex decode — mirrors `decodeCommand` in `osc633.ts` exactly.
/// That function uses `/\\x([0-9a-fA-F]{2})/g` — not a fixed table.
pub fn decode_command(enc: &str) -> String {
    let bytes = enc.as_bytes();
    let mut raw = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && bytes.get(i + 1) == Some(&b'x')
            && i + 3 < bytes.len()
            && let Ok(n) = u8::from_str_radix(&enc[i + 2..i + 4], 16)
        {
            raw.push(n);
            i += 4;
            continue;
        }
        raw.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(raw).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Percent-decode an `OSC 7` `file://<host>/<path>` (or raw path) URI into a UTF-8 filesystem path.
pub fn parse_osc7_cwd(raw: &str) -> Option<String> {
    let encoded_path = if let Some(after_scheme) = raw.strip_prefix("file://") {
        let slash_idx = after_scheme.find('/')?;
        &after_scheme[slash_idx..]
    } else if !raw.is_empty() {
        raw
    } else {
        return None;
    };

    let bytes = encoded_path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3])
            && let Ok(val) = u8::from_str_radix(hex_str, 16)
        {
            decoded.push(val);
            i += 3;
            continue;
        }
        decoded.push(bytes[i]);
        i += 1;
    }

    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut path = String::from_utf8_lossy(&decoded).into_owned();
    #[cfg(windows)]
    if path.len() >= 3
        && path.as_bytes()[0] == b'/'
        && path.as_bytes()[1].is_ascii_alphabetic()
        && path.as_bytes()[2] == b':'
    {
        path.remove(0);
    }

    if path.is_empty() { None } else { Some(path) }
}

/// Byte-fed OSC-633 and OSC-7 scanner.
///
/// Call [`Osc633Parser::feed`] with raw PTY bytes. Returns any events emitted in this batch.
/// Partial escape sequences and split multi-byte UTF-8 tails are buffered across calls.
#[derive(Default)]
pub struct Osc633Parser {
    /// Internal ring buffer — holds partial OSC sequences across calls.
    buf: Vec<u8>,
    /// True when we are currently between a `C` (PreExec) and `D` (Exit) marker.
    capturing: bool,
    /// Trailing 1..=3 incomplete UTF-8 byte decoder across output chunk boundaries.
    decoder: crate::utf8_stream::Utf8StreamDecoder,
}

impl Osc633Parser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed raw PTY bytes, return any events parsed from this batch.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Osc633Event> {
        self.buf.extend_from_slice(bytes);
        self.parse()
    }

    fn emit_output(&mut self, raw: &[u8], flush_incomplete: bool, events: &mut Vec<Osc633Event>) {
        if raw.is_empty() && (!flush_incomplete || !self.decoder.has_pending()) {
            return;
        }
        let mut out = if raw.is_empty() {
            String::new()
        } else {
            self.decoder.push(raw)
        };
        if flush_incomplete {
            out.push_str(&self.decoder.flush_lossy());
        }
        if !out.is_empty() {
            events.push(Osc633Event::Output(out));
        }
    }

    fn parse(&mut self) -> Vec<Osc633Event> {
        let mut events = Vec::new();
        let mut pos = 0;

        while pos < self.buf.len() {
            if let Some(osc_start) = find_osc_start(&self.buf[pos..]) {
                let abs_start = pos + osc_start;

                if abs_start > pos {
                    let raw = self.buf[pos..abs_start].to_vec();
                    self.emit_output(&raw, true, &mut events);
                } else if self.decoder.has_pending() {
                    self.emit_output(&[], true, &mut events);
                }

                match extract_osc_payload(&self.buf[abs_start..]) {
                    OscScan::Complete { payload, seq_len } => {
                        if let Some(ev) = self.process_marker(&payload) {
                            events.push(ev);
                        }
                        pos = abs_start + seq_len;
                    }
                    OscScan::Aborted { abort_len } => {
                        let aborted = self.buf[abs_start..abs_start + abort_len].to_vec();
                        self.emit_output(&aborted, true, &mut events);
                        pos = abs_start + abort_len;
                    }
                    OscScan::Incomplete => {
                        if self.buf.len() - abs_start > MAX_INCOMPLETE_OSC_BYTES {
                            let overflow = self.buf[abs_start..].to_vec();
                            self.buf.clear();
                            self.emit_output(&overflow, true, &mut events);
                            return events;
                        }
                        let remaining = self.buf[abs_start..].to_vec();
                        self.buf = remaining;
                        return events;
                    }
                }
            } else {
                let tail = self.buf[pos..].to_vec();
                self.buf.clear();
                self.emit_output(&tail, false, &mut events);
                return events;
            }
        }

        self.buf.clear();
        events
    }

    fn process_marker(&mut self, payload: &str) -> Option<Osc633Event> {
        if let Some(rest) = payload.strip_prefix("633;") {
            match rest {
                "A" => return Some(Osc633Event::PromptStart),
                "B" => return Some(Osc633Event::PromptEnd),
                "C" => {
                    self.capturing = true;
                    return Some(Osc633Event::PreExec);
                }
                _ if rest.starts_with("E;") => {
                    let cmd = decode_command(&rest[2..]);
                    return Some(Osc633Event::CommandLine(cmd));
                }
                _ if rest.starts_with("D;") || rest == "D" => {
                    self.capturing = false;
                    let exit_str = rest.strip_prefix("D;").unwrap_or("0");
                    let code = exit_str.parse::<i32>().unwrap_or(0);
                    return Some(Osc633Event::Exit(code));
                }
                _ if rest.starts_with("P;") => {
                    let prop = &rest[2..];
                    if let Some((key, value)) = prop.split_once('=') {
                        let decoded = if key == "Cwd" {
                            decode_command(value)
                        } else {
                            value.to_string()
                        };
                        return Some(Osc633Event::Property {
                            key: key.to_string(),
                            value: decoded,
                        });
                    }
                }
                _ => {}
            }
        } else if let Some(rest) = payload.strip_prefix("7;") {
            return parse_osc7_cwd(rest).map(Osc633Event::Osc7Cwd);
        }
        None
    }
}

/// Find the start of `ESC ] 633 ;` or `ESC ] 7 ;` (or a partial prefix thereof at the very end
/// of `buf`), returning the byte offset. Other OSC sequences (such as `ESC ] 2 ;`) are ignored
/// so they pass through as normal output.
fn find_osc_start(buf: &[u8]) -> Option<usize> {
    for i in 0..buf.len() {
        if buf[i] != b'\x1b' {
            continue;
        }
        let rest = &buf[i + 1..];
        if rest.is_empty() {
            // Lone ESC at buffer end — could become ESC ] 633; or ESC ] 7;
            return Some(i);
        }
        if rest[0] != b']' {
            continue;
        }
        let peek = &rest[1..];
        if peek.starts_with(b"633;")
            || peek.starts_with(b"7;")
            || b"633;".starts_with(peek)
            || b"7;".starts_with(peek)
        {
            return Some(i);
        }
    }
    None
}

enum OscScan {
    Complete { payload: String, seq_len: usize },
    Aborted { abort_len: usize },
    Incomplete,
}

/// Try to extract the OSC payload from `buf` starting at `ESC ]`.
/// Under ECMA-48 / VT500, `CAN` (`0x18`), `SUB` (`0x1a`), or any `ESC` (`0x1b`) not followed by
/// `\` (`ST`) aborts the current OSC string so subsequent markers are never swallowed.
fn extract_osc_payload(buf: &[u8]) -> OscScan {
    if buf.len() < 2 || buf[0] != b'\x1b' || buf[1] != b']' {
        return OscScan::Incomplete;
    }
    let mut i = 2;
    while i < buf.len() {
        match buf[i] {
            b'\x07' => {
                let payload = String::from_utf8_lossy(&buf[2..i]).into_owned();
                return OscScan::Complete {
                    payload,
                    seq_len: i + 1,
                };
            }
            b'\x18' | b'\x1a' => {
                return OscScan::Aborted { abort_len: i + 1 };
            }
            b'\x1b' => match buf.get(i + 1) {
                Some(&b'\\') => {
                    let payload = String::from_utf8_lossy(&buf[2..i]).into_owned();
                    return OscScan::Complete {
                        payload,
                        seq_len: i + 2,
                    };
                }
                Some(_) => {
                    return OscScan::Aborted { abort_len: i };
                }
                None => {
                    return OscScan::Incomplete;
                }
            },
            _ => {}
        }
        i += 1;
    }
    OscScan::Incomplete
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_osc_and_split_st_terminator_preserve_following_markers() {
        let mut parser = Osc633Parser::new();

        // 1. Unterminated OSC 7 interrupted mid-sequence by SIGINT + OSC 633;D;130:
        let evs = parser.feed(b"\x1b]7;file://localhost/work/dir\x1b]633;D;130\x07");
        assert!(
            evs.iter().any(
                |e| matches!(e, Osc633Event::Output(s) if s.contains("file://localhost/work/dir"))
            ),
            "aborted OSC prefix must flush as plain output, got {evs:?}"
        );
        assert!(
            evs.contains(&Osc633Event::Exit(130)),
            "following 633;D;130 marker must not be swallowed, got {evs:?}"
        );

        // 2. CAN (0x18) and SUB (0x1a) also abort incomplete OSC without eating next marker:
        let can_evs = parser.feed(b"\x1b]633;E;broken\x18\x1b]633;A\x07");
        assert!(can_evs.contains(&Osc633Event::PromptStart));
        let sub_evs = parser.feed(b"\x1b]633;E;broken\x1a\x1b]633;B\x07");
        assert!(sub_evs.contains(&Osc633Event::PromptEnd));

        // 3. Split ST terminator (`\x1b` at end of chunk 1, `\` at start of chunk 2):
        let st_part1 = parser.feed(b"\x1b]633;D;42\x1b");
        assert!(
            st_part1.is_empty(),
            "trailing ESC before ST backslash must wait for next byte, got {st_part1:?}"
        );
        let st_part2 = parser.feed(b"\\after\n");
        assert!(st_part2.contains(&Osc633Event::Exit(42)));
        assert!(st_part2.contains(&Osc633Event::Output("after\n".to_string())));

        // 4. Escaped control/delimiter bytes in `633;P;Cwd=` decode via `decode_command`:
        let prop_evs = parser.feed(b"\x1b]633;P;Cwd=/work/a\\x3bb\\x0ac\\x07d\\x1be\\x5cf\x07");
        assert!(prop_evs.contains(&Osc633Event::Property {
            key: "Cwd".to_string(),
            value: "/work/a;b\nc\x07d\x1be\\f".to_string(),
        }));
    }
}
