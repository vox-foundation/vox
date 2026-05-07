#[cfg(test)]
mod unit {
    use super::super::{A2AAck, A2APage, InMemoryMeshStore, MeshStore};
    use crate::transport::{A2AStoredMessage, DispatchResponse, RemoteExecLeaseRow};

    fn a2a(id: u64, rcv: &str) -> A2AStoredMessage {
        A2AStoredMessage {
            id,
            sender_agent_id: "sender".into(),
            receiver_agent_id: rcv.into(),
            message_type: "job_submit".into(),
            payload: "{}".into(),
            created_unix_ms: 1000 * id,
            acknowledged: false,
            lease_holder_node_id: None,
            lease_expires_unix_ms: None,
            privacy_class: None,
            idempotency_dedupe_key: None,
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
            jwe_payload: None,
            priority: 128,
            task_kind: None,
            model_id: None,
            sender_node_id: None,
            traceparent: None,
        }
    }

    fn lease(id: &str) -> RemoteExecLeaseRow {
        RemoteExecLeaseRow {
            lease_id: id.into(),
            scope_key: format!("task:{id}"),
            holder_node_id: "node-1".into(),
            expires_unix_ms: 99999999,
        }
    }

    fn dispatch_result(node: &str) -> DispatchResponse {
        DispatchResponse {
            success: true,
            output: "ok".into(),
            is_truncated: false,
            duration_ms: 10,
            exit_code: Some(0),
            error: None,
            node_id: node.into(),
            expires_unix_ms: None,
        }
    }

    #[tokio::test]
    async fn in_memory_a2a_round_trip() {
        let store = InMemoryMeshStore::new();
        store.put_a2a(&a2a(1, "agent-A")).await.unwrap();
        store.put_a2a(&a2a(2, "agent-A")).await.unwrap();
        store.put_a2a(&a2a(3, "agent-B")).await.unwrap();

        let all = store.list_a2a(A2APage::default()).await.unwrap();
        assert_eq!(all.len(), 3);

        let for_a = store
            .list_a2a(A2APage {
                receiver_agent_id: Some("agent-A".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(for_a.len(), 2);
    }

    #[tokio::test]
    async fn in_memory_ack_excludes_from_list() {
        let store = InMemoryMeshStore::new();
        store.put_a2a(&a2a(1, "agent-A")).await.unwrap();
        store
            .ack_a2a(
                1,
                A2AAck {
                    acknowledged: true,
                    acked_unix_ms: 12345,
                },
            )
            .await
            .unwrap();

        let pending = store.list_a2a(A2APage::default()).await.unwrap();
        assert!(pending.is_empty(), "acked row must be excluded");

        let all = store
            .list_a2a(A2APage {
                include_acked: true,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].acknowledged);
    }

    #[tokio::test]
    async fn in_memory_pagination_since_id() {
        let store = InMemoryMeshStore::new();
        for i in 1u64..=5 {
            store.put_a2a(&a2a(i, "A")).await.unwrap();
        }
        let page = store
            .list_a2a(A2APage {
                since_id: Some(3),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].id, 4);
        assert_eq!(page[1].id, 5);
    }

    #[tokio::test]
    async fn in_memory_pagination_limit() {
        let store = InMemoryMeshStore::new();
        for i in 1u64..=10 {
            store.put_a2a(&a2a(i, "A")).await.unwrap();
        }
        let page = store
            .list_a2a(A2APage {
                limit: Some(3),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.len(), 3);
    }

    #[tokio::test]
    async fn in_memory_lease_put_list_revoke() {
        let store = InMemoryMeshStore::new();
        store.put_exec_lease(&lease("L1")).await.unwrap();
        store.put_exec_lease(&lease("L2")).await.unwrap();

        let all = store.list_exec_leases().await.unwrap();
        assert_eq!(all.len(), 2);

        store.revoke_exec_lease("L1").await.unwrap();
        let after = store.list_exec_leases().await.unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].lease_id, "L2");
    }

    #[tokio::test]
    async fn in_memory_dispatch_get_or_none() {
        let store = InMemoryMeshStore::new();
        let missing = store.get_dispatch_result("absent").await.unwrap();
        assert!(missing.is_none());

        store
            .put_dispatch_result("d1", &dispatch_result("node-X"))
            .await
            .unwrap();
        let found = store.get_dispatch_result("d1").await.unwrap().unwrap();
        assert_eq!(found.node_id, "node-X");
    }

    #[tokio::test]
    async fn in_memory_integrity_dedupe_violation_detected() {
        let store = InMemoryMeshStore::new();
        let mut m1 = a2a(1, "A");
        m1.idempotency_dedupe_key = Some("key-x".into());
        let mut m2 = a2a(2, "A");
        m2.idempotency_dedupe_key = Some("key-x".into());
        store.put_a2a(&m1).await.unwrap();
        store.put_a2a(&m2).await.unwrap();

        let report = store.integrity_check().await.unwrap();
        assert!(!report.ok);
        assert!(report.findings.iter().any(|f| f.code == "dedupe_violation"));
    }

    #[tokio::test]
    async fn in_memory_load_all_includes_acked() {
        let store = InMemoryMeshStore::new();
        let mut m = a2a(1, "A");
        m.acknowledged = true;
        store.put_a2a(&m).await.unwrap();
        store.put_a2a(&a2a(2, "A")).await.unwrap();

        let all = store.load_all_a2a().await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
