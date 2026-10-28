//! Loader for `contracts/pipeline/canonical-ladder.v1.yaml` — SSOT for backwards emission verification.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One curated golden fixture and the language concerns it proves end-to-end.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LadderFixture {
    pub id: String,
    pub path: String,
    pub proves: Vec<String>,
    pub targets: Vec<String>,
}

/// Parsed canonical ladder contract.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CanonicalLadder {
    #[serde(rename = "x-vox-version")]
    pub schema_version: u32,
    pub fixtures: Vec<LadderFixture>,
}

impl CanonicalLadder {
    /// Load ladder YAML from `repo_root/contracts/pipeline/canonical-ladder.v1.yaml`.
    pub fn load_from_repo_root(repo_root: &Path) -> Result<Self, String> {
        let path = repo_root.join("contracts/pipeline/canonical-ladder.v1.yaml");
        let raw =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        serde_yaml::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
    }

    /// Find a fixture by stable `id`.
    #[must_use]
    pub fn fixture(&self, id: &str) -> Option<&LadderFixture> {
        self.fixtures.iter().find(|f| f.id == id)
    }

    /// Stable fixture ids (for k-complexity budget scoping).
    #[must_use]
    pub fn fixture_ids(&self) -> BTreeSet<String> {
        self.fixtures.iter().map(|f| f.id.clone()).collect()
    }

    /// Union of all `proves` tags across fixtures.
    #[must_use]
    pub fn all_proves_tags(&self) -> BTreeSet<String> {
        self.fixtures
            .iter()
            .flat_map(|f| f.proves.iter().cloned())
            .collect()
    }

    /// Absolute path to a golden `.vox` file under `repo_root`.
    #[must_use]
    pub fn fixture_vox_path(&self, repo_root: &Path, id: &str) -> Option<PathBuf> {
        self.fixture(id).map(|f| repo_root.join(&f.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn ladder_loads_twelve_fixtures() {
        let ladder = CanonicalLadder::load_from_repo_root(&repo_root()).expect("load ladder");
        assert_eq!(ladder.schema_version, 1);
        assert_eq!(ladder.fixtures.len(), 12, "{:?}", ladder.fixtures);
        assert!(ladder.fixture("hello").is_some());
        assert!(ladder.fixture("crud_api").is_some());
    }

    #[test]
    fn vox_02_canonical_module_emits_axum_tauri_and_ts_bundles() {
        use crate::codegen_rust::{self, RustAppShell};
        use crate::codegen_ts;
        use crate::projection_bundle::project_and_validate;
        use vox_compiler::pipeline::run_frontend_str;
        use vox_compiler::target::Target;

        let src = r#"table Task {
    title: str
    done: bool
}

@pure
fn score_task(done: bool) to int {
    if done {
        return 10
    }
    return 1
}

query list_task_count() to int {
    return score_task(true)
}

mutation mark_task_done(id: int) to bool {
    return id > 0
}

tool "Summarize task status" summarize_tasks(prefix: str) to str {
    return prefix
}

activity fetch_status(name: str) to Result[str] {
    return Ok(name)
}

workflow process_task(name: str) to Result[str] {
    let status = fetch_status(name)
    return status
}

actor Worker {
    on increment(count: int) to int {
        return count + 1
    }
}

component TaskBadge() {
    state label: str = "Ready"
    view: text() { label }
}
"#;
        let frontend = run_frontend_str(src, "parity_app.vox").expect("frontend ok");
        assert!(!frontend.has_errors(), "{:?}", frontend.diagnostics);
        let hir = &frontend.hir;

        for target in [
            Target::Interpreter,
            Target::RustAxum,
            Target::RustTauri,
            Target::TypeScript,
        ] {
            let bundle_res = project_and_validate(hir, target);
            assert!(
                bundle_res.is_ok(),
                "VOX-02 projection validation failed for {target:?}: {:?}",
                bundle_res.err()
            );
        }

        let axum_out = codegen_rust::generate(hir, "parity_axum", RustAppShell::AxumLocalServer)
            .expect("RustAxum codegen");
        assert!(axum_out.files.contains_key("Cargo.toml"));
        assert!(axum_out.files.contains_key("src/main.rs"));
        assert!(!axum_out.files["src/main.rs"].contains("compile_error!"));

        let tauri_out = codegen_rust::generate(hir, "parity_tauri", RustAppShell::TauriApp)
            .expect("RustTauri codegen");
        assert!(tauri_out.files.contains_key("Cargo.toml"));
        assert!(
            tauri_out
                .files
                .values()
                .all(|v| !v.contains("compile_error!"))
        );

        let ts_out = codegen_ts::generate(hir).expect("TypeScript codegen");
        assert!(
            !ts_out.files.is_empty(),
            "TypeScript codegen must emit files"
        );
    }

    #[test]
    fn vox_03_simulation_rpg_and_default_templates_emit_tier_deps() {
        use crate::codegen_rust::{self, RustAppShell};
        use vox_compiler::pipeline::run_frontend_str;

        let simulation_rpg = r#"table Character {
    name: str
    hp: int
    gold: int
}

actor GameMaster {
    on advance_world(hero: str) to str {
        return hero + " explores the realm"
    }
}

@pure
fn resolve_encounter(hp: int, damage: int) to int {
    if hp > damage {
        return hp - damage
    }
    return 0
}

query world_status() to str {
    let remaining = resolve_encounter(100, 15)
    return "Hero HP: " + str(remaining)
}
"#;
        let frontend = run_frontend_str(simulation_rpg, "simulation_rpg.vox").expect("frontend");
        assert!(!frontend.has_errors(), "{:?}", frontend.diagnostics);
        let rust_out =
            codegen_rust::generate(&frontend.hir, "sim_rpg_app", RustAppShell::AxumLocalServer)
                .expect("simulation-rpg RustAxum codegen");
        let cargo_toml = &rust_out.files["Cargo.toml"];
        assert!(
            cargo_toml.contains("vox-db"),
            "VOX-03: Tier 1/2/2.5 resolver must include vox-db for table declarations: {cargo_toml}"
        );
    }
}
