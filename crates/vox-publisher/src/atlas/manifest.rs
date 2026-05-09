//! AtlasManifest: T4 publication manifest aggregating T3 findings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasFinding {
    pub id: String,
    pub claim_text: String,
    pub nanopub_uri: String,
    /// true = hypothesis confirmed; false = null not rejected (negative result)
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasManifest {
    pub schema_version: u32,
    pub title: String,
    pub atlas_type: String,
    pub publication_date: String,
    pub findings: Vec<AtlasFinding>,
    pub zenodo_doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub osf_node_id: Option<String>,
    pub ro_crate_path: Option<String>,
}

impl AtlasManifest {
    pub fn negative_result_count(&self) -> usize {
        self.findings.iter().filter(|f| !f.supported).count()
    }

    pub fn positive_result_count(&self) -> usize {
        self.findings.iter().filter(|f| f.supported).count()
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

pub struct AtlasManifestBuilder {
    title: String,
    atlas_type: String,
    findings: Vec<AtlasFinding>,
}

impl AtlasManifestBuilder {
    pub fn new(title: String, atlas_type: String) -> Self {
        Self {
            title,
            atlas_type,
            findings: Vec::new(),
        }
    }

    pub fn add_finding(&mut self, finding: AtlasFinding) {
        self.findings.push(finding);
    }

    pub fn build(self, publication_date: &str) -> AtlasManifest {
        AtlasManifest {
            schema_version: 1,
            title: self.title,
            atlas_type: self.atlas_type,
            publication_date: publication_date.to_string(),
            findings: self.findings,
            zenodo_doi: None,
            arxiv_id: None,
            osf_node_id: None,
            ro_crate_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_manifest_build_includes_all_findings() {
        let mut builder = AtlasManifestBuilder::new(
            "Provider Reliability Atlas Q2-2026".into(),
            "provider-atlas".into(),
        );
        builder.add_finding(AtlasFinding {
            id: "f-001".into(),
            claim_text: "GPT-4o p95 latency increased by 120ms relative to GPT-4".into(),
            nanopub_uri: "https://vox.scientia/np/RAtest001".into(),
            supported: true,
        });
        builder.add_finding(AtlasFinding {
            id: "f-002".into(),
            claim_text: "Tool-call malformation rate for Gemini-1.5 decreased by 15%".into(),
            nanopub_uri: "https://vox.scientia/np/RAtest002".into(),
            supported: true,
        });
        let manifest = builder.build("2026-05-09");
        assert_eq!(manifest.title, "Provider Reliability Atlas Q2-2026");
        assert_eq!(manifest.findings.len(), 2);
        assert_eq!(manifest.schema_version, 1);
        let json = manifest.to_json();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["findings"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn atlas_manifest_negative_result_count() {
        let mut builder = AtlasManifestBuilder::new("Atlas".into(), "provider-atlas".into());
        builder.add_finding(AtlasFinding {
            id: "f-001".into(),
            claim_text: "Hypothesis A confirmed".into(),
            nanopub_uri: "uri-1".into(),
            supported: true,
        });
        builder.add_finding(AtlasFinding {
            id: "f-002".into(),
            claim_text: "Hypothesis B failed to reject null".into(),
            nanopub_uri: "uri-2".into(),
            supported: false,
        });
        let manifest = builder.build("2026-05-09");
        assert_eq!(manifest.negative_result_count(), 1);
        assert_eq!(manifest.positive_result_count(), 1);
    }
}
