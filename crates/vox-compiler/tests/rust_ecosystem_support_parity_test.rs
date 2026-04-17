use std::collections::HashSet;
use std::path::PathBuf;

use serde::Deserialize;
use vox_compiler::rust_interop_support::{
    RustInteropSemanticsState, RustInteropSupportClass, classify_rust_crate,
    semantics_state_for_rust_crate, supported_targets_for_rust_crate,
    template_managed_app_dependencies, template_managed_script_native_dependencies,
    template_managed_script_wasi_dependencies, wasi_unsupported_rust_imports,
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[derive(Debug, Deserialize)]
struct SupportRegistry {
    template_managed_dependencies: TemplateManagedDependencies,
    wasi_unsupported_rust_imports: Vec<String>,
    support_entries: Vec<SupportEntry>,
}

#[derive(Debug, Deserialize)]
struct TemplateManagedDependencies {
    app: Vec<String>,
    script_native: Vec<String>,
    script_wasi: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SupportEntry {
    crate_family: String,
    decision: String,
    semantics_state: String,
    supported_targets: Vec<String>,
}

#[test]
fn registry_decision_matches_classifier_for_each_crate_family_member() {
    let root = workspace_root();
    let registry_path = root.join("contracts/rust/ecosystem-support.yaml");
    let raw = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", registry_path.display()));
    let registry: SupportRegistry = serde_yaml::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", registry_path.display()));

    for entry in &registry.support_entries {
        for crate_name in entry.crate_family.split('+').map(str::trim) {
            assert!(
                !crate_name.is_empty(),
                "empty crate_name in crate_family '{}'",
                entry.crate_family
            );
            let got = classify_rust_crate(crate_name).as_label();
            assert_eq!(
                got, entry.decision,
                "crate '{}' from family '{}' mismatched decision",
                crate_name, entry.crate_family
            );
        }
    }
}

#[test]
fn schema_decision_enum_matches_classifier_labels() {
    let root = workspace_root();
    let schema_path = root.join("contracts/rust/ecosystem-support.schema.json");
    let raw = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let schema_json: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", schema_path.display()));

    let schema_values: HashSet<String> =
        schema_json["properties"]["support_entries"]["items"]["properties"]["decision"]["enum"]
            .as_array()
            .expect("decision enum array")
            .iter()
            .map(|v| v.as_str().expect("decision enum string").to_string())
            .collect();

    let classifier_values: HashSet<String> = [
        RustInteropSupportClass::FirstClassWrapper
            .as_label()
            .to_string(),
        RustInteropSupportClass::InternalRuntimeOnly
            .as_label()
            .to_string(),
        RustInteropSupportClass::EscapeHatchOnly
            .as_label()
            .to_string(),
        RustInteropSupportClass::Deferred.as_label().to_string(),
    ]
    .into_iter()
    .collect();

    assert_eq!(schema_values, classifier_values);
}

#[test]
fn registry_semantics_state_matches_classifier_mapping() {
    let root = workspace_root();
    let registry_path = root.join("contracts/rust/ecosystem-support.yaml");
    let raw = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", registry_path.display()));
    let registry: SupportRegistry = serde_yaml::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", registry_path.display()));

    for entry in &registry.support_entries {
        for crate_name in entry.crate_family.split('+').map(str::trim) {
            assert!(
                !crate_name.is_empty(),
                "empty crate_name in crate_family '{}'",
                entry.crate_family
            );
            let got = semantics_state_for_rust_crate(crate_name).as_label();
            assert_eq!(
                got, entry.semantics_state,
                "crate '{}' from family '{}' mismatched semantics_state",
                crate_name, entry.crate_family
            );
        }
    }
}

#[test]
fn schema_semantics_enum_matches_semantics_labels() {
    let root = workspace_root();
    let schema_path = root.join("contracts/rust/ecosystem-support.schema.json");
    let raw = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let schema_json: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", schema_path.display()));

    let schema_values: HashSet<String> = schema_json["properties"]["support_entries"]["items"]
        ["properties"]["semantics_state"]["enum"]
        .as_array()
        .expect("semantics_state enum array")
        .iter()
        .map(|v| v.as_str().expect("semantics_state enum string").to_string())
        .collect();

    let rust_values: HashSet<String> = [
        RustInteropSemanticsState::Implemented
            .as_label()
            .to_string(),
        RustInteropSemanticsState::PartiallyImplemented
            .as_label()
            .to_string(),
        RustInteropSemanticsState::Planned.as_label().to_string(),
        RustInteropSemanticsState::DocsOnly.as_label().to_string(),
    ]
    .into_iter()
    .collect();

    assert_eq!(schema_values, rust_values);
}

#[test]
fn registry_supported_targets_match_mapping_for_known_crates() {
    let root = workspace_root();
    let registry_path = root.join("contracts/rust/ecosystem-support.yaml");
    let raw = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", registry_path.display()));
    let registry: SupportRegistry = serde_yaml::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", registry_path.display()));

    for entry in &registry.support_entries {
        let expected: HashSet<String> = entry.supported_targets.iter().cloned().collect();
        for crate_name in entry.crate_family.split('+').map(str::trim) {
            let got_targets = supported_targets_for_rust_crate(crate_name)
                .unwrap_or_else(|| panic!("missing target mapping for crate '{crate_name}'"));
            let got: HashSet<String> = got_targets
                .iter()
                .map(|t| t.as_label().to_string())
                .collect();
            assert_eq!(
                got, expected,
                "crate '{}' from family '{}' mismatched supported_targets",
                crate_name, entry.crate_family
            );
        }
    }
}

#[test]
fn registry_yaml_validates_against_contract_schema() {
    let root = workspace_root();
    let schema_path = root.join("contracts/rust/ecosystem-support.schema.json");
    let schema_raw = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let schema_json: serde_json::Value = serde_json::from_str(&schema_raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", schema_path.display()));
    let validator = jsonschema::validator_for(&schema_json)
        .unwrap_or_else(|e| panic!("compile schema {}: {e}", schema_path.display()));

    let registry_path = root.join("contracts/rust/ecosystem-support.yaml");
    let registry_raw = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", registry_path.display()));
    let registry_json: serde_json::Value = serde_yaml::from_str(&registry_raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", registry_path.display()));

    if let Err(err) = validator.validate(&registry_json) {
        panic!(
            "registry {} failed schema {} validation: {err}",
            registry_path.display(),
            schema_path.display()
        );
    }
}

#[test]
fn template_managed_dependency_sets_match_contract_section() {
    let root = workspace_root();
    let registry_path = root.join("contracts/rust/ecosystem-support.yaml");
    let raw = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", registry_path.display()));
    let registry: SupportRegistry = serde_yaml::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", registry_path.display()));

    let expected_app: HashSet<String> = registry
        .template_managed_dependencies
        .app
        .into_iter()
        .collect();
    let expected_script_native: HashSet<String> = registry
        .template_managed_dependencies
        .script_native
        .into_iter()
        .collect();
    let expected_script_wasi: HashSet<String> = registry
        .template_managed_dependencies
        .script_wasi
        .into_iter()
        .collect();

    let got_app: HashSet<String> = template_managed_app_dependencies()
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let got_script_native: HashSet<String> = template_managed_script_native_dependencies()
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let got_script_wasi: HashSet<String> = template_managed_script_wasi_dependencies()
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    assert_eq!(
        got_app, expected_app,
        "template_managed_dependencies.app mismatch"
    );
    assert_eq!(
        got_script_native, expected_script_native,
        "template_managed_dependencies.script_native mismatch"
    );
    assert_eq!(
        got_script_wasi, expected_script_wasi,
        "template_managed_dependencies.script_wasi mismatch"
    );
}

#[test]
fn wasi_unsupported_import_set_matches_contract_section() {
    let root = workspace_root();
    let registry_path = root.join("contracts/rust/ecosystem-support.yaml");
    let raw = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", registry_path.display()));
    let registry: SupportRegistry = serde_yaml::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", registry_path.display()));

    let expected: HashSet<String> = registry.wasi_unsupported_rust_imports.into_iter().collect();
    let got: HashSet<String> = wasi_unsupported_rust_imports()
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(got, expected, "wasi_unsupported_rust_imports mismatch");
}

#[test]
fn wasi_deny_list_matches_supported_targets() {
    let root = workspace_root();
    let registry_path = root.join("contracts/rust/ecosystem-support.yaml");
    let raw = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", registry_path.display()));
    let registry: SupportRegistry = serde_yaml::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", registry_path.display()));

    let deny_list: HashSet<String> = registry.wasi_unsupported_rust_imports.into_iter().collect();
    for entry in &registry.support_entries {
        let supports_wasi = entry.supported_targets.iter().any(|t| t == "wasi");
        for crate_name in entry.crate_family.split('+').map(str::trim) {
            let in_deny_list = deny_list.contains(crate_name);
            if supports_wasi {
                assert!(
                    !in_deny_list,
                    "crate '{crate_name}' supports wasi but is present in wasi_unsupported_rust_imports"
                );
            } else {
                assert!(
                    in_deny_list,
                    "crate '{crate_name}' does not support wasi but is missing from wasi_unsupported_rust_imports"
                );
            }
        }
    }
}
