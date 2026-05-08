//! Baseline JSON helpers for TOESTUB stub-check.

use std::collections::HashMap;

use vox_code_audit::Finding;
use vox_code_audit::rules::Severity;

use super::fix_pipeline;

/// Load baseline map from JSON findings.
pub(crate) fn baseline_from_json(
    findings_json: &str,
) -> anyhow::Result<HashMap<(String, usize, String), Severity>> {
    let findings: Vec<Finding> = serde_json::from_str(findings_json)?;
    let mut map = HashMap::new();
    for f in findings {
        let key = fix_pipeline::norm_key(&f.file, f.line, &f.rule_id);
        map.insert(key, f.severity);
    }
    Ok(map)
}
