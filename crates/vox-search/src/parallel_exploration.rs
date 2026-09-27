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
    use super::CancellationToken;

    #[test]
    fn cancellation_is_shared_across_clones() {
        let token = CancellationToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        assert!(clone.is_cancelled());
    }
}
