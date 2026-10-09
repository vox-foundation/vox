//! Surface, crate and skill rows (D-02) and the GUI-surface join onto operation rows.
//!
//! Pure: no IO, no clock. Cross-references are by id (`refs`, `skills`), never nested.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use vox_capability_registry::{DocLink, GuiRef, IndexRow, RowKind, implicit_mcp_capability_id};

use super::super::gui_surface_registry::{RepresentationTier, SurfaceEntry, SurfaceRegistry};
use super::inputs::Inputs;
use super::join::{no_snapshot_coverage, single_line};
use super::owner::owner_ref;

/// Default page-level docs links (D-06: the repo has no per-row anchors yet).
const SURFACE_DOC: &str = "docs/src/reference/gui-navigation.md";
const CRATE_DOC: &str = "docs/src/architecture/where-things-live.md";
const SKILL_DOC: &str = "docs/src/reference/plugin-catalog.generated.md";
const GUI_CRATE: &str = "vox-gui";
const FALLBACK_DOMAIN: &str = "platform";

fn tier_str(t: RepresentationTier) -> &'static str {
    match t {
        RepresentationTier::None => "none",
        RepresentationTier::GenericForm => "generic_form",
        RepresentationTier::CuratedDecorator => "curated_decorator",
        RepresentationTier::LiveBackend => "live_backend",
    }
}

fn gui_ref(e: &SurfaceEntry, catalog_form: bool) -> GuiRef {
    GuiRef {
        view_key: e.view_key.clone(),
        nav_label: e.nav_label.clone(),
        representation_tier: Some(tier_str(e.representation_tier).to_string()),
        catalog_form,
    }
}

fn default_doc(path: &str) -> Vec<DocLink> {
    vec![DocLink {
        path: path.to_string(),
        origin: "default".to_string(),
    }]
}

/// Set `gui` on cli/mcp rows from the surface registry.
///
/// A cli row joins the entry whose `cli_group` is its first path segment; an mcp row joins by the
/// op-id namespace (text before the first `.`), matched against `view_key` then `cli_group`.
/// `catalog_form` is derived (RESEARCH A1): an active, ungated cli op is reachable by the generic
/// catalog form. mcp rows are never catalog-form.
pub(crate) fn apply_surface_join(rows: &mut [IndexRow], surfaces: &SurfaceRegistry) {
    let mut by_group: BTreeMap<&str, &SurfaceEntry> = BTreeMap::new();
    let mut by_view: BTreeMap<&str, &SurfaceEntry> = BTreeMap::new();
    for e in &surfaces.surfaces {
        if let Some(g) = e.cli_group.as_deref() {
            let slot = by_group.entry(g).or_insert(e);
            if slot.view_key.is_none() && e.view_key.is_some() {
                *slot = e;
            }
        }
        if let Some(v) = e.view_key.as_deref() {
            by_view.entry(v).or_insert(e);
        }
    }
    for r in rows.iter_mut() {
        match r.kind {
            RowKind::Cli => {
                let Some(c) = &r.cli else { continue };
                let Some(group) = c.path.first() else {
                    continue;
                };
                if let Some(e) = by_group.get(group.as_str()) {
                    let form = r.status.as_deref() == Some("active") && c.feature_gate.is_none();
                    r.gui = Some(gui_ref(e, form));
                }
            }
            RowKind::McpTool => {
                let Some(ns) = r.op_id.as_deref().and_then(|o| o.split('.').next()) else {
                    continue;
                };
                if let Some(e) = by_view.get(ns).or_else(|| by_group.get(ns)) {
                    r.gui = Some(gui_ref(e, false));
                }
            }
            _ => {}
        }
    }
}

/// A row of a non-operation kind: everything null/empty except what the caller sets.
fn bare_row(id: String, kind: RowKind, title: &str, doc: &str) -> IndexRow {
    IndexRow {
        id,
        kind,
        title: title.to_string(),
        summary: None,
        op_id: None,
        status: None,
        domain: None,
        product_lane: None,
        side_effect_class: None,
        tier: None,
        cli: None,
        mcp: None,
        gui: None,
        owner: None,
        source_path: None,
        docs: default_doc(doc),
        skills: Vec::new(),
        equivalents: Vec::new(),
        refs: Vec::new(),
        coverage: None,
    }
}

fn surface_row(e: &SurfaceEntry, view_key: &str, rows: &[IndexRow], inputs: &Inputs) -> IndexRow {
    let mut row = bare_row(
        format!("surface.{view_key}"),
        RowKind::Surface,
        e.nav_label.as_deref().unwrap_or(view_key),
        SURFACE_DOC,
    );
    row.summary = e.notes.as_deref().and_then(single_line);
    row.gui = Some(gui_ref(e, false));
    let d = &inputs.domains;
    row.domain = Some(
        e.cli_group
            .as_deref()
            .and_then(|g| d.cli_groups.get(g))
            .map(|g| g.domain.clone())
            .or_else(|| d.mcp_prefixes.get(view_key).cloned())
            .unwrap_or_else(|| "gui".to_string()),
    );
    row.owner = Some(owner_ref(&inputs.layers, GUI_CRATE, "surface_crate"));
    row.source_path = Some(format!("crates/{GUI_CRATE}/"));
    let mut refs: Vec<String> = rows
        .iter()
        .filter(|r| r.gui.as_ref().and_then(|g| g.view_key.as_deref()) == Some(view_key))
        .map(|r| r.id.clone())
        .collect();
    refs.sort();
    row.refs = refs;
    row.coverage = Some(no_snapshot_coverage());
    row
}

fn crate_row(name: &str, inputs: &Inputs) -> IndexRow {
    let mut row = bare_row(format!("crate.{name}"), RowKind::Crate, name, CRATE_DOC);
    row.summary = inputs
        .crate_descriptions
        .get(name)
        .and_then(|d| single_line(d));
    row.domain = Some(
        inputs
            .domains
            .crates
            .get(name)
            .cloned()
            .unwrap_or_else(|| FALLBACK_DOMAIN.to_string()),
    );
    row.owner = Some(owner_ref(&inputs.layers, name, "self"));
    row.source_path = Some(format!("crates/{name}/"));
    row
}

/// Surface, crate and skill rows (D-02); also writes `skills` back-references into the mcp rows
/// a skill exposes. Tools that are not index rows are not linked.
pub(crate) fn non_operation_rows(rows: &mut [IndexRow], inputs: &Inputs) -> Result<Vec<IndexRow>> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for e in &inputs.surfaces.surfaces {
        let Some(vk) = e.view_key.as_deref() else {
            continue;
        };
        if !seen.insert(vk) {
            bail!("duplicate surface view_key `{vk}` in contracts/gui/surface-registry.v1.yaml");
        }
        out.push(surface_row(e, vk, rows, inputs));
    }
    out.extend(inputs.layers.keys().map(|n| crate_row(n, inputs)));

    let mcp_ids: BTreeSet<String> = rows
        .iter()
        .filter(|r| r.kind == RowKind::McpTool)
        .map(|r| r.id.clone())
        .collect();
    let mut back: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for s in &inputs.skills {
        let id = format!("skill.{}", s.id);
        let mut row = bare_row(id.clone(), RowKind::Skill, &s.id, SKILL_DOC);
        row.summary = single_line(&s.description);
        row.domain = Some("skills".to_string());
        row.source_path = s.source_path.clone();
        let linked: BTreeSet<String> = s
            .exposes_tools
            .iter()
            .map(|t| implicit_mcp_capability_id(t))
            .filter(|m| mcp_ids.contains(m))
            .collect();
        for m in &linked {
            back.entry(m.clone()).or_default().insert(id.clone());
        }
        row.refs = linked.into_iter().collect();
        out.push(row);
    }
    for r in rows.iter_mut() {
        if let Some(skills) = back.remove(&r.id) {
            r.skills = skills.into_iter().collect();
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::inputs::{SkillInput, test_op};
    use super::super::join::build_index;
    use super::*;
    use crate::commands::ci::gui_surface_registry::{RepresentationTier, SurfaceEntry};
    use vox_capability_registry::CapabilityIndex;

    fn entry(
        view_key: Option<&str>,
        cli_group: Option<&str>,
        tier: RepresentationTier,
        label: Option<&str>,
    ) -> SurfaceEntry {
        SurfaceEntry {
            view_key: view_key.map(str::to_string),
            cli_group: cli_group.map(str::to_string),
            representation_tier: tier,
            nav_label: label.map(str::to_string),
            nav_icon: None,
            nav_group: None,
            parent_surface: None,
            notes: None,
        }
    }

    fn inputs_with(
        ops: Vec<super::super::inputs::OperationRow>,
        surfaces: Vec<SurfaceEntry>,
    ) -> Inputs {
        Inputs {
            ops,
            surfaces: SurfaceRegistry {
                x_vox_version: 1,
                schema_version: 1,
                surfaces,
            },
            ..Default::default()
        }
    }

    fn mesh_index() -> CapabilityIndex {
        build_index(&inputs_with(
            vec![
                test_op(
                    "mesh.nodes",
                    Some((&["mesh", "nodes"], "active")),
                    Some("vox_mesh_nodes"),
                    "d",
                ),
                test_op("zz.x", Some((&["zz", "x"], "active")), None, "d"),
            ],
            vec![entry(
                Some("mesh"),
                Some("mesh"),
                RepresentationTier::LiveBackend,
                Some("Mesh"),
            )],
        ))
        .expect("build")
    }

    #[test]
    fn cli_row_gets_gui_from_cli_group() {
        let idx = build_index(&inputs_with(
            vec![
                test_op(
                    "secrets.get",
                    Some((&["secrets", "get"], "active")),
                    None,
                    "d",
                ),
                test_op("zz.x", Some((&["zz", "x"], "active")), None, "d"),
            ],
            vec![entry(None, Some("secrets"), RepresentationTier::None, None)],
        ))
        .expect("build");
        let gui = idx
            .row("cli.secrets.get")
            .and_then(|r| r.gui.as_ref())
            .expect("gui on cli.secrets.get");
        assert_eq!(gui.representation_tier.as_deref(), Some("none"));
        assert!(gui.catalog_form, "active, ungated cli op is catalog-form");
        assert!(idx.row("cli.zz.x").expect("row").gui.is_none());
    }

    #[test]
    fn mcp_row_gets_gui_from_namespace() {
        let idx = mesh_index();
        let gui = idx
            .row("mcp.vox_mesh_nodes")
            .and_then(|r| r.gui.as_ref())
            .expect("gui on mcp row");
        assert_eq!(gui.view_key.as_deref(), Some("mesh"));
        assert!(!gui.catalog_form, "mcp rows are never catalog-form");
    }

    #[test]
    fn surface_row_refs_joined_rows() {
        let idx = mesh_index();
        let s = idx.row("surface.mesh").expect("surface.mesh row");
        assert_eq!(s.refs, vec!["cli.mesh.nodes", "mcp.vox_mesh_nodes"]);
        assert_eq!(s.title, "Mesh");
        assert_eq!(s.owner.as_ref().expect("owner").crate_name, "vox-gui");
        assert_eq!(s.docs[0].path, "docs/src/reference/gui-navigation.md");
    }

    #[test]
    fn crate_rows_from_layers() {
        let mut inp = inputs_with(vec![], vec![]);
        inp.layers = [("a-crate", 1u8), ("b-crate", 2)]
            .into_iter()
            .map(|(n, l)| (n.to_string(), Some(l)))
            .collect();
        inp.crate_descriptions = [("a-crate".to_string(), "desc a".to_string())].into();
        let idx = build_index(&inp).expect("build");
        let a = idx.row("crate.a-crate").expect("crate.a-crate");
        assert_eq!(a.summary.as_deref(), Some("desc a"));
        let owner = a.owner.as_ref().expect("owner");
        assert_eq!(
            (owner.crate_name.as_str(), owner.layer, owner.rule.as_str()),
            ("a-crate", Some(1), "self")
        );
        assert_eq!(a.docs[0].path, "docs/src/architecture/where-things-live.md");
        assert!(idx.row("crate.b-crate").expect("b").summary.is_none());
    }

    #[test]
    fn skill_rows_link_tools() {
        let mut inp = inputs_with(vec![test_op("x", None, Some("vox_x"), "d")], vec![]);
        inp.skills = vec![SkillInput {
            id: "skill-x".into(),
            description: "Does x".into(),
            exposes_tools: vec!["vox_x".into(), "vox_absent".into()],
            source_path: None,
        }];
        let idx = build_index(&inp).expect("build");
        let s = idx.row("skill.skill-x").expect("skill row");
        assert_eq!(s.refs, vec!["mcp.vox_x"], "absent tools are not linked");
        assert_eq!(
            idx.row("mcp.vox_x").expect("mcp").skills,
            vec!["skill.skill-x"]
        );
    }

    #[test]
    fn duplicate_view_key_is_error() {
        let e = entry(Some("dup"), None, RepresentationTier::None, None);
        let err = build_index(&inputs_with(vec![], vec![e.clone(), e]))
            .expect_err("duplicate view_key must fail");
        assert!(err.to_string().contains("dup"), "got: {err}");
    }
}
