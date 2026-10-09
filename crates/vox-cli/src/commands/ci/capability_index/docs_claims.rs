//! Frontmatter `capabilities:` claims (D-06): which capability ids a docs page explains.
//!
//! Frontmatter that is malformed but makes no `capabilities` claim is skipped (the doc lint owns
//! the general frontmatter rules, RESEARCH Pitfall 10). A page that does write a `capabilities:`
//! key the index cannot read fails the generator with the page path: the lint never parses YAML,
//! so skipping it silently would lose the claim with no diagnostic.

use anyhow::{Result, bail};
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

/// Parse a page's frontmatter (a leading BOM is ignored). `Ok(None)` when there is no
/// frontmatter, or it is not valid for the keys the index reads and makes no capability claim.
/// A page that writes a `capabilities` key the index cannot read is an error naming `path`:
/// dropping it silently would drift the index with no diagnostic anywhere (the doc lint is a
/// line scanner that never parses YAML).
pub(crate) fn parse_meta(path: &str, content: &str) -> Result<Option<PageMeta>> {
    let text = content.strip_prefix('\u{feff}').unwrap_or(content);
    let claims = |s: &str| s.lines().any(|l| l.starts_with("capabilities:"));
    let Some(yaml) = split_frontmatter(text) else {
        if text.starts_with("---") && claims(text) {
            bail!("{path}: frontmatter declares `capabilities` but has no closing `---` line");
        }
        return Ok(None);
    };
    match serde_yaml::from_str(yaml) {
        Ok(meta) => Ok(Some(meta)),
        Err(e) if claims(yaml) => bail!(
            "{path}: frontmatter `capabilities` is unreadable (expected a list of capability ids): {e}"
        ),
        Err(_) => Ok(None),
    }
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
            let m = parse_meta("p.md", page).expect("ok").expect("meta");
            assert_eq!(m.capabilities, vec!["cli.a", "mcp.b"]);
            assert_eq!(m.title.as_deref(), Some("T"));
        }
    }

    #[test]
    fn unreadable_capabilities_claim_is_an_error_naming_the_page() {
        for page in [
            "---\ntitle: T\ncapabilities: cli.secrets\n---\n",
            "---\ntitle: [unclosed\ncapabilities: {\n---\n",
            "---\ntitle: T\ncapabilities: [cli.a]\n",
        ] {
            let e = parse_meta("docs/src/x.md", page).unwrap_err().to_string();
            assert!(
                e.contains("docs/src/x.md") && e.contains("capabilities"),
                "{e}"
            );
        }
    }

    #[test]
    fn malformed_frontmatter_without_a_claim_is_skipped() {
        assert!(
            parse_meta("p.md", "---\ntitle: [unclosed\n---\n")
                .unwrap()
                .is_none()
        );
        assert!(
            parse_meta("p.md", "no frontmatter\ncapabilities: x\n")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn bom_before_frontmatter_is_read() {
        let page = "\u{feff}---\ntitle: T\ncapabilities: [cli.a]\n---\n";
        let m = parse_meta("p.md", page).unwrap().expect("meta");
        assert_eq!(m.capabilities, vec!["cli.a"]);
    }

    #[test]
    fn crlf_is_normalized() {
        let page = "---\r\ntitle: T\r\ncapabilities: [cli.a]\r\n---\r\nbody\r\n";
        let m = parse_meta("p.md", page).unwrap().expect("meta");
        assert_eq!(m.capabilities, vec!["cli.a"]);
        assert_eq!(m.title.as_deref(), Some("T"));
    }
}
