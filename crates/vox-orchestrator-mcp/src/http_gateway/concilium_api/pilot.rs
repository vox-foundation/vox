//! Pilot action execution, durable board persistence, and stamped event emission
//! for `/api/v2/concilium/action`.

use axum::Json;
use serde_json::Value;
use vox_orchestrator::bulletin::BulletinBoard;
use vox_orchestrator::events::AgentEventKind;
use vox_orchestrator::types::AgentId;
use vox_orchestrator_types::concilium::{
    CausalSwarmStamp, ComputeTier, ConciliumPilotActionInput, ConciliumPilotActionKind,
    SequenceBoardFilter, SwarmWorkspaceMode,
};

use super::to_json;
use crate::http_gateway::GatewayState;
use crate::services::routes::err;

fn resolve_agent_id(gs: &GatewayState, raw: &str) -> AgentId {
    let key = raw.trim().trim_start_matches('@');
    if let Ok(n) = key.parse::<u64>() {
        return AgentId(n);
    }
    if let Some(n) = gs
        .server_state
        .orchestrator
        .topology_snapshot()
        .nodes
        .into_iter()
        .find(|n| n.name.eq_ignore_ascii_case(key))
    {
        return n.agent_id;
    }
    AgentId(vox_orchestrator::orchestrator::dispatch::stable_hash(key))
}

async fn persist_seq_entry(
    gs: &GatewayState,
    entry: &vox_orchestrator_types::concilium::SequenceBoardEntry,
) {
    if let Some(ref db) = gs.server_state.db {
        let repo_id = &gs.server_state.repository.repository_id;
        if let Err(error) = db.append_concilium_seq_entry(repo_id, entry).await {
            let seq = entry.seq;
            tracing::warn!(seq, %error, "failed to persist concilium seq entry");
        }
    }
}

pub(super) async fn execute_pilot_action(
    gs: &GatewayState,
    input: ConciliumPilotActionInput,
) -> Json<Value> {
    let key = input.agent_id.trim().trim_start_matches('@').to_string();
    if key.is_empty() {
        return err("bad_request", "agent_id must not be empty");
    }
    let orch = &gs.server_state.orchestrator;
    let res = orch.bulletin().apply_pilot_action(&input);
    if !res.ok {
        return err("bad_request", &res.message);
    }

    let stamp = if res.seq > 0
        && let Some(entry) = orch
            .bulletin()
            .read_board(&SequenceBoardFilter {
                seqs: Some(vec![res.seq]),
                limit: Some(1),
                ..Default::default()
            })
            .into_iter()
            .next()
    {
        persist_seq_entry(gs, &entry).await;
        let stamp = CausalSwarmStamp {
            swarm_seq: entry.seq,
            channel: entry.channel.clone(),
            traceparent: entry.traceparent.clone(),
            span_id: entry.span_id.clone(),
            compute_tier: entry.compute_tier,
            file_lock_prefix: entry.file_lock_prefix.clone(),
            workspace_mode: entry.workspace_mode,
            parked_since_seq: entry.parked_since_seq,
        };
        orch.event_bus().emit_stamped(
            AgentEventKind::SequenceBoardPosted {
                seq: entry.seq,
                channel: entry.channel,
                sender: entry.sender,
                mentions: entry.mentions,
            },
            stamp.clone(),
        );
        stamp
    } else {
        CausalSwarmStamp {
            swarm_seq: res.seq,
            channel: BulletinBoard::normalize_channel(input.channel.as_deref().unwrap_or("#main")),
            traceparent: String::new(),
            span_id: String::new(),
            compute_tier: input.target_tier.unwrap_or(ComputeTier::InlineCpu),
            file_lock_prefix: None,
            workspace_mode: input.workspace_mode.unwrap_or(SwarmWorkspaceMode::Inherit),
            parked_since_seq: None,
        }
    };

    let agent_id = resolve_agent_id(gs, &key);
    let channel = BulletinBoard::normalize_channel(input.channel.as_deref().unwrap_or("#main"));
    let kind = match input.action {
        ConciliumPilotActionKind::ParkAgent => AgentEventKind::AgentParked {
            agent_id,
            parked_since_seq: res.seq,
            channel,
        },
        ConciliumPilotActionKind::SteerAgent => AgentEventKind::AgentWoken {
            agent_id,
            woken_by_seq: res.seq,
            mention: format!("@{key}"),
        },
        ConciliumPilotActionKind::ForkAgent => {
            let forked = res
                .forked_agent_id
                .clone()
                .unwrap_or_else(|| format!("{key}-fork"));
            AgentEventKind::AgentSpawned {
                agent_id: resolve_agent_id(gs, &forked),
                name: forked,
            }
        }
        ConciliumPilotActionKind::SwapAgentCompute => AgentEventKind::AgentComputeSwapped {
            agent_id,
            target_tier: input.target_tier.unwrap_or(ComputeTier::LocalMensGpu),
            target_node_id: input.target_node_id.clone(),
            target_model_id: input.target_model_id.clone(),
        },
    };
    orch.event_bus().emit_stamped(kind, stamp);

    let _ = gs.topic_tx.send(super::super::scientia_feed::TopicMessage {
        topic: "concilium.delta".into(),
        data: serde_json::json!({
            "seq": res.seq,
            "action": input.action,
            "agent_id": input.agent_id,
            "forked_agent_id": res.forked_agent_id,
            "message": res.message,
        }),
    });
    to_json(&res)
}
