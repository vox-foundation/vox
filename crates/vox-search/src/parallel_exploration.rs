use futures::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchBranch {
    pub branch_id: String,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchResult {
    pub branch_id: String,
    pub query: String,
    pub snippets: Vec<String>,
    pub success: bool,
}

pub struct ParallelExplorationCoordinator;

impl ParallelExplorationCoordinator {
    pub async fn execute_branches_resilient<F, Fut>(
        branches: Vec<ResearchBranch>,
        concurrency: usize,
        timeout_per_branch: Duration,
        cancellation: Option<CancellationToken>,
        fetcher: F,
    ) -> Vec<BranchResult>
    where
        F: Fn(String) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = anyhow::Result<Vec<String>>> + Send + 'static,
    {
        let stream = futures::stream::iter(branches).map(|branch| {
            let fetcher = fetcher.clone();
            let cancel = cancellation.clone();
            async move {
                if let Some(c) = &cancel
                    && c.is_cancelled() {
                        return BranchResult {
                            branch_id: branch.branch_id,
                            query: branch.query,
                            snippets: vec![],
                            success: false,
                        };
                    }
                match tokio::time::timeout(timeout_per_branch, fetcher(branch.query.clone())).await {
                    Ok(Ok(snippets)) => BranchResult {
                        branch_id: branch.branch_id,
                        query: branch.query,
                        snippets,
                        success: true,
                    },
                    Ok(Err(err)) => {
                        tracing::warn!(branch = %branch.branch_id, error = %err, "Branch fetcher failed");
                        BranchResult {
                            branch_id: branch.branch_id,
                            query: branch.query,
                            snippets: vec![],
                            success: false,
                        }
                    }
                    Err(_) => {
                        tracing::warn!(branch = %branch.branch_id, "Branch fetcher timed out");
                        BranchResult {
                            branch_id: branch.branch_id,
                            query: branch.query,
                            snippets: vec![],
                            success: false,
                        }
                    }
                }
            }
        });

        stream.buffer_unordered(concurrency.max(1)).collect().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_token_starts_uncancelled_and_latches_on_cancel() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancellation_token_clone_shares_state() {
        let token = CancellationToken::new();
        let clone = token.clone();
        clone.cancel();
        assert!(
            token.is_cancelled(),
            "clones share the same cancellation flag"
        );
    }

    #[tokio::test]
    async fn execute_branches_resilient_reports_success_and_timeout() {
        let branches = vec![
            ResearchBranch {
                branch_id: "fast".to_string(),
                query: "q1".to_string(),
            },
            ResearchBranch {
                branch_id: "slow".to_string(),
                query: "q2".to_string(),
            },
        ];

        let results = ParallelExplorationCoordinator::execute_branches_resilient(
            branches,
            2,
            Duration::from_millis(50),
            None,
            |query| async move {
                if query == "q2" {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Ok(vec![format!("snippet-for-{query}")])
            },
        )
        .await;

        assert_eq!(results.len(), 2);
        let fast = results.iter().find(|r| r.branch_id == "fast").unwrap();
        assert!(fast.success);
        assert_eq!(fast.snippets, vec!["snippet-for-q1".to_string()]);
        let slow = results.iter().find(|r| r.branch_id == "slow").unwrap();
        assert!(!slow.success, "slow branch must time out and fail");
        assert!(slow.snippets.is_empty());
    }

    #[tokio::test]
    async fn execute_branches_resilient_skips_already_cancelled_branches() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let branches = vec![ResearchBranch {
            branch_id: "b1".to_string(),
            query: "q".to_string(),
        }];

        let results = ParallelExplorationCoordinator::execute_branches_resilient(
            branches,
            1,
            Duration::from_secs(1),
            Some(cancellation),
            |_query| async move { Ok(vec!["should not run".to_string()]) },
        )
        .await;

        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].snippets.is_empty());
    }
}
