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
