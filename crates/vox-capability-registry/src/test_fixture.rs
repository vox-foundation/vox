//! Shared synthetic index for the explain / card tests (never the bundled file).

use crate::index::{
    CapabilityIndex, CliRef, CoverageRef, DocLink, DomainDef, GuiRef, IndexRow, McpRef, OwnerRef,
    RowKind,
};

/// A bare row with every optional field absent.
pub(crate) fn row(id: &str, kind: RowKind, title: &str) -> IndexRow {
    IndexRow {
        id: id.into(),
        kind,
        title: title.into(),
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
        docs: vec![],
        skills: vec![],
        equivalents: vec![],
        refs: vec![],
        coverage: None,
    }
}

pub(crate) fn owner(krate: &str, layer: u8) -> Option<OwnerRef> {
    Some(OwnerRef {
        crate_name: krate.into(),
        layer: Some(layer),
        rule: "test".into(),
    })
}

pub(crate) fn cli_ref(path: &[&str]) -> Option<CliRef> {
    Some(CliRef {
        path: path.iter().map(|s| (*s).to_string()).collect(),
        invocation: format!("vox {}", path.join(" ")),
        feature_gate: None,
    })
}

fn no_snapshot_coverage() -> Option<CoverageRef> {
    Some(CoverageRef {
        kind: None,
        basis: "none".into(),
        ambiguous: false,
        reason: Some("no graph-coverage snapshot committed".into()),
    })
}

pub(crate) fn fixture_index() -> CapabilityIndex {
    let mut secrets = row("cli.secrets", RowKind::Cli, "Secrets");
    secrets.domain = Some("secrets".into());
    secrets.cli = cli_ref(&["secrets"]);
    secrets.owner = owner("vox-secrets", 2);

    let mut get = row("cli.secrets.get", RowKind::Cli, "Secrets Get");
    get.summary = Some("CLI operation `vox secrets get`".into());
    get.domain = Some("secrets".into());
    get.product_lane = Some("platform".into());
    get.tier = Some("advanced".into());
    get.cli = cli_ref(&["secrets", "get"]);
    get.owner = owner("vox-secrets", 2);
    get.source_path = Some("crates/vox-secrets/".into());
    get.docs = vec![DocLink {
        path: "docs/src/reference/secrets-ssot.md".into(),
        origin: "claim".into(),
    }];
    get.equivalents = vec!["mcp.vox_secrets_get".into()];
    get.coverage = Some(CoverageRef {
        kind: Some("Surfaced".into()),
        basis: "graph".into(),
        ambiguous: false,
        reason: None,
    });

    let mut mcp_get = row("mcp.vox_secrets_get", RowKind::McpTool, "vox_secrets_get");
    mcp_get.domain = Some("secrets".into());
    mcp_get.mcp = Some(McpRef {
        name: "vox_secrets_get".into(),
    });
    mcp_get.owner = owner("vox-secrets", 2);
    mcp_get.equivalents = vec!["cli.secrets.get".into()];

    let mut check = row("cli.check", RowKind::Cli, "Check");
    check.summary = Some("CLI operation `vox check`".into());
    check.domain = Some("platform".into());
    check.tier = Some("recommended".into());
    check.cli = cli_ref(&["check"]);
    check.owner = owner("vox-cli", 5);
    check.coverage = no_snapshot_coverage();

    let mut mesh = row("surface.mesh", RowKind::Surface, "Mesh");
    mesh.domain = Some("mesh".into());
    mesh.gui = Some(GuiRef {
        view_key: Some("mesh".into()),
        nav_label: Some("Mesh".into()),
        representation_tier: None,
        catalog_form: false,
    });
    mesh.refs = vec!["mcp.vox_mesh_nodes".into()];

    let mut nodes = row("mcp.vox_mesh_nodes", RowKind::McpTool, "vox_mesh_nodes");
    nodes.domain = Some("mesh".into());
    nodes.mcp = Some(McpRef {
        name: "vox_mesh_nodes".into(),
    });

    let mut krate = row("crate.vox-secrets", RowKind::Crate, "vox-secrets");
    krate.domain = Some("secrets".into());
    krate.owner = owner("vox-secrets", 2);

    let skill = row("skill.demo-skill", RowKind::Skill, "demo-skill");

    let domain = |id: &str, label: &str| DomainDef {
        id: id.into(),
        label: label.into(),
    };
    CapabilityIndex {
        schema_version: 1,
        generator: "test".into(),
        coverage_snapshot: None,
        domains: vec![
            domain("secrets", "Secrets"),
            domain("mesh", "Mesh"),
            domain("platform", "Platform"),
        ],
        rows: vec![secrets, get, mcp_get, check, mesh, nodes, krate, skill],
    }
}
