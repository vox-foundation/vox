use crate::scraper::{ScrapedDocument, fetch_and_extract};
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use url::Url;

const EXCLUDED_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".pdf", ".zip", ".tar", ".gz", ".exe",
    ".dmg", ".css", ".js", ".map", ".ico", ".woff", ".woff2",
];

pub fn normalize_crawl_url(url: &Url) -> Url {
    let mut clean = url.clone();
    clean.set_fragment(None);
    clean.set_query(None);
    clean
}

pub fn extract_candidate_links(html: &str, base_url: &str, allow_origin: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let Ok(base) = Url::parse(base_url) else {
        return Vec::new();
    };

    let mut links = Vec::new();
    let mut seen = HashSet::new();
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            let Ok(resolved) = base.join(href) else {
                continue;
            };
            let normalized = normalize_crawl_url(&resolved);
            let normalized_str = normalized.to_string();

            if normalized.origin().ascii_serialization() != allow_origin {
                continue;
            }
            let lower = normalized_str.to_lowercase();
            if EXCLUDED_EXTENSIONS.iter().any(|ext| lower.ends_with(ext)) {
                continue;
            }
            if seen.insert(normalized_str.clone()) {
                links.push(normalized_str);
            }
        }
    }
    links
}

pub fn score_and_prioritize_links(links: &[String]) -> Vec<(String, u32)> {
    let mut scored: Vec<(String, u32)> = links
        .iter()
        .map(|link| {
            let lower = link.to_lowercase();
            let mut score = 10u32;
            if lower.contains("struct.")
                || lower.contains("fn.")
                || lower.contains("trait.")
                || lower.contains("class.")
                || lower.contains("api")
            {
                score += 50;
            }
            if lower.contains("spec")
                || lower.contains("architecture")
                || lower.contains("guide")
                || lower.contains("internals")
            {
                score += 30;
            }
            (link.clone(), score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
}

pub async fn crawl_domain_depth(
    root_url: &str,
    max_depth: usize,
    max_pages: usize,
    timeout_ms: u64,
) -> anyhow::Result<Vec<ScrapedDocument>> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut results = Vec::new();

    let root_parsed = Url::parse(root_url)?;
    let normalized_root = normalize_crawl_url(&root_parsed);
    let allow_origin = normalized_root.origin().ascii_serialization();
    let start_url = normalized_root.to_string();

    queue.push_back((start_url.clone(), 0usize));
    visited.insert(start_url);

    while let Some((current_url, depth)) = queue.pop_front() {
        if results.len() >= max_pages {
            break;
        }

        match fetch_and_extract(&current_url, timeout_ms).await {
            Ok(doc) => {
                if depth < max_depth {
                    if let Some(html) = doc.raw_html.as_deref() {
                        let candidates = extract_candidate_links(html, &current_url, &allow_origin);
                        let prioritized = score_and_prioritize_links(&candidates);

                        for (link, _) in prioritized.into_iter().take(15) {
                            if visited.insert(link.clone()) {
                                queue.push_back((link, depth + 1));
                            }
                        }
                    }
                }

                results.push(doc);
            }
            Err(e) => {
                tracing::warn!(url = %current_url, error = %e, "Failed to crawl link");
            }
        }
    }

    Ok(results)
}
