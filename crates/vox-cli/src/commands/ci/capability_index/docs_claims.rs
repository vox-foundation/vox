//! Frontmatter `capabilities:` claims (D-06): which capability ids a docs page explains.
//!
//! Malformed frontmatter yields `None` (page skipped): the doc lint owns that error, so the
//! generator never fails on it (RESEARCH Pitfall 10).

use serde::Deserialize;

/// The frontmatter keys the index reads; every other key is ignored.
#[derive(Debug, Default, Clone, Deserialize)]
pub(crate) struct PageMeta {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

// vox:defactored-from vox-doc-pipeline 2026-10-08
/// The YAML between the opening `---` line and the next line that is exactly `---`.
pub(crate) fn split_frontmatter(content: &str) -> Option<&str> {
    let mut lines = content.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return None;
    }
    let start = first.len();
    let mut end = start;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Some(&content[start..end]);
        }
        end += line.len();
    }
    None
}

/// Parse a page's frontmatter; `None` when absent or not valid YAML.
pub(crate) fn parse_meta(content: &str) -> Option<PageMeta> {
    let yaml = split_frontmatter(content)?;
    serde_yaml::from_str(yaml).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_exact_delimiter_lines() {
        let page = "---\ntitle: \"a --- b\"\ndescription: x---y\n---\nbody\n---\n";
        let yaml = split_frontmatter(page).expect("frontmatter");
        assert!(yaml.contains("a --- b") && yaml.contains("x---y"), "{yaml}");
        assert!(!yaml.contains("body"));
        assert!(split_frontmatter("no frontmatter\n").is_none());
        assert!(split_frontmatter("---\ntitle: unterminated\n").is_none());
    }

    #[test]
    fn reads_flow_and_block_lists() {
        let flow = "---\ntitle: T\ncapabilities: [cli.a, mcp.b]\n---\n";
        let block = "---\ntitle: T\ncapabilities:\n  - cli.a\n  - mcp.b\n---\n";
        for page in [flow, block] {
            let m = parse_meta(page).expect("meta");
            assert_eq!(m.capabilities, vec!["cli.a", "mcp.b"]);
            assert_eq!(m.title.as_deref(), Some("T"));
        }
    }

    #[test]
    fn malformed_frontmatter_is_skipped() {
        assert!(parse_meta("---\ntitle: [unclosed\ncapabilities: {\n---\n").is_none());
        assert!(parse_meta("---\ncapabilities: not-a-list\n---\n").is_none());
    }

    #[test]
    fn crlf_is_normalized() {
        let page = "---\r\ntitle: T\r\ncapabilities: [cli.a]\r\n---\r\nbody\r\n";
        let m = parse_meta(page).expect("meta");
        assert_eq!(m.capabilities, vec!["cli.a"]);
        assert_eq!(m.title.as_deref(), Some("T"));
    }
}
