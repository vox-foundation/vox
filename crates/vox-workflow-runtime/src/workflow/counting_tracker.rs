//! Fault-injecting tracker wrapper for crash-window tests.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::anyhow;
use serde_json::Value;

use super::tracker::WorkflowTracker;

/// Wraps a tracker and fails after a configured number of successful calls.
pub struct CountingTracker<T> {
    inner: T,
    fail_after: usize,
    calls: AtomicUsize,
}

impl<T> CountingTracker<T> {
    /// Create a tracker that permits `fail_after` successful write calls.
    pub fn new(inner: T, fail_after: usize) -> Self {
        Self {
            inner,
            fail_after,
            calls: AtomicUsize::new(0),
        }
    }

    fn bump(&self) -> anyhow::Result<()> {
        let calls = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        if calls > self.fail_after {
            return Err(anyhow!(
                "CountingTracker crash after {} calls",
                self.fail_after
            ));
        }
        Ok(())
    }
}

impl<T: WorkflowTracker> WorkflowTracker for CountingTracker<T> {
    fn is_activity_completed(
        &self,
        workflow_name: &str,
        activity_id: &str,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .is_activity_completed(workflow_name, activity_id)
                .await
        }
    }

    fn load_activity_result(
        &self,
        workflow_name: &str,
        activity_id: &str,
    ) -> impl Future<Output = anyhow::Result<Option<Value>>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .load_activity_result(workflow_name, activity_id)
                .await
        }
    }

    fn on_workflow_started(
        &mut self,
        workflow_name: &str,
        plan_len: usize,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .on_workflow_started(workflow_name, plan_len)
                .await
        }
    }

    fn on_activity_started(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .on_activity_started(workflow_name, activity_name, activity_id)
                .await
        }
    }

    fn on_activity_attempt_started(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
        attempt: u32,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .on_activity_attempt_started(workflow_name, activity_name, activity_id, attempt)
                .await
        }
    }

    fn on_activity_attempt_failed(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
        attempt: u32,
        error: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .on_activity_attempt_failed(
                    workflow_name,
                    activity_name,
                    activity_id,
                    attempt,
                    error,
                )
                .await
        }
    }

    fn on_activity_attempt_completed(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
        attempt: u32,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .on_activity_attempt_completed(workflow_name, activity_name, activity_id, attempt)
                .await
        }
    }

    fn next_activity_attempt_start(
        &self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> impl Future<Output = anyhow::Result<u32>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .next_activity_attempt_start(workflow_name, activity_name, activity_id)
                .await
        }
    }

    fn on_activity_completed(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
        result: &Value,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .on_activity_completed(workflow_name, activity_name, activity_id, result)
                .await
        }
    }

    fn on_workflow_completed(
        &mut self,
        workflow_name: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner.on_workflow_completed(workflow_name).await
        }
    }

    fn load_workflow_patch(
        &self,
        workflow_name: &str,
        change_id: &str,
    ) -> impl Future<Output = anyhow::Result<Option<u32>>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .load_workflow_patch(workflow_name, change_id)
                .await
        }
    }

    fn record_workflow_patch(
        &mut self,
        workflow_name: &str,
        change_id: &str,
        version: u32,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .record_workflow_patch(workflow_name, change_id, version)
                .await
        }
    }

    fn load_cached_activity_result(
        &self,
        activity_id: &str,
        arg_hash_hex: &str,
        now_unix_ms: u64,
    ) -> impl Future<Output = anyhow::Result<Option<Value>>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .load_cached_activity_result(activity_id, arg_hash_hex, now_unix_ms)
                .await
        }
    }

    fn record_cached_activity_result(
        &mut self,
        activity_id: &str,
        arg_hash_hex: &str,
        result: &Value,
        produced_at_unix_ms: u64,
        dedup_window_ms: u64,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner
                .record_cached_activity_result(
                    activity_id,
                    arg_hash_hex,
                    result,
                    produced_at_unix_ms,
                    dedup_window_ms,
                )
                .await
        }
    }

    fn persist_run_args(
        &mut self,
        args_json: &Value,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner.persist_run_args(args_json).await
        }
    }

    fn load_run_args(&self) -> impl Future<Output = anyhow::Result<Option<Value>>> + Send {
        let bump = self.bump();
        async move {
            bump?;
            self.inner.load_run_args().await
        }
    }

    fn run_status(&self) -> impl Future<Output = anyhow::Result<Option<String>>> + Send {
        self.inner.run_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::DefaultTracker;

    #[tokio::test]
    async fn fails_after_configured_successful_calls() {
        let mut tracker = CountingTracker::new(DefaultTracker, 1);
        tracker
            .on_workflow_started("pair", 0)
            .await
            .expect("first write succeeds");
        let error = tracker
            .on_workflow_completed("pair")
            .await
            .expect_err("second write crashes");
        assert_eq!(error.to_string(), "CountingTracker crash after 1 calls");
    }

    #[tokio::test]
    async fn read_hooks_count_toward_the_crash_threshold() {
        let tracker = CountingTracker::new(DefaultTracker, 0);
        let error = tracker
            .is_activity_completed("pair", "activity-1")
            .await
            .expect_err("first read crashes");
        assert_eq!(error.to_string(), "CountingTracker crash after 0 calls");
    }
}
