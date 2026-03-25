//! Dynamic mdBook `SUMMARY.md` generator and documentation linter for Vox.
//!
//! ## Modes
//!
//! - Default: regenerates `docs/src/SUMMARY.md` and runs the doc linter.
//! - `--check`: validates that `SUMMARY.md` is up-to-date and all markdown docs are structurally clean; exits non-zero on failure.
//! - `--lint-only`: runs the linter without regenerating `SUMMARY.md`.
//!
//! ## Lint checks performed on every `.md` file in `docs/src/`
//!
//! 1. **Code-fence balance**: every opening ` ``` ` must have a matching closing ` ``` `.
//!    Unbalanced fences (odd counts, or fences with fewer than 3 backticks) cause mdBook to
//!    render raw text into the sidebar and page body.
//! 2. **Frontmatter presence**: files without a `---` YAML block are flagged as warnings.
//! 3. **Generic descriptions**: descriptions that match the batch-script template string
//!    `"Official documentation for ... in the Vox programming language ecosystem."` are
//!    flagged as errors — the description must be hand-written and specific.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

// ────────────────────────────────────────────────────────────────────────────
// Data structures
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct Page {
    title: String,
    path: String,
    sort_order: i32,
    /// YAML `description:` value — used in the RSS feed.
    description: Option<String>,
    /// YAML `last_updated:` value (ISO `YYYY-MM-DD`) — used to sort and date feed items.
    last_updated: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct LintError {
    file: PathBuf,
    line: usize,
    kind: LintKind,
}

#[derive(Debug)]
enum LintKind {
    /// File ends with an open code fence (odd number of ``` markers).
    UnclosedCodeFence,
    /// A code fence opener has fewer than 3 backticks (e.g. `` ` `` or ` `` `).
    ShortCodeFence { backticks: usize, at_line: usize },
    /// The YAML `description:` field contains the generic batch-script template text.
    GenericDescription,
    /// The file has no YAML frontmatter at all.
    MissingFrontmatter,
}

// ────────────────────────────────────────────────────────────────────────────
// Entry point
// ────────────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let check_mode = args.contains(&"--check".to_string());
    let lint_only = args.contains(&"--lint-only".to_string());

    let docs_src = Path::new("docs/src");
    if !docs_src.exists() {
        eprintln!("Error: docs/src/ not found. Run from repo root.");
        std::process::exit(1);
    }

    // ── Phase 1: lint all markdown files ─────────────────────────────────────
    let mut lint_errors: Vec<LintError> = Vec::new();
    collect_lint_errors(docs_src, &mut lint_errors);

    if !lint_errors.is_empty() {
        eprintln!("\n── vox-doc-pipeline: doc lint errors ──────────────────────────────");
        for e in &lint_errors {
            let rel = e.file.strip_prefix(docs_src).unwrap_or(&e.file);
            match &e.kind {
                LintKind::UnclosedCodeFence => {
                    eprintln!(
                        "  ERROR  {} — unclosed code fence (file ends with open ```)",
                        rel.display()
                    );
                }
                LintKind::ShortCodeFence { backticks, at_line } => {
                    eprintln!(
                        "  ERROR  {}:{} — code fence has only {} backtick(s); mdBook requires 3 (```)",
                        rel.display(),
                        at_line,
                        backticks
                    );
                }
                LintKind::GenericDescription => {
                    eprintln!(
                        "  ERROR  {} — description is the auto-generated template text; replace with a specific, hand-written description",
                        rel.display()
                    );
                }
                LintKind::MissingFrontmatter => {
                    eprintln!(
                        "  WARN   {} — no YAML frontmatter block; add title/description/category",
                        rel.display()
                    );
                }
            }
        }

        let hard_errors = lint_errors
            .iter()
            .filter(|e| !matches!(e.kind, LintKind::MissingFrontmatter))
            .count();
        if hard_errors > 0 {
            eprintln!(
                "\n{} hard error(s) — fix before building docs.",
                hard_errors
            );
            std::process::exit(1);
        }
        eprintln!(); // warnings only — continue
    }

    if lint_only {
        println!("Lint complete — no hard errors.");
        return;
    }

    // ── Phase 2: generate SUMMARY.md ─────────────────────────────────────────
    let mut sections: BTreeMap<String, Vec<Page>> = BTreeMap::new();
    let mut root_pages = Vec::new();
    let mut all_pages: Vec<Page> = Vec::new();

    walk_dir(
        docs_src,
        docs_src,
        &mut sections,
        &mut root_pages,
        &mut all_pages,
    );

    let mut output = String::from("# Summary\n\n");

    root_pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
    for page in root_pages {
        output.push_str(&format!("- [{}]({})\n", page.title, page.path));
    }
    output.push('\n');

    // Diátaxis-inspired section ordering
    let section_order = [
        "Getting Started",
        "Tutorials",
        "How-To Guides",
        "Language Reference",
        "API Reference \u{2014} Keywords",
        "API Reference \u{2014} Decorators",
        "API Reference \u{2014} Crates",
        "Examples",
        "Explanations",
        "Architecture Decisions (ADRs)",
        "Architecture SSOTs",
        "CI & Quality",
        "Reference",
    ];

    for section_name in section_order {
        if let Some(mut pages) = sections.remove(section_name) {
            output.push_str(&format!("# {}\n\n", section_name));
            pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
            for page in pages {
                output.push_str(&format!("- [{}]({})\n", page.title, page.path));
            }
            output.push('\n');
        }
    }

    // Emit any remaining sections not in the ordered list
    for (name, mut pages) in sections {
        output.push_str(&format!("# {}\n\n", name));
        pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
        for page in pages {
            output.push_str(&format!("- [{}]({})\n", page.title, page.path));
        }
        output.push('\n');
    }

    let summary_path = docs_src.join("SUMMARY.md");
    if check_mode {
        let current = fs::read_to_string(&summary_path).unwrap_or_default();
        if current.trim() != output.trim() {
            eprintln!(
                "SUMMARY.md is out of sync with docs/src. Run `cargo run -p vox-doc-pipeline` to update."
            );
            std::process::exit(1);
        }
        println!("vox-doc-pipeline check passed.");
    } else {
        fs::write(&summary_path, output).expect("Failed to write SUMMARY.md");
        println!("Successfully generated SUMMARY.md with all pages.");

        // ── Phase 3: generate feed.xml ────────────────────────────────────
        generate_feed(docs_src, &all_pages);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// RSS feed generator
// ────────────────────────────────────────────────────────────────────────────

/// Parse an ISO `YYYY-MM-DD` date string to RFC 822 (`Tue, 24 Mar 2026 00:00:00 GMT`).
fn iso_to_rfc822(iso: &str) -> Option<String> {
    let parts: Vec<&str> = iso.trim().split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: u32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    let month_str = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => return None,
    };
    // 0-based weekday (Zeller's congruence, simplified)
    let (m, y) = if month < 3 {
        (month + 12, year - 1)
    } else {
        (month, year)
    };
    let k = (y % 100) as i32;
    let j = (y / 100) as i32;
    let h = (day as i32 + (13 * (m as i32 + 1)) / 5 + k + k / 4 + j / 4 - 2 * j) % 7;
    let dow = match ((h + 6) % 7) as u32 {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        _ => "Sat",
    };
    Some(format!("{dow}, {day:02} {month_str} {year} 00:00:00 GMT"))
}

/// Return the current wall-clock time as an RFC 822 string.
/// Prefers the `SOURCE_DATE_EPOCH` environment variable (Unix timestamp) for
/// reproducible builds (set by GitHub Actions from the commit timestamp).
fn build_date_rfc822() -> String {
    // Try SOURCE_DATE_EPOCH first (reproducible builds / CI)
    if let Ok(epoch_str) = std::env::var("SOURCE_DATE_EPOCH") {
        // The GH Actions timestamp is ISO 8601; accept both Unix int and ISO string.
        if let Ok(epoch_secs) = epoch_str.trim().parse::<u64>() {
            // Convert Unix timestamp → rough RFC 822 (year/month/day only needed)
            let secs_per_day: u64 = 86_400;
            let days_since_epoch = epoch_secs / secs_per_day;
            // Approximate: use ISO representation by back-computing date
            let time_of_day = epoch_secs % secs_per_day;
            let h = time_of_day / 3600;
            let mins = (time_of_day % 3600) / 60;
            let s = time_of_day % 60;
            // Simple Julian Day Number conversion
            let jd = days_since_epoch as i64 + 2_440_588; // Unix epoch is JD 2440588
            let a = jd + 32044;
            let b = (4 * a + 3) / 146_097;
            let c = a - (146_097 * b) / 4;
            let d = (4 * c + 3) / 1_461;
            let e = c - (1_461 * d) / 4;
            let m = (5 * e + 2) / 153;
            let day = e - (153 * m + 2) / 5 + 1;
            let month = m + 3 - 12 * (m / 10);
            let year = 100 * b + d - 4800 + m / 10;
            let month_str = match month {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "Jan",
            };
            let dow_idx = (days_since_epoch + 4) % 7; // 1970-01-01 was Thursday (index 4)
            let dow = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][dow_idx as usize % 7];
            return format!("{dow}, {day:02} {month_str} {year} {h:02}:{mins:02}:{s:02} GMT");
        }
        // ISO 8601 timestamp from GitHub Actions (e.g. "2026-03-24T19:00:00Z")
        if let Some(date_part) = epoch_str.trim().split('T').next() {
            if let Some(rfc) = iso_to_rfc822(date_part) {
                return rfc;
            }
        }
    }
    // Fallback: current system time expressed as a fixed-format string.
    // We avoid pulling in `time`/`chrono` to keep the crate dependency-free.
    // This path is only hit locally; CI always has SOURCE_DATE_EPOCH.
    use std::time::{SystemTime, UNIX_EPOCH};
    let epoch_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days_since_epoch = epoch_secs / 86_400;
    let time_of_day = epoch_secs % 86_400;
    let (h, mins, s) = (
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60,
    );
    let jd = days_since_epoch as i64 + 2_440_588;
    let a = jd + 32044;
    let b = (4 * a + 3) / 146_097;
    let c = a - (146_097 * b) / 4;
    let d_val = (4 * c + 3) / 1_461;
    let e = c - (1_461 * d_val) / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d_val - 4800 + m / 10;
    let month_str = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "Jan",
    };
    let dow_idx = (days_since_epoch + 4) % 7;
    let dow = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][dow_idx as usize % 7];
    format!("{dow}, {day:02} {month_str} {year} {h:02}:{mins:02}:{s:02} GMT")
}

/// Escape XML special characters in a string.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Generate `docs/src/feed.xml` from all pages that have a `last_updated` field.
/// Pages are sorted newest-first; only the top 20 are included.
/// The static version-history items from the previous hand-written feed are
/// preserved as pinned items at the bottom so no existing feed entries are lost.
fn generate_feed(docs_src: &Path, pages: &[Page]) {
    const BASE_URL: &str = "https://vox-foundation.github.io/vox";
    const MAX_ITEMS: usize = 20;

    // Collect pages that have both last_updated and a description.
    let mut dated: Vec<&Page> = pages.iter().filter(|p| p.last_updated.is_some()).collect();

    // Sort newest-first by last_updated string (ISO dates sort lexicographically).
    dated.sort_by(|a, b| {
        b.last_updated
            .as_deref()
            .unwrap_or("")
            .cmp(a.last_updated.as_deref().unwrap_or(""))
    });
    dated.truncate(MAX_ITEMS);

    let build_date = build_date_rfc822();

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n\
         <rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n\
         <channel>\n",
    );
    xml.push_str(&format!("  <title>Vox Language Updates</title>\n"));
    xml.push_str(&format!("  <link>{BASE_URL}/</link>\n"));
    xml.push_str("  <description>Changelog, release notes, and documentation updates for the Vox AI-native programming language, maintained by the Vox Foundation.</description>\n");
    xml.push_str("  <language>en-us</language>\n");
    xml.push_str(&format!("  <lastBuildDate>{build_date}</lastBuildDate>\n"));
    xml.push_str(&format!(
        "  <atom:link href=\"{BASE_URL}/feed.xml\" rel=\"self\" type=\"application/rss+xml\" />\n"
    ));
    xml.push('\n');

    for page in &dated {
        // Convert the relative path (e.g. "index.md") to an HTML slug.
        let slug = page.path.trim_end_matches(".md").replace('\\', "/");
        let url = format!("{BASE_URL}/{slug}.html");
        let title = xml_escape(&page.title);
        let description = xml_escape(page.description.as_deref().unwrap_or(&page.title));
        let pub_date = page
            .last_updated
            .as_deref()
            .and_then(iso_to_rfc822)
            .unwrap_or_else(|| build_date.clone());

        xml.push_str("  <item>\n");
        xml.push_str(&format!("    <title>{title}</title>\n"));
        xml.push_str(&format!("    <link>{url}</link>\n"));
        xml.push_str(&format!("    <guid isPermaLink=\"true\">{url}</guid>\n"));
        xml.push_str(&format!("    <description>{description}</description>\n"));
        xml.push_str(&format!("    <pubDate>{pub_date}</pubDate>\n"));
        xml.push_str("  </item>\n\n");
    }

    // Append legacy hand-written release notes
    xml.push_str(
r#"  <item>
    <title>v0.8.0 — @require, @pure, @deprecated Decorators; 10 LSP Features</title>
    <link>https://vox-foundation.github.io/vox/changelog.html</link>
    <guid>https://vox-foundation.github.io/vox/changelog.html#v0.8.0</guid>
    <description>Added @require, @pure, and @deprecated decorators. Implemented 10 Language Server Protocol features including hover, go-to-definition, and inline diagnostics.</description>
    <pubDate>Thu, 26 Feb 2026 00:00:00 GMT</pubDate>
  </item>

  <item>
    <title>v0.7.0 — QLoRA Training Pipeline; Socrates Anti-Hallucination Protocol</title>
    <link>https://vox-foundation.github.io/vox/changelog.html</link>
    <guid>https://vox-foundation.github.io/vox/changelog.html#v0.7.0</guid>
    <description>Native QLoRA fine-tuning via Candle and qlora-rs. Socrates confidence protocol integrated into the orchestrator for anti-hallucination validation of agent outputs.</description>
    <pubDate>Mon, 03 Feb 2026 00:00:00 GMT</pubDate>
  </item>

  <item>
    <title>v0.6.0 — Mens Transport; Durable Workflow Runtime MVP</title>
    <link>https://vox-foundation.github.io/vox/changelog.html</link>
    <guid>https://vox-foundation.github.io/vox/changelog.html#v0.6.0</guid>
    <description>CPU-first mens registry with optional HTTP control plane. Interpreted workflow runtime MVP supporting local and mens activity hooks.</description>
    <pubDate>Thu, 15 Jan 2026 00:00:00 GMT</pubDate>
  </item>
"#);

    xml.push_str("</channel>\n</rss>\n");

    let feed_path = docs_src.join("feed.xml");
    fs::write(&feed_path, xml).expect("Failed to write feed.xml");
    println!(
        "Successfully generated feed.xml with {} item(s).",
        dated.len()
    );
}

// ────────────────────────────────────────────────────────────────────────────
// Linter
// ────────────────────────────────────────────────────────────────────────────

/// Recursively walk `dir` and collect lint errors for every `.md` file.
fn collect_lint_errors(dir: &Path, errors: &mut Vec<LintError>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_lint_errors(&path, errors);
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let rel = path.to_str().unwrap_or_default();
                if rel.contains("SUMMARY.md") {
                    continue; // generated file — skip
                }
                let content = fs::read_to_string(&path).unwrap_or_default();
                lint_file(&path, &content, errors);
            }
        }
    }
}

/// Run all lint checks on a single file's content.
fn lint_file(path: &Path, content: &str, errors: &mut Vec<LintError>) {
    let mut fence_open = false;
    let mut fence_start_line = 0_usize;

    // Check for missing frontmatter
    if !content.trim_start().starts_with("---") {
        errors.push(LintError {
            file: path.to_owned(),
            line: 1,
            kind: LintKind::MissingFrontmatter,
        });
    }

    // Check for generic batch-script descriptions
    if content.contains("Official documentation for ")
        && content.contains("in the Vox programming language ecosystem.")
    {
        errors.push(LintError {
            file: path.to_owned(),
            line: 0,
            kind: LintKind::GenericDescription,
        });
    }

    // Code-fence balance checks
    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();

        // Count leading backtick runs
        let backtick_count = trimmed.chars().take_while(|&c| c == '`').count();

        if backtick_count >= 1
            && trimmed
                .chars()
                .all(|c| c == '`' || c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
            && (trimmed == "`".repeat(backtick_count)
                || trimmed.starts_with(&"`".repeat(backtick_count)))
        {
            if backtick_count < 3 && backtick_count >= 1 {
                // This looks like it's trying to be a code fence but isn't valid
                if !fence_open {
                    errors.push(LintError {
                        file: path.to_owned(),
                        line: line_no,
                        kind: LintKind::ShortCodeFence {
                            backticks: backtick_count,
                            at_line: line_no,
                        },
                    });
                }
            } else if backtick_count >= 3 {
                if fence_open {
                    fence_open = false;
                } else {
                    fence_open = true;
                    fence_start_line = line_no;
                }
            }
        }
    }

    if fence_open {
        errors.push(LintError {
            file: path.to_owned(),
            line: fence_start_line,
            kind: LintKind::UnclosedCodeFence,
        });
    }
}

// ────────────────────────────────────────────────────────────────────────────
// SUMMARY.md builder
// ────────────────────────────────────────────────────────────────────────────

fn walk_dir(
    root: &Path,
    dir: &Path,
    sections: &mut BTreeMap<String, Vec<Page>>,
    root_pages: &mut Vec<Page>,
    all_pages: &mut Vec<Page>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(root, &path, sections, root_pages, all_pages);
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let rel_path = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/");
                if rel_path == "SUMMARY.md" {
                    continue;
                }

                let content = fs::read_to_string(&path).unwrap_or_default();
                let (title, category, sort_order, description, last_updated) =
                    parse_frontmatter(&content, &path);

                let page = Page {
                    title,
                    path: rel_path.clone(),
                    sort_order,
                    description,
                    last_updated,
                };
                all_pages.push(page);

                // Also push a lightweight copy to the section/root buckets for SUMMARY.
                let page2 = Page {
                    title: all_pages.last().unwrap().title.clone(),
                    path: rel_path,
                    sort_order: all_pages.last().unwrap().sort_order,
                    description: None,
                    last_updated: None,
                };
                if let Some(cat) = category {
                    sections.entry(cat).or_default().push(page2);
                } else {
                    root_pages.push(page2);
                }
            }
        }
    }
}

fn parse_frontmatter(
    content: &str,
    path: &Path,
) -> (String, Option<String>, i32, Option<String>, Option<String>) {
    let mut title = path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .replace('-', " ")
        .replace('_', " ");
    let mut category = None;
    let mut sort_order = 100i32;
    let mut description: Option<String> = None;
    let mut last_updated: Option<String> = None;

    if content.starts_with("---") {
        let after_dash = &content[3..];
        if let Some(end) = after_dash.find("---") {
            let yaml = &after_dash[..end];
            for line in yaml.lines() {
                let line = line.trim();
                if let Some(t) = line.strip_prefix("title:") {
                    title = t.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                } else if let Some(d) = line.strip_prefix("description:") {
                    let raw = d.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                    if !raw.is_empty() {
                        description = Some(raw);
                    }
                } else if let Some(lu) = line.strip_prefix("last_updated:") {
                    let raw = lu
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string();
                    if !raw.is_empty() {
                        last_updated = Some(raw);
                    }
                } else if let Some(c) = line.strip_prefix("category:") {
                    let cat = c.trim().trim_matches(|c| c == '"' || c == '\'');
                    category = Some(
                        match cat {
                            "getting-started" => "Getting Started",
                            "tutorial" => "Tutorials",
                            "how-to" => "How-To Guides",
                            "ref" | "reference" => "Reference",
                            "lang-ref" | "language-reference" => "Language Reference",
                            "api-keyword" => "API Reference \u{2014} Keywords",
                            "api-decorator" => "API Reference \u{2014} Decorators",
                            "api-crate" => "API Reference \u{2014} Crates",
                            "example" => "Examples",
                            "explanation" => "Explanations",
                            "adr" => "Architecture Decisions (ADRs)",
                            "architecture" | "ssot" => "Architecture SSOTs",
                            "ci" | "quality" => "CI & Quality",
                            _ => cat,
                        }
                        .to_string(),
                    );
                } else if let Some(s) = line.strip_prefix("sort_order:") {
                    sort_order = s.trim().parse().unwrap_or(100);
                }
            }
        }
    } else {
        title = title_case(&title);
    }

    (title, category, sort_order, description, last_updated)
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>()
                        + chars.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
