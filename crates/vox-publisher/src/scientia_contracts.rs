//! SCIENTIA contract-bound data models (canonical metadata, evidence pack, route profiles).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const CANONICAL_METADATA_SCHEMA_REL: &str =
    "contracts/scientia/canonical-publication-metadata.v1.schema.json";
pub const EVIDENCE_PACK_SCHEMA_REL: &str = "contracts/scientia/evidence-pack.v1.schema.json";
pub const ROUTE_PROFILE_REQUIREMENTS_REL: &str =
    "contracts/scientia/route-profile-requirements.v1.yaml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalPublicationMetadataV1 {
    pub version: String,
    pub identity: CanonicalIdentity,
    pub contributors: CanonicalContributors,
    pub provenance: CanonicalProvenance,
    pub policy: CanonicalPolicy,
    pub rights_and_funding: CanonicalRightsAndFunding,
    pub distribution: CanonicalDistribution,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalIdentity {
    pub title: String,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub keywords: Vec<String>,
    pub target_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalContributors {
    pub authors: Vec<CanonicalAuthor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalAuthor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<CanonicalAffiliation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalAffiliation {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ror: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalProvenance {
    pub publication_id: String,
    pub manifest_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_pack_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalPolicy {
    pub ai_disclosure: CanonicalAiDisclosure,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ethics_statement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broader_impact_statement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double_blind_ready: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalAiDisclosure {
    pub declared: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<CanonicalAiTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalAiTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CanonicalRightsAndFunding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub funding: Vec<CanonicalFunding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_of_interest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalFunding {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub award_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub funder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalDistribution {
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidencePackV1 {
    pub version: String,
    pub publication_id: String,
    pub manifest_digest: String,
    pub baseline: EvidenceRunRef,
    pub candidate: EvidenceRunRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pair_integrity_passed: Option<bool>,
    pub replay_instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceRunRef {
    pub run_id: String,
    pub config_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eval_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteProfileRequirementsV1 {
    pub version: String,
    pub profiles: BTreeMap<String, RouteProfileRequirementsEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteProfileRequirementsEntry {
    #[serde(default)]
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub recommended_fields: Vec<String>,
    #[serde(default)]
    pub hard_gate_signals: Vec<String>,
}

pub fn parse_canonical_metadata_v1(json: &str) -> Result<CanonicalPublicationMetadataV1> {
    serde_json::from_str(json).context("parse canonical publication metadata v1")
}

pub fn parse_evidence_pack_v1(json: &str) -> Result<EvidencePackV1> {
    serde_json::from_str(json).context("parse evidence pack v1")
}

pub fn parse_route_profile_requirements_v1(yaml: &str) -> Result<RouteProfileRequirementsV1> {
    serde_yaml::from_str(yaml).context("parse route profile requirements v1")
}

pub fn load_route_profile_requirements_from_repo_root(
    repo_root: &Path,
) -> Result<RouteProfileRequirementsV1> {
    let path = repo_root.join(ROUTE_PROFILE_REQUIREMENTS_REL);
    let raw = vox_bounded_fs::read_utf8_path_capped(&path)
        .with_context(|| format!("read {}", path.display()))?;
    parse_route_profile_requirements_v1(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonschema::validator_for;
    use serde_json::Value;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root")
    }

    #[test]
    fn scientia_contracts_roundtrip_and_schema() {
        let root = repo_root();
        let canonical_schema: Value = serde_json::from_str(
            &vox_bounded_fs::read_utf8_path_capped(&root.join(CANONICAL_METADATA_SCHEMA_REL))
                .expect("read canonical schema"),
        )
        .expect("parse canonical schema");
        let canonical_instance: Value =
            serde_json::from_str(
                &vox_bounded_fs::read_utf8_path_capped(&root.join(
                    "crates/vox-publisher/tests/fixtures/scientia_canonical_metadata.v1.json",
                ))
                .expect("read canonical fixture"),
            )
            .expect("parse canonical fixture");
        let validator = validator_for(&canonical_schema).expect("compile canonical schema");
        assert!(validator.validate(&canonical_instance).is_ok());
        let typed = parse_canonical_metadata_v1(
            &serde_json::to_string(&canonical_instance).expect("canonical fixture string"),
        )
        .expect("typed canonical parse");
        assert_eq!(typed.version, "v1");
        assert!(!typed.contributors.authors.is_empty());

        let evidence_schema: Value = serde_json::from_str(
            &vox_bounded_fs::read_utf8_path_capped(&root.join(EVIDENCE_PACK_SCHEMA_REL))
                .expect("read evidence schema"),
        )
        .expect("parse evidence schema");
        let evidence_instance: Value = serde_json::from_str(
            &vox_bounded_fs::read_utf8_path_capped(
                &root.join("crates/vox-publisher/tests/fixtures/scientia_evidence_pack.v1.json"),
            )
            .expect("read evidence fixture"),
        )
        .expect("parse evidence fixture");
        let validator = validator_for(&evidence_schema).expect("compile evidence schema");
        assert!(validator.validate(&evidence_instance).is_ok());
        let evidence = parse_evidence_pack_v1(
            &serde_json::to_string(&evidence_instance).expect("evidence fixture string"),
        )
        .expect("typed evidence parse");
        assert_eq!(evidence.version, "v1");
        assert!(!evidence.replay_instructions.is_empty());

        let route_raw =
            vox_bounded_fs::read_utf8_path_capped(&root.join(ROUTE_PROFILE_REQUIREMENTS_REL))
                .expect("read route profile requirements");
        let route = parse_route_profile_requirements_v1(&route_raw)
            .expect("parse route profile requirements");
        assert_eq!(route.version, "v1");
        assert!(route.profiles.contains_key("journal"));
        assert!(
            route
                .profiles
                .get("journal")
                .expect("journal profile")
                .required_fields
                .iter()
                .any(|f| f == "identity.title")
        );
    }
}
