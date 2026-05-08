use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn assert_missing(root: &Path, rel: &str) {
    let p = root.join(rel);
    assert!(
        !p.exists(),
        "dedup regression: expected wrapper module to stay removed: {}",
        p.display()
    );
}

#[test]
fn bounded_fs_wrapper_modules_stay_removed_in_converged_crates() {
    let root = workspace_root();
    let removed = [
        "crates/vox-orchestrator/src/bounded_fs.rs",
        "crates/vox-mcp/src/bounded_fs.rs",
        "crates/vox-code-audit/src/bounded_fs.rs",
        "crates/vox-doc-inventory/src/bounded_fs.rs",
        "crates/vox-doc-pipeline/src/pipeline/bounded_fs.rs",
        "crates/vox-publisher/src/bounded_fs.rs",
        "crates/vox-gamify/src/bounded_fs.rs",
        "crates/vox-git/src/bounded_fs.rs",
        "crates/vox-repository/src/bounded_fs.rs",
        "crates/vox-secrets/src/bounded_fs.rs",
        "crates/vox-populi/src/bounded_fs.rs",
        "crates/vox-corpus/src/bounded_fs.rs",
        "crates/vox-lsp/src/bounded_fs.rs",
    ];
    for rel in removed {
        assert_missing(&root, rel);
    }
}

#[test]
fn legacy_mcp_extract_script_stays_explicitly_gated() {
    let root = workspace_root();
    let script = root.join("scripts/extract_mcp_tool_registry.py");
    if !script.exists() {
        // Preferred: legacy Python helper removed (AGENTS.md — no new `scripts/*.py`).
        return;
    }
    let body = std::fs::read_to_string(&script)
        .unwrap_or_else(|e| panic!("read {}: {e}", script.display()));
    assert!(
        body.contains("VOX_ALLOW_LEGACY_MCP_EXTRACT"),
        "expected legacy env gate in {}",
        script.display()
    );
    assert!(
        body.contains("--allow-legacy"),
        "expected explicit legacy flag gate in {}",
        script.display()
    );
}
