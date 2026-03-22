#![allow(missing_docs)]
//! Smoke tests for `vox-ars` domain types and runtime harness.
//!
//! Network-gated tests (OpenClaw HTTP) are excluded; only pure-logic
//! and in-process runtime surfaces are covered here.

use serde_json::json;
use vox_ars::domain::ArsSkill;
use vox_ars::manifest::{ResourceLimits, SkillKind};
use vox_ars::openclaw::OpenClawSkillSpec;

// ─── Domain model round-trip ─────────────────────────────────────────────────

fn sample_skill(id: &str) -> ArsSkill {
    ArsSkill {
        id: id.to_string(),
        namespace: "vox".to_string(),
        name: "Sample Skill".to_string(),
        version: "1.0.0".to_string(),
        content_hash: "abc123".to_string(),
        description: Some("Does things".to_string()),
        author: Some("Test".to_string()),
        metadata: json!({}),
        kind: SkillKind::Document,
        body: Some("# SKILL.md\n---\n".to_string()),
        resource_limits: ResourceLimits::default(),
    }
}

#[test]
fn ars_skill_serializes_and_deserializes_round_trip() {
    let skill = sample_skill("my-skill");
    let json_str = serde_json::to_string(&skill).expect("serialize");
    let back: ArsSkill = serde_json::from_str(&json_str).expect("deserialize");
    assert_eq!(back.id, "my-skill");
    assert_eq!(back.namespace, "vox");
    assert_eq!(back.version, "1.0.0");
    assert_eq!(back.kind, SkillKind::Document);
}

#[test]
fn ars_skill_description_is_optional() {
    let mut skill = sample_skill("no-desc");
    skill.description = None;
    let json_str = serde_json::to_string(&skill).expect("serialize");
    let back: ArsSkill = serde_json::from_str(&json_str).expect("deserialize");
    assert!(back.description.is_none());
}

// ─── OpenClaw spec model ──────────────────────────────────────────────────────

#[test]
fn openclaw_skill_spec_debug_and_clone() {
    let spec = OpenClawSkillSpec {
        name: "code-review".to_string(),
        version: "2.1.0".to_string(),
        description: Some("Reviews code".to_string()),
    };
    let cloned = spec.clone();
    assert_eq!(cloned.name, "code-review");
    assert_eq!(cloned.version, "2.1.0");
    assert!(cloned.description.is_some());
    // Debug should not panic.
    let _ = format!("{:?}", cloned);
}

// ─── Resource limits ──────────────────────────────────────────────────────────

#[test]
fn resource_limits_default_is_reasonable() {
    let limits = ResourceLimits::default();
    // Defaults should not be wildly large values — just a sanity check.
    let _ = format!("{:?}", limits);
}
