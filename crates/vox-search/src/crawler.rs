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
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            let Ok(resolved) = base.join(href) else {
                continue;
            };
            let normalized = normalize_crawl_url(&resolved);
            let normalized_str = normalized.to_string();

            if !normalized_str.starts_with(allow_origin) {
                continue;
            }
            let lower = normalized_str.to_lowercase();
            if EXCLUDED_EXTENSIONS.iter().any(|ext| lower.ends_with(ext)) {
                continue;
            }
            if !links.contains(&normalized_str) {
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
    let allow_origin = root_parsed.origin().ascii_serialization();

    queue.push_back((root_url.to_string(), 0usize));
    visited.insert(root_url.to_string());

    while let Some((current_url, depth)) = queue.pop_front() {
        if results.len() >= max_pages {
            break;
        }

        match fetch_and_extract(&current_url, timeout_ms).await {
            Ok(doc) => {
                results.push(doc);

                if depth < max_depth {
                    // Extract links from candidate
                    let client = vox_http_client::client_builder()
                        .timeout(std::time::Duration::from_millis(timeout_ms))
                        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
                        .build()?;
                    if let Ok(resp) = client.get(&current_url).send().await {
                        if let Ok(html) = resp.text().await {
                            let candidates =
                                extract_candidate_links(&html, &current_url, &allow_origin);
                            let prioritized = score_and_prioritize_links(&candidates);

                            for (link, _) in prioritized {
                                if !visited.contains(&link) && visited.len() < max_pages * 3 {
                                    visited.insert(link.clone());
                                    queue.push_back((link, depth + 1));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(url = %current_url, error = %e, "Failed to crawl link");
            }
        }
    }

    Ok(results)
}
