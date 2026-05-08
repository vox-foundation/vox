//! Structured suppressions loaded from JSON matching `contracts/toestub/suppression.v1.schema.json`.

use std::path::Path;

use globset::{Glob, GlobMatcher};
use serde_json::Value;

use crate::rules::Finding;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SuppressionFile {
    #[allow(dead_code)]
    version: u32,
    suppressions: Vec<SuppressionEntry>,
}

#[derive(Debug, Deserialize)]
struct SuppressionEntry {
    rule_id_prefix: String,
    #[serde(default)]
    path_glob: Option<String>,
    #[serde(default)]
    line: Option<u32>,
    #[allow(dead_code)]
    reason: String,
    #[allow(dead_code)]
    owner: String,
    #[serde(default)]
    #[allow(dead_code)]
    expires_after: Option<String>,
}

struct Compiled {
    rule_prefix: String,
    path_matcher: GlobMatcher,
    line: Option<usize>,
}

pub struct SuppressionStore {
    entries: Vec<Compiled>,
}

fn validate_suppression_json_instance(raw: &str) -> anyhow::Result<()> {
    let schema_path = Path::new("contracts/toestub/suppression.v1.schema.json");
    let Ok(schema_raw) = vox_bounded_fs::read_utf8_path_capped(schema_path) else {
        tracing::debug!(
            "TOESTUB suppression schema missing at {}; skipping instance validation",
            schema_path.display()
        );
        return Ok(());
    };
    let schema_val: Value = serde_json::from_str(&schema_raw)
        .map_err(|e| anyhow::anyhow!("parse suppression schema: {e}"))?;
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())
        .map_err(|e| anyhow::anyhow!("{e:#}"))?;
    let instance: Value =
        serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("parse suppressions JSON: {e}"))?;
    vox_jsonschema_util::validate(
        &instance,
        &validator,
        format!("suppressions vs {}", schema_path.display()),
    )
    .map_err(|e| anyhow::anyhow!("{e:#}"))?;
    Ok(())
}

impl SuppressionStore {
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Parse JSON from disk; returns empty store if path missing.
    pub fn load_optional(path: Option<&Path>) -> anyhow::Result<Self> {
        let Some(path) = path else {
            return Ok(Self::empty());
        };
        if !path.is_file() {
            return Ok(Self::empty());
        }
        let raw = vox_bounded_fs::read_utf8_path_capped(path)
            .map_err(|e| anyhow::anyhow!("read suppressions {}: {e}", path.display()))?;
        validate_suppression_json_instance(&raw)?;
        let doc: SuppressionFile = serde_json::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("parse suppressions JSON: {e}"))?;
        if doc.version != 1 {
            anyhow::bail!("suppressions: expected version 1");
        }
        let mut entries = Vec::with_capacity(doc.suppressions.len());
        for s in doc.suppressions {
            let Some(glob_str) = s
                .path_glob
                .as_ref()
                .map(|g| g.trim())
                .filter(|g| !g.is_empty())
            else {
                anyhow::bail!(
                    "suppression for `{}` must set non-empty `path_glob` (no blanket rule-wide suppressions)",
                    s.rule_id_prefix
                );
            };
            let g = Glob::new(glob_str)
                .map_err(|e| anyhow::anyhow!("bad path_glob {glob_str}: {e}"))?;
            let path_matcher = g.compile_matcher();
            entries.push(Compiled {
                rule_prefix: s.rule_id_prefix,
                path_matcher,
                line: s.line.map(|n| n as usize),
            });
        }
        Ok(Self { entries })
    }

    pub fn suppresses(&self, finding: &Finding) -> bool {
        let rel = finding.file.to_string_lossy().replace('\\', "/");
        for e in &self.entries {
            if !finding.rule_id.starts_with(&e.rule_prefix) {
                continue;
            }
            if !e.path_matcher.is_match(Path::new(rel.as_str())) {
                continue;
            }
            if let Some(ln) = e.line
                && finding.line != ln
            {
                continue;
            }
            return true;
        }
        false
    }
}
