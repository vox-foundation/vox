//! Golden JSON for `vox_cli::pipeline::format_check_for_llm_json`.

use std::path::{Path, PathBuf};

#[test]
fn check_for_llm_envelope_shape_rust_import_fixture() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let fixture = fixture_dir.join("golden_rust_import_lowering.vox");
    let source = std::fs::read_to_string(&fixture).expect("read fixture");
    let file_label = Path::new("tests/fixtures/golden_rust_import_lowering.vox");

    let raw = vox_cli::pipeline::format_check_for_llm_json(&source, file_label);
    let mut v: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");

    // Normalize file_path for Windows CI.
    if let Some(fp) = v.get_mut("file_path").and_then(|x| x.as_str()) {
        *v.get_mut("file_path").unwrap() = serde_json::json!(fp.replace('\\', "/"));
    }

    insta::assert_json_snapshot!(v);
}
