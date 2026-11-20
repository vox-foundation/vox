//! `vox ci readme-assets` — README / docs index may only embed raster images.
//!
//! The docs site rewrites `<img src>` but not `srcset`, and README diagrams must be
//! raster (.webp/.png), never .svg. Strict by design: any `srcset=`, `<source`, or an
//! embedded `.svg` image (`<img src=...svg` or `![](...svg)`) fails. Plain links to
//! .svg files are fine.

use anyhow::{Result, bail};
use regex::Regex;
use std::path::Path;

const FILES: [&str; 2] = ["README.md", "docs/src/index.mdx"];

pub fn violations(name: &str, text: &str) -> Vec<String> {
    let rules = [
        (r"(?i)srcset\s*=", "srcset= (site does not rewrite it)"),
        (r"(?i)<source\b", "<source> tag"),
        (
            r#"(?i)<img\b[^>]*?\bsrc\s*=\s*["'][^"']*\.svg(?:[?#][^"']*)?["']"#,
            ".svg <img>",
        ),
        (
            r"(?i)!\[[^\]]*\]\([^)\s]*\.svg(?:[?#][^)\s]*)?(?:\s+[^)]*)?\)",
            ".svg markdown image",
        ),
    ];
    let mut out = Vec::new();
    for (re, what) in rules {
        let re = Regex::new(re).unwrap();
        for m in re.find_iter(text) {
            let line = text[..m.start()].matches('\n').count() + 1;
            out.push(format!("{name}:{line}: {what}"));
        }
    }
    out
}

pub fn run(root: &Path) -> Result<()> {
    let mut bad = Vec::new();
    for f in FILES {
        // A missing or unreadable file must fail the guard, not pass it silently.
        let t = std::fs::read_to_string(root.join(f))
            .map_err(|e| anyhow::anyhow!("readme-assets: cannot read {f}: {e}"))?;
        bad.extend(violations(f, &t));
    }
    if bad.is_empty() {
        println!("readme-assets: ok");
        return Ok(());
    }
    bail!(
        "readme-assets: use raster <img src=...webp|png> only:\n  {}",
        bad.join("\n  ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_raster_passes() {
        let t = "<img src=\"a.webp\" alt=\"x\" />\n![d](b.png)\n[spec](flow.svg)\n";
        assert!(violations("R", t).is_empty());
    }

    #[test]
    fn flags_picture_srcset_source_and_svg() {
        let t =
            "<picture>\n<source srcset=\"a.svg\">\n<img src=\"a.svg\">\n</picture>\n![d](x.svg)\n";
        let v = violations("R", t);
        assert!(v.iter().any(|s| s.contains("srcset")));
        assert!(v.iter().any(|s| s.contains("<source>")));
        assert!(v.iter().any(|s| s.contains(".svg <img>")));
        assert!(v.iter().any(|s| s.contains(".svg markdown image")));
        assert!(v.iter().any(|s| s.starts_with("R:3:")));
    }

    #[test]
    fn run_checks_files_in_root() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("docs/src")).unwrap();
        std::fs::write(d.path().join("docs/src/index.mdx"), "plain").unwrap();
        std::fs::write(d.path().join("README.md"), "<img src=\"x.webp\">").unwrap();
        run(d.path()).unwrap();
        std::fs::write(d.path().join("README.md"), "<img src=\"x.svg\">").unwrap();
        assert!(run(d.path()).is_err());
    }

    #[test]
    fn run_fails_when_a_checked_file_is_missing() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("README.md"), "<img src=\"x.webp\">").unwrap();
        // docs/src/index.mdx is absent: must not report ok.
        let err = run(d.path()).expect_err("missing index.mdx must fail the guard");
        assert!(err.to_string().contains("docs/src/index.mdx"), "{err}");
    }
}
