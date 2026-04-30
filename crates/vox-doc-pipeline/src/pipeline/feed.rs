//! RSS `feed.xml` generation for mdBook output.

use std::fs;
use std::path::Path;

use super::types::Page;

const FEED_BASE_URL: &str = "https://vox-lang.org";
const CHANGELOG_URL: &str = "https://vox-lang.org/changelog.html";

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
fn build_date_rfc822() -> String {
    if let Ok(epoch_str) = std::env::var("SOURCE_DATE_EPOCH") {
        if let Ok(epoch_secs) = epoch_str.trim().parse::<u64>() {
            let secs_per_day: u64 = 86_400;
            let days_since_epoch = epoch_secs / secs_per_day;
            let time_of_day = epoch_secs % secs_per_day;
            let h = time_of_day / 3600;
            let mins = (time_of_day % 3600) / 60;
            let s = time_of_day % 60;
            let jd = days_since_epoch as i64 + 2_440_588;
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
            let dow_idx = (days_since_epoch + 4) % 7;
            let dow = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][dow_idx as usize % 7];
            return format!("{dow}, {day:02} {month_str} {year} {h:02}:{mins:02}:{s:02} GMT");
        }
        if let Some(date_part) = epoch_str.trim().split('T').next()
            && let Some(rfc) = iso_to_rfc822(date_part)
        {
            return rfc;
        }
    }
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

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Trim a description to at most `max_chars` characters, breaking only at word
/// boundaries.  Appends `…` when the text is actually shortened.
fn trim_description(s: &str, max_chars: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    // Walk backwards from the limit to find the last whitespace boundary.
    let truncated: String = s.chars().take(max_chars).collect();
    let trimmed = match truncated.rfind(|c: char| c.is_whitespace()) {
        Some(pos) => truncated[..pos].trim_end(),
        None => truncated.trim_end(),
    };
    // Strip any trailing punctuation that looks odd before the ellipsis.
    let trimmed = trimmed.trim_end_matches(|c| c == ',' || c == ';');
    format!("{trimmed}\u{2026}")
}

/// Generate `docs/src/feed.xml` from pages that have `last_updated`.
pub(crate) fn generate_feed(docs_src: &Path, pages: &[Page]) {
    const MAX_ITEMS: usize = 25;

    let mut dated: Vec<&Page> = pages
        .iter()
        .filter(|p| {
            p.last_updated.is_some()
                && !matches!(p.status.as_deref(), Some("deprecated") | Some("legacy"))
                && !p.path.contains("-ARCHIVED")
                && !p.path.starts_with("architecture/research-")
                && !p.path.starts_with("architecture/planning-meta/")
                && !p.path.starts_with("ci/")
                && !p.path.starts_with("operations/")
        })
        .collect();
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
    xml.push_str("  <title>Vox Language Updates</title>\n");
    xml.push_str(&format!("  <link>{FEED_BASE_URL}/</link>\n"));
    xml.push_str("  <description>Changelog, release notes, and documentation updates for the Vox AI-native programming language, maintained by the Vox Foundation.</description>\n");
    xml.push_str("  <language>en-us</language>\n");
    xml.push_str(&format!("  <lastBuildDate>{build_date}</lastBuildDate>\n"));
    xml.push_str(&format!(
        "  <atom:link href=\"{FEED_BASE_URL}/feed.xml\" rel=\"self\" type=\"application/rss+xml\" />\n"
    ));
    xml.push('\n');

    for page in &dated {
        let slug = page.path.trim_end_matches(".md").replace('\\', "/");
        let url = format!("{FEED_BASE_URL}/{slug}.html");
        let title = xml_escape(&page.title);
        let raw_desc = page.description.as_deref().unwrap_or(&page.title);
        let description = xml_escape(&trim_description(raw_desc, 160));
        let pub_date = page
            .last_updated
            .as_deref()
            .and_then(iso_to_rfc822)
            .unwrap_or_else(|| build_date.clone());

        let feed_category =
            if page.path.starts_with("tutorials/") || page.path.starts_with("journeys/") {
                "Tutorial"
            } else if page.path.starts_with("how-to/") {
                "How-To"
            } else if page.path.starts_with("explanation/") {
                "Explanation"
            } else if page.path.starts_with("reference/") || page.path.starts_with("api/") {
                "Reference"
            } else {
                "Documentation"
            };

        xml.push_str("  <item>\n");
        xml.push_str(&format!("    <title>{title}</title>\n"));
        xml.push_str(&format!("    <link>{url}</link>\n"));
        xml.push_str(&format!("    <guid isPermaLink=\"true\">{url}</guid>\n"));
        xml.push_str(&format!("    <description>{description}</description>\n"));
        xml.push_str(&format!("    <pubDate>{pub_date}</pubDate>\n"));
        xml.push_str(&format!("    <category>{feed_category}</category>\n"));
        if let Some(st) = page.schema_type.as_deref() {
            let st = st.trim();
            if !st.is_empty() {
                xml.push_str(&format!(
                    "    <category domain=\"https://schema.org\">{}</category>\n",
                    xml_escape(st)
                ));
            }
        }
        xml.push_str("  </item>\n\n");
    }

    xml.push_str(&format!(
        r#"  <item>
    <title>v0.8.0 — @require, @pure, @deprecated Decorators; 10 LSP Features</title>
    <link>{changelog_url}</link>
    <guid>{changelog_url}#v0.8.0</guid>
    <description>Added @require, @pure, and @deprecated decorators. Implemented 10 Language Server Protocol features including hover, go-to-definition, and inline diagnostics.</description>
    <pubDate>Thu, 26 Feb 2026 00:00:00 GMT</pubDate>
  </item>

  <item>
    <title>v0.7.0 — QLoRA Training Pipeline; Socrates Anti-Hallucination Protocol</title>
    <link>{changelog_url}</link>
    <guid>{changelog_url}#v0.7.0</guid>
    <description>Native QLoRA fine-tuning via Candle and qlora-rs. Socrates confidence protocol integrated into the orchestrator for anti-hallucination validation of agent outputs.</description>
    <pubDate>Mon, 03 Feb 2026 00:00:00 GMT</pubDate>
  </item>

  <item>
    <title>v0.6.0 — Mens Transport; Durable Workflow Runtime MVP</title>
    <link>{changelog_url}</link>
    <guid>{changelog_url}#v0.6.0</guid>
    <description>CPU-first mens registry with optional HTTP control plane. Interpreted workflow runtime MVP supporting local and mens activity hooks.</description>
    <pubDate>Thu, 15 Jan 2026 00:00:00 GMT</pubDate>
  </item>
"#,
        changelog_url = CHANGELOG_URL
    ));

    xml.push_str("</channel>\n</rss>\n");

    let feed_path = docs_src.join("feed.xml");
    fs::write(&feed_path, xml).expect("Failed to write feed.xml");
    println!(
        "Successfully generated feed.xml with {} item(s).",
        dated.len()
    );
}
