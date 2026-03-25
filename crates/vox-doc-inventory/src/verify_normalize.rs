//! Drift verification against committed `doc-inventory.json`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde_json::Value;

use crate::inventory_gen::generate;

/// Strip `generated_at` for drift comparison.
pub fn strip_generated_at(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        obj.remove("generated_at");
    }
    v
}

pub(crate) fn normalize_json_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map
                .into_iter()
                .map(|(k, v)| (k, normalize_json_value(v)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(entries.into_iter().collect())
        }
        Value::Array(a) => Value::Array(a.into_iter().map(normalize_json_value).collect()),
        other => other,
    }
}

/// Verify committed inventory matches a fresh generation (ignoring `generated_at`).
pub fn verify_fresh(root: &Path, committed_path: &Path) -> Result<()> {
    if !committed_path.is_file() {
        return Err(anyhow!(
            "missing: {} (run: vox ci doc-inventory generate)",
            committed_path.display()
        ));
    }
    let before_raw = crate::bounded_fs::read_utf8_path_capped(committed_path)?;
    let before: Value = serde_json::from_str(&before_raw)
        .with_context(|| format!("parse {}", committed_path.display()))?;
    let sv = before
        .get("schema_version")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    if sv < 3 {
        return Err(anyhow!("doc-inventory.json: expected schema_version >= 3"));
    }

    let tmp = std::env::temp_dir().join(format!(
        "vox-doc-inventory-verify-{}.json",
        std::process::id()
    ));
    generate(root, &tmp)?;
    let after_raw = crate::bounded_fs::read_utf8_path_capped(&tmp)?;
    let _ = fs::remove_file(&tmp);
    let after: Value = serde_json::from_str(&after_raw)?;

    let b = normalize_json_value(strip_generated_at(before));
    let a = normalize_json_value(strip_generated_at(after));
    if a != b {
        return Err(anyhow!(
            "doc-inventory.json is out of date; run: vox ci doc-inventory generate --output docs/agents/doc-inventory.json"
        ));
    }
    Ok(())
}
