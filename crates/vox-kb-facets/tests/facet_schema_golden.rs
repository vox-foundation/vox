//! D-02 drift guard: the committed JSON Schema must equal the schemars output of `Facets`.

#[test]
fn facet_schema_matches_committed_contract() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/documentation/kb-facets.v1.schema.json");
    let mut want = serde_json::to_string_pretty(&vox_kb_facets::facet_json_schema())
        .expect("schema serializes");
    want.push('\n');
    if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::write(&path, &want).expect("write schema");
        return;
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        have == want,
        "contracts/documentation/kb-facets.v1.schema.json is stale. Regenerate with: \
         UPDATE_GOLDEN=1 cargo nextest run -p vox-kb-facets --test facet_schema_golden"
    );
}
