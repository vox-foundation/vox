//! Committed inputs of the capability index.
//!
//! Reads only checked-in files (contracts, the plugin catalog toml), never the catalog compiled into
//! the binary or `command_catalog::build_catalog()`: the clap tree varies
//! with cargo features, the committed catalog does not.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use vox_bounded_fs::read_utf8_path_capped;
use vox_capability_registry::DomainDef;

use super::super::gui_surface_registry::{SurfaceRegistry, load_registry};
pub(crate) use super::super::operations_catalog::OperationRow;
use super::super::operations_catalog::read_catalog;
use super::coverage::{CoverageSnapshot, SNAPSHOT_REL};
use super::docs_claims::{PageMeta, parse_meta};
use super::nodes::PINNED_DOCS;
use super::owner::{CLI_COMMANDS_DIR, Layers};

const LAYERS_REL: &str = "docs/src/architecture/layers.toml";
const DOMAINS_REL: &str = "contracts/capabilities/domains.v1.yaml";
const CATALOG_REL: &str = "crates/vox-plugin-catalog/catalog.toml";

/// Everything `join::build_index` needs; pure data so the join is testable without IO.
#[derive(Default)]
pub(crate) struct Inputs {
    pub ops: Vec<OperationRow>,
    /// `layers.toml [crates]`: crate name -> layer.
    pub layers: Layers,
    /// `/`-normalised repo-relative `.rs` files under `crates/vox-cli/src/commands/`.
    pub cli_command_files: BTreeSet<String>,
    /// `contracts/capabilities/domains.v1.yaml`.
    pub domains: Domains,
    /// `contracts/gui/surface-registry.v1.yaml` (schema-validated by `gui_surface_registry`).
    pub surfaces: SurfaceRegistry,
    /// `crates/<name>/Cargo.toml` `package.description`, for layers keys whose manifest has one.
    pub crate_descriptions: BTreeMap<String, String>,
    /// Skill plugins (with the MCP tools they expose) and skill bundles from the plugin catalog.
    pub skills: Vec<SkillInput>,
    /// Committed graph-coverage snapshot (D-07); `None` when the file is absent.
    pub coverage: Option<CoverageSnapshot>,
    /// Opt-in docs pages (D-02, Pitfall 11): `/`-separated repo path -> frontmatter. Holds pages
    /// that claim capabilities plus the pinned default targets; never every page.
    pub doc_pages: BTreeMap<String, PageMeta>,
}

/// One skill plugin or skill bundle from the plugin catalog.
#[derive(Debug, Clone, Default)]
pub(crate) struct SkillInput {
    pub id: String,
    pub description: String,
    pub exposes_tools: Vec<String>,
    /// Repo-relative skill directory (bundles only).
    pub source_path: Option<String>,
}

/// `contracts/capabilities/domains.v1.yaml`: the domain vocabulary and its match tables (D-09).
/// Values are checked by `owner::validate_domains`, not here.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Domains {
    #[serde(default)]
    #[allow(dead_code)] // deserialised so deny_unknown_fields accepts the yaml key; never read
    pub schema_version: u32,
    #[serde(default)]
    pub domains: Vec<DomainDef>,
    #[serde(default)]
    pub cli_groups: BTreeMap<String, GroupEntry>,
    #[serde(default)]
    pub mcp_prefixes: BTreeMap<String, String>,
    #[serde(default)]
    pub crates: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GroupEntry {
    pub domain: String,
    #[serde(default)]
    pub owner: Option<String>,
}

pub(crate) fn load(repo_root: &Path) -> Result<Inputs> {
    let layers = load_layers(repo_root)?;
    let crate_descriptions = load_crate_descriptions(repo_root, &layers)?;
    let ops = read_catalog(repo_root)?.operations;
    let hand_surfaces = load_registry(repo_root)?;
    let surfaces = super::super::gui_coverage_gate::resolve_effective(&hand_surfaces, &ops);
    Ok(Inputs {
        ops,
        layers,
        cli_command_files: load_cli_command_files(repo_root)?,
        domains: load_domains(repo_root)?,
        surfaces,
        crate_descriptions,
        skills: load_skills(repo_root)?,
        coverage: load_coverage(repo_root)?,
        doc_pages: load_doc_pages(repo_root)?,
    })
}

const DOCS_DIR: &str = "docs/src";
const ARCHIVE_DIR: &str = "docs/src/archive";

/// Opt-in docs pages (Pitfall 11): pages claiming capabilities plus the pinned default targets.
/// `docs/src/archive/` is pruned before any read (AGENTS.md Archival Protocol). Frontmatter that
/// is malformed but claims nothing is skipped (Pitfall 10); an unreadable page or an unreadable
/// `capabilities:` claim is an error naming the page (see `docs_claims::parse_meta`).
fn load_doc_pages(repo_root: &Path) -> Result<BTreeMap<String, PageMeta>> {
    let rel_of = |p: &Path| {
        p.strip_prefix(repo_root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    };
    let archive = repo_root.join(ARCHIVE_DIR);
    let mut out = BTreeMap::new();
    let walk = walkdir::WalkDir::new(repo_root.join(DOCS_DIR))
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| !e.path().starts_with(&archive));
    for entry in walk {
        let entry = entry.with_context(|| format!("walk {DOCS_DIR}"))?;
        if !entry.file_type().is_file() || entry.path().extension().is_none_or(|e| e != "md") {
            continue;
        }
        let rel = rel_of(entry.path());
        // An unreadable page could hold a claim; fail loudly instead of dropping it.
        let text = read_utf8_path_capped(entry.path()).with_context(|| format!("read {rel}"))?;
        let meta = parse_meta(&rel, &text)?;
        let pinned = PINNED_DOCS.contains(&rel.as_str());
        match meta {
            Some(m) if pinned || !m.capabilities.is_empty() => {
                out.insert(rel, m);
            }
            None if pinned => {
                out.insert(rel, PageMeta::default());
            }
            _ => {}
        }
    }
    Ok(out)
}

/// The committed coverage snapshot; a missing file is `None` (rows then say why), never an error.
fn load_coverage(repo_root: &Path) -> Result<Option<CoverageSnapshot>> {
    let path = repo_root.join(SNAPSHOT_REL);
    if !path.is_file() {
        return Ok(None);
    }
    serde_json::from_str(&read_utf8_path_capped(&path)?)
        .map(Some)
        .with_context(|| format!("parse {SNAPSHOT_REL}"))
}

/// `package.description` of every layers crate whose manifest exists and carries a plain string.
fn load_crate_descriptions(repo_root: &Path, layers: &Layers) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for name in layers.keys() {
        let path = repo_root.join("crates").join(name).join("Cargo.toml");
        if !path.is_file() {
            continue;
        }
        let doc: toml::Value = toml::from_str(&read_utf8_path_capped(&path)?)
            .with_context(|| format!("parse {} as TOML", path.display()))?;
        if let Some(d) = doc
            .get("package")
            .and_then(|p| p.get("description"))
            .and_then(toml::Value::as_str)
        {
            out.insert(name.clone(), d.to_string());
        }
    }
    Ok(out)
}

/// The two catalog tables the index reads; other tables (`bundle`, `component`) are ignored.
/// Same table names as `vox_plugin_catalog`'s private `CatalogFile`.
#[derive(Deserialize)]
struct SkillCatalog {
    #[serde(default, rename = "plugin")]
    plugins: Vec<vox_plugin_catalog::schema::PluginCatalogEntry>,
    #[serde(default, rename = "skill-bundle")]
    skill_bundles: Vec<vox_plugin_catalog::schema::SkillBundleEntry>,
}

/// Skill plugins (payload kind skill) then skill bundles, read from the checked-in
/// `crates/vox-plugin-catalog/catalog.toml` (not the copy compiled into the running binary, which
/// is stale right after `plugin-catalog-sync --write` rewrote the file).
fn load_skills(repo_root: &Path) -> Result<Vec<SkillInput>> {
    let path = repo_root.join(CATALOG_REL);
    let cat: SkillCatalog = toml::from_str(&read_utf8_path_capped(&path)?)
        .with_context(|| format!("parse {CATALOG_REL}"))?;
    let plugins = cat
        .plugins
        .into_iter()
        .filter(|p| {
            matches!(
                p.payload_kind,
                vox_plugin_catalog::schema::PayloadKind::Skill
            )
        })
        .map(|p| SkillInput {
            id: p.id,
            description: p.description,
            exposes_tools: p.exposes_tools.unwrap_or_default(),
            source_path: None,
        });
    let bundles = cat.skill_bundles.into_iter().map(|b| SkillInput {
        id: b.id,
        description: b.description,
        exposes_tools: Vec::new(),
        source_path: Some(b.bundle_path),
    });
    Ok(plugins.chain(bundles).collect())
}

fn load_domains(repo_root: &Path) -> Result<Domains> {
    let path = repo_root.join(DOMAINS_REL);
    serde_yaml::from_str(&read_utf8_path_capped(&path)?)
        .with_context(|| format!("parse {DOMAINS_REL}"))
}

pub(crate) fn load_layers(repo_root: &Path) -> Result<Layers> {
    let path = repo_root.join(LAYERS_REL);
    let doc: toml::Value = toml::from_str(&read_utf8_path_capped(&path)?)
        .with_context(|| format!("parse {} as TOML", path.display()))?;
    let crates = doc
        .get("crates")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| anyhow!("{LAYERS_REL} has no [crates] table"))?;
    Ok(crates
        .iter()
        .map(|(name, v)| {
            let layer = v
                .get("layer")
                .and_then(toml::Value::as_integer)
                .and_then(|l| u8::try_from(l).ok());
            (name.clone(), layer)
        })
        .collect())
}

fn load_cli_command_files(repo_root: &Path) -> Result<BTreeSet<String>> {
    let dir = repo_root.join(CLI_COMMANDS_DIR);
    let mut files = BTreeSet::new();
    for entry in walkdir::WalkDir::new(&dir) {
        let entry = entry.with_context(|| format!("walk {}", dir.display()))?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "rs") {
            let rel = entry.path().strip_prefix(repo_root).unwrap_or(entry.path());
            files.insert(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(files)
}

/// Synthetic catalog row for unit tests. `cli` is `(path, status)`.
#[cfg(test)]
pub(crate) fn test_op(
    id: &str,
    cli: Option<(&[&str], &str)>,
    mcp: Option<&str>,
    description: &str,
) -> OperationRow {
    let cli = cli.map(|(path, status)| serde_json::json!({ "path": path, "status": status }));
    let mcp = mcp.map(|name| serde_json::json!({ "name": name }));
    serde_json::from_value(serde_json::json!({
        "id": id,
        "title": format!("Title {id}"),
        "description": description,
        "product_lane": "platform",
        "cli": cli,
        "mcp": mcp,
    }))
    .expect("synthetic OperationRow")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skills_come_from_the_checked_in_catalog_file() {
        const CATALOG: &str = "[[plugin]]\nid = \"skill-zz\"\npayload-kind = \"skill\"\n\
            description = \"d\"\nexposes-tools = [\"vox_a\"]\ndefault-source = \"local:x\"\n\n\
            [[plugin]]\nid = \"code-zz\"\npayload-kind = \"code\"\ndescription = \"c\"\n\
            default-source = \"local:y\"\n\n\
            [[skill-bundle]]\nid = \"b\"\ndescription = \"bd\"\nlicense = \"MIT\"\nsource = \"s\"\n\
            pin = \"p\"\nbundle-path = \"assets/skills/b\"\n";
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(CATALOG_REL);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, CATALOG).expect("write catalog");
        let skills = load_skills(dir.path()).expect("load");
        let ids: Vec<&str> = skills.iter().map(|s| s.id.as_str()).collect();
        // Only the file's rows: not the (different) catalog compiled into this binary.
        assert_eq!(ids, ["skill-zz", "b"]);
        assert_eq!(skills[0].exposes_tools, ["vox_a"]);
        assert_eq!(skills[1].source_path.as_deref(), Some("assets/skills/b"));
        let empty = tempfile::tempdir().expect("tempdir");
        assert!(
            load_skills(empty.path()).is_err(),
            "missing catalog must fail"
        );
    }

    #[test]
    fn test_op_builds_both_projections() {
        let op = test_op("a.b", Some((&["a", "b"], "active")), Some("vox_a_b"), "d");
        assert_eq!(op.cli.expect("cli").path, vec!["a", "b"]);
        assert_eq!(op.mcp.expect("mcp").name, "vox_a_b");
    }
}
