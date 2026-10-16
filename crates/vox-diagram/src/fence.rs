//! Markdown fenced-block scanner shared by every diagram consumer (doc lint, render manifest,
//! CLI, MCP). Built on `pulldown-cmark` so bodies agree with the docs renderer on indentation,
//! list/blockquote prefixes and nesting.
//!
//! Known limitation: MDX JSX blocks are HTML to CommonMark, so fences nested inside JSX are
//! best-effort.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// One fenced code block whose info string matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    /// Concatenated content lines with container indentation / `>` prefixes stripped.
    pub body: String,
    /// 1-based line of the opening fence.
    pub open_line: u32,
    /// Raw byte range of the content lines in the input.
    pub body_range: Range<usize>,
    /// Whether a closing fence was found before EOF.
    pub closed: bool,
}

/// Every fenced block in `md` whose trimmed info string equals `info` exactly.
pub fn extract(md: &str, info: &str) -> Vec<Fence> {
    let parser = Parser::new_ext(md, Options::ENABLE_YAML_STYLE_METADATA_BLOCKS).into_offset_iter();
    let mut out = Vec::new();
    // (range of the whole block, body so far) while inside a matching fence.
    let mut cur: Option<(Range<usize>, String)> = None;
    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(i))) if i.trim() == info => {
                cur = Some((range, String::new()));
            }
            Event::Text(t) => {
                if let Some((_, body)) = cur.as_mut() {
                    body.push_str(&t);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((block, body)) = cur.take() {
                    out.push(finish(md, block, body));
                }
            }
            _ => {}
        }
    }
    out
}

fn finish(md: &str, block: Range<usize>, body: String) -> Fence {
    let text = &md[block.clone()];
    let open_line = u32::try_from(md[..block.start].matches('\n').count() + 1).unwrap_or(u32::MAX);
    let first = text.trim_start_matches([' ', '\t', '>']);
    let fence_char = first.chars().next().unwrap_or('`');
    let open_len = first.chars().take_while(|&c| c == fence_char).count();
    let body_start = text.find('\n').map_or(text.len(), |i| i + 1);
    let trimmed = text.trim_end();
    let last_start = trimmed.rfind('\n').map_or(0, |i| i + 1);
    let last = trimmed[last_start..].trim_start_matches([' ', '\t', '>']);
    // An opener-only block has a single line; a closer needs its own line.
    let closed = last_start > 0
        && last_start >= body_start
        && last.len() >= open_len
        && last.chars().all(|c| c == fence_char);
    let end = if closed { last_start } else { text.len() };
    Fence {
        body,
        open_line,
        body_range: block.start + body_start.min(end)..block.start + end,
        closed,
    }
}

/// CRLF and lone CR to LF, then trailing whitespace trimmed (Rust half of the manifest key).
pub fn normalize(body: &str) -> String {
    body.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_basic_fence() {
        let md = "# T\n\n```vox-diagram\ngraph {}\n```\n";
        let f = extract(md, "vox-diagram");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].body, "graph {}\n");
        assert_eq!(f[0].open_line, 3);
        assert_eq!(&md[f[0].body_range.clone()], "graph {}\n");
        assert!(f[0].closed);
    }

    #[test]
    fn info_matches_exactly() {
        let md = "```vox\na\n```\n\n```vox-diagram,x\nb\n```\n\n```mermaid\nc\n```\n\n```  vox-diagram  \nd\n```\n";
        let f = extract(md, "vox-diagram");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].body, "d\n");
    }

    #[test]
    fn unclosed_fence_runs_to_eof() {
        let md = "```vox-diagram\ngraph {}\n";
        let f = extract(md, "vox-diagram");
        assert_eq!(f.len(), 1);
        assert!(!f[0].closed);
        assert_eq!(f[0].body_range.end, md.len());
    }

    #[test]
    fn normalize_crlf_and_trailing_blank() {
        assert_eq!(normalize("a\r\nb\r\n\r\n"), "a\nb");
        assert_eq!(normalize("a\rb\r"), "a\nb");
    }
}
