#![allow(missing_docs)]
//! External integration tests for `vox_skills::SkillRegistry`.

use vox_skills::bundle::VoxSkillBundle;
use vox_skills::manifest::{SkillCategory, SkillManifest};
use vox_skills::registry::SkillRegistry;

fn bundle(id: &str, cat: SkillCategory) -> VoxSkillBundle {
    let m = SkillManifest::new(id, id, "1.0.0", "vox", "desc", cat);
    VoxSkillBundle::new(m, "# Skill\nInstructions.")
}

#[tokio::test]
async fn install_returns_id_and_version() {
    let reg = SkillRegistry::new();
    let res = reg
        .install_bundle(&bundle("vox.compiler", SkillCategory::Compiler))
        .await
        .expect("install");
    assert_eq!(res.id, "vox.compiler");
    assert_eq!(res.version, "1.0.0");
    assert!(!res.already_installed);
    assert!(!res.hash.is_empty());
}

#[tokio::test]
async fn install_same_version_twice_is_already_installed() {
    let reg = SkillRegistry::new();
    let b = bundle("vox.idem", SkillCategory::Testing);
    reg.install_bundle(&b).await.expect("first");
    let r2 = reg.install_bundle(&b).await.expect("second");
    assert!(r2.already_installed);
}

#[tokio::test]
async fn get_returns_manifest_after_install() {
    let reg = SkillRegistry::new();
    reg.install_bundle(&bundle("vox.fs", SkillCategory::Analysis))
        .await
        .expect("install");
    let m = reg.get("vox.fs").expect("should exist");
    assert_eq!(m.id, "vox.fs");
}

#[tokio::test]
async fn get_returns_none_for_missing_skill() {
    let reg = SkillRegistry::new();
    assert!(reg.get("never.installed").is_none());
}

#[tokio::test]
async fn list_returns_all_installed() {
    let reg = SkillRegistry::new();
    reg.install_bundle(&bundle("a", SkillCategory::Compiler))
        .await
        .expect("a");
    reg.install_bundle(&bundle("b", SkillCategory::Testing))
        .await
        .expect("b");
    let all = reg.list(None);
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn list_filters_by_category() {
    let reg = SkillRegistry::new();
    reg.install_bundle(&bundle("vox.c", SkillCategory::Compiler))
        .await
        .expect("c");
    reg.install_bundle(&bundle("vox.t", SkillCategory::Testing))
        .await
        .expect("t");
    let compilers = reg.list(Some(&SkillCategory::Compiler));
    assert_eq!(compilers.len(), 1);
    assert_eq!(compilers[0].id, "vox.c");
}

#[tokio::test]
async fn search_finds_by_id_substring() {
    let reg = SkillRegistry::new();
    reg.install_bundle(&bundle("vox.git-helper", SkillCategory::Git))
        .await
        .expect("install");
    let hits = reg.search("git");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].id, "vox.git-helper");
}

#[tokio::test]
async fn search_returns_empty_for_no_match() {
    let reg = SkillRegistry::new();
    reg.install_bundle(&bundle("vox.foo", SkillCategory::Compiler))
        .await
        .expect("install");
    assert!(reg.search("zzz_no_match").is_empty());
}

#[tokio::test]
async fn uninstall_removes_from_registry() {
    let reg = SkillRegistry::new();
    reg.install_bundle(&bundle("vox.docs", SkillCategory::Documentation))
        .await
        .expect("install");
    let r = reg.uninstall("vox.docs").await.expect("uninstall");
    assert!(r.was_installed);
    assert!(reg.get("vox.docs").is_none());
}

#[tokio::test]
async fn uninstall_nonexistent_skill_returns_false() {
    let reg = SkillRegistry::new();
    let r = reg.uninstall("never.existed").await.expect("uninstall");
    assert!(!r.was_installed);
}

#[tokio::test]
async fn list_is_empty_after_all_uninstalled() {
    let reg = SkillRegistry::new();
    let b = bundle("vox.tmp", SkillCategory::Testing);
    reg.install_bundle(&b).await.expect("install");
    reg.uninstall("vox.tmp").await.expect("uninstall");
    assert!(reg.list(None).is_empty());
}
