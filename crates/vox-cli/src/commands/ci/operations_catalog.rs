//! Unified operations catalog sync/verify for CLI + MCP + planner metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use super::command_compliance::registry::{
    MCP_TOOL_REGISTRY_REL, REGISTRY_REL, validate_mcp_tool_registry_against_json_schema,
    validate_registry_against_json_schema,
};
use super::operations_catalog_exposure::ensure_mcp_classified;
use crate::command_registry_model::{RegistryFile, RegistryOperation};
use vox_bounded_fs::read_utf8_path_capped;
use vox_capability_registry::{
    CAPABILITY_REGISTRY_REL, CapabilityRegistryDoc, CuratedCapability, Exemptions,
    RuntimeBuiltinMap,
};

pub const OPERATIONS_CATALOG_REL: &str = "contracts/operations/catalog.v1.yaml";
pub const OPERATIONS_CATALOG_SCHEMA_REL: &str = "contracts/operations/catalog.v1.schema.json";
pub const OPERATIONS_INVENTORY_REPORT_REL: &str =
    "contracts/reports/operations-catalog-inventory.v1.json";

/// Wire aliases accepted by MCP (`tool_aliases.rs`); used when verifying `input_schemas.rs` coverage.
const MCP_TOOL_WIRE_ALIASES: &[(&str, &str)] = &[
    ("vox_get_config", "vox_config_get"),
    ("vox_set_config", "vox_config_set"),
    ("vox_map_opencode_session", "vox_map_agent_session"),
    ("vox_map_vscode_session", "vox_map_agent_session"),
    ("vox_budget_history", "vox_cost_history"),
    ("vox_model_list", "vox_list_models"),
    (
        "vox_ludus_notifications_list",
        "vox_gamify_notifications_list",
    ),
    (
        "vox_ludus_progress_snapshot",
        "vox_gamify_progress_snapshot",
    ),
    ("vox_ludus_notification_ack", "vox_gamify_notification_ack"),
    (
        "vox_ludus_notifications_ack_all",
        "vox_gamify_notifications_ack_all",
    ),
    ("vox_ludus_quest_list", "vox_gamify_quest_list"),
    ("vox_ludus_shop_catalog", "vox_gamify_shop_catalog"),
    ("vox_ludus_shop_buy", "vox_gamify_shop_buy"),
    ("vox_ludus_collegium_join", "vox_gamify_collegium_join"),
    ("vox_ludus_battle_start", "vox_gamify_battle_start"),
    ("vox_ludus_battle_submit", "vox_gamify_battle_submit"),
];

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OperationsCatalog {
    pub schema_version: u32,
    #[serde(default)]
    pub capability: CapabilityBlock,
    #[serde(default)]
    pub operations: Vec<OperationRow>,
    #[serde(default)]
    pub exemptions: CatalogExemptions,
}

/// Static capability-registry fields edited only via the operations catalog (`capability:` block).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CapabilityBlock {
    #[serde(default = "default_bool_true")]
    pub auto_mcp_capabilities: bool,
    #[serde(default = "default_bool_true")]
    pub auto_cli_capabilities: bool,
    #[serde(default)]
    pub runtime_builtin_maps: Vec<RuntimeBuiltinMapRow>,
    /// Umbrella CLI paths that skip discrete planner capabilities (mirrors generated capability-registry `exemptions`).
    #[serde(default)]
    pub exemptions: CatalogExemptions,
}

impl Default for CapabilityBlock {
    fn default() -> Self {
        Self {
            auto_mcp_capabilities: true,
            auto_cli_capabilities: true,
            runtime_builtin_maps: Vec::new(),
            exemptions: CatalogExemptions::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeBuiltinMapRow {
    pub namespace: String,
    pub method: String,
    pub capability_id: String,
}

fn default_bool_true() -> bool {
    true
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CatalogExemptions {
    #[serde(default)]
    pub mcp_tools: Vec<String>,
    #[serde(default)]
    pub cli_paths: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OperationRow {
    pub id: String,
    pub title: String,
    pub description: String,
    /// Optional human-facing description for CLI-only capabilities (planner); defaults to [`Self::description`].
    #[serde(default)]
    pub description_human: Option<String>,
    /// Optional owning-crate override for the capability index (must be a `layers.toml [crates]` key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_crate: Option<String>,
    /// Optional capability-index domain override (must be an id in `contracts/capabilities/domains.v1.yaml`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub product_lane: String,
    #[serde(default)]
    pub intent_tags: Vec<String>,
    #[serde(default)]
    pub side_effect_class: Option<String>,
    #[serde(default)]
    pub scope_kind: Option<String>,
    #[serde(default)]
    pub reversible: Option<bool>,
    #[serde(default)]
    pub requires_repo: Option<bool>,
    #[serde(default)]
    pub preferred_for_models: Option<bool>,
    #[serde(default)]
    pub human_takeover_friendly: Option<bool>,
    #[serde(default)]
    pub mens_planner_visible: Option<bool>,
    #[serde(default)]
    pub canonical_name: Option<String>,
    #[serde(default)]
    pub latin_aliases: Option<Vec<String>>,
    #[serde(default)]
    pub mcp: Option<McpProjection>,
    #[serde(default)]
    pub cli: Option<CliProjection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpProjection {
    pub name: String,
    #[serde(default)]
    pub http_read_role_eligible: bool,
    /// `public | operator | internal`; required for every MCP op (capability profiles).
    #[serde(default)]
    pub exposure: Option<String>,
    /// Subsystem ids the tool needs booted; `[]` when none. Required for every MCP op.
    #[serde(default)]
    pub requires: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CliProjection {
    pub path: Vec<String>,
    pub status: String,
    #[serde(default)]
    pub latin_ns: Option<String>,
    #[serde(default)]
    pub handler_rust: Option<String>,
    #[serde(default)]
    pub feature_gate: Option<String>,
    #[serde(default)]
    pub catalog_group: Option<String>,
    #[serde(default)]
    pub ref_cli_required: Option<bool>,
    #[serde(default)]
    pub reachability_required: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct McpCanonicalRegistry {
    version: u32,
    tools: Vec<McpCanonicalTool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct McpCanonicalTool {
    name: String,
    description: String,
    product_lane: String,
    #[serde(default)]
    http_read_role_eligible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exposure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    side_effect_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requires: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct InventoryReport {
    catalog_operations: usize,
    paired_operations: usize,
    mcp_only_operations: usize,
    cli_only_operations: usize,
    mcp_tool_count: usize,
    cli_path_count: usize,
}

pub(crate) fn normalize_lf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

pub(crate) fn read_catalog(repo_root: &Path) -> Result<OperationsCatalog> {
    let raw = read_utf8_path_capped(&repo_root.join(OPERATIONS_CATALOG_REL))
        .with_context(|| format!("read {}", OPERATIONS_CATALOG_REL))?;
    serde_yaml::from_str(&raw).context("parse operations catalog")
}

fn validate_operations_catalog_against_json_schema(
    repo_root: &Path,
    yaml_text: &str,
) -> Result<()> {
    let schema_path = repo_root.join(OPERATIONS_CATALOG_SCHEMA_REL);
    let schema_val: serde_json::Value = serde_json::from_str(&read_utf8_path_capped(&schema_path)?)
        .with_context(|| format!("parse {} as JSON", schema_path.display()))?;
    let instance: serde_json::Value =
        serde_yaml::from_str(yaml_text).context("parse operations catalog to JSON value")?;
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())
        .context("compile operations catalog JSON Schema")?;
    vox_jsonschema_util::validate(
        &instance,
        &validator,
        "catalog.v1.yaml vs catalog.v1.schema.json",
    )
    .map_err(|e| anyhow!("{e:#}\nInstance dump: {:?}", e))?;
    Ok(())
}

fn implicit_id_from_mcp_tool(name: &str) -> String {
    name.trim_start_matches("vox_").replace('_', ".")
}

fn implicit_id_from_cli_path(path: &[String]) -> String {
    path.join(".")
}

fn title_from_id(id: &str) -> String {
    id.split('.')
        .map(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_catalog_from_live_registries(repo_root: &Path) -> Result<OperationsCatalog> {
    let prev = read_catalog(repo_root).context(
        "operations-sync --target catalog requires an existing catalog.v1.yaml (SSOT edit source)",
    )?;

    let mcp_raw = read_utf8_path_capped(&repo_root.join(MCP_TOOL_REGISTRY_REL))
        .with_context(|| format!("read {}", MCP_TOOL_REGISTRY_REL))?;
    validate_mcp_tool_registry_against_json_schema(repo_root, &mcp_raw)?;
    let mcp: McpCanonicalRegistry =
        serde_yaml::from_str(&mcp_raw).context("parse tool-registry.canonical.yaml")?;

    let cli_raw = read_utf8_path_capped(&repo_root.join(REGISTRY_REL))
        .with_context(|| format!("read {}", REGISTRY_REL))?;
    validate_registry_against_json_schema(repo_root, &cli_raw)?;
    let cli: RegistryFile =
        serde_yaml::from_str(&cli_raw).context("parse command-registry.yaml")?;

    let mut pairing_from_cap: BTreeMap<String, String> = BTreeMap::new();
    let cap_path = repo_root.join(CAPABILITY_REGISTRY_REL);
    if cap_path.is_file() {
        let cap_raw = read_utf8_path_capped(&cap_path)
            .with_context(|| format!("read {}", cap_path.display()))?;
        if let Ok(cap_doc) = serde_yaml::from_str::<CapabilityRegistryDoc>(&cap_raw) {
            for row in &cap_doc.curated {
                if let (Some(mcp_tool), Some(cli_path)) = (&row.mcp_tool, &row.cli_path) {
                    pairing_from_cap.insert(mcp_tool.clone(), implicit_id_from_cli_path(cli_path));
                }
            }
        }
    }

    let prev_by_id: BTreeMap<String, OperationRow> = prev
        .operations
        .iter()
        .cloned()
        .map(|r| (r.id.clone(), r))
        .collect();

    let mut rows: BTreeMap<String, OperationRow> = BTreeMap::new();

    for t in mcp.tools {
        let id = pairing_from_cap
            .get(&t.name)
            .cloned()
            .unwrap_or_else(|| implicit_id_from_mcp_tool(&t.name));
        let row = rows.entry(id.clone()).or_insert_with(|| OperationRow {
            id: id.clone(),
            title: title_from_id(&id),
            description: t.description.clone(),
            description_human: None,
            owner_crate: None,
            domain: None,
            product_lane: t.product_lane.clone(),
            intent_tags: Vec::new(),
            side_effect_class: None,
            scope_kind: None,
            reversible: None,
            requires_repo: None,
            preferred_for_models: None,
            human_takeover_friendly: None,
            mens_planner_visible: None,
            canonical_name: None,
            latin_aliases: None,
            mcp: None,
            cli: None,
        });
        if row.description.is_empty() {
            row.description = t.description.clone();
        }
        row.product_lane = t.product_lane.clone();
        if row.side_effect_class.is_none() {
            row.side_effect_class = t.side_effect_class.clone();
        }
        row.mcp = Some(McpProjection {
            name: t.name,
            http_read_role_eligible: t.http_read_role_eligible,
            exposure: t.exposure,
            requires: t.requires,
        });
    }

    for op in cli
        .operations
        .into_iter()
        .filter(|o| o.surface == "vox-cli")
    {
        let id = implicit_id_from_cli_path(&op.path);
        let row = rows.entry(id.clone()).or_insert_with(|| OperationRow {
            id: id.clone(),
            title: title_from_id(&id),
            description: format!("CLI operation `vox {}`", op.path.join(" ")),
            description_human: None,
            owner_crate: None,
            domain: None,
            product_lane: op
                .product_lane
                .clone()
                .unwrap_or_else(|| "platform".to_string()),
            intent_tags: Vec::new(),
            side_effect_class: None,
            scope_kind: None,
            reversible: None,
            requires_repo: None,
            preferred_for_models: None,
            human_takeover_friendly: None,
            mens_planner_visible: None,
            canonical_name: None,
            latin_aliases: None,
            mcp: None,
            cli: None,
        });
        if let Some(lane) = &op.product_lane {
            row.product_lane = lane.clone();
        }
        row.cli = Some(CliProjection {
            path: op.path.clone(),
            status: op.status.clone(),
            latin_ns: op.latin_ns.clone(),
            handler_rust: op.handler_rust.clone(),
            feature_gate: op.feature_gate.clone(),
            catalog_group: op.catalog_group.clone(),
            ref_cli_required: Some(op.ref_cli_required),
            reachability_required: op.reachability_required,
        });
    }

    // Curated nomenclature (`canonical_name` / `latin_aliases`) may live on a hand-written alias
    // row (e.g. id `ludus`, path `gamify`) whose CLI path the registries derive under another id
    // (`gamify`). Merge it onto the derived row so the alias survives regeneration without a
    // duplicate row for the same CLI path.
    let mut nomenclature_by_cli_path: BTreeMap<String, (Option<String>, Option<Vec<String>>)> =
        BTreeMap::new();
    for p in prev_by_id.values() {
        if let (Some(cli), true) = (
            &p.cli,
            p.canonical_name.is_some() || p.latin_aliases.is_some(),
        ) {
            nomenclature_by_cli_path.insert(
                cli.path.join("."),
                (p.canonical_name.clone(), p.latin_aliases.clone()),
            );
        }
    }

    for row in rows.values_mut() {
        apply_prev_planner_metadata(row, prev_by_id.get(&row.id));
        if row.canonical_name.is_none()
            && row.latin_aliases.is_none()
            && let Some(cli) = &row.cli
            && let Some((canon, aliases)) = nomenclature_by_cli_path.get(&cli.path.join("."))
        {
            row.canonical_name = canon.clone();
            row.latin_aliases = aliases.clone();
        }
    }

    Ok(OperationsCatalog {
        schema_version: 1,
        capability: prev.capability,
        operations: rows.into_values().collect(),
        exemptions: prev.exemptions,
    })
}

fn apply_prev_planner_metadata(row: &mut OperationRow, prev: Option<&OperationRow>) {
    let Some(p) = prev else {
        return;
    };
    if row.id != p.id {
        return;
    }
    row.title = p.title.clone();
    row.intent_tags = p.intent_tags.clone();
    row.side_effect_class = p.side_effect_class.clone();
    row.scope_kind = p.scope_kind.clone();
    row.reversible = p.reversible;
    row.requires_repo = p.requires_repo;
    row.preferred_for_models = p.preferred_for_models;
    row.human_takeover_friendly = p.human_takeover_friendly;
    row.mens_planner_visible = p.mens_planner_visible;
    row.description_human = p.description_human.clone();
    row.owner_crate = p.owner_crate.clone();
    row.domain = p.domain.clone();
    row.canonical_name = p.canonical_name.clone();
    row.latin_aliases = p.latin_aliases.clone();
}

fn write_inventory_report(repo_root: &Path, catalog: &OperationsCatalog) -> Result<()> {
    let mut paired = 0usize;
    let mut mcp_only = 0usize;
    let mut cli_only = 0usize;
    let mut mcp_count = 0usize;
    let mut cli_count = 0usize;
    for row in &catalog.operations {
        match (row.mcp.is_some(), row.cli.is_some()) {
            (true, true) => paired += 1,
            (true, false) => mcp_only += 1,
            (false, true) => cli_only += 1,
            (false, false) => {}
        }
        if row.mcp.is_some() {
            mcp_count += 1;
        }
        if row.cli.is_some() {
            cli_count += 1;
        }
    }
    let report = InventoryReport {
        catalog_operations: catalog.operations.len(),
        paired_operations: paired,
        mcp_only_operations: mcp_only,
        cli_only_operations: cli_only,
        mcp_tool_count: mcp_count,
        cli_path_count: cli_count,
    };
    let path = repo_root.join(OPERATIONS_INVENTORY_REPORT_REL);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&report)? + "\n";
    if path.is_file() {
        let existing =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        if existing == json {
            return Ok(());
        }
    }
    fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn verify(repo_root: &Path) -> Result<()> {
    let raw = read_utf8_path_capped(&repo_root.join(OPERATIONS_CATALOG_REL))
        .with_context(|| format!("read {}", OPERATIONS_CATALOG_REL))?;
    // Constraint: the JSON Schema rejects rows missing exposure/requires/side_effect_class before `ensure_mcp_classified` runs; the per-op listing comes from `operations-sync --target mcp`.
    validate_operations_catalog_against_json_schema(repo_root, &raw)?;
    let catalog: OperationsCatalog =
        serde_yaml::from_str(&raw).context("parse operations catalog")?;

    let mcp_raw = read_utf8_path_capped(&repo_root.join(MCP_TOOL_REGISTRY_REL))
        .with_context(|| format!("read {}", MCP_TOOL_REGISTRY_REL))?;
    let mcp: McpCanonicalRegistry =
        serde_yaml::from_str(&mcp_raw).context("parse tool-registry.canonical.yaml")?;
    let mcp_map: BTreeMap<String, McpCanonicalTool> =
        mcp.tools.into_iter().map(|t| (t.name.clone(), t)).collect();

    let cli_raw = read_utf8_path_capped(&repo_root.join(REGISTRY_REL))
        .with_context(|| format!("read {}", REGISTRY_REL))?;
    let reg: RegistryFile =
        serde_yaml::from_str(&cli_raw).context("parse command-registry.yaml")?;
    let cli_map: BTreeMap<Vec<String>, RegistryOperation> = reg
        .operations
        .into_iter()
        .filter(|o| o.surface == "vox-cli")
        .map(|o| (o.path.clone(), o))
        .collect();

    let mut seen_mcp = BTreeSet::<String>::new();
    let mut seen_cli = BTreeSet::<Vec<String>>::new();

    for row in &catalog.operations {
        if row.mcp.is_none() && row.cli.is_none() {
            return Err(anyhow!(
                "catalog operation '{}' must define at least one projection (mcp or cli)",
                row.id
            ));
        }
        if let Some(m) = &row.mcp {
            let tool = mcp_map.get(&m.name).ok_or_else(|| {
                anyhow!(
                    "catalog operation '{}' references unknown MCP tool '{}'",
                    row.id,
                    m.name
                )
            })?;
            if tool.product_lane != row.product_lane {
                return Err(anyhow!(
                    "catalog operation '{}' product_lane mismatch: catalog='{}' mcp='{}'",
                    row.id,
                    row.product_lane,
                    tool.product_lane
                ));
            }
            if tool.description != row.description {
                return Err(anyhow!(
                    "catalog operation '{}' description mismatch with MCP tool '{}'",
                    row.id,
                    m.name
                ));
            }
            if tool.http_read_role_eligible != m.http_read_role_eligible {
                return Err(anyhow!(
                    "catalog operation '{}' read-role flag mismatch for MCP tool '{}'",
                    row.id,
                    m.name
                ));
            }
            seen_mcp.insert(m.name.clone());
        }
        if let Some(c) = &row.cli {
            let op = cli_map.get(&c.path).ok_or_else(|| {
                anyhow!(
                    "catalog operation '{}' references unknown CLI path {:?}",
                    row.id,
                    c.path
                )
            })?;
            if op.status != c.status {
                return Err(anyhow!(
                    "catalog operation '{}' CLI status mismatch for path {:?}",
                    row.id,
                    c.path
                ));
            }
            if op.product_lane.as_deref().unwrap_or("platform") != row.product_lane {
                return Err(anyhow!(
                    "catalog operation '{}' product_lane mismatch for CLI path {:?}",
                    row.id,
                    c.path
                ));
            }
            if let Some(handler) = &c.handler_rust
                && op.handler_rust.as_deref() != Some(handler.as_str())
            {
                return Err(anyhow!(
                    "catalog operation '{}' handler mismatch for CLI path {:?}",
                    row.id,
                    c.path
                ));
            }
            seen_cli.insert(c.path.clone());
        }
    }

    let exempt_mcp: BTreeSet<String> = catalog.exemptions.mcp_tools.iter().cloned().collect();
    for tool in mcp_map.keys() {
        if !seen_mcp.contains(tool) && !exempt_mcp.contains(tool) {
            return Err(anyhow!(
                "operations catalog orphan MCP tool '{}'; add row or exemption",
                tool
            ));
        }
    }
    let exempt_cli: BTreeSet<Vec<String>> = catalog.exemptions.cli_paths.iter().cloned().collect();
    for path in cli_map.keys() {
        if !seen_cli.contains(path) && !exempt_cli.contains(path) {
            return Err(anyhow!(
                "operations catalog orphan CLI path {:?}; add row or exemption",
                path
            ));
        }
    }
    verify_catalog_nomenclature(&catalog)?;
    ensure_mcp_classified(&catalog)?;
    verify_mcp_dispatch_coverage(repo_root, &seen_mcp)?;
    verify_mcp_input_schema_coverage(repo_root, &seen_mcp)?;
    verify_catalog_read_role_vs_governance(repo_root, &catalog)?;
    verify_derived_registry_artifacts(repo_root, &catalog)?;

    write_inventory_report(repo_root, &catalog)?;
    println!(
        "operations-verify OK ({} catalog rows, inventory report updated)",
        catalog.operations.len()
    );
    Ok(())
}

#[derive(Debug, Deserialize)]
struct HttpReadRoleGovernanceFile {
    #[allow(dead_code)]
    version: u32,
    read_role_tools: Vec<String>,
}

fn verify_catalog_read_role_vs_governance(
    repo_root: &Path,
    catalog: &OperationsCatalog,
) -> Result<()> {
    let gov_path = repo_root.join("contracts/mcp/http-read-role-governance.yaml");
    let gov_raw =
        read_utf8_path_capped(&gov_path).with_context(|| format!("read {}", gov_path.display()))?;
    let gov: HttpReadRoleGovernanceFile =
        serde_yaml::from_str(&gov_raw).context("parse http-read-role-governance.yaml")?;
    let mut expected: Vec<String> = catalog
        .operations
        .iter()
        .filter_map(|row| {
            let m = row.mcp.as_ref()?;
            m.http_read_role_eligible.then(|| m.name.clone())
        })
        .collect();
    expected.sort();
    expected.dedup();
    let mut actual = gov.read_role_tools.clone();
    actual.sort();
    if expected != actual {
        return Err(anyhow!(
            "catalog `http_read_role_eligible` tool set != http-read-role-governance read_role_tools.\nexpected (from catalog): {:?}\nactual (governance file): {:?}\nupdate governance or catalog flags, then `vox ci operations-sync --target mcp --write` if registry changed",
            expected,
            actual
        ));
    }
    Ok(())
}

fn tool_input_schema_fn_slice(repo_root: &Path) -> Result<String> {
    let p = repo_root.join("crates/vox-orchestrator-mcp/src/input_schemas.rs");
    let s = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    let start = s
        .find("pub(super) fn tool_input_schema")
        .ok_or_else(|| anyhow!("tool_input_schema not found in {}", p.display()))?;
    let tail = &s[start..];
    let brace0 = tail
        .find('{')
        .ok_or_else(|| anyhow!("tool_input_schema: expected '{{'"))?;
    let mut depth = 0usize;
    for (i, ch) in tail[brace0..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(tail[..brace0 + i + ch.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    Err(anyhow!("tool_input_schema: unbalanced braces"))
}

fn canonical_mcp_tool_for_schema(mut name: &str) -> &str {
    for (alias, c) in MCP_TOOL_WIRE_ALIASES {
        if *alias == name {
            name = c;
            break;
        }
    }
    name
}

fn verify_mcp_input_schema_coverage(repo_root: &Path, mcp_names: &BTreeSet<String>) -> Result<()> {
    let body = tool_input_schema_fn_slice(repo_root)?;
    for raw_name in mcp_names {
        let name = canonical_mcp_tool_for_schema(raw_name.as_str());
        let needle = format!("\"{name}\"");
        if !body.contains(&needle) {
            return Err(anyhow!(
                "MCP tool `{raw_name}` has no explicit `tool_input_schema` arm (missing {needle} in crates/vox-orchestrator-mcp/src/input_schemas.rs)"
            ));
        }
    }
    Ok(())
}

/// Ensures committed MCP / CLI / capability registry files match catalog projections (no hand drift).
fn verify_derived_registry_artifacts(repo_root: &Path, catalog: &OperationsCatalog) -> Result<()> {
    let mcp_text = mcp_registry_yaml_text(catalog)?;
    let path_mcp = repo_root.join(MCP_TOOL_REGISTRY_REL);
    write_or_verify_text(
        &path_mcp,
        &mcp_text,
        false,
        "run `vox ci operations-sync --target mcp --write`",
    )?;

    let cli_text = cli_registry_yaml_text(repo_root, catalog)?;
    let path_cli = repo_root.join(REGISTRY_REL);
    write_or_verify_text(
        &path_cli,
        &cli_text,
        false,
        "run `vox ci operations-sync --target cli --write`",
    )?;

    let cap_text = capability_registry_yaml_text(catalog)?;
    let path_cap = repo_root.join(CAPABILITY_REGISTRY_REL);
    write_or_verify_text(
        &path_cap,
        &cap_text,
        false,
        "run `vox ci operations-sync --target capability --write`",
    )?;
    Ok(())
}

fn verify_mcp_dispatch_coverage(repo_root: &Path, mcp_names: &BTreeSet<String>) -> Result<()> {
    let dispatch_path = repo_root.join("crates/vox-orchestrator-mcp/src/dispatch.rs");
    let dispatch = read_utf8_path_capped(&dispatch_path)
        .with_context(|| format!("read {}", dispatch_path.display()))?;
    for name in mcp_names {
        let needle = format!("\"{name}\"");
        if !dispatch.contains(&needle) {
            return Err(anyhow!(
                "operations catalog MCP tool `{}` not found in vox-orchestrator dispatch match arms ({})",
                name,
                dispatch_path.display()
            ));
        }
    }
    Ok(())
}

fn project_mcp_registry(catalog: &OperationsCatalog) -> McpCanonicalRegistry {
    let mut tools: Vec<McpCanonicalTool> = catalog
        .operations
        .iter()
        .filter_map(|row| {
            let m = row.mcp.as_ref()?;
            Some(McpCanonicalTool {
                name: m.name.clone(),
                description: row.description.clone(),
                product_lane: row.product_lane.clone(),
                http_read_role_eligible: m.http_read_role_eligible,
                exposure: m.exposure.clone(),
                side_effect_class: row.side_effect_class.clone(),
                requires: m.requires.clone(),
            })
        })
        .collect();
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    McpCanonicalRegistry { version: 1, tools }
}

fn project_cli_registry(repo_root: &Path, catalog: &OperationsCatalog) -> Result<RegistryFile> {
    let raw = read_utf8_path_capped(&repo_root.join(REGISTRY_REL))
        .with_context(|| format!("read {}", REGISTRY_REL))?;
    let existing: RegistryFile =
        serde_yaml::from_str(&raw).context("parse command-registry.yaml")?;
    let mut vox_cli: Vec<RegistryOperation> = Vec::new();
    for row in &catalog.operations {
        let Some(cli) = &row.cli else {
            continue;
        };
        vox_cli.push(RegistryOperation {
            surface: "vox-cli".to_string(),
            path: cli.path.clone(),
            status: cli.status.clone(),
            latin_ns: cli.latin_ns.clone(),
            product_lane: Some(row.product_lane.clone()),
            feature_gate: cli.feature_gate.clone(),
            catalog_group: cli.catalog_group.clone(),
            ref_cli_required: cli.ref_cli_required.unwrap_or(true),
            reachability_required: cli.reachability_required,
            handler_rust: cli.handler_rust.clone(),
        });
    }
    vox_cli.sort_by(|a, b| a.path.cmp(&b.path));
    let mut rest: Vec<RegistryOperation> = existing
        .operations
        .into_iter()
        .filter(|o| o.surface != "vox-cli")
        .collect();
    let mut operations = vox_cli;
    operations.append(&mut rest);
    Ok(RegistryFile {
        schema_version: existing.schema_version,
        operations,
        script_duals: existing.script_duals,
        env_var_ssot_index: existing.env_var_ssot_index,
    })
}

fn curated_entries_from_row(row: &OperationRow) -> Vec<CuratedCapability> {
    let mut out = Vec::new();
    if let Some(m) = &row.mcp {
        out.push(CuratedCapability {
            id: format!("mcp.{}", m.name),
            title: Some(row.title.clone()),
            description_human: row.description_human.clone(),
            description_model: Some(row.description.clone()),
            intent_tags: row.intent_tags.clone(),
            side_effect_class: row.side_effect_class.clone(),
            scope_kind: row.scope_kind.clone(),
            reversible: row.reversible,
            requires_repo: row.requires_repo,
            requires_git: None,
            preferred_for_models: row.preferred_for_models,
            human_takeover_friendly: row.human_takeover_friendly,
            mens_planner_visible: row.mens_planner_visible,
            mcp_tool: Some(m.name.clone()),
            cli_path: None,
            parameters: None,
        });
    }
    if let Some(c) = &row.cli {
        // `validate_cross_registry` only knows active CLI paths; skip retired/deprecated rows.
        if c.status != "active" {
            return out;
        }
        let (description_model, description_human) = if row.mcp.is_some() {
            (
                None,
                row.description_human
                    .clone()
                    .or_else(|| Some(row.description.clone())),
            )
        } else if row.description_human.is_some() {
            (
                None,
                row.description_human
                    .clone()
                    .or_else(|| Some(row.description.clone())),
            )
        } else {
            (Some(row.description.clone()), None)
        };
        out.push(CuratedCapability {
            id: format!("cli.{}", c.path.join(".")),
            title: Some(row.title.clone()),
            description_human,
            description_model,
            intent_tags: row.intent_tags.clone(),
            side_effect_class: row.side_effect_class.clone(),
            scope_kind: row.scope_kind.clone(),
            reversible: row.reversible,
            requires_repo: row.requires_repo,
            requires_git: None,
            preferred_for_models: row.preferred_for_models,
            human_takeover_friendly: row.human_takeover_friendly,
            mens_planner_visible: row.mens_planner_visible,
            mcp_tool: None,
            cli_path: Some(c.path.clone()),
            parameters: None,
        });
    }
    out
}

fn project_capability_registry_doc(catalog: &OperationsCatalog) -> CapabilityRegistryDoc {
    let mut curated: Vec<CuratedCapability> = catalog
        .operations
        .iter()
        .flat_map(curated_entries_from_row)
        .collect();
    curated.sort_by(|a, b| a.id.cmp(&b.id));
    let cap = &catalog.capability;
    let runtime_builtin_maps: Vec<RuntimeBuiltinMap> = cap
        .runtime_builtin_maps
        .iter()
        .map(|r| RuntimeBuiltinMap {
            namespace: r.namespace.clone(),
            method: r.method.clone(),
            capability_id: r.capability_id.clone(),
        })
        .collect();
    let exemptions = if cap.exemptions.cli_paths.is_empty() {
        None
    } else {
        Some(Exemptions {
            cli_paths: cap.exemptions.cli_paths.clone(),
        })
    };
    CapabilityRegistryDoc {
        schema_version: 1,
        auto_mcp_capabilities: cap.auto_mcp_capabilities,
        auto_cli_capabilities: cap.auto_cli_capabilities,
        curated,
        runtime_builtin_maps,
        exemptions,
    }
}

fn mcp_registry_yaml_text(catalog: &OperationsCatalog) -> Result<String> {
    let mcp = project_mcp_registry(catalog);
    let mut text = String::new();
    text.push_str("# Canonical MCP tool names + descriptions (generated).\n");
    text.push_str("# GENERATED FROM contracts/operations/catalog.v1.yaml via `vox ci operations-sync --target mcp --write`.\n");
    text.push_str("# Do not hand-edit this file.\n");
    text.push_str(&serde_yaml::to_string(&mcp).context("serialize MCP registry")?);
    Ok(text)
}

fn cli_registry_yaml_text(repo_root: &Path, catalog: &OperationsCatalog) -> Result<String> {
    let reg = project_cli_registry(repo_root, catalog)?;
    let mut text = String::new();
    text.push_str("# Machine-readable Vox command surface (generated).\n");
    text.push_str("# GENERATED FROM contracts/operations/catalog.v1.yaml via `vox ci operations-sync --target cli --write`.\n");
    text.push_str("# Do not hand-edit vox-cli rows here; edit the operations catalog instead.\n");
    text.push_str("# Non-CLI surfaces (compilerd, dei-d daemon, …) + script_duals/env_var_ssot_index are carried from the last committed registry.\n");
    text.push_str(&serde_yaml::to_string(&reg).context("serialize command registry")?);
    Ok(text)
}

fn capability_registry_yaml_text(catalog: &OperationsCatalog) -> Result<String> {
    let doc = project_capability_registry_doc(catalog);
    let mut text = String::new();
    text.push_str("# Transport-independent capability SSOT (generated; see capability-registry.schema.json).\n");
    text.push_str("# GENERATED FROM contracts/operations/catalog.v1.yaml via `vox ci operations-sync --target capability --write`.\n");
    text.push_str("# Do not hand-edit this file.\n");
    text.push_str(&serde_yaml::to_string(&doc).context("serialize capability registry")?);
    Ok(text)
}

pub(crate) fn write_or_verify_text(
    path: &Path,
    text: &str,
    write: bool,
    stale_hint: &str,
) -> Result<()> {
    if write {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, normalize_lf(text)).with_context(|| format!("write {}", path.display()))?;
        println!("wrote {}", path.display());
        return Ok(());
    }
    if !path.is_file() {
        return Err(anyhow!("missing {}; {stale_hint}", path.display()));
    }
    let existing = read_utf8_path_capped(path)?;
    if normalize_lf(&existing) != normalize_lf(text) {
        return Err(anyhow!("{} is stale; {stale_hint}", path.display()));
    }
    println!("OK {}", path.display());
    Ok(())
}

pub fn sync(repo_root: &Path, target: &str, write: bool) -> Result<()> {
    match target {
        "catalog" => {
            let catalog = build_catalog_from_live_registries(repo_root)?;
            let text = serde_yaml::to_string(&catalog).context("serialize operations catalog")?;
            let path = repo_root.join(OPERATIONS_CATALOG_REL);
            write_or_verify_text(
                &path,
                &text,
                write,
                "run `vox ci operations-sync --target catalog --write`",
            )?;
            write_inventory_report(repo_root, &catalog)?;
            Ok(())
        }
        "mcp" => {
            let catalog = read_catalog(repo_root)?;
            ensure_mcp_classified(&catalog)?;
            let text = mcp_registry_yaml_text(&catalog)?;
            let path = repo_root.join(MCP_TOOL_REGISTRY_REL);
            write_or_verify_text(
                &path,
                &text,
                write,
                "run `vox ci operations-sync --target mcp --write`",
            )
        }
        "cli" => {
            let catalog = read_catalog(repo_root)?;
            let text = cli_registry_yaml_text(repo_root, &catalog)?;
            let path = repo_root.join(REGISTRY_REL);
            write_or_verify_text(
                &path,
                &text,
                write,
                "run `vox ci operations-sync --target cli --write`",
            )
        }
        "capability" => {
            let catalog = read_catalog(repo_root)?;
            let text = capability_registry_yaml_text(&catalog)?;
            let path = repo_root.join(CAPABILITY_REGISTRY_REL);
            write_or_verify_text(
                &path,
                &text,
                write,
                "run `vox ci operations-sync --target capability --write`",
            )
        }
        "all" => {
            for (t, label) in [
                ("mcp", "MCP tool registry"),
                ("cli", "CLI command registry"),
                ("capability", "capability registry"),
            ] {
                sync(repo_root, t, write)
                    .with_context(|| format!("operations-sync --target all ({label})"))?;
            }
            Ok(())
        }
        other => Err(anyhow!(
            "unknown operations-sync target `{other}` (expected: catalog|mcp|cli|capability|all)"
        )),
    }
}
fn verify_catalog_nomenclature(catalog: &OperationsCatalog) -> Result<()> {
    let mut alias_to_id = BTreeMap::new();
    for row in &catalog.operations {
        let is_core_canonical = [
            "orchestrator",
            "skills",
            "forge",
            "database",
            "secrets",
            "speech",
            "ml",
            "gamification",
            "tutorial",
            "pm",
            "package_manager",
        ]
        .contains(&row.canonical_name.as_deref().unwrap_or(""));

        let has_alias = row
            .latin_aliases
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if is_core_canonical && !has_alias {
            return Err(anyhow!(
                "command-compliance T045: English canonical command '{}' must declare at least one Latin alias in `latin_aliases`",
                row.id
            ));
        }

        if let Some(aliases) = &row.latin_aliases {
            for alias in aliases {
                // Check grammar T053: invalid alias grammar tags
                if !alias.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                    return Err(anyhow!(
                        "command-compliance T053: Latin alias '{}' has invalid grammar tag (must be lower-kebab-case)",
                        alias
                    ));
                }

                if Some(alias.as_str()) == row.canonical_name.as_deref() {
                    return Err(anyhow!(
                        "command-compliance T046: Latin alias '{}' cannot be the canonical structural identifier for '{}'",
                        alias,
                        row.id
                    ));
                }

                if let Some(existing_id) = alias_to_id.insert(alias.clone(), row.id.clone()) {
                    if existing_id != row.id {
                        return Err(anyhow!(
                            "command-compliance T047: Latin alias collision for '{}' between '{}' and '{}'",
                            alias,
                            existing_id,
                            row.id
                        ));
                    }
                }
            }
        }

        if let Some(c) = &row.cli {
            if c.status == "retired" && has_alias {
                return Err(anyhow!(
                    "command-compliance T050: retired command '{}' cannot have Latin aliases",
                    row.id
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_row(
        id: &str,
        canon: Option<&str>,
        aliases: Option<Vec<&str>>,
        status: &str,
    ) -> OperationRow {
        OperationRow {
            id: id.to_string(),
            title: "".to_string(),
            description: "".to_string(),
            description_human: None,
            owner_crate: None,
            domain: None,
            product_lane: "platform".to_string(),
            intent_tags: vec![],
            side_effect_class: None,
            scope_kind: None,
            reversible: None,
            requires_repo: None,
            preferred_for_models: None,
            human_takeover_friendly: None,
            mens_planner_visible: None,
            canonical_name: canon.map(|s| s.to_string()),
            latin_aliases: aliases.map(|a| a.into_iter().map(|s| s.to_string()).collect()),
            mcp: None,
            cli: Some(CliProjection {
                path: vec![id.to_string()],
                status: status.to_string(),
                latin_ns: None,
                handler_rust: None,
                feature_gate: None,
                catalog_group: None,
                ref_cli_required: None,
                reachability_required: None,
            }),
        }
    }

    #[test]
    fn test_t045_missing_alias() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![create_row(
                "orchestrator",
                Some("orchestrator"),
                None,
                "active",
            )],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T045"));
    }

    #[test]
    fn test_t046_alias_cannot_be_canonical() {
        // alias "dei" == canonical_name "dei" → must fire T046
        // (use a valid kebab-case alias so T053 grammar check passes first)
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![create_row(
                "test-op",
                Some("dei"),
                Some(vec!["dei"]),
                "active",
            )],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T046"));
    }

    #[test]
    fn test_t047_alias_collision() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![
                create_row("op1", Some("op1"), Some(vec!["shared-alias"]), "active"),
                create_row("op2", Some("op2"), Some(vec!["shared-alias"]), "active"),
            ],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T047"));
    }

    #[test]
    fn regeneration_preserves_curated_nomenclature() {
        let prev = create_row("ludus", Some("gamify"), Some(vec!["ludus"]), "active");
        let mut fresh = create_row("ludus", None, None, "active");
        apply_prev_planner_metadata(&mut fresh, Some(&prev));
        assert_eq!(fresh.canonical_name.as_deref(), Some("gamify"));
        assert_eq!(fresh.latin_aliases, Some(vec!["ludus".to_string()]));
    }

    #[test]
    fn test_t050_retired_command_aliases() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![create_row(
                "op1",
                Some("op1"),
                Some(vec!["alias"]),
                "retired",
            )],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T050"));
    }

    #[test]
    fn test_t053_invalid_grammar() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![create_row(
                "op1",
                Some("op1"),
                Some(vec!["UPPERCASE"]),
                "active",
            )],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T053"));
    }

    #[test]
    fn test_t054_positive_parity_metadata() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![create_row(
                "dei",
                Some("orchestrator"),
                Some(vec!["dei"]),
                "active",
            )],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_ok());
    }

    #[test]
    fn mcp_dispatch_and_input_schema_paths_exist() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(
            repo_root
                .join("crates/vox-orchestrator-mcp/src/dispatch.rs")
                .is_file(),
            "expected vox-orchestrator-mcp dispatch.rs (operations-verify anchor)"
        );
        assert!(
            repo_root
                .join("crates/vox-orchestrator-mcp/src/input_schemas.rs")
                .is_file(),
            "expected vox-orchestrator-mcp input_schemas.rs (operations-verify anchor)"
        );
    }

    #[test]
    fn mcp_projection_emits_exposure_metadata_and_no_tier() {
        let mut row = create_row("x.ok", None, None, "active");
        row.cli = None;
        row.side_effect_class = Some("read_only".to_string());
        row.mcp = Some(McpProjection {
            name: "vox_x_ok".to_string(),
            http_read_role_eligible: false,
            exposure: Some("public".to_string()),
            requires: Some(vec!["db".to_string(), "search_index".to_string()]),
        });
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![row],
        };
        let text = mcp_registry_yaml_text(&cat).expect("serialize");
        assert!(text.contains("  exposure: public\n"), "{text}");
        assert!(text.contains("  side_effect_class: read_only\n"), "{text}");
        assert!(
            text.contains("  requires:\n  - db\n  - search_index\n"),
            "{text}"
        );
        assert!(!text.contains("tier"), "{text}");
    }

    const UNCLASSIFIED_CATALOG: &str = "schema_version: 1
operations:
- id: x.unclassified
  title: Unclassified
  description: d
  product_lane: ai
  mcp:
    name: vox_x_unclassified
";

    /// Schema-valid but gate-failing: a read-role tool whose side effect is not `read_only`.
    const LEAKY_CATALOG: &str = "schema_version: 1
operations:
- id: x.leaky
  title: Leaky
  description: d
  product_lane: ai
  side_effect_class: writes_files
  mcp:
    name: vox_x_leaky
    http_read_role_eligible: true
    exposure: public
    requires: []
";

    const LEAKY_MCP_REGISTRY: &str = "version: 1
tools:
- name: vox_x_leaky
  description: d
  product_lane: ai
  http_read_role_eligible: true
";

    fn real_repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn write_rel(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(p, text).expect("write fixture");
    }

    #[test]
    fn sync_mcp_target_fails_on_unclassified_catalog() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_rel(dir.path(), OPERATIONS_CATALOG_REL, UNCLASSIFIED_CATALOG);
        let err = sync(dir.path(), "mcp", false).unwrap_err().to_string();
        assert!(err.contains("MCP exposure-metadata problem(s)"), "{err}");
        assert!(err.contains("missing `exposure`"), "{err}");
    }

    #[test]
    fn verify_runs_classification_gate_after_structural_checks() {
        // `verify` validates the JSON Schema first, which already rejects a row with no
        // `exposure`/`requires`/`side_effect_class`; the gate is therefore only reachable with a
        // schema-valid row that breaks a semantic invariant (read-role tool that is not read_only).
        let dir = tempfile::tempdir().expect("tempdir");
        let schema = fs::read_to_string(real_repo_root().join(OPERATIONS_CATALOG_SCHEMA_REL))
            .expect("read real schema");
        write_rel(dir.path(), OPERATIONS_CATALOG_SCHEMA_REL, &schema);
        write_rel(dir.path(), OPERATIONS_CATALOG_REL, LEAKY_CATALOG);
        write_rel(dir.path(), MCP_TOOL_REGISTRY_REL, LEAKY_MCP_REGISTRY);
        write_rel(
            dir.path(),
            REGISTRY_REL,
            "schema_version: 1\noperations: []\n",
        );
        let err = verify(dir.path()).unwrap_err().to_string();
        assert!(err.contains("MCP exposure-metadata problem(s)"), "{err}");
        assert!(err.contains("http_read_role_eligible"), "{err}");
    }

    #[test]
    fn verify_rejects_unclassified_row_at_schema_validation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let schema = fs::read_to_string(real_repo_root().join(OPERATIONS_CATALOG_SCHEMA_REL))
            .expect("read real schema");
        write_rel(dir.path(), OPERATIONS_CATALOG_SCHEMA_REL, &schema);
        write_rel(dir.path(), OPERATIONS_CATALOG_REL, UNCLASSIFIED_CATALOG);
        let err = format!("{:#}", verify(dir.path()).unwrap_err());
        assert!(!err.contains("MCP exposure-metadata problem(s)"), "{err}");
        assert!(
            err.contains("catalog.v1.schema.json") && err.contains("/operations/0/mcp"),
            "{err}"
        );
    }

    #[test]
    fn catalog_schema_enums_match_vox_mcp_registry_vocabulary() {
        use std::collections::BTreeSet;
        use vox_mcp_registry::{Exposure, SideEffectClass, Subsystem};

        // Compile-time guards: adding a variant forces this list (and the schema) to be updated.
        let exposures = [Exposure::Public, Exposure::Operator, Exposure::Internal];
        for e in exposures {
            match e {
                Exposure::Public | Exposure::Operator | Exposure::Internal => {}
            }
        }
        let effects = [
            SideEffectClass::None,
            SideEffectClass::ReadOnly,
            SideEffectClass::WritesFiles,
            SideEffectClass::SpawnsProcess,
            SideEffectClass::Network,
            SideEffectClass::RemoteMutation,
            SideEffectClass::Destructive,
        ];
        for e in effects {
            match e {
                SideEffectClass::None
                | SideEffectClass::ReadOnly
                | SideEffectClass::WritesFiles
                | SideEffectClass::SpawnsProcess
                | SideEffectClass::Network
                | SideEffectClass::RemoteMutation
                | SideEffectClass::Destructive => {}
            }
        }

        let raw = fs::read_to_string(real_repo_root().join(OPERATIONS_CATALOG_SCHEMA_REL))
            .expect("read schema");
        let schema: serde_json::Value = serde_json::from_str(&raw).expect("schema json");
        let enum_set = |v: &serde_json::Value| -> (usize, BTreeSet<String>) {
            let arr = v.as_array().expect("enum array");
            let set: BTreeSet<String> = arr
                .iter()
                .map(|x| x.as_str().expect("enum string").to_string())
                .collect();
            (arr.len(), set)
        };
        let mcp_props = &schema["definitions"]["mcpProjection"]["properties"];
        let cases: [(&str, (usize, BTreeSet<String>), Vec<&str>); 3] = [
            (
                "exposure",
                enum_set(&mcp_props["exposure"]["enum"]),
                exposures.iter().map(|e| e.as_str()).collect(),
            ),
            (
                "side_effect_class",
                enum_set(
                    &schema["definitions"]["operationRow"]["then"]["properties"]["side_effect_class"]
                        ["enum"],
                ),
                effects.iter().map(|e| e.as_str()).collect(),
            ),
            (
                "requires",
                enum_set(&mcp_props["requires"]["items"]["enum"]),
                Subsystem::ALL.iter().map(|s| s.as_str()).collect(),
            ),
        ];
        for (name, (len, got), want) in cases {
            let want_set: BTreeSet<String> = want.iter().map(|s| s.to_string()).collect();
            assert_eq!(len, want.len(), "{name}: schema enum length");
            assert_eq!(got, want_set, "{name}: schema enum vs vox_mcp_registry");
        }
    }
}
