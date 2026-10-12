//! Per-run LLM usage metering (METER-01).
//!
//! Every research-stage LLM call reports its `LlmResponse` through
//! [`meter_response`] (or [`meter_failure`] when the cascade gave up). The
//! pipeline wraps a run in [`scope`], so calls made anywhere inside it —
//! including `buffered` claim verification on the same task — accumulate into
//! one [`RunUsage`]. Calls outside a scope are not metered. Work moved onto a
//! separately spawned task leaves the scope and is not metered either.
//!
//! The scope is also one trace, and [`tag_candidates`] stamps the run's session
//! onto each call so LLM telemetry rows name the research session.

use std::cell::RefCell;

use serde::{Deserialize, Serialize};
use vox_actor_runtime::llm::{LlmConfig, LlmResponse};

/// Time to first token for a run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "ms", rename_all = "snake_case")]
pub enum Ttft {
    /// No streamed call ran, so there is no first token to time. Non-streaming
    /// responses report whole-response latency as `ttft_ms`, which is not a TTFT.
    #[default]
    NotApplicable,
    /// TTFT of the first streamed call in the run.
    Measured(u64),
}

/// Token, cost, latency, and step counts summed over a run's LLM calls.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunUsage {
    pub llm_calls: u32,
    /// Calls whose cascade returned an error; their token use is unknown.
    pub failed_llm_calls: u32,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_read_tokens: u64,
    /// Sum over calls that reported a cost; see `calls_with_unknown_cost`.
    pub cost_usd: f64,
    pub calls_with_unknown_cost: u32,
    /// Sum of per-call latency; calls can overlap, so this may exceed run duration.
    pub llm_latency_ms: u64,
    pub tool_calls: u32,
    pub ttft: Ttft,
}

impl RunUsage {
    /// Adds one successful call. `streamed` says whether `response.ttft_ms` is a
    /// real first-token time.
    pub fn record(&mut self, response: &LlmResponse, streamed: bool) {
        self.llm_calls = self.llm_calls.saturating_add(1);
        self.prompt_tokens += u64::from(response.prompt_tokens);
        self.completion_tokens += u64::from(response.completion_tokens);
        self.cache_read_tokens += u64::from(response.cache_read_tokens);
        match response.cost_usd {
            Some(cost) => self.cost_usd += cost,
            None => self.calls_with_unknown_cost = self.calls_with_unknown_cost.saturating_add(1),
        }
        self.llm_latency_ms = self.llm_latency_ms.saturating_add(response.latency_ms);
        let tool_calls = response.tool_calls.as_ref().map_or(0, Vec::len);
        self.tool_calls = self
            .tool_calls
            .saturating_add(u32::try_from(tool_calls).unwrap_or(u32::MAX));
        if streamed
            && self.ttft == Ttft::NotApplicable
            && let Some(ms) = response.ttft_ms
        {
            self.ttft = Ttft::Measured(ms);
        }
    }

    pub fn record_failure(&mut self) {
        self.failed_llm_calls = self.failed_llm_calls.saturating_add(1);
    }
}

#[derive(Default)]
struct RunState {
    usage: RunUsage,
    session_label: Option<String>,
}

tokio::task_local! {
    static RUN_STATE: RefCell<RunState>;
}

/// Runs `fut` with a fresh usage accumulator inside one trace.
pub async fn scope<F: std::future::Future>(fut: F) -> F::Output {
    vox_actor_runtime::llm::run_trace_scope(RUN_STATE.scope(RefCell::new(RunState::default()), fut))
        .await
}

/// Usage recorded so far in the current scope; default outside one.
pub fn snapshot() -> RunUsage {
    RUN_STATE
        .try_with(|state| state.borrow().usage.clone())
        .unwrap_or_default()
}

/// Records a successful non-streaming call into the current scope.
pub fn meter_response(response: &LlmResponse) {
    let _ = RUN_STATE.try_with(|state| state.borrow_mut().usage.record(response, false));
}

/// Records a call whose cascade failed into the current scope.
pub fn meter_failure() {
    let _ = RUN_STATE.try_with(|state| state.borrow_mut().usage.record_failure());
}

/// Names the run's session for LLM telemetry attribution.
pub fn set_run_session(label: String) {
    let _ = RUN_STATE.try_with(|state| state.borrow_mut().session_label = Some(label));
}

/// Attributes `candidates` to the current run's session unless a caller already did.
pub fn tag_candidates(candidates: &mut [LlmConfig]) {
    let Ok(Some(label)) = RUN_STATE.try_with(|state| state.borrow().session_label.clone()) else {
        return;
    };
    for candidate in candidates
        .iter_mut()
        .filter(|c| c.telemetry_session_id.is_none())
    {
        candidate.telemetry_session_id = Some(label.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(cost_usd: Option<f64>, tool_calls: usize) -> LlmResponse {
        let calls: Vec<_> = (0..tool_calls)
            .map(|i| serde_json::json!({"id": format!("c{i}"), "name": "f", "arguments": null}))
            .collect();
        serde_json::from_value(serde_json::json!({
            "content": "x",
            "prompt_tokens": 100,
            "completion_tokens": 20,
            "model": "m",
            "cost_usd": cost_usd,
            "tool_calls": (tool_calls > 0).then_some(calls),
            "latency_ms": 300,
            "cache_read_tokens": 5,
            "ttft_ms": 300,
        }))
        .expect("LlmResponse fixture")
    }

    #[test]
    fn record_sums_tokens_cost_latency_and_tool_calls() {
        let mut usage = RunUsage::default();
        usage.record(&response(Some(0.25), 2), false);
        usage.record(&response(None, 0), false);
        assert_eq!(usage.llm_calls, 2);
        assert_eq!(usage.prompt_tokens, 200);
        assert_eq!(usage.completion_tokens, 40);
        assert_eq!(usage.cache_read_tokens, 10);
        assert_eq!(usage.cost_usd, 0.25);
        assert_eq!(usage.calls_with_unknown_cost, 1);
        assert_eq!(usage.llm_latency_ms, 600);
        assert_eq!(usage.tool_calls, 2);
    }

    #[test]
    fn non_streaming_latency_is_not_reported_as_ttft() {
        let mut usage = RunUsage::default();
        usage.record(&response(None, 0), false);
        assert_eq!(usage.ttft, Ttft::NotApplicable);
        usage.record(&response(None, 0), true);
        assert_eq!(usage.ttft, Ttft::Measured(300));
    }

    #[test]
    fn failures_are_counted_without_inventing_tokens() {
        let mut usage = RunUsage::default();
        usage.record_failure();
        assert_eq!(usage.failed_llm_calls, 1);
        assert_eq!(usage.llm_calls, 0);
        assert_eq!(usage.prompt_tokens, 0);
    }

    #[tokio::test]
    async fn calls_inside_a_scope_accumulate_and_outside_are_ignored() {
        meter_response(&response(Some(1.0), 0));
        let inside = scope(async {
            meter_response(&response(Some(1.0), 0));
            meter_failure();
            snapshot()
        })
        .await;
        assert_eq!(inside.llm_calls, 1);
        assert_eq!(inside.failed_llm_calls, 1);
        assert_eq!(snapshot(), RunUsage::default());
    }

    fn candidate(session: Option<&str>) -> LlmConfig {
        LlmConfig {
            telemetry_session_id: session.map(str::to_string),
            ..LlmConfig::ollama("m")
        }
    }

    #[tokio::test]
    async fn candidates_carry_the_run_session_not_anon() {
        let tagged = scope(async {
            set_run_session("research_session:42".into());
            let mut candidates = vec![candidate(None), candidate(Some("caller"))];
            tag_candidates(&mut candidates);
            candidates
        })
        .await;
        assert_eq!(
            tagged[0].telemetry_session_id.as_deref(),
            Some("research_session:42")
        );
        assert_eq!(tagged[1].telemetry_session_id.as_deref(), Some("caller"));

        let mut outside = vec![candidate(None)];
        tag_candidates(&mut outside);
        assert_eq!(outside[0].telemetry_session_id, None);
    }

    #[test]
    fn usage_roundtrips_and_defaults_from_empty_json() {
        let mut usage = RunUsage::default();
        usage.record(&response(Some(0.5), 1), true);
        let back: RunUsage =
            serde_json::from_str(&serde_json::to_string(&usage).expect("ser")).expect("de");
        assert_eq!(back, usage);
        assert_eq!(
            serde_json::from_str::<RunUsage>("{}").expect("de"),
            RunUsage::default()
        );
        assert_eq!(
            serde_json::to_value(Ttft::NotApplicable).expect("ser"),
            serde_json::json!({"kind": "not_applicable"})
        );
    }
}
