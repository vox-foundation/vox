//! Dedicated `CommandOutputHub` & `\r` Terminal Progress-Bar Normalization (`T3-STREAM-GUEST-02`).
//!
//! Adapted from:
//! - `apps/server/src/orchestration-v2/CommandOutputHub.ts` (`MAX_CHUNKS_PER_STREAM = 64`, `MAX_BYTES_PER_STREAM = 262_144`)
//! - `packages/shared/src/terminalOutput.ts` (`normalizeTerminalOutput`)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Maximum number of chunks retained per active command output stream.
pub const MAX_CHUNKS_PER_STREAM: usize = 64;

/// Maximum UTF-8 byte budget retained per active command output stream (`256 KiB`).
pub const MAX_BYTES_PER_STREAM: usize = 262_144;

/// Fold carriage-return (`\r`) in-place terminal overwrites (progress bars, spinners)
/// while preserving `\n` and `\r\n` line breaks.
///
/// For each line segment, a bare `\r` (not immediately followed by `\n`) resets the cursor
/// to column `0` of the current line, overwriting characters from the start of the line.
pub fn normalize_terminal_output(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let unified = raw.replace("\r\n", "\n");
    let lines: Vec<&str> = unified.split('\n').collect();
    for (line_idx, line) in lines.iter().enumerate() {
        if line_idx > 0 {
            out.push('\n');
        }
        if !line.contains('\r') {
            out.push_str(line);
            continue;
        }
        let mut current_chars: Vec<char> = Vec::new();
        for segment in line.split('\r') {
            for (col, ch) in segment.chars().enumerate() {
                if col < current_chars.len() {
                    current_chars[col] = ch;
                } else {
                    current_chars.push(ch);
                }
            }
        }
        for ch in current_chars {
            out.push(ch);
        }
    }
    out
}

/// Sequenced command output chunk stored in `CommandOutputHub` (outside the orchestration event log).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutputChunk {
    /// 1-based monotonic sequence within the command stream.
    pub sequence: u64,
    /// Normalized terminal text chunk.
    pub text: String,
}

#[derive(Debug, Default, Clone)]
struct CommandStreamBuffer {
    next_sequence: u64,
    total_bytes: usize,
    chunks: VecDeque<CommandOutputChunk>,
}

/// Bounded in-memory hub for live subprocess stdout/stderr streams (`MAX_CHUNKS_PER_STREAM = 64`,
/// `MAX_BYTES_PER_STREAM = 262_144`). Keeps high-frequency command output out of the SQLite event log.
#[derive(Debug, Default, Clone)]
pub struct CommandOutputHub {
    streams: HashMap<String, CommandStreamBuffer>,
}

impl CommandOutputHub {
    /// Create an empty `CommandOutputHub`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append raw stdout/stderr output for `command_id`, normalizing `\r` overwrites and enforcing
    /// both `MAX_CHUNKS_PER_STREAM` and `MAX_BYTES_PER_STREAM`.
    pub fn append_output(&mut self, command_id: &str, raw_chunk: &str) -> u64 {
        let normalized = normalize_terminal_output(raw_chunk);
        let entry = self.streams.entry(command_id.to_string()).or_default();
        entry.next_sequence += 1;
        let seq = entry.next_sequence;
        let chunk_bytes = normalized.len();
        entry.total_bytes += chunk_bytes;
        entry.chunks.push_back(CommandOutputChunk {
            sequence: seq,
            text: normalized,
        });

        while entry.chunks.len() > MAX_CHUNKS_PER_STREAM
            || (entry.total_bytes > MAX_BYTES_PER_STREAM && entry.chunks.len() > 1)
        {
            if let Some(evicted) = entry.chunks.pop_front() {
                entry.total_bytes = entry.total_bytes.saturating_sub(evicted.text.len());
            }
        }
        seq
    }

    /// Replay buffered chunks for `command_id` with `sequence > after_sequence`.
    pub fn replay_after(&self, command_id: &str, after_sequence: u64) -> Vec<CommandOutputChunk> {
        self.streams
            .get(command_id)
            .map(|buf| {
                buf.chunks
                    .iter()
                    .filter(|c| c.sequence > after_sequence)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Return the concatenated normalized output currently retained for `command_id`.
    pub fn settled_output(&self, command_id: &str) -> String {
        let Some(buf) = self.streams.get(command_id) else {
            return String::new();
        };
        let mut combined = String::with_capacity(buf.total_bytes);
        for chunk in &buf.chunks {
            combined.push_str(&chunk.text);
        }
        normalize_terminal_output(&combined)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandOutputHub, MAX_BYTES_PER_STREAM, MAX_CHUNKS_PER_STREAM, normalize_terminal_output,
    };

    #[test]
    fn normalize_terminal_output_folds_carriage_return_progress_bars_and_preserves_crlf() {
        let raw = "Downloading 10%\rDownloading 55%\rDownloading 100%\r\nDone\n";
        assert_eq!(normalize_terminal_output(raw), "Downloading 100%\nDone\n");

        // Partial overwrite retains trailing characters from longer prior segment on same line
        let partial = "ABCDEF\rXY";
        assert_eq!(normalize_terminal_output(partial), "XYCDEF");
    }

    #[test]
    fn command_output_hub_enforces_chunk_and_byte_caps() {
        let mut hub = CommandOutputHub::new();
        for i in 0..(MAX_CHUNKS_PER_STREAM + 10) {
            hub.append_output("cmd-1", &format!("line-{i}\n"));
        }
        let replayed = hub.replay_after("cmd-1", 0);
        assert_eq!(replayed.len(), MAX_CHUNKS_PER_STREAM);
        assert_eq!(replayed.first().map(|c| c.sequence), Some(11));

        // Byte cap eviction
        let mut byte_hub = CommandOutputHub::new();
        let half_cap = "X".repeat((MAX_BYTES_PER_STREAM / 2) + 1024);
        byte_hub.append_output("cmd-bytes", &half_cap);
        byte_hub.append_output("cmd-bytes", &half_cap);
        let chunks = byte_hub.replay_after("cmd-bytes", 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].sequence, 2);
    }
}
