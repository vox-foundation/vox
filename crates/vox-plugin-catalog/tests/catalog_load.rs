use vox_plugin_catalog::{all_bundles, all_plugins};

#[test]
fn catalog_has_at_least_one_plugin() {
    let plugins = all_plugins();
    assert!(!plugins.is_empty(), "catalog has zero plugins");
    assert!(plugins.iter().any(|p| p.id == "mens-candle-cuda"));
}

#[test]
fn catalog_has_at_least_one_bundle() {
    let bundles = all_bundles();
    assert!(!bundles.is_empty(), "catalog has zero bundles");
    assert!(bundles.iter().any(|b| b.id == "vox-base"));
}

#[test]
fn catalog_has_all_nine_code_plugins() {
    let plugins = all_plugins();
    let code_ids: Vec<&str> = plugins
        .iter()
        .filter(|p| matches!(p.payload_kind, vox_plugin_catalog::schema::PayloadKind::Code | vox_plugin_catalog::schema::PayloadKind::Composite))
        .map(|p| p.id.as_str())
        .collect();
    let expected = [
        "tensor-burn-wgpu", "mens-candle-cuda", "oratio", "oratio-mic",
        "cloud", "populi-mesh", "script-execution", "execution-api", "stub-check",
    ];
    for id in expected {
        assert!(code_ids.contains(&id), "missing code/composite plugin: {id}");
    }
}
