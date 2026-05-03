//! Tests discover() with on-disk fixture plugin directories. We can't yet
//! exercise code-plugin loading (needs noop-code in batch 6), but skill
//! discovery is fully testable here.

use std::fs;
use vox_plugin_host::discover;

const SKILL_PLUGIN_TOML: &str = r#"
[plugin]
id = "discover-test-skill"
name = "Discover Test Skill"
version = "0.1.0"
description = "Test fixture for discover()."

[plugin.host]
min-vox-version = "0.5.0"

[plugin.payload]
kind = "skill"
format-version = 1
skill-md = "discover-test.skill.md"

[plugin.payload.tools]
exposes = ["fake_tool"]
"#;

const SKILL_MD: &str = "# Discover Test Skill\n\nFake content for testing.\n";

#[test]
fn discover_finds_a_skill_plugin() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let plugin_dir = tmp.path().join("discover-test-skill").join("0.1.0");
    fs::create_dir_all(&plugin_dir).expect("mkdir");
    fs::write(plugin_dir.join("Plugin.toml"), SKILL_PLUGIN_TOML).expect("write toml");
    fs::write(plugin_dir.join("discover-test.skill.md"), SKILL_MD).expect("write md");

    let registry = discover(tmp.path()).expect("discover should succeed");
    assert!(registry.has("discover-test-skill"));

    let skill = registry.skills.lookup("discover-test-skill").expect("lookup");
    assert_eq!(skill.exposed_tools, vec!["fake_tool".to_string()]);
    assert!(skill.body.contains("Fake content"));
}

#[test]
fn discover_handles_missing_root_gracefully() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let nonexistent = tmp.path().join("does-not-exist");
    let registry = discover(&nonexistent).expect("should not error on missing dir");
    assert!(registry.list_ids().is_empty());
}

#[test]
fn discover_skips_directories_without_manifest() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("not-a-plugin")).expect("mkdir");
    let registry = discover(tmp.path()).expect("should succeed");
    assert!(registry.list_ids().is_empty());
}
