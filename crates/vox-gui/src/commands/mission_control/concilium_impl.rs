//! Pure-RPC implementation helpers for Mission Control + Vox Concilium Tauri commands.

use vox_orchestrator::{
    ConciliumPilotActionInput, ConciliumPilotActionResult, ConciliumSnapshotDto,
    PopuliPlacementDto, SequenceBoardEntry,
};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query};

pub(super) async fn call_orchestrator_daemon(
    backend: &BackendState,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    backend.call(method, params).await.map_err(String::from)
}

pub(crate) async fn concilium_snapshot_impl<B: BackendCalls>(
    b: &B,
) -> Result<ConciliumSnapshotDto, String> {
    api_data(b.api_v2_get("/concilium/snapshot").await?)
}

pub(crate) async fn concilium_board_impl<B: BackendCalls>(
    b: &B,
    since_seq: Option<u64>,
    seqs: Option<Vec<u64>>,
    channel: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SequenceBoardEntry>, String> {
    let seqs_param = seqs
        .filter(|v| !v.is_empty())
        .map(|v| v.iter().map(u64::to_string).collect::<Vec<_>>().join(","));
    let path = path_with_query(
        "/concilium/board",
        &[
            ("since_seq", since_seq.map(|s| s.to_string())),
            ("seqs", seqs_param),
            ("channel", channel.filter(|s| !s.trim().is_empty())),
            ("limit", limit.map(|l| l.to_string())),
        ],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn concilium_placement_impl<B: BackendCalls>(
    b: &B,
) -> Result<PopuliPlacementDto, String> {
    api_data(b.api_v2_get("/concilium/placement").await?)
}

pub(crate) async fn concilium_pilot_action_impl<B: BackendCalls>(
    b: &B,
    input: &ConciliumPilotActionInput,
) -> Result<ConciliumPilotActionResult, String> {
    if input.agent_id.trim().trim_start_matches('@').is_empty() {
        return Err("agent_id must not be empty".to_string());
    }
    let json = serde_json::to_string(input).map_err(|e| e.to_string())?;
    let path = path_with_query("/concilium/action", &[("payload", Some(json))]);
    api_data(b.api_v2_get(&path).await?)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::commands::daemon::PersistentDaemon;
    use crate::remote::calls::fake::FakeBackend;
    use crate::remote::gateway_client::test_support::{duplex_client, profile};
    use vox_foundation::protocol::orch_daemon_method;
    use vox_orchestrator::{ComputeTier, ConciliumPilotActionKind, SwarmWorkspaceMode};

    #[tokio::test]
    async fn concilium_commands_route_through_api_v2_without_vox_db() {
        let snap_payload = serde_json::json!({
            "head_seq": 2, "window_duration_ms": 10000, "active_workers": 1,
            "parked_workers": 1, "active_locks": 0, "now_playing_ticker": "Head #seq-2",
            "lanes": [], "wakeup_arcs": [], "board_tail": []
        });
        let place_payload = serde_json::json!({
            "generated_at_ms": 100, "head_seq": 2, "gossip_cadence_ms": 250,
            "source": "orchestrator", "queue_depth": 0, "nodes": [], "lock_matrix": []
        });
        let action_input = ConciliumPilotActionInput {
            action: ConciliumPilotActionKind::ParkAgent,
            agent_id: "parser".into(),
            channel: Some("#parser".into()),
            directive: None,
            fork_name: None,
            workspace_mode: Some(SwarmWorkspaceMode::Inherit),
            target_tier: Some(ComputeTier::InlineCpu),
            target_node_id: None,
            target_model_id: None,
        };
        let encoded_payload = serde_json::to_string(&action_input).expect("serialize action");
        let action_path =
            path_with_query("/concilium/action", &[("payload", Some(encoded_payload))]);
        let b = FakeBackend::new()
            .on_get(
                "/concilium/snapshot",
                Ok(serde_json::json!({"v":1,"data":snap_payload})),
            )
            .on_get(
                "/concilium/board?since_seq=1&seqs=2%2C3&channel=%23parser&limit=10",
                Ok(serde_json::json!({"v":1,"data":[]})),
            )
            .on_get(
                "/concilium/placement",
                Ok(serde_json::json!({"v":1,"data":place_payload})),
            )
            .on_get(
                &action_path,
                Ok(serde_json::json!({"v":1,"data":{"ok":true,"seq":3,"message":"Parked @parser"}})),
            );

        assert_eq!(
            concilium_snapshot_impl(&b)
                .await
                .expect("snapshot")
                .head_seq,
            2
        );
        assert!(
            concilium_board_impl(
                &b,
                Some(1),
                Some(vec![2, 3]),
                Some("#parser".into()),
                Some(10)
            )
            .await
            .expect("board")
            .is_empty()
        );
        assert_eq!(
            concilium_placement_impl(&b)
                .await
                .expect("placement")
                .head_seq,
            2
        );
        assert_eq!(
            concilium_pilot_action_impl(&b, &action_input)
                .await
                .expect("pilot action")
                .seq,
            3
        );
    }

    #[tokio::test]
    async fn remote_backend_surfaces_typed_not_remote_capable_error() {
        let (client, _server) = duplex_client(profile(&["mission-control"])).await;
        let backend = BackendState::new(Arc::new(PersistentDaemon::default()));
        backend.set_remote(Arc::new(client), None).await;
        let err =
            call_orchestrator_daemon(&backend, orch_daemon_method::PING, serde_json::json!({}))
                .await
                .expect_err("expected remote error");
        let v: serde_json::Value = serde_json::from_str(&err).expect("error envelope");
        assert_eq!(v["error"]["code"], "not_remote_capable");
        assert_eq!(v["error"]["method"], orch_daemon_method::PING);
    }
}
