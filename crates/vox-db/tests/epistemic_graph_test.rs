use vox_db::VoxDb;

#[tokio::test]
async fn test_epistemic_graph_cte_cycle_protection() {
    let db = VoxDb::in_memory().await.expect("db init");

    // Insert nodes via canonical ops
    db.upsert_knowledge_node(
        "node:A",
        "Node A",
        "Node A content",
        Some("concept"),
        None,
        None,
    )
    .await
    .unwrap();
    db.upsert_knowledge_node(
        "node:B",
        "Node B",
        "Node B content",
        Some("concept"),
        None,
        None,
    )
    .await
    .unwrap();
    db.upsert_knowledge_node(
        "node:C",
        "Node C",
        "Node C content",
        Some("concept"),
        None,
        None,
    )
    .await
    .unwrap();

    // Create cycle A -> B -> A and branch B -> C
    db.create_knowledge_edge("node:A", "node:B", "links_to", 1.0, None)
        .await
        .unwrap();
    db.create_knowledge_edge("node:B", "node:A", "links_to", 1.0, None)
        .await
        .unwrap();
    db.create_knowledge_edge("node:B", "node:C", "links_to", 1.0, None)
        .await
        .unwrap();

    // Must terminate cleanly without infinite loop or recursion limit abort
    let reachable_forward = db
        .find_reachable_knowledge_nodes_cte("node:A", 5, "forward")
        .await
        .unwrap();
    assert_eq!(reachable_forward.len(), 3);
    assert!(reachable_forward.contains(&"node:A".to_string()));
    assert!(reachable_forward.contains(&"node:B".to_string()));
    assert!(reachable_forward.contains(&"node:C".to_string()));

    // Reverse from C should reach B and A
    let reachable_reverse = db
        .find_reachable_knowledge_nodes_cte("node:C", 5, "reverse")
        .await
        .unwrap();
    assert_eq!(reachable_reverse.len(), 3);
    assert!(reachable_reverse.contains(&"node:C".to_string()));
    assert!(reachable_reverse.contains(&"node:B".to_string()));
    assert!(reachable_reverse.contains(&"node:A".to_string()));

    // Undirected from C reaches all nodes
    let reachable_undirected = db
        .find_reachable_knowledge_nodes_cte("node:C", 5, "undirected")
        .await
        .unwrap();
    assert_eq!(reachable_undirected.len(), 3);
    assert!(reachable_undirected.contains(&"node:C".to_string()));
    assert!(reachable_undirected.contains(&"node:B".to_string()));
    assert!(reachable_undirected.contains(&"node:A".to_string()));

    // Max depth 0 respects depth bound
    let depth_0 = db
        .find_reachable_knowledge_nodes_cte("node:A", 0, "forward")
        .await
        .unwrap();
    assert_eq!(depth_0, vec!["node:A".to_string()]);

    // Max depth 1 respects depth bound (A -> B, but not C)
    let depth_1 = db
        .find_reachable_knowledge_nodes_cte("node:A", 1, "forward")
        .await
        .unwrap();
    assert_eq!(depth_1.len(), 2);
    assert!(depth_1.contains(&"node:A".to_string()));
    assert!(depth_1.contains(&"node:B".to_string()));
}
