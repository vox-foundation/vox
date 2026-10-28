use super::*;
use crate::commands::ci::gui_surface_registry::{RepresentationTier, SurfaceEntry};
use crate::commands::ci::operations_catalog::{CliProjection, OperationRow};
use std::collections::BTreeSet;

#[allow(dead_code)]
fn test_entry(g: &str, tier: RepresentationTier) -> SurfaceEntry {
    SurfaceEntry {
        view_key: None,
        cli_group: Some(g.into()),
        mcp_namespace: None,
        representation_tier: tier,
        nav_label: None,
        nav_icon: None,
        nav_group: None,
        parent_surface: None,
        notes: None,
    }
}

fn test_cli_op(id: &str, path: &[&str], gate: Option<&str>) -> OperationRow {
    OperationRow {
        id: id.into(),
        title: id.into(),
        description: id.into(),
        description_human: None,
        owner_crate: None,
        domain: None,
        product_lane: "core".into(),
        intent_tags: vec![],
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
        cli: Some(CliProjection {
            path: path.iter().map(|s| s.to_string()).collect(),
            status: "active".into(),
            latin_ns: None,
            handler_rust: None,
            feature_gate: gate.map(|s| s.to_string()),
            catalog_group: None,
            ref_cli_required: None,
            reachability_required: None,
        }),
    }
}

#[test]
fn pure_unit_gate_all_green() {
    let top: BTreeSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    let mut effective = BTreeMap::new();
    effective.insert("a".to_string(), RepresentationTier::CuratedDecorator);
    effective.insert("b".to_string(), RepresentationTier::GenericForm);
    effective.insert("c".to_string(), RepresentationTier::None);
    let ex = Exemption {
        group: "c".to_string(),
        owner: "vox-cli".to_string(),
        reason: "headless batch tool".to_string(),
        review_by: "2027-01-31".to_string(),
    };
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert!(r.violations.is_empty(), "violations: {:?}", r.violations);
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
}

#[test]
fn unrepresented_group_fails() {
    let top: BTreeSet<String> = ["a", "missing"].iter().map(|s| s.to_string()).collect();
    let mut effective = BTreeMap::new();
    effective.insert("a".to_string(), RepresentationTier::CuratedDecorator);
    effective.insert("missing".to_string(), RepresentationTier::None);
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let r = gate_violations(&top, &effective, &[], &crates, today);
    assert_eq!(r.violations.len(), 1);
    assert!(r.violations[0].contains("unrepresented"));
    assert!(r.violations[0].contains("missing"));
}

#[test]
fn expired_exemption_fails() {
    let top: BTreeSet<String> = ["a"].iter().map(|s| s.to_string()).collect();
    let mut effective = BTreeMap::new();
    effective.insert("a".to_string(), RepresentationTier::None);
    let ex = Exemption {
        group: "a".to_string(),
        owner: "vox-cli".to_string(),
        reason: "expired".to_string(),
        review_by: "2026-10-10".to_string(),
    };
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert_eq!(r.violations.len(), 1);
    assert!(r.violations[0].contains("expired"));
}

#[test]
fn unknown_owner_fails() {
    let top: BTreeSet<String> = ["a"].iter().map(|s| s.to_string()).collect();
    let mut effective = BTreeMap::new();
    effective.insert("a".to_string(), RepresentationTier::None);
    let ex = Exemption {
        group: "a".to_string(),
        owner: "bogus-crate".to_string(),
        reason: "unknown".to_string(),
        review_by: "2027-01-31".to_string(),
    };
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert_eq!(r.violations.len(), 1);
    assert!(r.violations[0].contains("owner `bogus-crate` is not a crate in layers.toml"));
}

#[test]
fn stale_exemption_fails() {
    let top: BTreeSet<String> = ["a"].iter().map(|s| s.to_string()).collect();
    let mut effective = BTreeMap::new();
    effective.insert("a".to_string(), RepresentationTier::CuratedDecorator);
    let ex = Exemption {
        group: "a".to_string(),
        owner: "vox-cli".to_string(),
        reason: "stale".to_string(),
        review_by: "2027-01-31".to_string(),
    };
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert_eq!(r.violations.len(), 1);
    assert!(r.violations[0].contains("stale exemption"));
}

#[test]
fn non_top_level_exemption_fails() {
    let top: BTreeSet<String> = ["a"].iter().map(|s| s.to_string()).collect();
    let mut effective = BTreeMap::new();
    effective.insert("a".to_string(), RepresentationTier::CuratedDecorator);
    let ex = Exemption {
        group: "nonexistent".to_string(),
        owner: "vox-cli".to_string(),
        reason: "bogus".to_string(),
        review_by: "2027-01-31".to_string(),
    };
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert_eq!(r.violations.len(), 1);
    assert!(r.violations[0].contains("exemption for unknown top-level group"));
}

#[test]
fn expiry_warning_window() {
    let top: BTreeSet<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
    let mut effective = BTreeMap::new();
    effective.insert("a".to_string(), RepresentationTier::CuratedDecorator);
    effective.insert("b".to_string(), RepresentationTier::None);
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();

    let ex = Exemption {
        group: "b".to_string(),
        owner: "vox-cli".to_string(),
        reason: "r".to_string(),
        review_by: "2026-10-12".to_string(),
    };
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert!(r.violations.is_empty());
    assert_eq!(r.warnings.len(), 1);

    let ex = Exemption {
        group: "b".to_string(),
        owner: "vox-cli".to_string(),
        reason: "r".to_string(),
        review_by: "2026-11-11".to_string(),
    };
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert!(r.violations.is_empty());
    assert_eq!(r.warnings.len(), 1);

    let ex = Exemption {
        group: "b".to_string(),
        owner: "vox-cli".to_string(),
        reason: "r".to_string(),
        review_by: "2026-11-12".to_string(),
    };
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert!(r.violations.is_empty());
    assert!(r.warnings.is_empty());
}

#[test]
fn new_compiled_group_passes() {
    let top: BTreeSet<String> = ["a", "b", "n"].iter().map(|s| s.to_string()).collect();
    let mut effective = BTreeMap::new();
    effective.insert("a".to_string(), RepresentationTier::GenericForm);
    effective.insert("b".to_string(), RepresentationTier::None);
    effective.insert("n".to_string(), RepresentationTier::GenericForm);
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let ex = Exemption {
        group: "b".to_string(),
        owner: "vox-cli".to_string(),
        reason: "r".to_string(),
        review_by: "2027-01-31".to_string(),
    };
    let r = gate_violations(&top, &effective, &[ex], &crates, today);
    assert!(r.violations.is_empty());
}

#[test]
fn violations_sorted_by_group() {
    let top: BTreeSet<String> = ["z", "a"].iter().map(|s| s.to_string()).collect();
    let effective = BTreeMap::new();
    let crates: BTreeSet<String> = ["vox-cli"].iter().map(|s| s.to_string()).collect();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let r = gate_violations(&top, &effective, &[], &crates, today);
    assert_eq!(r.violations.len(), 2);
    assert!(r.violations[0].contains("group `a`"));
    assert!(r.violations[1].contains("group `z`"));
}

#[test]
fn test_compiled_rows_and_effective_tier() {
    let ops = vec![
        test_cli_op("op1", &["g1", "sub1"], None),
        test_cli_op("op2", &["g1", "sub2"], Some("unknown-gate")),
        test_cli_op("op3", &["g2", "sub1"], Some("keyring-store")),
    ];
    let counts = compiled_rows_by_group(&ops);
    assert_eq!(counts.get("g1"), Some(&1));
    assert_eq!(counts.get("g2"), Some(&1));
    assert_eq!(
        effective_tier(RepresentationTier::None, 1),
        RepresentationTier::GenericForm
    );
    assert_eq!(
        effective_tier(RepresentationTier::None, 0),
        RepresentationTier::None
    );
    assert_eq!(
        effective_tier(RepresentationTier::CuratedDecorator, 0),
        RepresentationTier::CuratedDecorator
    );
}

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn committed_ledger_keeps_the_gate_green() {
    use crate::commands::ci::gui_surface_registry::{
        backfill, load_registry, missing_groups, top_level_groups,
    };
    let root = repo_root();
    let top = top_level_groups(&root).unwrap();
    let parsed = load_registry(&root).unwrap();
    let missing = missing_groups(&parsed, &top);
    let hand = backfill(parsed, &missing);
    let ops = crate::commands::ci::operations_catalog::read_catalog(&root)
        .unwrap()
        .operations;
    let effective = resolve_effective(&hand, &ops);
    let tiers: BTreeMap<String, RepresentationTier> = effective
        .surfaces
        .iter()
        .filter_map(|s| Some((s.cli_group.clone()?, s.representation_tier)))
        .collect();
    let exemptions = load_exemptions(&root).unwrap();
    let crates = crate::commands::ci::capability_index::layers_crates(&root).unwrap();
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let r = gate_violations(&top, &tiers, &exemptions, &crates, today);
    assert!(r.violations.is_empty(), "violations: {:#?}", r.violations);
}

#[test]
fn gate_ratchet_live_sets() {
    use super::{dedicated_ids, exemption_ids};
    use std::collections::BTreeSet;

    let exemptions = vec![
        Exemption {
            group: "dei".into(),
            owner: "vox-cli".into(),
            reason: "gated".into(),
            review_by: "2027-01-31".into(),
        },
        Exemption {
            group: "visus".into(),
            owner: "vox-cli".into(),
            reason: "gated".into(),
            review_by: "2027-01-31".into(),
        },
    ];
    let ex_ids = exemption_ids(&exemptions);
    assert_eq!(
        ex_ids,
        BTreeSet::from(["dei".to_string(), "visus".to_string()])
    );

    let mut tiers = BTreeMap::new();
    tiers.insert("gamify".to_string(), RepresentationTier::LiveBackend);
    tiers.insert("model".to_string(), RepresentationTier::CuratedDecorator);
    tiers.insert("check".to_string(), RepresentationTier::GenericForm);
    tiers.insert("dei".to_string(), RepresentationTier::None);
    let floor_ids = dedicated_ids(&tiers);
    assert_eq!(
        floor_ids,
        BTreeSet::from(["gamify".to_string(), "model".to_string()])
    );
}

#[test]
fn test_parse_today() {
    use crate::commands::ci::gui_surface_registry::parse_today;

    assert_eq!(
        parse_today("2026-10-12").unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()
    );
    let err1 = parse_today("2026-13-01").unwrap_err().to_string();
    assert!(err1.contains("--today"), "got: {err1}");
    let err2 = parse_today("today").unwrap_err().to_string();
    assert!(err2.contains("--today"), "got: {err2}");
}
