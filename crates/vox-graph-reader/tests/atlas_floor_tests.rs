//! Phase 25 integration tests: typed edges, `.vox` AST extraction, derived `crate -> file -> symbol`
//! containment, and budgeted LOD subgraph slicing (`lod::query_atlas_subgraph`).

use serde_json::{Value, json};
use std::fs;
use vox_graph_reader::ast::EXTRACTOR_VERSION;
use vox_graph_reader::lod::{AtlasIdentity, AtlasLevel, AtlasQuery, query_atlas_subgraph};
use vox_graph_reader::rebuild::{RebuildMeta, rebuild_graph};

#[test]
fn extractor_version_is_bumped_to_5() {
    assert_eq!(EXTRACTOR_VERSION, "5");
}

#[test]
fn rebuild_walks_vox_files_and_persists_typed_edges() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("crates/vox-demo/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("app.vox"),
        r#"
table Item {
    name: str
}

component ItemView(name: str) {
    <Header />
}

query fetch_items() -> List[Item] {
    helper_load()
}

fn helper_load() {}
"#,
    )
    .unwrap();
    fs::write(
        src.join("lib.rs"),
        "pub fn entry() { helper_rs(); }\npub fn helper_rs() {}\n",
    )
    .unwrap();

    let out = tmp.path().join("out/graph.json");
    let cache = tmp.path().join("out/file_cache");
    let meta = RebuildMeta {
        corpus_id: "demo".to_string(),
        git_sha: Some("deadbeef".to_string()),
        scope_path: "crates/vox-demo".to_string(),
        extraction_mode: Some("structural".to_string()),
        built_at_rfc3339: "2026-10-15T00:00:00Z".to_string(),
        ..Default::default()
    };
    rebuild_graph(tmp.path(), tmp.path(), &out, &cache, &meta).unwrap();

    let g: Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    let nodes = g["nodes"].as_array().unwrap();
    let links = g["links"].as_array().unwrap();

    assert!(
        nodes
            .iter()
            .any(|n| n["label"] == "Item" && n["kind"] == "table")
    );
    assert!(
        nodes
            .iter()
            .any(|n| n["label"] == "ItemView" && n["kind"] == "component")
    );
    assert!(
        nodes
            .iter()
            .any(|n| n["label"] == "fetch_items" && n["kind"] == "query")
    );
    assert!(
        nodes
            .iter()
            .any(|n| n["label"] == "entry" && n["kind"] == "fn")
    );

    assert!(
        links.iter().all(|l| l["kind"].as_str().is_some()),
        "every persisted link must carry a kind field: {links:?}"
    );
    assert!(links.iter().any(|l| {
        l["source"].as_str().unwrap().ends_with("::fetch_items")
            && l["target"].as_str().unwrap().ends_with("::helper_load")
            && l["kind"] == "calls"
    }));
}

#[test]
fn lod_derives_crate_file_symbol_containment_and_enforces_budget() {
    let graph = json!({
        "nodes": [
            {"id": "crates/vox-alpha/src/lib.rs::run", "label": "run", "kind": "fn", "community": "c_0"},
            {"id": "crates/vox-alpha/src/lib.rs::helper", "label": "helper", "kind": "fn", "community": "c_0"},
            {"id": "crates/vox-alpha/src/extra.rs::more", "label": "more", "kind": "fn", "community": "c_0"},
            {"id": "crates/vox-beta/src/lib.rs::consume", "label": "consume", "kind": "fn", "community": "c_1"}
        ],
        "links": [
            {"source": "crates/vox-alpha/src/lib.rs::run", "target": "crates/vox-alpha/src/lib.rs::helper", "confidence": "resolved", "kind": "calls"},
            {"source": "crates/vox-beta/src/lib.rs::consume", "target": "crates/vox-alpha/src/lib.rs::run", "confidence": "resolved", "kind": "calls"}
        ]
    });
    let crate_graph = json!({
        "crates": {
            "vox-beta": ["vox-alpha"],
            "vox-alpha": []
        }
    });

    // 1. Crate level overview
    let sub_crate = query_atlas_subgraph(
        &graph,
        Some(&crate_graph),
        None,
        &AtlasQuery {
            level: AtlasLevel::Crate,
            focus: None,
            branch_tip: None,
            kinds: vec![],
            limit: 400,
        },
        AtlasIdentity::default(),
    )
    .unwrap();
    assert_eq!(sub_crate.nodes.len(), 2);
    assert!(sub_crate.links.iter().any(|l| l["source"] == "vox-beta"
        && l["target"] == "vox-alpha"
        && l["kind"] == "crate_dep"));

    // 2. File level drill-in on vox-alpha derives `contains` edges from crate -> file
    let sub_file = query_atlas_subgraph(
        &graph,
        Some(&crate_graph),
        None,
        &AtlasQuery {
            level: AtlasLevel::File,
            focus: Some("vox-alpha".to_string()),
            branch_tip: None,
            kinds: vec![],
            limit: 400,
        },
        AtlasIdentity::default(),
    )
    .unwrap();
    assert_eq!(sub_file.parent.as_deref(), Some("vox-alpha"));
    assert!(sub_file.links.iter().any(|l| {
        l["source"] == "vox-alpha"
            && l["target"] == "crates/vox-alpha/src/lib.rs"
            && l["kind"] == "contains"
    }));

    // 3. Symbol level drill-in on crates/vox-alpha/src/lib.rs derives file -> symbol `contains` edges
    let sub_sym = query_atlas_subgraph(
        &graph,
        Some(&crate_graph),
        None,
        &AtlasQuery {
            level: AtlasLevel::Symbol,
            focus: Some("crates/vox-alpha/src/lib.rs".to_string()),
            branch_tip: None,
            kinds: vec![],
            limit: 400,
        },
        AtlasIdentity::default(),
    )
    .unwrap();
    assert_eq!(sub_sym.parent.as_deref(), Some("vox-alpha"));
    assert!(sub_sym.links.iter().any(|l| {
        l["source"] == "crates/vox-alpha/src/lib.rs"
            && l["target"] == "crates/vox-alpha/src/lib.rs::run"
            && l["kind"] == "contains"
    }));

    // 4. Budget cap truncation
    let sub_capped = query_atlas_subgraph(
        &graph,
        Some(&crate_graph),
        None,
        &AtlasQuery {
            level: AtlasLevel::Symbol,
            focus: None,
            branch_tip: None,
            kinds: vec![],
            limit: 2,
        },
        AtlasIdentity::default(),
    )
    .unwrap();
    assert!(sub_capped.truncated);
    assert_eq!(sub_capped.nodes.len(), 2);

    // 5. Stale extractor_version is rejected
    let stale_err = query_atlas_subgraph(
        &graph,
        None,
        None,
        &AtlasQuery {
            level: AtlasLevel::Crate,
            focus: None,
            branch_tip: None,
            kinds: vec![],
            limit: 400,
        },
        AtlasIdentity {
            extractor_version: "4".to_string(),
            ..AtlasIdentity::default()
        },
    );
    assert!(stale_err.is_err());
}
