use vox_plugin_catalog::schema::{BundleEntry, PayloadKind, PluginCatalogEntry};

#[test]
fn parses_a_minimal_code_plugin_entry() {
    let toml_src = r#"
        id = "mens-candle-cuda"
        payload-kind = "code"
        description = "Candle ML backend with CUDA."
        extension-points = ["MlBackend"]
        default-source = "github:vox-foundation/vox-plugin-mens-candle-cuda"
        bundled-in = ["vox-ml", "vox-dev"]
    "#;
    let entry: PluginCatalogEntry = toml::from_str(toml_src).expect("should parse");
    assert_eq!(entry.id, "mens-candle-cuda");
    assert!(matches!(entry.payload_kind, PayloadKind::Code));
    assert_eq!(
        entry.extension_points.as_deref(),
        Some(&["MlBackend".to_string()][..])
    );
}

#[test]
fn parses_a_minimal_skill_plugin_entry() {
    let toml_src = r#"
        id = "skill-compiler"
        payload-kind = "skill"
        description = "Compiler skill."
        exposes-tools = ["vox_validate_file"]
        default-source = "github:vox-foundation/vox-plugin-skill-compiler"
        bundled-in = ["vox-fullstack"]
    "#;
    let entry: PluginCatalogEntry = toml::from_str(toml_src).expect("should parse");
    assert!(matches!(entry.payload_kind, PayloadKind::Skill));
    assert_eq!(
        entry.exposes_tools.as_deref(),
        Some(&["vox_validate_file".to_string()][..])
    );
}

#[test]
fn parses_a_bundle_with_extends() {
    let toml_src = r#"
        id = "vox-ml"
        description = "Fullstack + ML."
        extends = "vox-fullstack"
        plugins = ["mens-candle-cuda", "tensor-burn-wgpu"]
    "#;
    let bundle: BundleEntry = toml::from_str(toml_src).expect("should parse");
    assert_eq!(bundle.extends.as_deref(), Some("vox-fullstack"));
    assert_eq!(bundle.plugins.len(), 2);
}
