//! Analysis-agnostic canonical JSON helpers.
//!
//! Extracted from `syntax_k` so analysis-side modules (`app_contract`,
//! `runtime_projection`) do not need to depend on the emit-side `syntax_k`
//! module (which itself transitively depends on `web_ir`). Keeping this
//! helper crate-local avoids an analysis→emit cycle once `syntax_k` moves
//! to `vox-codegen`.

/// Recursively sort JSON object keys for cross-toolchain-stable canonical bytes.
pub fn sort_json_value_keys(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(map) => {
            let mut pairs: Vec<(String, serde_json::Value)> = map
                .iter()
                .map(|(k, val)| (k.clone(), val.clone()))
                .collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            map.clear();
            for (k, mut val) in pairs {
                sort_json_value_keys(&mut val);
                map.insert(k, val);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                sort_json_value_keys(item);
            }
        }
        _ => {}
    }
}
