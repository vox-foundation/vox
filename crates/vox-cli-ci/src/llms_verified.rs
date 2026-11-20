//! `vox ci llms-verified` — fast pre-push twin of the TRUST-02 test in
//! `docs-astro/tests/build-output/llms.spec.ts`.
//!
//! Every `https://voxlang.org/...` link in the hand-maintained
//! `docs/src/.well-known/llms.txt` must resolve to a page whose `state` is
//! `verified` in `contracts/reports/docs-freshness/ledger.v1.json`, unless it sits
//! under the `## Needs review (not yet verified)` heading. A link absent from the
//! ledger always fails. Keep the two rules identical; this is a pure JSON join and
//! needs no site build.
//!
//! `--write` moves non-verified entries into the Needs-review section (tagging the
//! line with `<!-- from: <section> -->`) and moves them back once verified.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

const LLMS: &str = "docs/src/.well-known/llms.txt";
const LEDGER: &str = "contracts/reports/docs-freshness/ledger.v1.json";
const REVIEW_HEADING: &str = "## Needs review (not yet verified)";
const ORIGIN_OPEN: &str = " <!-- from: ";
const ORIGIN_CLOSE: &str = " -->";

#[derive(Debug, PartialEq, Eq)]
pub struct Offender {
    pub label: String,
    pub url: String,
    pub state: String,
}

/// github-slugger `slug()` per path segment, with its per-instance dedupe counter.
fn doc_slug(rel: &str) -> String {
    let rel = rel.replace('\\', "/");
    let lower = rel.to_lowercase();
    let no_ext = ["mdx", "md"]
        .iter()
        .find_map(|e| lower.strip_suffix(&format!(".{e}")))
        .unwrap_or(&lower);
    let mut seen: HashMap<String, u32> = HashMap::new();
    let segs: Vec<String> = no_ext
        .split('/')
        .map(|seg| {
            let base: String = seg
                .chars()
                .filter(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | ' '))
                .collect::<String>()
                .replace(' ', "-");
            let mut slug = base.clone();
            while seen.contains_key(&slug) {
                let n = seen.get_mut(&base).map(|n| {
                    *n += 1;
                    *n
                });
                slug = format!("{base}-{}", n.unwrap_or(1));
            }
            seen.insert(slug.clone(), 0);
            slug
        })
        .collect();
    let slug = segs.join("/");
    if slug == "index" {
        String::new()
    } else {
        slug.strip_suffix("/index")
            .map(str::to_string)
            .unwrap_or(slug)
    }
}

/// Ledger `state` for a `https://voxlang.org/...` URL; `None` = not in the ledger.
fn state_of(ledger: &Value, url: &str) -> Option<String> {
    let clean = url
        .strip_prefix("https://voxlang.org/")?
        .split('#')
        .next()?
        .split('?')
        .next()?
        .trim_matches('/')
        .to_string();
    for (k, page) in ledger.get("pages")?.as_object()? {
        let rel = k.strip_prefix("docs/src/").unwrap_or(k);
        let k_clean = rel
            .strip_suffix(".mdx")
            .or_else(|| rel.strip_suffix(".md"))
            .unwrap_or(rel);
        if doc_slug(rel) == clean || k_clean == clean || k == &clean {
            return Some(
                page.get("state")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            );
        }
    }
    None
}

/// `(label, url)` for every voxlang.org markdown link on a line (same regex as the TS test).
fn links(line: &str) -> Vec<(String, String)> {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());
    re.captures_iter(line)
        .map(|c| (c[1].to_string(), c[2].trim().to_string()))
        .filter(|(_, u)| u.starts_with("https://voxlang.org/"))
        .collect()
}

fn in_review(heading: &Option<String>) -> bool {
    heading.as_deref() == Some(REVIEW_HEADING)
}

/// Offenders: missing-from-ledger anywhere; non-verified outside the Needs-review section.
pub fn check(llms: &str, ledger: &Value) -> Vec<Offender> {
    let mut out = Vec::new();
    let mut heading: Option<String> = None;
    for line in llms.lines() {
        if line.starts_with("## ") {
            heading = Some(line.trim_end().to_string());
        }
        for (label, url) in links(line) {
            match state_of(ledger, &url) {
                None => out.push(Offender {
                    label,
                    url,
                    state: "missing from ledger".into(),
                }),
                Some(s) if s != "verified" && !in_review(&heading) => out.push(Offender {
                    label,
                    url,
                    state: s,
                }),
                _ => {}
            }
        }
    }
    out
}

fn entry_states(line: &str, ledger: &Value) -> Vec<Option<String>> {
    links(line)
        .iter()
        .map(|(_, u)| state_of(ledger, u))
        .collect()
}

/// Move non-verified curated entries into the Needs-review section and verified ones back.
pub fn rewrite(llms: &str, ledger: &Value) -> String {
    let mut lines: Vec<String> = llms.lines().map(str::to_string).collect();
    let mut generated = false;
    let mut heading: Option<String> = None;
    let mut move_out: Vec<(usize, String)> = Vec::new(); // (line idx, origin heading)
    let mut move_back: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("<!-- BEGIN GENERATED") {
            generated = true;
        }
        if line.contains("<!-- END GENERATED") {
            generated = false;
        }
        if line.starts_with("## ") {
            heading = Some(line.trim_end().to_string());
        }
        let states = entry_states(line, ledger);
        if generated || states.is_empty() || states.iter().any(Option::is_none) {
            continue; // generated block, no link, or unknown page: leave for `check` to report
        }
        let all_verified = states.iter().all(|s| s.as_deref() == Some("verified"));
        if in_review(&heading) {
            if all_verified && line.contains(ORIGIN_OPEN) {
                move_back.push(i);
            }
        } else if !all_verified {
            let origin = heading.clone().unwrap_or_default();
            move_out.push((i, origin.trim_start_matches("## ").to_string()));
        }
    }
    // Move back first (indices stay valid when removing from the end).
    let mut back: Vec<(String, String)> = Vec::new(); // (origin, clean line)
    for &i in move_back.iter().rev() {
        let l = lines.remove(i);
        let at = l.rfind(ORIGIN_OPEN).unwrap_or(l.len());
        let origin = l[at + ORIGIN_OPEN.len()..]
            .trim_end_matches(ORIGIN_CLOSE)
            .to_string();
        back.push((origin, l[..at].to_string()));
    }
    back.reverse();
    // Recompute out-indices after removals above (removed lines lie in the review section,
    // which is after nothing we move out of except itself), so shift by removed-before count.
    let mut out_lines: Vec<String> = Vec::new();
    for (i, origin) in move_out.iter().rev() {
        let shift = move_back.iter().filter(|&&b| b < *i).count();
        let l = lines.remove(*i - shift);
        out_lines.push(format!("{l}{ORIGIN_OPEN}{origin}{ORIGIN_CLOSE}"));
    }
    out_lines.reverse();
    for (origin, l) in back {
        insert_at_section_end(&mut lines, &format!("## {origin}"), l);
    }
    if !out_lines.is_empty() {
        if !lines.iter().any(|l| l.trim_end() == REVIEW_HEADING) {
            while lines.last().is_some_and(|l| l.trim().is_empty()) {
                lines.pop();
            }
            lines.push(String::new());
            lines.push(REVIEW_HEADING.to_string());
        }
        for l in out_lines {
            insert_at_section_end(&mut lines, REVIEW_HEADING, l);
        }
    }
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

/// Insert after the last non-blank line of the section headed `heading` (no-op if absent).
fn insert_at_section_end(lines: &mut Vec<String>, heading: &str, line: String) {
    let Some(start) = lines.iter().position(|l| l.trim_end() == heading) else {
        return;
    };
    let mut end = start + 1;
    while end < lines.len() && !lines[end].starts_with("## ") {
        end += 1;
    }
    let mut at = end;
    while at > start + 1 && lines[at - 1].trim().is_empty() {
        at -= 1;
    }
    lines.insert(at, line);
}

pub fn run(root: &Path, write: bool) -> Result<()> {
    let llms_path = root.join(LLMS);
    let llms = std::fs::read_to_string(&llms_path).with_context(|| format!("read {LLMS}"))?;
    let ledger: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(LEDGER)).with_context(|| format!("read {LEDGER}"))?,
    )
    .context("parse ledger")?;
    let text = if write {
        let new = rewrite(&llms, &ledger);
        if new != llms {
            std::fs::write(&llms_path, &new)?;
            println!("llms-verified: rewrote {LLMS}");
        }
        new
    } else {
        llms
    };
    let bad = check(&text, &ledger);
    if bad.is_empty() {
        println!("llms-verified: ok");
        return Ok(());
    }
    let mut msg = format!(
        "llms-verified: {} curated link(s) not verified (run `vox ci llms-verified --write`):",
        bad.len()
    );
    for o in bad.iter().take(10) {
        msg.push_str(&format!("\n  {} <{}> -> {}", o.label, o.url, o.state));
    }
    if bad.len() > 10 {
        msg.push_str(&format!("\n  ... and {} more", bad.len() - 10));
    }
    bail!(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ledger(a: &str, b: &str) -> Value {
        json!({"pages": {
            "docs/src/reference/cli.md": {"state": a},
            "docs/src/how-to/Guide Two.md": {"state": b},
            "docs/src/index.md": {"state": "verified"},
        }})
    }
    const LLMS_OK: &str = "# T\n\n## Main\n- [CLI](https://voxlang.org/reference/cli/): x\n- [G](https://voxlang.org/how-to/guide-two/): y\n";

    #[test]
    fn passes_when_all_verified() {
        assert!(check(LLMS_OK, &ledger("verified", "verified")).is_empty());
    }

    #[test]
    fn fails_listing_only_offenders() {
        let bad = check(LLMS_OK, &ledger("verified", "drifting"));
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].label, "G");
        assert_eq!(bad[0].state, "drifting");
    }

    #[test]
    fn missing_from_ledger_always_fails_even_in_review() {
        let t = format!("{LLMS_OK}\n{REVIEW_HEADING}\n- [N](https://voxlang.org/nope/)\n");
        let bad = check(&t, &ledger("verified", "verified"));
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].state, "missing from ledger");
    }

    #[test]
    fn review_section_tolerates_non_verified() {
        let t = format!("{LLMS_OK}\n{REVIEW_HEADING}\n- [R](https://voxlang.org/reference/cli/)\n");
        assert!(check(&t, &ledger("drifting", "verified")).len() == 1); // main-section CLI link still bad
        let t2 = "## Main\n\n## Needs review (not yet verified)\n- [R](https://voxlang.org/reference/cli/)\n";
        assert!(check(t2, &ledger("drifting", "verified")).is_empty());
    }

    #[test]
    fn root_and_slug_resolution() {
        let l = ledger("verified", "verified");
        assert_eq!(
            state_of(&l, "https://voxlang.org/").as_deref(),
            Some("verified")
        );
        assert_eq!(
            state_of(&l, "https://voxlang.org/how-to/guide-two/#a").as_deref(),
            Some("verified")
        );
        assert_eq!(
            doc_slug("architecture/qwen-3.7-eval.md"),
            "architecture/qwen-37-eval"
        );
        assert_eq!(doc_slug("tutorials/index.md"), "tutorials");
    }

    #[test]
    fn write_moves_out_and_tags_origin() {
        let out = rewrite(LLMS_OK, &ledger("verified", "drifting"));
        assert!(out.contains("- [CLI](https://voxlang.org/reference/cli/): x\n"));
        assert!(out.contains(&format!(
            "\n{REVIEW_HEADING}\n- [G](https://voxlang.org/how-to/guide-two/): y{ORIGIN_OPEN}Main{ORIGIN_CLOSE}\n"
        )));
        assert!(check(&out, &ledger("verified", "drifting")).is_empty());
    }

    #[test]
    fn write_moves_back_when_verified() {
        let moved = rewrite(LLMS_OK, &ledger("verified", "drifting"));
        let back = rewrite(&moved, &ledger("verified", "verified"));
        assert_eq!(
            back.trim_end(),
            LLMS_OK.trim_end().to_string() + "\n\n" + REVIEW_HEADING
        );
    }

    #[test]
    fn write_is_idempotent_and_preserves_other_lines() {
        let l = ledger("verified", "drifting");
        let once = rewrite(LLMS_OK, &l);
        assert_eq!(rewrite(&once, &l), once);
        assert!(once.starts_with("# T\n\n## Main\n- [CLI]"));
    }

    #[test]
    fn run_reads_files_and_write_fixes() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("docs/src/.well-known")).unwrap();
        std::fs::create_dir_all(root.join("contracts/reports/docs-freshness")).unwrap();
        std::fs::write(root.join(LLMS), LLMS_OK).unwrap();
        std::fs::write(
            root.join(LEDGER),
            ledger("verified", "drifting").to_string(),
        )
        .unwrap();
        let err = run(root, false).unwrap_err().to_string();
        assert!(
            err.contains("drifting") && err.contains("<https://voxlang.org/how-to/guide-two/>")
        );
        run(root, true).unwrap();
        run(root, false).unwrap();
    }
}
