//! Surface, crate and skill rows (D-02) and the GUI-surface join onto operation rows.
//!
//! Pure: no IO, no clock. Cross-references are by id (`refs`, `skills`), never nested.

use anyhow::{Result, bail};
use vox_capability_registry::IndexRow;

use super::super::gui_surface_registry::SurfaceRegistry;
use super::inputs::Inputs;

/// Set `gui` on cli/mcp rows from the surface registry.
pub(crate) fn apply_surface_join(rows: &mut [IndexRow], surfaces: &SurfaceRegistry) {
    // RED stub: clears gui until the join lands.
    for r in rows.iter_mut() {
        r.gui = None;
    }
    let _ = surfaces.surfaces.len();
}

/// Surface, crate and skill rows; also writes `skills` back-references into mcp rows.
pub(crate) fn non_operation_rows(_rows: &mut [IndexRow], inputs: &Inputs) -> Result<Vec<IndexRow>> {
    if inputs.surfaces.surfaces.is_empty() && inputs.layers.is_empty() && inputs.skills.is_empty() {
        return Ok(Vec::new());
    }
    bail!("RED stub: non_operation_rows not implemented")
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
