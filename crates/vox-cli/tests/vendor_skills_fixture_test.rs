//! Fixture tests for `assets/skills/SOURCES.toml` and vendored skill directories.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const RETAINED_SUPERPOWERS_SKILLS: &[&str] = &["brainstorming"];

const GSD_SKILLS: &[&str] = &[
    "gsd-add-tests",
    "gsd-audit-fix",
    "gsd-audit-milestone",
    "gsd-audit-uat",
    "gsd-autonomous",
    "gsd-capture",
    "gsd-code-review",
    "gsd-complete-milestone",
    "gsd-debug",
    "gsd-discuss-phase",
    "gsd-execute-phase",
    "gsd-explore",
    "gsd-extract-learnings",
    "gsd-fast",
    "gsd-health",
    "gsd-map-codebase",
    "gsd-mvp-phase",
    "gsd-new-milestone",
    "gsd-new-project",
    "gsd-next",
    "gsd-pause-work",
    "gsd-phase",
    "gsd-plan-phase",
    "gsd-progress",
    "gsd-quick",
    "gsd-resume-work",
    "gsd-ship",
    "gsd-spec-phase",
    "gsd-validate-phase",
    "gsd-verify-work",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

#[derive(Debug, serde::Deserialize)]
struct SourcesFile {
    #[serde(default)]
    source: Vec<SourceEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct SourceEntry {
    repo: String,
    license: String,
    pin: String,
    skills: Vec<String>,
}

fn load_sources() -> SourcesFile {
    let path = repo_root().join("assets/skills/SOURCES.toml");
    let raw = std::fs::read_to_string(&path).expect("read SOURCES.toml");
    toml::from_str(&raw).expect("parse SOURCES.toml")
}

fn all_skills_from_sources(sources: &SourcesFile) -> BTreeSet<String> {
    sources
        .source
        .iter()
        .flat_map(|entry| entry.skills.iter().cloned())
        .collect()
}

#[test]
fn sources_toml_lists_retained_brainstorming_and_gsd_skills() {
    let sources = load_sources();
    let superpowers = sources
        .source
        .iter()
        .find(|entry| entry.repo.contains("obra/superpowers"))
        .expect("superpowers [[source]] block");

    assert_eq!(superpowers.license, "MIT");
    assert!(
        !superpowers.pin.is_empty(),
        "superpowers pin must be non-empty"
    );

    for name in RETAINED_SUPERPOWERS_SKILLS {
        assert!(
            superpowers.skills.iter().any(|skill| skill == name),
            "missing retained brainstorming skill {name}"
        );
    }

    let gsd = sources
        .source
        .iter()
        .find(|entry| entry.repo.contains("open-gsd/gsd-core"))
        .expect("open-gsd/gsd-core [[source]] block");
    assert_eq!(gsd.license, "MIT");
    assert!(!gsd.pin.is_empty(), "gsd pin must be non-empty");
    for name in GSD_SKILLS {
        assert!(
            gsd.skills.iter().any(|skill| skill == name),
            "missing GSD skill {name}"
        );
    }
}

#[test]
fn repo_assets_skills_brainstorming_and_gsd_are_discoverable() {
    let root = repo_root();
    let roots = vox_config::paths::skill_search_roots(&root);
    let found = vox_plugin_host::external_skills::discover_external_skills(&roots);
    assert!(
        found
            .iter()
            .any(|s| s.bundle.manifest.id == "brainstorming"),
        "assets/skills brainstorming must be discoverable from repo root"
    );
    assert!(
        found
            .iter()
            .any(|s| s.bundle.manifest.id == "gsd-new-milestone"),
        "assets/skills gsd-new-milestone must be discoverable from repo root"
    );
}

#[test]
fn every_sources_skill_has_skill_md_on_disk() {
    let root = repo_root();
    let skills = all_skills_from_sources(&load_sources());

    assert!(
        !skills.is_empty(),
        "SOURCES.toml must declare at least one skill"
    );

    for name in skills {
        let skill_md = root.join("assets/skills").join(&name).join("SKILL.md");
        assert!(
            skill_md.is_file(),
            "missing vendored skill file: {}",
            skill_md.display()
        );
    }
}
