#![cfg(feature = "tree-sitter-grammars")]
use std::collections::HashMap;
use std::path::Path;
use vox_graph_reader::ast::extract_ast_in_module_with_wrappers;
use vox_graph_reader::rebuild::{RebuildMeta, rebuild_graph};

#[test]
fn invoke_and_wrapper_boundary_edges() {
    let mut wrappers = HashMap::new();
    wrappers.insert(
        "doubtTask".to_string(),
        "cmd:doubt_orchestrator_task".to_string(),
    );
    let content = "function save(){ invoke('save_settings'); voxTransport.doubtTask(7); \
        invoke('invoke_mcp_tool', { tool: 'vox_resolve_feedback' }); }";
    let g = extract_ast_in_module_with_wrappers(Path::new("S.tsx"), content, "S.tsx", &wrappers);
    let t: Vec<&str> = g.edges.iter().map(|e| e.target.as_str()).collect();
    assert!(t.contains(&"cmd:save_settings"), "edges: {:?}", g.edges);
    assert!(
        t.contains(&"cmd:doubt_orchestrator_task"),
        "edges: {:?}",
        g.edges
    );
    assert!(
        t.contains(&"tool:vox_resolve_feedback"),
        "edges: {:?}",
        g.edges
    );
    assert!(
        g.edges
            .iter()
            .filter(|e| e.target == "cmd:save_settings")
            .count()
            == 1,
        "no double-count"
    );
}

/// End-to-end `gui-wiring` mode: a tempdir with a surface that invokes a registered
/// command (`do_it`) and a missing one (`gone`). After rebuild, `graph.json` must carry
/// a `cmd:do_it` node (kind command) and a `missing`-flagged `cmd:gone` node, plus edges
/// from the surface to both.
#[test]
fn gui_wiring_registry_ingest_and_missing_nodes() {
    let tmp = std::env::temp_dir().join(format!("vox-gw-{}", std::process::id()));
    let gui = tmp.join("crates/vox-gui");
    std::fs::create_dir_all(gui.join("ui/src")).unwrap();
    std::fs::create_dir_all(gui.join("src/commands")).unwrap();
    std::fs::write(
        gui.join("ui/src/S.tsx"),
        "function go(){ invoke('do_it'); invoke('gone'); }",
    )
    .unwrap();
    std::fs::write(
        gui.join("src/commands/x.rs"),
        "#[tauri::command]\npub fn do_it() {}\n",
    )
    .unwrap();
    std::fs::write(
        gui.join("src/main.rs"),
        "fn main(){ tauri::generate_handler![commands::x::do_it]; }",
    )
    .unwrap();

    let out = tmp.join("out/graph.json");
    let cache = tmp.join("cache");
    let meta = RebuildMeta {
        corpus_id: "vox-gui-surface".into(),
        git_sha: None,
        scope_path: "crates/vox-gui".into(),
        extraction_mode: Some("gui-wiring".into()),
        built_at_rfc3339: "2026-06-26T00:00:00Z".into(),
        cli_catalog_json: None,
        ..Default::default()
    };
    rebuild_graph(&tmp, &gui, &out, &cache, &meta).unwrap();

    let graph: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    let do_it = nodes
        .iter()
        .find(|n| n["id"] == "cmd:do_it")
        .expect("cmd:do_it node");
    assert_eq!(do_it["kind"], "command");
    let gone = nodes
        .iter()
        .find(|n| n["id"] == "cmd:gone")
        .expect("cmd:gone node synthesized for the dead-end");
    assert_eq!(gone["missing"], serde_json::json!(true));

    let links = graph["links"].as_array().unwrap();
    assert!(
        links.iter().any(|e| e["target"] == "cmd:do_it"),
        "edge to cmd:do_it; links: {links:?}"
    );
    assert!(
        links.iter().any(|e| e["target"] == "cmd:gone"),
        "edge to cmd:gone; links: {links:?}"
    );
}

#[test]
fn registered_tool_nodes_resolve_only_referenced_names() {
    let tmp = std::env::temp_dir().join(format!("vox-tool-gw-{}", std::process::id()));
    let gui = tmp.join("crates/vox-gui");
    std::fs::create_dir_all(gui.join("ui/src")).unwrap();
    std::fs::create_dir_all(gui.join("src/commands")).unwrap();
    std::fs::write(
        gui.join("ui/src/S.tsx"),
        r#"function go(){
            invoke('invoke_mcp_tool', { tool: 'vox_mesh_nodes', args: {} });
            invoke('invoke_mcp_tool', { tool: 'vox_compiler::ast_inspect' });
            invoke('invoke_mcp_tool', { tool: 'vox_not_registered' });
        }"#,
    )
    .unwrap();
    std::fs::write(
        gui.join("src/commands/user_config.rs"),
        r#"fn flat_effective_value(spec: &LlmConfigKey) -> String {
            match spec.env {
                "vox_populi::inference_PROFILE" => "desktop_ollama",
                _ => "",
            }
        }"#,
    )
    .unwrap();
    std::fs::write(gui.join("src/main.rs"), "fn main(){}").unwrap();

    let out = tmp.join("out/graph.json");
    let cache = tmp.join("cache");
    let meta = RebuildMeta {
        corpus_id: "vox-gui-surface".into(),
        git_sha: None,
        scope_path: "crates/vox-gui".into(),
        extraction_mode: Some("gui-wiring".into()),
        built_at_rfc3339: "2026-06-26T00:00:00Z".into(),
        cli_catalog_json: None,
        mcp_tool_names: Some(vec![
            "vox_mesh_nodes".to_string(),
            "vox_compiler::ast_inspect".to_string(),
            "vox_unreferenced".to_string(),
        ]),
        ..Default::default()
    };
    rebuild_graph(&tmp, &gui, &out, &cache, &meta).unwrap();

    let graph: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let nodes = graph["nodes"].as_array().unwrap();

    // 1. no node with id `tool:vox_populi::inference_PROFILE`
    assert!(
        !nodes
            .iter()
            .any(|n| n["id"] == "tool:vox_populi::inference_PROFILE"),
        "env key must NOT become a tool node"
    );

    // 2. nodes `tool:vox_mesh_nodes` and `tool:vox_compiler::ast_inspect` exist with kind `tool` and without `missing: true`
    let mesh = nodes
        .iter()
        .find(|n| n["id"] == "tool:vox_mesh_nodes")
        .expect("tool:vox_mesh_nodes node must exist");
    assert_eq!(mesh["kind"], "tool");
    assert_ne!(mesh["missing"], serde_json::json!(true));

    let ast = nodes
        .iter()
        .find(|n| n["id"] == "tool:vox_compiler::ast_inspect")
        .expect("tool:vox_compiler::ast_inspect node must exist");
    assert_eq!(ast["kind"], "tool");
    assert_ne!(ast["missing"], serde_json::json!(true));

    // 3. no node `tool:vox_unreferenced`
    assert!(
        !nodes.iter().any(|n| n["id"] == "tool:vox_unreferenced"),
        "unreferenced registered tool must not become a node"
    );

    // 4. node `tool:vox_not_registered` exists with `missing: true`
    let missing = nodes
        .iter()
        .find(|n| n["id"] == "tool:vox_not_registered")
        .expect("tool:vox_not_registered node synthesized for the dead-end");
    assert_eq!(missing["missing"], serde_json::json!(true));

    let links = graph["links"].as_array().unwrap();
    assert!(links.iter().any(|e| e["target"] == "tool:vox_mesh_nodes"));
    assert!(
        links
            .iter()
            .any(|e| e["target"] == "tool:vox_compiler::ast_inspect")
    );
    assert!(
        links
            .iter()
            .any(|e| e["target"] == "tool:vox_not_registered")
    );
}

#[test]
fn no_registry_names_keeps_old_dangling_behavior() {
    let tmp = std::env::temp_dir().join(format!("vox-tool-none-{}", std::process::id()));
    let gui = tmp.join("crates/vox-gui");
    std::fs::create_dir_all(gui.join("ui/src")).unwrap();
    std::fs::create_dir_all(gui.join("src/commands")).unwrap();
    std::fs::write(
        gui.join("ui/src/S.tsx"),
        "function go(){ invoke('invoke_mcp_tool', { tool: 'vox_mesh_nodes' }); }",
    )
    .unwrap();
    std::fs::write(gui.join("src/main.rs"), "fn main(){}").unwrap();

    let out = tmp.join("out/graph.json");
    let cache = tmp.join("cache");
    let meta = RebuildMeta {
        corpus_id: "vox-gui-surface".into(),
        git_sha: None,
        scope_path: "crates/vox-gui".into(),
        extraction_mode: Some("gui-wiring".into()),
        built_at_rfc3339: "2026-06-26T00:00:00Z".into(),
        cli_catalog_json: None,
        mcp_tool_names: None,
        ..Default::default()
    };
    rebuild_graph(&tmp, &gui, &out, &cache, &meta).unwrap();

    let graph: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    let mesh = nodes
        .iter()
        .find(|n| n["id"] == "tool:vox_mesh_nodes")
        .expect("tool:vox_mesh_nodes synthesized missing node");
    assert_eq!(mesh["missing"], serde_json::json!(true));
}
