//! Tavily `/extract` uplift when search snippets are too thin for grounding.

#[cfg(feature = "tavily")]
use tracing::{info, warn};

/// Heuristic: snippet is too short or mostly non-alphanumeric noise for reliable grounding.
#[must_use]
pub fn snippet_quality_low(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.chars().count() < 80 {
        return true;
    }
    let alnum_or_space = trimmed
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .count();
    let total = trimmed.chars().count().max(1);
    alnum_or_space * 2 < total
}

#[derive(Debug, Clone)]
pub struct ExtractHit {
    pub url: String,
    pub content: String,
}

/// Replace thin `content` fields on the kept search rows via Tavily `/extract` (fail-open).
///
/// Charged to `budget` (1 credit per 5 URLs, Tavily's basic extract rate); skipped when
/// the budget cannot cover it. Callers pass only the rows they will keep, so no credit
/// is spent on a row the final cap would drop.
#[cfg(feature = "tavily")]
pub async fn uplift_low_quality_snippets(
    results: &mut [crate::searxng::SearxngResult],
    max_urls: usize,
    client: &crate::tavily::TavilyClient,
    budget: &crate::tavily_budget::TavilySessionBudget,
) {
    let urls: Vec<String> = results
        .iter()
        .filter(|r| snippet_quality_low(&r.content))
        .take(max_urls.max(1))
        .map(|r| r.url.clone())
        .collect();
    if urls.is_empty() {
        return;
    }
    if !budget.try_consume(urls.len().div_ceil(5)) {
        warn!("tavily extract uplift skipped: session credit budget exhausted");
        return;
    }
    match client.extract(&urls).await {
        Ok(extracted) => {
            info!(count = extracted.len(), "tavily extract uplift succeeded");
            for hit in extracted {
                if let Some(row) = results.iter_mut().find(|r| r.url == hit.url)
                    && !hit.content.trim().is_empty()
                {
                    row.content = hit.content;
                    row.engine = Some(
                        row.engine
                            .clone()
                            .map(|e| format!("{e}+tavily_extract"))
                            .unwrap_or_else(|| "tavily_extract".to_string()),
                    );
                }
            }
        }
        Err(e) => warn!(error = %e, "tavily extract uplift failed (fail-open)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_quality_low_flags_empty_and_short() {
        assert!(snippet_quality_low(""));
        assert!(snippet_quality_low("short"));
        assert!(!snippet_quality_low(
            "This is a sufficiently long snippet with enough alphanumeric content to pass the quality gate for grounding."
        ));
    }

    #[test]
    fn snippet_quality_low_flags_markup_heavy() {
        assert!(snippet_quality_low(&"<>[]{}".repeat(30)));
    }
}
