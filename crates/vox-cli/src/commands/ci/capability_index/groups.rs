//! Group rollup: merges index rows into [`CapabilityGroup`] table for the GUI.
//!
//! Emitted as `CAPABILITY_GROUPS` in `capabilityIndex.generated.ts` (D-28).

use anyhow::{Result, anyhow};
use std::collections::{BTreeMap, BTreeSet};
use vox_capability_registry::{CapabilityIndex, RowKind};

use super::super::gui_coverage_exemptions::Exemption;
use super::super::gui_surface_registry::{RepresentationTier, SurfaceRegistry};
use super::inputs::Domains;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupStatus {
    Dedicated,
    GenericForm,
    CliOnly,
}

impl GroupStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Dedicated => "dedicated",
            Self::GenericForm => "generic_form",
            Self::CliOnly => "cli_only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapabilityGroup {
    pub(crate) name: String,
    pub(crate) status: GroupStatus,
    pub(crate) view_key: Option<String>,
    pub(crate) domain: String,
    pub(crate) crate_name: Option<String>,
    pub(crate) layer: Option<u32>,
    pub(crate) verbs: usize,
    pub(crate) compiled: usize,
    pub(crate) summary: Option<String>,
    pub(crate) docs: Option<String>,
    pub(crate) row_id: Option<String>,
    pub(crate) exemption: Option<GroupExemption>,
}

/// Coverage exemption attached to a group for the Explorer (D-28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupExemption {
    pub(crate) owner: String,
    pub(crate) reason: String,
    pub(crate) review_by: String,
}

pub(crate) fn build_groups(
    index: &CapabilityIndex,
    effective: &SurfaceRegistry,
    top_level: &BTreeSet<String>,
    domains: &Domains,
) -> Result<Vec<CapabilityGroup>> {
    // 1. Verify every index cli row has a top-level group prefix
    for r in &index.rows {
        if r.kind == RowKind::Cli {
            if let Some(cli) = &r.cli {
                if let Some(first) = cli.path.first() {
                    if !top_level.contains(first) {
                        return Err(anyhow!(
                            "index cli row `{}` belongs to unknown group `{first}`",
                            r.id
                        ));
                    }
                }
            }
        }
    }

    // 2. Map registry entries by cli_group for quick lookup
    let mut reg_by_group = BTreeMap::new();
    for s in &effective.surfaces {
        if let Some(g) = &s.cli_group {
            reg_by_group.insert(g.as_str(), s);
        }
    }

    // 3. Build group entries for each top-level group
    let mut result = Vec::with_capacity(top_level.len());
    for g in top_level {
        let (status, view_key) = match reg_by_group.get(g.as_str()) {
            Some(s) => match s.representation_tier {
                RepresentationTier::LiveBackend | RepresentationTier::CuratedDecorator => {
                    (GroupStatus::Dedicated, s.view_key.clone())
                }
                RepresentationTier::GenericForm => (GroupStatus::GenericForm, None),
                RepresentationTier::None => (GroupStatus::CliOnly, None),
            },
            None => (GroupStatus::CliOnly, None),
        };

        // Gather index cli rows belonging to group g
        let mut group_rows = Vec::new();
        for r in &index.rows {
            if r.kind == RowKind::Cli {
                if let Some(cli) = &r.cli {
                    if cli.path.first().map(|s| s.as_str()) == Some(g.as_str()) {
                        group_rows.push(r);
                    }
                }
            }
        }

        let mut verbs = 0;
        let mut compiled = 0;
        let mut bare_summary = None;
        let mut bare_docs = None;
        let mut bare_row_id = None;
        let mut group_domain = None;
        let mut group_crate = None;
        let mut group_layer = None;

        for r in group_rows {
            if let Some(d) = &r.domain {
                group_domain = Some(d.clone());
            }
            if let Some(o) = &r.owner {
                group_crate = Some(o.crate_name.clone());
                group_layer = o.layer.map(|l| l as u32);
            }

            if let Some(cli) = &r.cli {
                if cli.path.len() == 1 {
                    bare_summary = r.summary.clone();
                    bare_docs = r.docs.first().map(|d| d.path.clone());
                    bare_row_id = Some(r.id.clone());
                } else if cli.path.len() >= 2 {
                    verbs += 1;
                    if r.gui.as_ref().is_some_and(|gui| gui.catalog_form) {
                        compiled += 1;
                    }
                }
            }
        }

        // Domain & crate fallback to domains.cli_groups if no index rows had them
        let domain = match group_domain {
            Some(d) if !d.is_empty() => d,
            _ => match domains.cli_groups.get(g) {
                Some(entry) => {
                    if group_crate.is_none() {
                        group_crate = entry.owner.clone();
                    }
                    entry.domain.clone()
                }
                None => return Err(anyhow!("group `{g}` has no domain in index or cli_groups")),
            },
        };

        // row_id fallback for dedicated surface with no bare row
        let row_id = match bare_row_id {
            Some(id) => Some(id),
            None => {
                if status == GroupStatus::Dedicated {
                    view_key.as_ref().map(|vk| format!("surface.{vk}"))
                } else {
                    None
                }
            }
        };

        result.push(CapabilityGroup {
            name: g.clone(),
            status,
            view_key,
            domain,
            crate_name: group_crate,
            layer: group_layer,
            verbs,
            compiled,
            summary: bare_summary,
            docs: bare_docs,
            row_id,
            exemption: None,
        });
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Attaches ledger exemptions to their groups (exact name match). Pure; never adds exemptions.
pub(crate) fn attach_exemptions(groups: &mut [CapabilityGroup], exemptions: &[Exemption]) {
    for g in groups {
        g.exemption = exemptions
            .iter()
            .find(|e| e.group == g.name)
            .map(|e| GroupExemption {
                owner: e.owner.clone(),
                reason: e.reason.clone(),
                review_by: e.review_by.clone(),
            });
    }
}

pub(crate) fn ts_groups(groups: &[CapabilityGroup]) -> String {
    let mut out = String::from(
        "export interface CapabilityGroup {\n  \
         name: string;\n  \
         status: 'dedicated' | 'generic_form' | 'cli_only';\n  \
         viewKey: string | null;\n  \
         domain: string;\n  \
         crate: string | null;\n  \
         layer: number | null;\n  \
         verbs: number;\n  \
         compiled: number;\n  \
         summary: string | null;\n  \
         docs: string | null;\n  \
         rowId: string | null;\n  \
         exemption: { owner: string; reason: string; reviewBy: string } | null;\n\
         }\n\
         export const CAPABILITY_GROUPS: CapabilityGroup[] = [\n",
    );
    for g in groups {
        let exemption = g.exemption.as_ref().map_or_else(
            || "null".to_string(),
            |e| {
                format!(
                    "{{ owner: {}, reason: {}, reviewBy: {} }}",
                    super::emit::ts_str(&e.owner),
                    super::emit::ts_str(&e.reason),
                    super::emit::ts_str(&e.review_by),
                )
            },
        );
        out.push_str(&format!(
            "  {{ name: {}, status: '{}', viewKey: {}, domain: {}, crate: {}, layer: {}, verbs: {}, compiled: {}, summary: {}, docs: {}, rowId: {}, exemption: {} }},\n",
            super::emit::ts_str(&g.name),
            g.status.as_str(),
            super::emit::ts_opt(g.view_key.as_deref()),
            super::emit::ts_str(&g.domain),
            super::emit::ts_opt(g.crate_name.as_deref()),
            g.layer.map_or_else(|| "null".to_string(), |l| l.to_string()),
            g.verbs,
            g.compiled,
            super::emit::ts_opt(g.summary.as_deref()),
            super::emit::ts_opt(g.docs.as_deref()),
            super::emit::ts_opt(g.row_id.as_deref()),
            exemption,
        ));
    }
    out.push_str("];\n");
    out
}

#[cfg(test)]
mod tests {
    use super::super::super::gui_surface_registry::SurfaceEntry;
    use super::super::inputs::GroupEntry;
    use super::*;
    use std::path::PathBuf;
    use vox_capability_registry::{CliRef, GuiRef, IndexRow};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn make_row(id: &str, path: &[&str], catalog_form: bool) -> IndexRow {
        IndexRow {
            id: id.to_string(),
            kind: RowKind::Cli,
            title: id.to_string(),
            summary: Some(format!("Summary for {id}")),
            op_id: None,
            status: Some("active".to_string()),
            domain: Some("compiler".to_string()),
            product_lane: None,
            side_effect_class: None,
            tier: None,
            cli: Some(CliRef {
                path: path.iter().map(|s| s.to_string()).collect(),
                invocation: format!("vox {}", path.join(" ")),
                feature_gate: None,
            }),
            mcp: None,
            gui: Some(GuiRef {
                view_key: None,
                nav_label: None,
                representation_tier: None,
                catalog_form,
            }),
            owner: None,
            source_path: None,
            docs: vec![],
            skills: vec![],
            equivalents: vec![],
            refs: vec![],
            coverage: None,
        }
    }

    #[test]
    fn group_status_maps_effective_tier() {
        let mut top_level = BTreeSet::new();
        top_level.insert("chat".to_string());
        top_level.insert("build".to_string());
        top_level.insert("secret".to_string());

        let reg = SurfaceRegistry {
            x_vox_version: 2,
            schema_version: 1,
            surfaces: vec![
                SurfaceEntry {
                    view_key: Some("chat".to_string()),
                    cli_group: Some("chat".to_string()),
                    mcp_namespace: None,
                    representation_tier: RepresentationTier::LiveBackend,
                    nav_label: None,
                    nav_icon: None,
                    nav_group: None,
                    parent_surface: None,
                    notes: None,
                },
                SurfaceEntry {
                    view_key: None,
                    cli_group: Some("build".to_string()),
                    mcp_namespace: None,
                    representation_tier: RepresentationTier::GenericForm,
                    nav_label: None,
                    nav_icon: None,
                    nav_group: None,
                    parent_surface: None,
                    notes: None,
                },
                SurfaceEntry {
                    view_key: None,
                    cli_group: Some("secret".to_string()),
                    mcp_namespace: None,
                    representation_tier: RepresentationTier::None,
                    nav_label: None,
                    nav_icon: None,
                    nav_group: None,
                    parent_surface: None,
                    notes: None,
                },
            ],
        };

        let mut dom = Domains::default();
        dom.cli_groups.insert(
            "chat".to_string(),
            GroupEntry {
                domain: "orchestrator".to_string(),
                owner: None,
            },
        );
        dom.cli_groups.insert(
            "build".to_string(),
            GroupEntry {
                domain: "compiler".to_string(),
                owner: None,
            },
        );
        dom.cli_groups.insert(
            "secret".to_string(),
            GroupEntry {
                domain: "secrets".to_string(),
                owner: None,
            },
        );

        let idx = CapabilityIndex {
            schema_version: 1,
            generator: "test".to_string(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![],
        };

        let groups = build_groups(&idx, &reg, &top_level, &dom).unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "build");
        assert_eq!(groups[0].status, GroupStatus::GenericForm);
        assert_eq!(groups[1].name, "chat");
        assert_eq!(groups[1].status, GroupStatus::Dedicated);
        assert_eq!(groups[1].view_key.as_deref(), Some("chat"));
        assert_eq!(groups[2].name, "secret");
        assert_eq!(groups[2].status, GroupStatus::CliOnly);
    }

    #[test]
    fn bare_only_group_has_zero_verbs() {
        let mut top_level = BTreeSet::new();
        top_level.insert("run".to_string());
        let reg = SurfaceRegistry::default();
        let mut dom = Domains::default();
        dom.cli_groups.insert(
            "run".to_string(),
            GroupEntry {
                domain: "runtime".to_string(),
                owner: None,
            },
        );

        let idx = CapabilityIndex {
            schema_version: 1,
            generator: "test".to_string(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![make_row("cli.run", &["run"], true)],
        };

        let groups = build_groups(&idx, &reg, &top_level, &dom).unwrap();
        assert_eq!(groups[0].verbs, 0);
        assert_eq!(groups[0].compiled, 0);
        assert_eq!(groups[0].row_id.as_deref(), Some("cli.run"));
    }

    #[test]
    fn verbs_and_compiled_counts() {
        let mut top_level = BTreeSet::new();
        top_level.insert("g".to_string());
        let reg = SurfaceRegistry::default();
        let mut dom = Domains::default();
        dom.cli_groups.insert(
            "g".to_string(),
            GroupEntry {
                domain: "compiler".to_string(),
                owner: None,
            },
        );

        let idx = CapabilityIndex {
            schema_version: 1,
            generator: "test".to_string(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![
                make_row("cli.g", &["g"], true),
                make_row("cli.g.a", &["g", "a"], true),
                make_row("cli.g.b", &["g", "b"], false),
            ],
        };

        let groups = build_groups(&idx, &reg, &top_level, &dom).unwrap();
        assert_eq!(groups[0].verbs, 2);
        assert_eq!(groups[0].compiled, 1);
    }

    #[test]
    fn rowless_group_takes_domain_from_cli_groups() {
        let mut top_level = BTreeSet::new();
        top_level.insert("attention".to_string());
        let reg = SurfaceRegistry::default();
        let mut dom = Domains::default();
        dom.cli_groups.insert(
            "attention".to_string(),
            GroupEntry {
                domain: "orchestrator".to_string(),
                owner: Some("vox-cli".to_string()),
            },
        );

        let idx = CapabilityIndex {
            schema_version: 1,
            generator: "test".to_string(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![],
        };

        let groups = build_groups(&idx, &reg, &top_level, &dom).unwrap();
        assert_eq!(groups[0].domain, "orchestrator");
        assert_eq!(groups[0].crate_name.as_deref(), Some("vox-cli"));
    }

    #[test]
    fn group_without_domain_errors() {
        let mut top_level = BTreeSet::new();
        top_level.insert("mystery".to_string());
        let reg = SurfaceRegistry::default();
        let dom = Domains::default();
        let idx = CapabilityIndex {
            schema_version: 1,
            generator: "test".to_string(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![],
        };

        let err = build_groups(&idx, &reg, &top_level, &dom).unwrap_err();
        assert!(err.to_string().contains("mystery"));
    }

    #[test]
    fn index_group_outside_top_level_errors() {
        let top_level = BTreeSet::new();
        let reg = SurfaceRegistry::default();
        let dom = Domains::default();
        let idx = CapabilityIndex {
            schema_version: 1,
            generator: "test".to_string(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![make_row("cli.orphan.sub", &["orphan", "sub"], true)],
        };

        let err = build_groups(&idx, &reg, &top_level, &dom).unwrap_err();
        assert!(err.to_string().contains("orphan"));
    }

    #[test]
    fn ts_groups_is_escaped_and_sorted() {
        let groups = vec![
            CapabilityGroup {
                name: "b\"name".to_string(),
                status: GroupStatus::GenericForm,
                view_key: None,
                domain: "compiler".to_string(),
                crate_name: Some("vox-cli".to_string()),
                layer: Some(1),
                verbs: 2,
                compiled: 1,
                summary: Some("Summary with \"quotes\" and \\backslash".to_string()),
                docs: None,
                row_id: Some("cli.b".to_string()),
                exemption: None,
            },
            CapabilityGroup {
                name: "a".to_string(),
                status: GroupStatus::Dedicated,
                view_key: Some("view-a".to_string()),
                domain: "runtime".to_string(),
                crate_name: None,
                layer: None,
                verbs: 0,
                compiled: 0,
                summary: None,
                docs: None,
                row_id: None,
                exemption: None,
            },
        ];
        let ts = ts_groups(&groups);
        assert!(ts.contains("export interface CapabilityGroup"));
        assert!(ts.contains("export const CAPABILITY_GROUPS: CapabilityGroup[] = ["));
        assert!(ts.contains("\\\"quotes\\\""));
        assert!(ts.contains("exemption: null"));
    }

    #[test]
    fn exemption_attaches_and_emits_escaped() {
        let mk = |name: &str| CapabilityGroup {
            name: name.to_string(),
            status: GroupStatus::CliOnly,
            view_key: None,
            domain: "runtime".to_string(),
            crate_name: None,
            layer: None,
            verbs: 0,
            compiled: 0,
            summary: None,
            docs: None,
            row_id: None,
            exemption: None,
        };
        let mut groups = vec![mk("x"), mk("y")];
        let ex = vec![Exemption {
            group: "x".to_string(),
            owner: "vox-cli".to_string(),
            reason: "has \"quote\" and \\slash".to_string(),
            review_by: "2027-01-31".to_string(),
        }];
        attach_exemptions(&mut groups, &ex);
        assert!(groups[0].exemption.is_some());
        assert!(groups[1].exemption.is_none());
        let ts = ts_groups(&groups);
        assert!(ts.contains("reason: \"has \\\"quote\\\" and \\\\slash\""));
        assert!(ts.contains("reviewBy: \"2027-01-31\""));
        assert!(ts.contains("exemption: null"));
    }

    #[test]
    fn real_inputs_yield_one_group_per_top_level_group() {
        let root = repo_root();
        let top_level = crate::commands::ci::gui_surface_registry::top_level_groups(&root).unwrap();
        let loaded = super::super::inputs::load(&root).unwrap();
        let index = super::super::join::build_index(&loaded).unwrap();

        let groups = build_groups(&index, &loaded.surfaces, &top_level, &loaded.domains).unwrap();
        assert_eq!(groups.len(), top_level.len());
        for g in &groups {
            assert!(!g.domain.is_empty(), "group {} has empty domain", g.name);
        }
    }
}
