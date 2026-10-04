//! Golden JSON for `format_diagnostics_json_pretty` / `vox --json check` diagnostic shape.

use std::path::{Path, PathBuf};

use vox_cli::pipeline::{format_diagnostics_json_pretty, run_frontend_str};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/check_rust_import_lowering.json")
}

/// Normalize `file` fields to forward slashes for cross-platform golden comparison.
fn normalize_diagnostic_json(mut v: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Array(ref mut arr) = v {
        for item in arr.iter_mut() {
            if let Some(obj) = item.as_object_mut()
                && let Some(serde_json::Value::String(path)) = obj.get_mut("file")
            {
                *path = path.replace('\\', "/");
            }
        }
    }
    v
}

#[test]
fn golden_rust_import_lowering_diagnostic_json() {
    let fixture = fixture_dir().join("golden_rust_import_lowering.vox");
    let source = std::fs::read_to_string(&fixture).expect("read fixture");
    // Stable `file` field in JSON (matches `vox --json check` when cwd is the package root).
    let file_label = Path::new("tests/fixtures/golden_rust_import_lowering.vox");
    let result = run_frontend_str(&source, file_label, false).expect("frontend");
    assert!(
        result.has_errors(),
        "fixture should produce at least one error diagnostic"
    );

    let actual_raw = format_diagnostics_json_pretty(&result, file_label);
    if std::env::var("BLESS").is_ok() {
        std::fs::write(golden_path(), &actual_raw).unwrap();
    }
    let actual_val = serde_json::from_str::<serde_json::Value>(&actual_raw).expect("actual JSON");
    let actual_val = normalize_diagnostic_json(actual_val);

    let expected_raw = std::fs::read_to_string(golden_path()).expect("read golden");
    let expected_val =
        serde_json::from_str::<serde_json::Value>(&expected_raw).expect("golden JSON");
    let expected_val = normalize_diagnostic_json(expected_val);

    assert_eq!(
        actual_val, expected_val,
        "diagnostic JSON drift — update tests/golden/check_rust_import_lowering.json if intentional.\nActual:\n{actual_raw}"
    );
}
