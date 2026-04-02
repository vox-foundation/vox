//! Durable workflow execution tracker trait and no-op default.

use serde_json::Value;

/// An engine tracker that allows the interpreted runner to persist durable states.
pub trait WorkflowTracker: Send + Sync {
    /// Check if a specific step was already completed in a prior, durable run.
    fn is_activity_completed(
        &self,
        _workflow_name: &str,
        _activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<bool>> + Send {
        async { Ok(false) }
    }

    /// Load the stored result payload for a completed durable step, when available.
    fn load_activity_result(
        &self,
        _workflow_name: &str,
        _activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<Value>>> + Send {
        async { Ok(None) }
    }

    /// Called when the workflow plan begins.
    fn on_workflow_started(
        &mut self,
        _workflow_name: &str,
        _plan_len: usize,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Called when an activity starts execution.
    fn on_activity_started(
        &mut self,
        _workflow_name: &str,
        _activity_name: &str,
        _activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Called when one activity attempt starts under the execution boundary.
    fn on_activity_attempt_started(
        &mut self,
        _workflow_name: &str,
        _activity_name: &str,
        _activity_id: &str,
        _attempt: u32,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Called when one activity attempt fails.
    fn on_activity_attempt_failed(
        &mut self,
        _workflow_name: &str,
        _activity_name: &str,
        _activity_id: &str,
        _attempt: u32,
        _error: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Called when one activity attempt succeeds.
    fn on_activity_attempt_completed(
        &mut self,
        _workflow_name: &str,
        _activity_name: &str,
        _activity_id: &str,
        _attempt: u32,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Return the next attempt number to use for this activity.
    fn next_activity_attempt_start(
        &self,
        _workflow_name: &str,
        _activity_name: &str,
        _activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<u32>> + Send {
        async { Ok(1) }
    }

    /// Called when an activity fully completes.
    fn on_activity_completed(
        &mut self,
        _workflow_name: &str,
        _activity_name: &str,
        _activity_id: &str,
        _result: &Value,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Called when the workflow successfully completes all steps.
    fn on_workflow_completed(
        &mut self,
        _workflow_name: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }
}

/// A default no-op tracker used if none is provided.
#[derive(Default)]
pub struct DefaultTracker;

impl WorkflowTracker for DefaultTracker {}
