//! Pilot intervention handlers (`park_agent`, `steer_agent`, `fork_agent`, `swap_agent_compute`)
//! for [`BulletinBoard`].

use vox_orchestrator_types::concilium::{
    ComputeTier, ConciliumPilotActionInput, ConciliumPilotActionKind, ConciliumPilotActionResult,
    SwarmWorkerStatus, SwarmWorkspaceMode,
};

use super::BulletinBoard;
use super::concilium_board::{MAX_LANE_SPANS, default_lane};

fn default_model_for_tier(tier: ComputeTier) -> &'static str {
    match tier {
        ComputeTier::LocalMensGpu => "vox-mens-local",
        ComputeTier::PopuliQuicPeer => "populi-quic-peer",
        ComputeTier::InlineCpu => "inline-cpu",
        ComputeTier::CloudBandit => vox_config::model_defaults::PREMIUM_CODEGEN,
    }
}

impl BulletinBoard {
    /// Phase 454 (CONC-03): Apply a pilot steering action onto the `#seq` board and lane state.
    pub fn apply_pilot_action(
        &self,
        input: &ConciliumPilotActionInput,
    ) -> ConciliumPilotActionResult {
        let key = input.agent_id.trim().trim_start_matches('@').to_string();
        if key.is_empty() {
            return ConciliumPilotActionResult {
                ok: false,
                seq: 0,
                message: "agent_id must not be empty".to_string(),
                forked_agent_id: None,
            };
        }
        let ch = input.channel.as_deref().unwrap_or("#main");
        let post = |body: String, tier, mode, lock, badge: &str| {
            self.post_board_entry(
                ch,
                "pilot",
                &body,
                tier,
                mode,
                lock,
                None,
                Some(badge.into()),
                false,
            )
            .seq
        };
        match input.action {
            ConciliumPilotActionKind::ParkAgent => {
                let seq = post(
                    format!("Pilot parked worker {key} at 0 tok/s"),
                    ComputeTier::InlineCpu,
                    SwarmWorkspaceMode::Inherit,
                    None,
                    "PARK 0 tok/s",
                );
                let _ = self.park_worker(&key, &key, Some(ch));
                ConciliumPilotActionResult {
                    ok: true,
                    seq,
                    message: format!("Parked @{key} at 0 tok/s"),
                    forked_agent_id: None,
                }
            }
            ConciliumPilotActionKind::SteerAgent => {
                let d = input
                    .directive
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Inspect #seq board.");
                let already_mentions = Self::extract_mentions(d)
                    .iter()
                    .any(|m| m.trim_start_matches('@').eq_ignore_ascii_case(&key));
                let body = if already_mentions {
                    d.to_string()
                } else {
                    format!("@{key} {d}")
                };
                let seq = post(
                    body,
                    ComputeTier::InlineCpu,
                    SwarmWorkspaceMode::Inherit,
                    None,
                    "PILOT STEER",
                );
                ConciliumPilotActionResult {
                    ok: true,
                    seq,
                    message: format!("Steered @{key} via #seq-{seq}"),
                    forked_agent_id: None,
                }
            }
            ConciliumPilotActionKind::ForkAgent => {
                let mode = input.workspace_mode.unwrap_or(SwarmWorkspaceMode::Share);
                let explicit_fork = input
                    .fork_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim_start_matches('@').to_string());
                if let Some(ref fname) = explicit_fork
                    && fname.eq_ignore_ascii_case(&key)
                {
                    return ConciliumPilotActionResult {
                        ok: false,
                        seq: 0,
                        message: "fork_name must differ from source agent_id".to_string(),
                        forked_agent_id: None,
                    };
                }
                let mut lanes = self.concilium.lanes.write();
                let fork_id = explicit_fork.unwrap_or_else(|| {
                    let base = format!("{key}-fork");
                    if !lanes.contains_key(&base) {
                        base
                    } else {
                        let mut idx = 2usize;
                        loop {
                            let cand = format!("{key}-fork-{idx}");
                            if !lanes.contains_key(&cand) {
                                break cand;
                            }
                            idx += 1;
                        }
                    }
                });
                let mut f = lanes
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| default_lane(&fork_id, &fork_id));
                f.agent_id = fork_id.clone();
                f.roster_name = fork_id.clone();
                f.role = format!("fork:{key}");
                f.status = SwarmWorkerStatus::Working;
                f.workspace_mode = mode;
                f.parked_since_seq = None;
                f.tok_per_sec = 38.0;
                f.kv_cache_hit_pct = Some(94.5);
                f.current_action = format!("warm-forked from @{key} ({})", mode.as_str());
                if f.spans.len() > MAX_LANE_SPANS {
                    let excess = f.spans.len() - MAX_LANE_SPANS;
                    f.spans.drain(..excess);
                }
                let (tier, lock) = (f.compute_tier, f.file_lock_prefix.clone());
                lanes.insert(fork_id.clone(), f);
                drop(lanes);
                let seq = post(
                    format!("Forked @{key} -> @{fork_id} [{}]", mode.as_str()),
                    tier,
                    mode,
                    lock,
                    &format!("FORK {}", mode.as_str()),
                );
                ConciliumPilotActionResult {
                    ok: true,
                    seq,
                    message: format!("Forked @{key} -> @{fork_id}"),
                    forked_agent_id: Some(fork_id),
                }
            }
            ConciliumPilotActionKind::SwapAgentCompute => {
                let tier = input.target_tier.unwrap_or(ComputeTier::LocalMensGpu);
                let node_id = input
                    .target_node_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| tier.as_str().to_string());
                let explicit_model = input
                    .target_model_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                {
                    let mut lanes = self.concilium.lanes.write();
                    let lane = lanes
                        .entry(key.clone())
                        .or_insert_with(|| default_lane(&key, &key));
                    lane.compute_tier = tier;
                    lane.node_id = node_id.clone();
                    if let Some(m) = explicit_model {
                        lane.model_id = m;
                    } else if lane.model_id.is_empty() {
                        lane.model_id = default_model_for_tier(tier).to_string();
                    }
                    lane.current_action = format!("migrated to {} ({node_id})", tier.as_str());
                }
                let seq = post(
                    format!(
                        "Swapped @{key} compute tier -> {} ({node_id})",
                        tier.as_str()
                    ),
                    tier,
                    SwarmWorkspaceMode::Inherit,
                    None,
                    &format!("SWAP {}", tier.as_str()),
                );
                ConciliumPilotActionResult {
                    ok: true,
                    seq,
                    message: format!("Swapped @{key} to {}", tier.as_str()),
                    forked_agent_id: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_orchestrator_types::concilium::SequenceBoardFilter;

    #[test]
    fn pilot_actions_handle_steer_prefix_self_fork_and_model_preservation() {
        let board = BulletinBoard::new(16);
        let empty = board.apply_pilot_action(&ConciliumPilotActionInput {
            action: ConciliumPilotActionKind::ParkAgent,
            agent_id: "   ".into(),
            channel: None,
            directive: None,
            fork_name: None,
            workspace_mode: None,
            target_tier: None,
            target_node_id: None,
            target_model_id: None,
        });
        assert!(!empty.ok);

        let steer = board.apply_pilot_action(&ConciliumPilotActionInput {
            action: ConciliumPilotActionKind::SteerAgent,
            agent_id: "parser".into(),
            channel: Some("#parser".into()),
            directive: Some("sync with @parser-b".into()),
            fork_name: None,
            workspace_mode: None,
            target_tier: None,
            target_node_id: None,
            target_model_id: None,
        });
        assert!(steer.ok);
        let entries = board.read_board(&SequenceBoardFilter::default());
        assert!(entries[0].body.starts_with("@parser "));

        let self_fork = board.apply_pilot_action(&ConciliumPilotActionInput {
            action: ConciliumPilotActionKind::ForkAgent,
            agent_id: "parser".into(),
            channel: None,
            directive: None,
            fork_name: Some("Parser".into()),
            workspace_mode: None,
            target_tier: None,
            target_node_id: None,
            target_model_id: None,
        });
        assert!(!self_fork.ok);

        let swap1 = board.apply_pilot_action(&ConciliumPilotActionInput {
            action: ConciliumPilotActionKind::SwapAgentCompute,
            agent_id: "parser".into(),
            channel: None,
            directive: None,
            fork_name: None,
            workspace_mode: None,
            target_tier: Some(ComputeTier::LocalMensGpu),
            target_node_id: Some("local-gpu".into()),
            target_model_id: Some("qwen3.5-9b".into()),
        });
        assert!(swap1.ok);
        let swap2 = board.apply_pilot_action(&ConciliumPilotActionInput {
            action: ConciliumPilotActionKind::SwapAgentCompute,
            agent_id: "parser".into(),
            channel: None,
            directive: None,
            fork_name: None,
            workspace_mode: None,
            target_tier: Some(ComputeTier::PopuliQuicPeer),
            target_node_id: Some("iroh-2".into()),
            target_model_id: None,
        });
        assert!(swap2.ok);
        let (lanes, _) = board.lanes_and_arcs();
        let lane = lanes.iter().find(|l| l.agent_id == "parser").expect("lane");
        assert_eq!(lane.model_id, "qwen3.5-9b");
    }
}
