//! Topology enrichment and 3-tier Populi placement assembly for `/api/v2/concilium/*`.

use std::collections::HashSet;

use vox_orchestrator::types::now_unix_ms;
use vox_orchestrator_types::concilium::{
    ComputeNodeStatus, ComputeTier, ConciliumAgentLane, ConciliumSnapshotDto,
    DockedSwarmWorkerPill, FileLockCellDto, HybridComputeNodeCardDto, PopuliPlacementDto,
    SwarmWorkerStatus, SwarmWorkspaceMode,
};

use crate::http_gateway::GatewayState;

pub(super) fn build_enriched_snapshot(gs: &GatewayState) -> ConciliumSnapshotDto {
    let orch = &gs.server_state.orchestrator;
    let mut snap = orch.bulletin().concilium_snapshot();
    let topo = orch.topology_snapshot();
    let locks = orch.lock_manager().list_locks();
    for node in &topo.nodes {
        let id_str = node.agent_id.0.to_string();
        if snap.lanes.iter().any(|l| {
            l.agent_id == id_str
                || l.roster_name.eq_ignore_ascii_case(&node.name)
                || l.agent_id.eq_ignore_ascii_case(&node.name)
        }) {
            continue;
        }
        let status = if node.in_progress {
            SwarmWorkerStatus::Working
        } else if node.paused {
            SwarmWorkerStatus::Parked
        } else if node.queued > 0 {
            SwarmWorkerStatus::Waiting
        } else {
            SwarmWorkerStatus::Idle
        };
        let lock_prefix = locks
            .iter()
            .find(|(_, h, _)| *h == node.agent_id)
            .map(|(p, _, _)| p.display().to_string());
        let state_label = if node.in_progress {
            "executing"
        } else if node.paused {
            "paused"
        } else {
            "idle"
        };
        snap.lanes.push(ConciliumAgentLane {
            agent_id: id_str,
            roster_name: node.name.clone(),
            role: format!("{:?}", node.role).to_ascii_lowercase(),
            status,
            compute_tier: ComputeTier::CloudBandit,
            node_id: "cloud-bandit-router".into(),
            model_id: vox_config::model_defaults::PREMIUM_CODEGEN.into(),
            workspace_mode: SwarmWorkspaceMode::Inherit,
            file_lock_prefix: lock_prefix,
            parked_since_seq: None,
            tok_per_sec: if status == SwarmWorkerStatus::Working {
                36.0
            } else {
                0.0
            },
            used_tokens: 0,
            max_tokens: 200_000,
            kv_cache_hit_pct: None,
            current_action: format!("{state_label} ({} queued)", node.queued),
            spans: Vec::new(),
        });
    }
    snap.active_locks = snap
        .active_locks
        .max(orch.lock_manager().active_lock_count());
    snap.active_workers = snap
        .lanes
        .iter()
        .filter(|l| l.status == SwarmWorkerStatus::Working)
        .count();
    snap.parked_workers = snap
        .lanes
        .iter()
        .filter(|l| l.status == SwarmWorkerStatus::Parked)
        .count();
    snap.now_playing_ticker = format!(
        "Head #seq-{} · {} active · {} parked (0 tok/s)",
        snap.head_seq, snap.active_workers, snap.parked_workers
    );
    snap
}

fn dock_pills(
    lanes: &[ConciliumAgentLane],
    pred: impl Fn(&ConciliumAgentLane) -> bool,
) -> Vec<DockedSwarmWorkerPill> {
    lanes
        .iter()
        .filter(|l| pred(l))
        .map(|l| DockedSwarmWorkerPill {
            agent_id: l.agent_id.clone(),
            roster_name: l.roster_name.clone(),
            status: l.status,
            tok_per_sec: l.tok_per_sec,
            workspace_mode: l.workspace_mode,
            file_lock_prefix: l.file_lock_prefix.clone(),
        })
        .collect()
}

fn make_card(
    node_id: String,
    display_name: String,
    tier: ComputeTier,
    status: ComputeNodeStatus,
    gpu_summary: Option<String>,
    advertised_models: Vec<String>,
    docked_workers: Vec<DockedSwarmWorkerPill>,
) -> HybridComputeNodeCardDto {
    HybridComputeNodeCardDto {
        node_id,
        display_name,
        tier,
        status,
        host_triple: None,
        gpu_summary,
        vram_used_mb: None,
        vram_total_mb: None,
        kv_cache_warm_prefixes: None,
        rtt_ms: None,
        bandit_weight: None,
        advertised_models,
        docked_workers,
        last_seen_unix_ms: Some(now_unix_ms()),
    }
}

fn card_from_record(
    rec: &vox_populi::NodeRecord,
    tier: ComputeTier,
    display_name: String,
    rtt_ms: u64,
    docked: Vec<DockedSwarmWorkerPill>,
) -> HybridComputeNodeCardDto {
    let status = if rec.quarantined == Some(true) {
        ComputeNodeStatus::Quarantined
    } else if rec.maintenance == Some(true) {
        ComputeNodeStatus::Maintenance
    } else {
        ComputeNodeStatus::Online
    };
    let gpu = rec.gpu_model_name.clone().or_else(|| {
        if rec.capabilities.gpu_cuda {
            Some("CUDA GPU".into())
        } else if rec.capabilities.gpu_metal {
            Some("Apple Metal GPU".into())
        } else {
            Some(format!(
                "{} CPU cores",
                rec.capabilities.cpu_cores.unwrap_or(1).max(1)
            ))
        }
    });
    let models = rec.loaded_llm_models.clone().unwrap_or_else(|| {
        rec.advertised_models
            .as_ref()
            .map(|ms| ms.iter().map(|m| m.model_id.clone()).collect())
            .unwrap_or_default()
    });
    let mut card = make_card(
        rec.id.clone(),
        display_name,
        tier,
        status,
        gpu,
        models,
        docked,
    );
    card.host_triple = rec.host_triple.clone();
    card.vram_used_mb = rec
        .gpu_vram_total_mb
        .zip(rec.accelerator_free_mb)
        .map(|(t, f)| t.saturating_sub(f));
    card.vram_total_mb = rec.gpu_vram_total_mb;
    card.rtt_ms = Some(rtt_ms);
    card.last_seen_unix_ms = Some(rec.last_seen_unix_ms);
    card
}

pub(super) fn build_placement(gs: &GatewayState) -> PopuliPlacementDto {
    let orch = &gs.server_state.orchestrator;
    let snap = build_enriched_snapshot(gs);
    let topo = orch.topology_snapshot();
    let proc_rec = vox_populi::populi_registration_record_for_process();
    let local_reg = vox_populi::LocalRegistry::new(vox_populi::local_registry_path())
        .load()
        .ok();

    let mut local_card = card_from_record(
        &proc_rec,
        ComputeTier::LocalMensGpu,
        format!("Local Host MENS ({})", proc_rec.id),
        0,
        dock_pills(&snap.lanes, |l| {
            matches!(
                l.compute_tier,
                ComputeTier::LocalMensGpu | ComputeTier::InlineCpu
            )
        }),
    );
    let warm_count = snap
        .lanes
        .iter()
        .filter(|l| {
            matches!(
                l.compute_tier,
                ComputeTier::LocalMensGpu | ComputeTier::InlineCpu
            ) && l.kv_cache_hit_pct.is_some()
        })
        .count();
    local_card.kv_cache_warm_prefixes = Some(u32::try_from(warm_count).unwrap_or(u32::MAX));
    let mut nodes = vec![local_card];
    let mut seen_peers: HashSet<String> = HashSet::from([proc_rec.id]);

    if let Some(ref reg) = local_reg {
        for peer in reg.nodes.iter().filter(|p| seen_peers.insert(p.id.clone())) {
            let docked = dock_pills(&snap.lanes, |l| {
                l.compute_tier == ComputeTier::PopuliQuicPeer && l.node_id == peer.id
            });
            nodes.push(card_from_record(
                peer,
                ComputeTier::PopuliQuicPeer,
                format!("Populi Peer ({})", peer.id),
                12,
                docked,
            ));
        }
    }
    for lane in &snap.lanes {
        if lane.compute_tier == ComputeTier::PopuliQuicPeer
            && seen_peers.insert(lane.node_id.clone())
        {
            let nid = lane.node_id.clone();
            let mut card = make_card(
                nid.clone(),
                format!("Populi QUIC Peer ({nid})"),
                ComputeTier::PopuliQuicPeer,
                ComputeNodeStatus::Online,
                Some("iroh QUIC peer".into()),
                vec![lane.model_id.clone()],
                dock_pills(&snap.lanes, |l| {
                    l.compute_tier == ComputeTier::PopuliQuicPeer && l.node_id == nid
                }),
            );
            card.rtt_ms = Some(18);
            nodes.push(card);
        }
    }

    let cloud_models = orch
        .models_handle()
        .read()
        .map(|g| g.list_models().into_iter().take(6).map(|m| m.id).collect())
        .unwrap_or_default();
    let mut cloud = make_card(
        "cloud-bandit-router".into(),
        "Cloud Frontier Bandit Router".into(),
        ComputeTier::CloudBandit,
        ComputeNodeStatus::Online,
        Some("Thompson Sampling Facade".into()),
        cloud_models,
        dock_pills(&snap.lanes, |l| l.compute_tier == ComputeTier::CloudBandit),
    );
    cloud.bandit_weight = Some(1.0);
    nodes.push(cloud);

    let deadlocks = orch.lock_manager().deadlock_check();
    let mut lock_matrix: Vec<FileLockCellDto> = orch
        .lock_manager()
        .list_locks()
        .into_iter()
        .map(|(path, holder_id, exclusive)| {
            let holder_name = topo
                .nodes
                .iter()
                .find(|n| n.agent_id == holder_id)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| format!("agent-{}", holder_id.0));
            let lane = snap.lanes.iter().find(|l| {
                l.agent_id == holder_id.0.to_string()
                    || l.roster_name.eq_ignore_ascii_case(&holder_name)
            });
            FileLockCellDto {
                prefix: path.display().to_string(),
                holder_agent: holder_name,
                exclusive,
                compute_tier: lane
                    .map(|l| l.compute_tier)
                    .unwrap_or(ComputeTier::CloudBandit),
                workspace_mode: lane
                    .map(|l| l.workspace_mode)
                    .unwrap_or(SwarmWorkspaceMode::Inherit),
                waiters: Vec::new(),
                contended: deadlocks.iter().any(|(_, _, p)| p == &path),
                acquired_seq: snap.head_seq,
            }
        })
        .collect();
    for lane in &snap.lanes {
        if let Some(ref prefix) = lane.file_lock_prefix
            && !lock_matrix.iter().any(|c| &c.prefix == prefix)
        {
            lock_matrix.push(FileLockCellDto {
                prefix: prefix.clone(),
                holder_agent: lane.roster_name.clone(),
                exclusive: true,
                compute_tier: lane.compute_tier,
                workspace_mode: lane.workspace_mode,
                waiters: Vec::new(),
                contended: false,
                acquired_seq: snap.head_seq,
            });
        }
    }

    PopuliPlacementDto {
        generated_at_ms: now_unix_ms(),
        head_seq: snap.head_seq,
        gossip_cadence_ms: 250,
        source: if local_reg.is_some() {
            "orchestrator+local_registry".into()
        } else {
            "orchestrator".into()
        },
        queue_depth: orch.status().total_queued,
        nodes,
        lock_matrix,
    }
}
