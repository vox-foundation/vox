//! Socrates grounding snippets and telemetry for chat / inline / ghost tools.

use serde::Deserialize;
use serde_json::Value;
use vox_socrates_policy::{ConfidencePolicy, RiskDecision};

use crate::server::ServerState;

/// JSON shape of the `socrates` field returned to MCP clients (must match [`socrates_tool_meta`]).
#[derive(Debug, Deserialize)]
pub(crate) struct SocratesJsonMeta {
    pub(crate) risk_decision: RiskDecision,
    pub(crate) confidence_estimate: f64,
    pub(crate) contradiction_ratio: f64,
}

#[must_use]
pub(crate) fn socrates_system_rider(policy: &ConfidencePolicy) -> String {
    let p = policy;
    format!(
        "\n## Socrates (grounding)\n\
         - Below {:.0}% calibrated confidence: do not speculate; state what evidence is missing.\n\
         - {:.0}–{:.0}%: answer with explicit uncertainty or ask one focused clarifying question.\n\
         - Above {:.0}%: answer normally; tie claims to files or tools you used.\n",
        p.abstain_threshold * 100.0,
        p.abstain_threshold * 100.0,
        p.ask_for_help_threshold * 100.0,
        p.ask_for_help_threshold * 100.0,
    )
}

pub(crate) fn spawn_socrates_telemetry(
    state: &ServerState,
    surface: &'static str,
    socrates_value: Value,
    model_used: Option<String>,
) {
    let Some(db) = state.db.clone() else {
        return;
    };
    let repository_id = state.repository.repository_id.clone();
    tokio::spawn(async move {
        let meta = match serde_json::from_value::<SocratesJsonMeta>(socrates_value.clone()) {
            Ok(m) => m,
            Err(e) => {
                let payload = serde_json::to_string(&socrates_value)
                    .unwrap_or_else(|_| "<non-serializable>".into());
                let snippet: String = payload.chars().take(400).collect();
                tracing::warn!(
                    surface,
                    error = %e,
                    payload_snippet = %snippet,
                    "socrates telemetry: JSON shape mismatch (must match socrates_tool_meta)"
                );
                return;
            }
        };
        match db
            .record_socrates_surface_event(
                &repository_id,
                surface,
                meta.risk_decision,
                meta.confidence_estimate,
                meta.contradiction_ratio,
                model_used.as_deref(),
            )
            .await
        {
            Ok(id) => {
                tracing::info!(
                    target: "vox_socrates_telemetry",
                    row_id = id,
                    surface,
                    repository_id = %repository_id,
                    decision = ?meta.risk_decision,
                    "persisted socrates_surface"
                );
            }
            Err(e) => tracing::warn!(
                error = %e,
                surface,
                "socrates telemetry insert failed"
            ),
        }
    });
}

#[must_use]
pub(crate) fn socrates_tool_meta(
    policy: &ConfidencePolicy,
    grounding_score: f64,
    contradiction_hint: bool,
) -> Value {
    let p = policy;
    let cr = if contradiction_hint {
        p.abstain_threshold
    } else {
        0.0_f64
    };
    let decision = p.evaluate_risk_decision(grounding_score, cr);
    serde_json::json!({
        "risk_decision": decision,
        "confidence_estimate": grounding_score,
        "contradiction_ratio": cr,
    })
}
