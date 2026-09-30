use std::fmt::Write as _;

const MODEL_DEFAULTS: &str = "../../contracts/orchestration/model-defaults.v1.yaml";

fn main() {
    println!("cargo:rerun-if-changed=../../contracts/orchestration/model-pins.v1.yaml");
    println!("cargo:rerun-if-changed=../../contracts/orchestration/model-routing.v1.yaml");
    println!("cargo:rerun-if-changed={MODEL_DEFAULTS}");
    generate_model_defaults();
}

/// Task 14: generate `vox_config::model_defaults` consts from the single
/// model-defaults contract, so a const context (registry rows, `pub const`
/// fallbacks) still reads the contract rather than a hard-coded literal.
fn generate_model_defaults() {
    let raw = std::fs::read_to_string(MODEL_DEFAULTS).expect("read model-defaults.v1.yaml");
    let doc: serde_yaml::Value = serde_yaml::from_str(&raw).expect("parse model-defaults.v1.yaml");
    let entries = doc["defaults"]
        .as_sequence()
        .expect("model-defaults.v1.yaml: `defaults` must be a list");
    let lit = |s: &str| format!("{s:?}");
    let mut consts = String::new();
    let mut all = String::from("static ALL_DEFAULTS: &[ModelDefault] = &[\n");
    let mut seen = std::collections::BTreeSet::new();
    for e in entries {
        let role = e["role"]
            .as_str()
            .expect("every entry needs a string `role`");
        assert!(seen.insert(role.to_string()), "duplicate role {role}");
        assert!(
            role.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "role {role} must be snake_case"
        );
        let name = role.to_ascii_uppercase();
        let models: Vec<String> = match (&e["model"], &e["models"]) {
            (serde_yaml::Value::String(m), serde_yaml::Value::Null) => {
                writeln!(consts, "/// `{role}` — see `model-defaults.v1.yaml`.").unwrap();
                writeln!(consts, "pub const {name}: &str = {};", lit(m)).unwrap();
                vec![m.clone()]
            }
            (serde_yaml::Value::Null, serde_yaml::Value::Sequence(ms)) => {
                let ms: Vec<String> = ms
                    .iter()
                    .map(|m| {
                        m.as_str()
                            .expect("`models` entries are strings")
                            .to_string()
                    })
                    .collect();
                assert!(!ms.is_empty(), "role {role}: `models` must not be empty");
                writeln!(
                    consts,
                    "/// `{role}` (ordered) — see `model-defaults.v1.yaml`."
                )
                .unwrap();
                let items: Vec<String> = ms.iter().map(|m| lit(m)).collect();
                writeln!(
                    consts,
                    "pub const {name}: &[&str] = &[{}];",
                    items.join(", ")
                )
                .unwrap();
                ms
            }
            _ => panic!("role {role}: exactly one of `model` or `models` is required"),
        };
        assert!(
            models.iter().all(|m| !m.trim().is_empty()),
            "role {role}: blank model id"
        );
        let items: Vec<String> = models.iter().map(|m| lit(m)).collect();
        writeln!(
            all,
            "    ModelDefault {{ role: {}, models: &[{}] }},",
            lit(role),
            items.join(", ")
        )
        .unwrap();
    }
    all.push_str("];\n");
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap())
        .join("model_defaults_generated.rs");
    std::fs::write(out, format!("{consts}\n{all}")).expect("write model_defaults_generated.rs");
}
