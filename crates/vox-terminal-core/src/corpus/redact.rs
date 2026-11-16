//! Transcript redaction. The implementation moved to the `vox-redact` crate so
//! non-terminal crates can reuse it; re-exported here to keep the existing
//! `super::redact::redact_owned` / `corpus::redact_owned` paths stable.

pub use vox_redact::redact_owned;

pub const DEFAULT_BLOCK_HEAD_LINES: usize = 40;
pub const DEFAULT_BLOCK_TAIL_LINES: usize = 160;
pub const DEFAULT_BLOCK_MAX_BYTES: usize = 16 * 1024;

/// Normalize `\r` progress overwrites, strip full ECMA-48 escape sequences, and
/// apply `vox_redact::redact_owned` so ANSI escapes inside secrets or paths cannot
/// evade redaction.
#[must_use]
pub fn scrub_ansi_and_redact(raw: &str) -> String {
    scrub_ansi_and_redact_with(raw, &|s: &str| s.to_owned())
}

/// Alias for [`scrub_ansi_and_redact`] used by downstream terminal host and MCP bridges.
#[must_use]
pub fn scrub_terminal_text(raw: &str) -> String {
    scrub_ansi_and_redact(raw)
}

/// 3-stage terminal scrubber: ECMA-48/CR normalize -> caller `extra_scrub` (e.g. Clavis) -> `vox_redact`.
#[must_use]
pub fn scrub_ansi_and_redact_with<F>(input: &str, extra_scrub: &F) -> String
where
    F: Fn(&str) -> String + ?Sized,
{
    if input.is_empty() {
        return String::new();
    }
    let clean = crate::block::strip_ansi_and_normalize(input);
    let after_extra = extra_scrub(&clean);
    vox_redact::redact_owned(&after_extra)
}

/// Clone `block` and scrub `input`, every `output` chunk, `cwd`, and `git_branch` via 3-stage redaction.
#[must_use]
pub fn redact_block_with<F>(block: &crate::block::Block, extra_scrub: &F) -> crate::block::Block
where
    F: Fn(&str) -> String + ?Sized,
{
    let mut b = block.clone();
    b.input = scrub_ansi_and_redact_with(&b.input, extra_scrub);
    for chunk in &mut b.output {
        chunk.text = scrub_ansi_and_redact_with(&chunk.text, extra_scrub);
    }
    b.cwd = b
        .cwd
        .as_deref()
        .map(|s| scrub_ansi_and_redact_with(s, extra_scrub));
    b.git_branch = b
        .git_branch
        .as_deref()
        .map(|s| scrub_ansi_and_redact_with(s, extra_scrub));
    b
}

/// Convenience wrapper for [`redact_block_with`] without an extra Clavis scrubber.
#[must_use]
pub fn redact_block(block: &crate::block::Block) -> crate::block::Block {
    redact_block_with(block, &|s: &str| s.to_owned())
}

fn floor_utf8_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_utf8_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Enforce a head + tail line budget and maximum UTF-8 byte budget on a scrubbed
/// terminal block, returning `(formatted_text, was_truncated, omitted_lines)`.
#[must_use]
pub fn truncate_terminal_block(
    text: &str,
    session_id: &str,
    block_id: u64,
    head_lines: usize,
    tail_lines: usize,
    max_bytes: usize,
) -> (String, bool, usize) {
    if text.is_empty() {
        return (String::new(), false, 0);
    }
    if max_bytes == 0 {
        return (String::new(), true, text.lines().count());
    }

    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len();
    let max_lines = head_lines.saturating_add(tail_lines);

    if total_lines <= max_lines && text.len() <= max_bytes {
        return (text.to_string(), false, 0);
    }

    let trailing_newline = if text.ends_with('\n') { "\n" } else { "" };
    let (candidate, omitted_lines) = if max_lines > 0 && total_lines > max_lines {
        let omitted = total_lines - max_lines;
        let start = head_lines + 1;
        let end = total_lines - tail_lines;
        let head_part = lines[..head_lines].join("\n");
        let tail_part = lines[total_lines - tail_lines..].join("\n");
        let marker = format!(
            "\n... [omitted {omitted} lines (L{start}..L{end}); call vox_terminal_read(session_id=\"{session_id}\", block_id={block_id}, line_start={start}, line_end={end}) to inspect] ...\n"
        );
        (
            format!("{head_part}{marker}{tail_part}{trailing_newline}"),
            omitted,
        )
    } else {
        (text.to_string(), 0)
    };

    if candidate.len() <= max_bytes {
        return (candidate, true, omitted_lines);
    }

    let byte_marker = if omitted_lines > 0 {
        let start = head_lines + 1;
        let end = total_lines - tail_lines;
        format!(
            "\n... [omitted {omitted_lines} lines (L{start}..L{end}), capped at {max_bytes} bytes; call vox_terminal_read(session_id=\"{session_id}\", block_id={block_id}, line_start={start}, line_end={end}) to inspect] ...\n"
        )
    } else {
        let end_line = total_lines.max(1);
        format!(
            "\n... [output capped at {max_bytes} bytes; call vox_terminal_read(session_id=\"{session_id}\", block_id={block_id}, line_start=1, line_end={end_line}) to inspect] ...\n"
        )
    };

    if max_bytes <= byte_marker.len() {
        let cut = floor_utf8_boundary(&candidate, max_bytes);
        return (candidate[..cut].to_string(), true, omitted_lines);
    }

    let payload_budget = max_bytes - byte_marker.len();
    let head_budget = if max_lines > 0 {
        payload_budget.saturating_mul(head_lines) / max_lines
    } else {
        payload_budget / 5
    };
    let head_end = floor_utf8_boundary(text, head_budget.min(text.len()));
    let tail_budget = payload_budget.saturating_sub(head_end);
    let raw_tail_start = text.len().saturating_sub(tail_budget).max(head_end);
    let tail_start = ceil_utf8_boundary(text, raw_tail_start);

    let out = format!("{}{byte_marker}{}", &text[..head_end], &text[tail_start..]);
    debug_assert!(out.len() <= max_bytes);
    (out, true, omitted_lines)
}

/// Convenience wrapper around [`truncate_terminal_block`] when session/block context is omitted.
#[must_use]
pub fn truncate_head_tail(
    text: &str,
    head_lines: usize,
    tail_lines: usize,
    max_bytes: usize,
) -> (String, bool, usize) {
    truncate_terminal_block(text, "", 0, head_lines, tail_lines, max_bytes)
}

/// Return the 1-indexed inclusive line slice `line_start..=line_end` (`0` clamps to `1`).
#[must_use]
pub fn slice_lines_1based(text: &str, line_start: usize, line_end: usize) -> String {
    if text.is_empty() {
        return String::new();
    }
    let start = line_start.max(1);
    let end = line_end.max(1);
    if end < start {
        return String::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    if start > lines.len() {
        return String::new();
    }
    let end_clamped = end.min(lines.len());
    lines[start - 1..end_clamped].join("\n")
}

#[cfg(test)]
mod tests {
    // These now exercise `vox_redact::redact_owned` through the re-export — a free
    // cross-crate behavior check that the moved redactor still elides what it must.
    use super::*;

    #[test]
    fn email_redacted() {
        let r = redact_owned("contact me at alice@example.com please");
        assert!(!r.contains("alice@example.com"));
        assert!(r.contains("[REDACTED_EMAIL]"));
    }

    #[test]
    fn api_key_redacted() {
        let r = redact_owned("Authorization: Bearer sk-abcdefghijklmnopqrstuvwxyz123456");
        assert!(r.contains("[REDACTED_KEY]"), "got: {r}");
    }

    #[test]
    fn loopback_ip_redacted() {
        let r = redact_owned("server at 127.0.0.1:8080");
        assert!(r.contains("[REDACTED_IP]"), "got: {r}");
    }

    #[test]
    fn clean_text_unchanged() {
        let text = "hello world, run: cargo test --lib";
        assert_eq!(redact_owned(text), text);
    }

    #[test]
    fn scrub_ansi_and_redact_strips_escapes_and_cr_before_redacting() {
        assert_eq!(scrub_ansi_and_redact(""), "");
        // vox-arch-check: allow abs-path
        let raw = "Downloading 10%\rDownloading 100%\nToken: sk-\x1b[31mproj-abcdefghijklmnopqrstuvwxyz1234\x1b[0m in /Users/\x1b[1malice\x1b[0m/.env";
        let scrubbed = scrub_ansi_and_redact(raw);
        assert_eq!(
            scrubbed,
            "Downloading 100%\n[REDACTED_KEY] in ~[REDACTED_PATH]/.env"
        );
        assert!(!scrubbed.contains("sk-"));
        assert!(!scrubbed.contains("alice"));
        assert_eq!(scrub_terminal_text(raw), scrubbed);

        let bare_split = "val=sk-\x1b[31mproj-abcdefghijklmnopqrstuvwxyz1234\x1b[0m end";
        assert_eq!(scrub_ansi_and_redact(bare_split), "val=[REDACTED_KEY] end");
    }

    #[test]
    fn truncate_terminal_block_enforces_head_40_tail_160_and_utf8_byte_cap() {
        let (empty, trunc_empty, omitted_empty) = truncate_terminal_block(
            "",
            "s1",
            7,
            DEFAULT_BLOCK_HEAD_LINES,
            DEFAULT_BLOCK_TAIL_LINES,
            DEFAULT_BLOCK_MAX_BYTES,
        );
        assert_eq!(empty, "");
        assert!(!trunc_empty);
        assert_eq!(omitted_empty, 0);

        let lines_300 = (1..=300)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (formatted, was_truncated, omitted) = truncate_terminal_block(
            &lines_300,
            "s1",
            7,
            DEFAULT_BLOCK_HEAD_LINES,
            DEFAULT_BLOCK_TAIL_LINES,
            DEFAULT_BLOCK_MAX_BYTES,
        );
        assert!(was_truncated);
        assert_eq!(omitted, 100);
        assert!(formatted.starts_with("line-1\n"));
        assert!(formatted.contains("\nline-40\n"));
        assert!(!formatted.contains("\nline-41\n"));
        assert!(!formatted.contains("\nline-140\n"));
        assert!(formatted.contains("\nline-141\n"));
        assert!(formatted.ends_with("\nline-300"));
        assert!(formatted.contains(
            r#"vox_terminal_read(session_id="s1", block_id=7, line_start=41, line_end=140)"#
        ));

        // Single 32 KiB line filled with multi-byte UTF-8 emojis & box-drawing chars
        let huge_utf8 = "🦀═".repeat(5_000); // 35,000 bytes (7 bytes per pair)
        assert!(huge_utf8.len() > DEFAULT_BLOCK_MAX_BYTES * 2);
        let (capped, capped_trunc, capped_omitted) = truncate_terminal_block(
            &huge_utf8,
            "s1",
            8,
            DEFAULT_BLOCK_HEAD_LINES,
            DEFAULT_BLOCK_TAIL_LINES,
            DEFAULT_BLOCK_MAX_BYTES,
        );
        assert!(capped_trunc);
        assert_eq!(capped_omitted, 0);
        assert!(capped.len() <= DEFAULT_BLOCK_MAX_BYTES);
        assert!(!capped.contains('\u{FFFD}'));
        assert!(capped.contains("vox_terminal_read"));

        assert_eq!(
            slice_lines_1based(&lines_300, 41, 43),
            "line-41\nline-42\nline-43"
        );
        assert_eq!(slice_lines_1based(&lines_300, 0, 2), "line-1\nline-2");
        assert_eq!(slice_lines_1based("", 1, 10), "");
        assert_eq!(slice_lines_1based(&lines_300, 305, 310), "");
    }
}
