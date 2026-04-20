//! Terminal Markdown renderer for `vox-cli`.
#![allow(dead_code)]
//!
//! Converts a Markdown string into human-readable terminal output that respects
//! `NO_COLOR` and the global [`crate::diagnostics::ColorChoice`]. The stdout
//! path is used (plan output), so [`crate::diagnostics::should_color_stdout`]
//! controls all styling.
//!
//! Intentionally zero new external dependencies — uses `owo-colors` (already a
//! workspace dep) and the standard library only.
//!
//! # SSOT contract
//! - All human-in-the-loop approval prompts in `vox-cli` MUST route through
//!   [`render_human_prompt`] / [`confirm_or_abort`].
//! - All plan Markdown output MUST route through [`render_markdown`].

use owo_colors::OwoColorize;

/// Whether to emit ANSI colour codes.
#[inline]
fn color() -> bool {
    crate::diagnostics::should_color_stdout()
}

// ── Code fence ──────────────────────────────────────────────────────────────

/// Render a single fenced code block with box-drawing borders.
///
/// ```text
///   ┌─ rust ──────────────────────┐
///     fn main() { println!("hi"); }
///   └─────────────────────────────┘
/// ```
pub(crate) fn render_code_block(lang: &str, body: &str) -> String {
    const WIDTH: usize = 42;
    let c = color();
    let mut out = String::with_capacity(body.len() + 128);

    let label = if lang.is_empty() { "code" } else { lang };

    // Top border  ┌─ lang ──────...─┐
    let header_inner = format!(" {label} ");
    let dashes = WIDTH.saturating_sub(header_inner.len() + 2);
    let top = format!("  ┌─{header_inner}{:─<dashes$}┐", "", dashes = dashes);
    if c {
        out.push_str(&top.cyan().to_string());
    } else {
        out.push_str(&top);
    }
    out.push('\n');

    // Code lines — 4-space indent
    for line in body.lines() {
        let indented = format!("    {line}");
        if c {
            out.push_str(&indented.dimmed().to_string());
        } else {
            out.push_str(&indented);
        }
        out.push('\n');
    }

    // Bottom border  └──────────...─┘
    let bottom = format!("  └{:─<WIDTH$}┘", "", WIDTH = WIDTH);
    if c {
        out.push_str(&bottom.cyan().to_string());
    } else {
        out.push_str(&bottom);
    }
    out.push('\n');

    out
}

// ── Markdown renderer ────────────────────────────────────────────────────────

/// Render a Markdown string to human-readable terminal output.
///
/// Handles:
/// - Fenced code blocks (` ```lang\n…\n``` `)
/// - Inline code (`` `…` ``)
/// - ATX headings (`#`, `##`)
/// - Checked/unchecked list items (`- [x]`, `- [ ]`, `- `)
/// - Paragraph breathing (collapses 3+ blank lines to 2)
pub(crate) fn render_markdown(src: &str) -> String {
    let c = color();
    let mut out = String::with_capacity(src.len() * 2);
    let mut in_fence = false;
    let mut fence_lang = String::new();
    let mut fence_body: Vec<String> = Vec::new();
    let mut blank_run = 0usize;

    for raw_line in src.lines() {
        // ── Fence open/close ────────────────────────────────────────────────
        if let Some(rest) = raw_line.trim_start().strip_prefix("```") {
            if !in_fence {
                // Opening fence
                in_fence = true;
                fence_lang = rest.trim().to_string();
                fence_body.clear();
            } else {
                // Closing fence — flush block
                in_fence = false;
                let rendered = render_code_block(&fence_lang, &fence_body.join("\n"));
                // Ensure blank line before
                if !out.ends_with("\n\n") && !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&rendered);
                out.push('\n');
                blank_run = 0;
                fence_lang.clear();
                fence_body.clear();
            }
            continue;
        }

        if in_fence {
            fence_body.push(raw_line.to_string());
            continue;
        }

        // ── Blank-line collapsing ────────────────────────────────────────────
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                out.push('\n');
            }
            continue;
        }
        blank_run = 0;

        // ── ATX headings ─────────────────────────────────────────────────────
        if let Some(heading_rest) = trimmed.strip_prefix("### ") {
            out.push('\n');
            let h = format!("  {}", heading_rest.to_uppercase());
            if c {
                out.push_str(&h.bold().to_string());
            } else {
                out.push_str(&h);
            }
            out.push('\n');
            continue;
        }
        if let Some(heading_rest) = trimmed.strip_prefix("## ") {
            out.push('\n');
            let h = format!("  {}", heading_rest.to_uppercase());
            if c {
                out.push_str(&h.bold().cyan().to_string());
            } else {
                out.push_str(&h);
            }
            out.push('\n');
            continue;
        }
        if let Some(heading_rest) = trimmed.strip_prefix("# ") {
            out.push('\n');
            let h = format!("  {}", heading_rest.to_uppercase());
            if c {
                out.push_str(&h.bold().bright_cyan().to_string());
            } else {
                out.push_str(&h);
            }
            out.push('\n');
            continue;
        }

        // ── List items ───────────────────────────────────────────────────────
        if let Some(rest) = trimmed
            .strip_prefix("- [x] ")
            .or(trimmed.strip_prefix("- [X] "))
        {
            let marker = if c {
                "  ✓ ".green().to_string()
            } else {
                "  ✓ ".to_string()
            };
            out.push_str(&marker);
            out.push_str(&render_inline_code(rest, c));
            out.push('\n');
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            let marker = if c {
                "  ○ ".dimmed().to_string()
            } else {
                "  ○ ".to_string()
            };
            out.push_str(&marker);
            out.push_str(&render_inline_code(rest, c));
            out.push('\n');
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("- ").or(trimmed.strip_prefix("* ")) {
            let marker = if c {
                "  • ".cyan().to_string()
            } else {
                "  • ".to_string()
            };
            out.push_str(&marker);
            out.push_str(&render_inline_code(rest, c));
            out.push('\n');
            continue;
        }

        // ── Numbered list items (e.g. `1. `) ────────────────────────────────
        let num_item = {
            let mut chars = trimmed.chars();
            let is_num = chars.by_ref().take_while(|ch| ch.is_ascii_digit()).count() > 0
                && trimmed.contains(". ");
            if is_num {
                let dot = trimmed.find(". ").unwrap_or(0);
                let num = &trimmed[..dot];
                let rest = &trimmed[dot + 2..];
                Some((num.to_string(), rest.to_string()))
            } else {
                None
            }
        };
        if let Some((num, rest)) = num_item {
            let marker = if c {
                format!("  {}. ", num).cyan().to_string()
            } else {
                format!("  {num}. ")
            };
            out.push_str(&marker);
            out.push_str(&render_inline_code(&rest, c));
            out.push('\n');
            continue;
        }

        // ── Normal prose line ─────────────────────────────────────────────────
        out.push_str("  ");
        out.push_str(&render_inline_code(trimmed, c));
        out.push('\n');
    }

    // Flush an unclosed fence gracefully (defensive)
    if in_fence && !fence_body.is_empty() {
        out.push_str(&render_code_block(&fence_lang, &fence_body.join("\n")));
    }

    out
}

/// Replace `` `…` `` spans with dimmed/yellow text (no recursive nesting).
fn render_inline_code(line: &str, c: bool) -> String {
    if !line.contains('`') {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len() + 16);
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        out.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        if let Some(end) = rest.find('`') {
            let code = &rest[..end];
            rest = &rest[end + 1..];
            if c {
                out.push_str(&code.yellow().to_string());
            } else {
                out.push_str(code);
            }
        } else {
            // Unclosed backtick — emit as-is
            out.push('`');
            out.push_str(rest);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

// ── Human-in-the-loop prompt ─────────────────────────────────────────────────

/// Render a human-in-the-loop approval prompt with a visual attention box.
///
/// All CLI HiTL approval prompts MUST use this function — it is the SSOT.
///
/// ```text
///   ╔══════════════════════════════════════════╗
///   ║  ❓  Action required                     ║
///   ║  Do you want to execute this PLAN?       ║
///   ╚══════════════════════════════════════════╝
/// ```
pub(crate) fn render_human_prompt(question: &str) {
    const W: usize = 48;
    let c = color();

    // Top border
    let top = format!("  ╔{:═<W$}╗", "", W = W);
    let label_line = format!("  ║  ❓  {:<width$}  ║", "Action required", width = W - 10);
    let q_trimmed = if question.len() > W - 6 {
        format!("{}…", &question[..W - 7])
    } else {
        question.to_string()
    };
    let q_line = format!("  ║  {:<width$}  ║", q_trimmed, width = W - 6);
    let bot = format!("  ╚{:═<W$}╝", "", W = W);

    if c {
        println!("{}", top.yellow());
        println!("{}", label_line.bold().yellow());
        println!("{}", q_line.yellow());
        println!("{}", bot.yellow());
    } else {
        println!("{top}");
        println!("{label_line}");
        println!("{q_line}");
        println!("{bot}");
    }
}

/// Prompt the user for `[Y/n]` confirmation using raw stdin.
///
/// Returns `true` if the user accepted (empty input, `y`, or `yes`).
/// Does **not** depend on `dialoguer` — uses `std::io::stdin` directly.
pub(crate) fn confirm_or_abort(question: &str) -> anyhow::Result<bool> {
    render_human_prompt(question);
    let c = color();
    if c {
        print!("  {} ", "[Y/n]:".bold());
    } else {
        print!("  [Y/n]: ");
    }
    // Flush prompt before blocking on stdin
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    let answer = buf.trim().to_ascii_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes" | ""))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_block_no_ansi_when_color_off() {
        // Should only contain printable ASCII + box-drawing chars, no ESC sequences.
        let rendered = render_code_block("rust", "fn main() {}");
        assert!(
            !rendered.contains('\x1b'),
            "unexpected ANSI in: {rendered:?}"
        );
        assert!(rendered.contains("┌"), "missing top border");
        assert!(rendered.contains("└"), "missing bottom border");
        assert!(
            rendered.contains("    fn main() {}"),
            "missing indented code"
        );
    }

    #[test]
    fn render_markdown_indents_code_fence() {
        let md = "Here is some code:\n\n```rust\nfn foo() {}\n```\n\nDone.";
        let out = render_markdown(md);
        assert!(
            out.contains("    fn foo() {}"),
            "code not indented in:\n{out}"
        );
        assert!(!out.contains("```"), "raw fence leaked into output:\n{out}");
    }

    #[test]
    fn render_markdown_no_fence_leakage_in_prose() {
        let md = "# Overview\n\nThis plan has **bold** text and `inline code`.\n\n- task one\n- [x] complete\n- [ ] pending";
        let out = render_markdown(md);
        assert!(!out.contains("```"), "fence in prose output");
        // Heading normalized to uppercase
        assert!(out.to_uppercase().contains("OVERVIEW"), "heading missing");
        assert!(out.contains("✓"), "checked item marker missing");
        assert!(out.contains("○"), "unchecked item marker missing");
    }

    #[test]
    fn render_inline_code_leaves_plain_text_unchanged() {
        assert_eq!(render_inline_code("hello world", false), "hello world");
    }

    #[test]
    fn render_inline_code_strips_backtick_pair() {
        let out = render_inline_code("`foo` and `bar`", false);
        // Color off → backticks disappear, text preserved
        assert_eq!(out, "foo and bar");
    }
}
