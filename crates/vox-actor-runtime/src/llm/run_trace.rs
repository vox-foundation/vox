//! One trace per logical run.
//!
//! `infer_with_retry` stamps each call with the ambient `TRACE_CTX` trace id and
//! mints a fresh one when none is active, so without a scope every call in a
//! multi-stage run lands in its own trace.

use std::future::Future;

/// Runs `fut` under the caller's trace context, or under a fresh one when none is
/// active, so every LLM call inside shares one `trace_id`.
pub async fn run_trace_scope<F: Future>(fut: F) -> F::Output {
    match vox_telemetry::current_trace_context() {
        Some(_) => fut.await,
        None => {
            vox_telemetry::TRACE_CTX
                .scope(vox_telemetry::TraceContext::default(), fut)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_telemetry::{TRACE_CTX, TraceContext, current_trace_ctx};

    #[tokio::test]
    async fn calls_in_one_run_share_a_trace_id() {
        let (first, second) =
            run_trace_scope(async { (current_trace_ctx().trace_id, current_trace_ctx().trace_id) })
                .await;
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn separate_runs_get_separate_trace_ids() {
        let a = run_trace_scope(async { current_trace_ctx().trace_id }).await;
        let b = run_trace_scope(async { current_trace_ctx().trace_id }).await;
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn an_outer_trace_is_kept() {
        let outer = TraceContext::root(7);
        let inner = TRACE_CTX
            .scope(
                outer.clone(),
                run_trace_scope(async { current_trace_ctx() }),
            )
            .await;
        assert_eq!(inner.trace_id, outer.trace_id);
        assert_eq!(inner.task_id, Some(7));
    }
}
