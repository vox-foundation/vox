use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const REPORT_PATH: &str = "contracts/reports/gui-surface-coverage.v1.json";
const OPERATIONS_CATALOG: &str = "contracts/operations/catalog.v1.yaml";
const GUI_NAV: &str = "crates/vox-gui/ui/src/lib/navigation.ts";
const GUI_APP: &str = "crates/vox-gui/ui/src/App.tsx";
const GUI_MAIN: &str = "crates/vox-gui/src/main.rs";
const COMMAND_REGISTRY: &str = "contracts/cli/command-registry.yaml";

#[derive(Debug, Serialize)]
struct CapabilityStatus {
    capability: &'static str,
    status: &'static str,
    evidence: Vec<String>,
    priority: &'static str,
}

/// One `vox` command path, listed whether or not the running binary compiled it in.
#[derive(Debug, Serialize)]
struct CliCommandPath {
    path: String,
    /// Cargo feature expression from the registry (`null` = always available).
    feature_gate: Option<String>,
}

#[derive(Debug, Serialize)]
struct SurfaceCoverageReport {
    schema_version: u8,
    cli_command_paths: Vec<CliCommandPath>,
    operations_rows: Vec<String>,
    gui_routes: Vec<String>,
    gui_ipc_commands: Vec<String>,
    capabilities: Vec<CapabilityStatus>,
}

fn parse_operations_ids(repo_root: &Path) -> Result<Vec<String>> {
    let path = repo_root.join(OPERATIONS_CATALOG);
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let doc: serde_yaml::Value =
        serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    let rows = doc
        .get("operations")
        .and_then(|v| v.as_sequence())
        .ok_or_else(|| anyhow!("{} missing `operations` sequence", path.display()))?;
    let mut ids = Vec::new();
    for row in rows {
        if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
            ids.push(id.to_string());
        }
    }
    ids.sort();
    Ok(ids)
}

fn parse_gui_routes(repo_root: &Path) -> Result<Vec<String>> {
    let mut routes = BTreeSet::new();

    // Parent/child navigation SSOT (vox-gui dock model).
    let nav_path = repo_root.join(GUI_NAV);
    if nav_path.is_file() {
        let raw = fs::read_to_string(&nav_path)
            .with_context(|| format!("read {}", nav_path.display()))?;
        let map_key_re = Regex::new(r"(?m)^\s*(?:'([^']+)'|([a-z][a-z0-9\-]*)):\s*\{\s*parent:")
            .expect("navigation map key regex");
        for caps in map_key_re.captures_iter(&raw) {
            let name = caps
                .get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str().to_string());
            if let Some(name) = name {
                routes.insert(name);
            }
        }
        let top_level_re =
            Regex::new(r#"['"]([a-z][a-z0-9\-]*)['"]"#).expect("top-level view regex");
        if let Some(start) = raw.find("TOP_LEVEL_VIEWS") {
            let slice = &raw[start..];
            if let Some(end) = slice.find("] as const") {
                for caps in top_level_re.captures_iter(&slice[..end]) {
                    if let Some(name) = caps.get(1) {
                        routes.insert(name.as_str().to_string());
                    }
                }
            }
        }
    }

    // Legacy switch/case routes still present in App.tsx event handlers.
    let app_path = repo_root.join(GUI_APP);
    if app_path.is_file() {
        let raw = fs::read_to_string(&app_path)
            .with_context(|| format!("read {}", app_path.display()))?;
        let route_re = Regex::new(r#"case '([a-z\-]+)'"#).expect("route regex");
        for caps in route_re.captures_iter(&raw) {
            if let Some(name) = caps.get(1) {
                routes.insert(name.as_str().to_string());
            }
        }
    }

    Ok(routes.into_iter().collect())
}

fn parse_gui_ipc_commands(repo_root: &Path) -> Result<Vec<String>> {
    let path = repo_root.join(GUI_MAIN);
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let cmd_re = Regex::new(r#"commands::[a-z_]+::([a-z0-9_]+)"#).expect("command regex");
    let mut cmds = BTreeSet::new();
    for caps in cmd_re.captures_iter(&raw) {
        if let Some(name) = caps.get(1) {
            cmds.insert(name.as_str().to_string());
        }
    }
    Ok(cmds.into_iter().collect())
}

fn capability_status(report: &SurfaceCoverageReport) -> Vec<CapabilityStatus> {
    let has_route = |name: &str| report.gui_routes.iter().any(|r| r == name);
    let has_ipc = |needle: &str| report.gui_ipc_commands.iter().any(|c| c.contains(needle));
    let has_op = |needle: &str| report.operations_rows.iter().any(|o| o.contains(needle));
    let has_op_exact = |id: &str| report.operations_rows.iter().any(|o| o == id);

    let mk_status = |ok_full: bool, ok_partial: bool| -> &'static str {
        if ok_full {
            "real"
        } else if ok_partial {
            "partial"
        } else {
            "missing-surface"
        }
    };

    vec![
        CapabilityStatus {
            capability: "orchestrator_control_plane",
            status: mk_status(
                has_route("dashboard") && has_ipc("orchestrator") && has_op_exact("submit.task"),
                has_route("dashboard") || has_ipc("orchestrator"),
            ),
            evidence: vec![
                format!("route:dashboard={}", has_route("dashboard")),
                format!("ipc:*orchestrator*={}", has_ipc("orchestrator")),
                format!("op:submit.task={}", has_op_exact("submit.task")),
            ],
            priority: "p0",
        },
        CapabilityStatus {
            capability: "repository_harness_actions",
            status: mk_status(
                has_route("catalog") && has_op("workspace"),
                has_route("catalog"),
            ),
            evidence: vec![
                format!("route:catalog={}", has_route("catalog")),
                format!("op:*workspace*={}", has_op("workspace")),
            ],
            priority: "p0",
        },
        CapabilityStatus {
            capability: "vox_compile_check_run_loop",
            status: mk_status(
                has_op("vox_check") || has_op("compile"),
                has_route("catalog"),
            ),
            evidence: vec![
                format!(
                    "op:*vox_check|compile*={}",
                    has_op("vox_check") || has_op("compile")
                ),
                format!("route:catalog={}", has_route("catalog")),
            ],
            priority: "p0",
        },
        CapabilityStatus {
            capability: "model_routing_selection",
            status: mk_status(has_route("models") && has_ipc("model"), has_route("models")),
            evidence: vec![
                format!("route:models={}", has_route("models")),
                format!("ipc:*model*={}", has_ipc("model")),
            ],
            priority: "p0",
        },
        CapabilityStatus {
            capability: "memory_provenance",
            status: mk_status(
                has_route("memory") && has_ipc("memory"),
                has_route("memory"),
            ),
            evidence: vec![
                format!("route:memory={}", has_route("memory")),
                format!("ipc:*memory*={}", has_ipc("memory")),
            ],
            priority: "p1",
        },
        CapabilityStatus {
            capability: "runs_replay_timeline",
            status: mk_status(has_route("runs") && has_ipc("run"), has_route("runs")),
            evidence: vec![
                format!("route:runs={}", has_route("runs")),
                format!("ipc:*run*={}", has_ipc("run")),
            ],
            priority: "p1",
        },
        CapabilityStatus {
            capability: "mesh_compute_surface",
            status: mk_status(
                has_route("mesh"),
                has_route("dashboard") || has_route("matrix"),
            ),
            evidence: vec![
                format!("route:mesh={}", has_route("mesh")),
                format!(
                    "dashboard_or_matrix={}",
                    has_route("dashboard") || has_route("matrix")
                ),
            ],
            priority: "p1",
        },
        CapabilityStatus {
            capability: "gamification_feedback",
            status: mk_status(
                has_route("gamify") && has_op_exact("ludus.notifications.list"),
                has_route("gamify"),
            ),
            evidence: vec![
                format!("route:gamify={}", has_route("gamify")),
                format!(
                    "op:ludus.notifications.list={}",
                    has_op_exact("ludus.notifications.list")
                ),
            ],
            priority: "p1",
        },
    ]
}

fn enforce_policy(report: &SurfaceCoverageReport) -> Result<()> {
    const MUST_BE_REAL: &[&str] = &[
        "orchestrator_control_plane",
        "repository_harness_actions",
        "vox_compile_check_run_loop",
        "model_routing_selection",
    ];
    for capability in MUST_BE_REAL {
        let status = report
            .capabilities
            .iter()
            .find(|c| c.capability == *capability)
            .map(|c| c.status)
            .ok_or_else(|| anyhow!("gui-surface-coverage missing capability `{capability}`"))?;
        if status != "real" {
            return Err(anyhow!(
                "gui-surface-coverage policy failed: capability `{capability}` must be real, found `{status}`"
            ));
        }
    }
    Ok(())
}

/// Command paths come from `contracts/cli/command-registry.yaml` (verified against clap by
/// `command-sync`), NOT the compiled clap tree: the tree varies with cargo features, so a
/// feature-built CI binary and a default-built local binary would disagree on the committed
/// report. Feature-gated commands are listed unconditionally, marked with their gate.
fn collect_command_paths(repo_root: &Path) -> Result<Vec<CliCommandPath>> {
    use vox_cli_core::command_contract::merged_feature_gate_from_vox_cli_ops;
    use vox_cli_core::command_registry_model::RegistryFile;

    let path = repo_root.join(COMMAND_REGISTRY);
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let reg: RegistryFile =
        serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    let ops: Vec<_> = reg
        .operations
        .into_iter()
        .filter(|op| op.surface == "vox-cli" && op.status != "retired")
        .collect();
    let paths: BTreeSet<Vec<String>> = ops.iter().map(|op| op.path.clone()).collect();
    Ok(paths
        .into_iter()
        .map(|p| CliCommandPath {
            feature_gate: merged_feature_gate_from_vox_cli_ops(&ops, &p),
            path: p.join(" "),
        })
        .collect())
}

pub fn run(repo_root: &Path, write: bool) -> Result<()> {
    let cli_command_paths = collect_command_paths(repo_root)?;
    let operations_rows = parse_operations_ids(repo_root)?;
    let gui_routes = parse_gui_routes(repo_root)?;
    let gui_ipc_commands = parse_gui_ipc_commands(repo_root)?;

    let mut report = SurfaceCoverageReport {
        schema_version: 2,
        cli_command_paths,
        operations_rows,
        gui_routes,
        gui_ipc_commands,
        capabilities: Vec::new(),
    };
    report.capabilities = capability_status(&report);
    enforce_policy(&report)?;

    let serialized = serde_json::to_string_pretty(&json!(report)).context("serialize report")?;
    let report_path = repo_root.join(REPORT_PATH);
    if write {
        fs::write(&report_path, format!("{serialized}\n"))
            .with_context(|| format!("write {}", report_path.display()))?;
        println!("gui-surface-coverage: wrote {}", report_path.display());
        return Ok(());
    }

    let existing = fs::read_to_string(&report_path).with_context(|| {
        format!(
            "read {} (run `vox ci gui-surface-coverage --write` to generate)",
            report_path.display()
        )
    })?;
    if existing.trim() != serialized.trim() {
        return Err(anyhow!(
            "gui-surface-coverage drift detected at {} (run `vox ci gui-surface-coverage --write`)",
            report_path.display()
        ));
    }
    println!("gui-surface-coverage: report is up to date");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed report must be byte-identical whatever cargo features the
    /// running `vox` was built with. `vox gamify` is compiled out of a default
    /// build (`extras-ludus`) and `vox mcp` is compiled in but runtime-gated
    /// (`mcp-server`); both must be listed, each marked with its real gate, and
    /// the whole list must come from the feature-independent command registry,
    /// not the compiled clap tree.
    #[test]
    fn command_paths_are_feature_independent_and_marked_with_gate() {
        let root = vox_repository::resolve_repo_root_for_ci();
        let paths = collect_command_paths(&root).expect("collect command paths");
        let rendered: Vec<String> = paths.iter().map(|p| format!("{p:?}")).collect();
        let gamify = rendered
            .iter()
            .find(|p| p.contains("\"gamify\"") && !p.contains("hud"))
            .unwrap_or_else(|| {
                panic!(
                    "`gamify` missing from {} paths: list depends on the compiled clap tree",
                    rendered.len()
                )
            });
        assert!(
            gamify.contains("extras-ludus"),
            "gamify must carry its gate: {gamify}"
        );
        let mcp = rendered
            .iter()
            .find(|p| p.contains("\"mcp\""))
            .expect("`mcp` listed");
        assert!(mcp.contains("mcp-server"), "mcp must carry its gate: {mcp}");
    }
}
