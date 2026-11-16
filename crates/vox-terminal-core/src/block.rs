use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    VoxNative,
    Shell,
    AgentTurn,
    SlashCommand,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockStatus {
    Running,
    AwaitingInput,
    Ok,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stream {
    Stdout,
    Stderr,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputChunk {
    pub stream: Stream,
    pub text: String,
}

impl OutputChunk {
    pub fn text(stream: Stream, s: impl Into<String>) -> Self {
        Self {
            stream,
            text: s.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockOrigin {
    #[default]
    User,
    Agent {
        agent_id: String,
    },
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub kind: BlockKind,
    #[serde(default)]
    pub origin: BlockOrigin,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub credential_prompt: bool,
    pub input: String,
    pub output: Vec<OutputChunk>,
    pub status: BlockStatus,
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

impl Block {
    pub fn new(id: BlockId, kind: BlockKind, input: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            origin: BlockOrigin::User,
            pinned: false,
            credential_prompt: false,
            input: input.into(),
            output: vec![],
            status: BlockStatus::Running,
            exit_code: None,
            cwd: None,
            git_branch: None,
        }
    }

    pub fn with_origin(mut self, origin: BlockOrigin) -> Self {
        self.origin = origin;
        self
    }

    pub fn push(&mut self, c: OutputChunk) {
        self.output.push(c);
    }

    pub fn finish(&mut self, exit: i32) {
        self.exit_code = Some(exit);
        self.status = if exit == 0 {
            BlockStatus::Ok
        } else {
            BlockStatus::Failed
        };
    }

    pub fn cancel(&mut self) {
        self.status = BlockStatus::Cancelled;
        self.exit_code = Some(130);
    }

    /// Text projection for agent context + transcript: output with `\r`
    /// progress overwrites normalized and ANSI/VT escapes stripped. Visual
    /// rendering is the front-end's responsibility.
    pub fn plain_output(&self) -> String {
        let raw: String = self.output.iter().map(|c| c.text.as_str()).collect();
        strip_ansi_and_normalize(&raw)
    }
}

/// Normalize carriage-return (`\r` / `CSI K`) overwrites and strip full ECMA-48 / VT
/// escape sequences (CSI, OSC, DCS, SOS, PM, APC, charset designators, 2-byte ESC,
/// and non-printable C0/DEL control bytes except `\n` and `\t`).
#[must_use]
pub fn strip_ansi_and_normalize(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let normalized = crate::utf8_stream::normalize_cr_overwrites(raw);
    strip_ansi(&normalized)
}

fn consume_st_terminated(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(ch) = chars.next() {
        if ch == '\x07' {
            break;
        }
        if ch == '\x1b' {
            if matches!(chars.peek(), Some('\\')) {
                chars.next();
            }
            break;
        }
    }
}

/// Full ECMA-48 / VT escape and C0 control stripper for plain-text export and redaction.
/// Full VT grid rendering lives in the front-end (`alacritty_terminal` for TUI,
/// `xterm.js` for GUI).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek().copied() {
                Some('[') => {
                    chars.next(); // consume '['
                    // CSI: consume until a final byte in 0x40..=0x7E
                    for ch in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&ch) {
                            break;
                        }
                    }
                }
                Some(']') | Some('P') | Some('X') | Some('^') | Some('_') => {
                    chars.next(); // consume introducer
                    // OSC / DCS / SOS / PM / APC: consume until BEL (0x07) or ST (ESC \)
                    consume_st_terminated(&mut chars);
                }
                Some(inter) if ('\x20'..='\x2f').contains(&inter) => {
                    // nF escape sequence (charset designations ESC ( B, ESC ) 0, ESC * A, ESC + B, etc.)
                    chars.next();
                    while let Some(&next_ch) = chars.peek() {
                        if ('\x20'..='\x2f').contains(&next_ch) {
                            chars.next();
                        } else {
                            if ('\x30'..='\x7e').contains(&next_ch) {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                Some(fin) if ('\x30'..='\x7e').contains(&fin) => {
                    // 2-byte ESC sequence (e.g. ESC 7, ESC 8, ESC c)
                    chars.next();
                }
                _ => {} // lone ESC — skip it
            }
        } else if c == '\n' || c == '\t' || ((c as u32) >= 0x20 && c != '\x7f') {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_and_normalize_covers_full_ecma48_and_cr_overwrites() {
        assert_eq!(strip_ansi_and_normalize(""), "");

        let raw = concat!(
            "Step 10%\rStep 100%\n",
            "\x1b[38;2;255;0;0mred\x1b[0m ",
            "\x1b]633;A\x07",
            "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\ ",
            "\x1bP1$r0m\x1b\\",
            "\x1b_Gi=1\x1b\\",
            "\x1bXignored\x1b\\",
            "\x1b^pm\x1b\\",
            "\x1b(B\x1b)0",
            "\x1b7\x1b8\x1bc",
            "a\x00b\x07c\x08d\x7fe\tf\n",
            "🦀═ok"
        );
        assert_eq!(
            strip_ansi_and_normalize(raw),
            "Step 100%\nred link abcde\tf\n🦀═ok"
        );
    }

    #[test]
    fn block_serde_defaults_pinned_and_credential_prompt_and_awaiting_input_roundtrips() {
        let legacy_json =
            r#"{"id":9,"kind":"Shell","input":"sudo ls","output":[],"status":"Ok","exit_code":0}"#;
        let parsed: Block = serde_json::from_str(legacy_json).expect("legacy block deserializes");
        assert!(!parsed.pinned);
        assert!(!parsed.credential_prompt);

        let mut b = Block::new(BlockId(10), BlockKind::Shell, "ssh prod");
        assert!(!b.pinned);
        assert!(!b.credential_prompt);
        b.status = BlockStatus::AwaitingInput;
        b.pinned = true;
        b.credential_prompt = true;

        let encoded = serde_json::to_string(&b).expect("serialize block");
        let decoded: Block = serde_json::from_str(&encoded).expect("deserialize block");
        assert_eq!(decoded.status, BlockStatus::AwaitingInput);
        assert!(decoded.pinned);
        assert!(decoded.credential_prompt);
    }
}
