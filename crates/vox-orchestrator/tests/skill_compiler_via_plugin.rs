//! SP4 batch 3 e2e test: with the vox-plugin-skill-compiler directory
//! installed at a known plugins root, the orchestrator's skill registry
//! must contain the compiler skill (registered via the plugin-host bridge).
//!
//! The skill id registered comes from parsing the SKILL.md frontmatter via
//! `vox_skills::parser::parse_skill_md`, which reads `id = "vox.compiler"`.
//! The plugin-host discover step uses `manifest.plugin.id = "skill-compiler"`
//! as the lookup key, but the bridge re-parses the body so vox-skills ends up
//! with the frontmatter id "vox.compiler".

use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn compiler_skill_loaded_via_plugin_bridge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // vox-plugin-host discover() walks for Plugin.toml; layout: <root>/<any-dir>/Plugin.toml
    let plugin_dir = tmp.path().join("skill-compiler").join("0.1.0");
    std::fs::create_dir_all(&plugin_dir).expect("mkdir plugin_dir");

    // Copy the in-tree skill plugin to the tempdir install layout.
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates parent")
        .join("vox-plugin-skill-compiler");
    for f in ["Plugin.toml", "compiler.skill.md"] {
        std::fs::copy(src.join(f), plugin_dir.join(f))
            .unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }

    // Build a fresh registry, install builtins (7 skills, no compiler),
    // then bridge in plugin-host skills from the tempdir.
    let registry: Arc<vox_skills::SkillRegistry> = vox_skills::new_registry_arc();
    let builtin_count = vox_skills::install_builtins(&registry)
        .await
        .expect("install_builtins");
    // vox.compiler was removed from builtins in SP4 batch 1; expect exactly 7.
    assert!(
        builtin_count <= 7,
        "expected ≤7 builtins (compiler removed), got {builtin_count}"
    );
    assert!(
        registry.get("vox.compiler").is_none(),
        "vox.compiler should NOT be in builtins after SP4 batch 1"
    );

    // Bridge: discover plugins from tempdir and install into registry.
    vox_orchestrator::mcp_tools::plugin_skills_bridge::install_discovered_skills(
        &registry,
        tmp.path(),
    )
    .await;

    // The bridge parses the SKILL.md frontmatter, which declares id = "vox.compiler".
    let manifest = registry.get("vox.compiler");
    eprintln!(
        "registry.get(\"vox.compiler\") = {:?}",
        manifest.as_ref().map(|m| &m.id)
    );
    assert!(
        manifest.is_some(),
        "expected vox.compiler in registry after bridge; bridge must have parsed SKILL.md frontmatter"
    );
    let m = manifest.unwrap();
    assert_eq!(m.id, "vox.compiler");
    assert_eq!(m.version, "0.1.0");
}
