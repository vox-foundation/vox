use std::collections::BTreeMap;

use super::gui_surface_registry::{RepresentationTier, SurfaceRegistry};
use super::operations_catalog::OperationRow;

pub(crate) use super::gui_coverage_exemptions::Exemption;
#[cfg(test)]
pub(crate) use super::gui_coverage_exemptions::load_exemptions;

/// Default features of vox-cli compiled into the GUI build.
pub(crate) const GUI_BUILD_FEATURES: &[&str] = &["keyring-store", "script-execution"];

/// Pure predicate deciding if a CLI operation is compiled into the GUI build.
pub(crate) fn in_gui_build(status: &str, feature_gate: Option<&str>) -> bool {
    if status != "active" {
        return false;
    }
    let Some(gate) = feature_gate else {
        return true;
    };
    let gate = gate.trim();
    if gate.is_empty() {
        return true;
    }
    gate.split(['|', '+', ','])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .all(|token| GUI_BUILD_FEATURES.contains(&token))
}

/// Counts, per top-level CLI group (first path segment), the number of CLI rows that are in_gui_build.
pub(crate) fn compiled_rows_by_group(ops: &[OperationRow]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for op in ops {
        if let Some(cli) = &op.cli {
            if in_gui_build(&cli.status, cli.feature_gate.as_deref()) {
                if let Some(first) = cli.path.first() {
                    *counts.entry(first.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    counts
}

/// Computes the effective representation tier for a group given its hand tier and compiled verb count.
pub(crate) fn effective_tier(hand: RepresentationTier, compiled: usize) -> RepresentationTier {
    match hand {
        RepresentationTier::None if compiled > 0 => RepresentationTier::GenericForm,
        other => other,
    }
}

/// Derives effective representation tiers into a cloned `SurfaceRegistry`.
pub(crate) fn resolve_effective(hand: &SurfaceRegistry, ops: &[OperationRow]) -> SurfaceRegistry {
    let counts = compiled_rows_by_group(ops);
    let mut resolved = hand.clone();
    for surface in &mut resolved.surfaces {
        if let Some(group) = &surface.cli_group {
            let compiled = counts.get(group).copied().unwrap_or(0);
            surface.representation_tier = effective_tier(surface.representation_tier, compiled);
        }
    }
    resolved
}

/// Returns sorted list of violations: duplicate `cli_group`s or `cli_group`s not in `top_level`.
pub(crate) fn registry_violations(
    reg: &SurfaceRegistry,
    top_level: &std::collections::BTreeSet<String>,
) -> Vec<String> {
    let mut violations = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut duplicates = std::collections::BTreeSet::new();
    for e in &reg.surfaces {
        if let Some(g) = &e.cli_group {
            if !seen.insert(g.clone()) {
                duplicates.insert(g.clone());
            }
            if !top_level.contains(g) {
                violations.push(format!("cli_group '{g}' is not a top-level CLI group"));
            }
        }
    }
    for dup in duplicates {
        violations.push(format!("duplicate cli_group '{dup}' in surface registry"));
    }
    violations.sort();
    violations
}

/// Report returned by [`gate_violations`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GateReport {
    pub violations: Vec<String>,
    pub warnings: Vec<String>,
}

/// Evaluates coverage gate rules for top-level CLI groups against effective representation tiers,
/// exemptions, and declared crates in `layers.toml`.
pub(crate) fn gate_violations(
    top_level: &std::collections::BTreeSet<String>,
    effective: &BTreeMap<String, RepresentationTier>,
    exemptions: &[Exemption],
    crates: &std::collections::BTreeSet<String>,
    today: chrono::NaiveDate,
) -> GateReport {
    let mut raw_violations: Vec<(String, String)> = Vec::new();
    let mut raw_warnings: Vec<(String, String)> = Vec::new();
    let mut seen_exemptions = std::collections::BTreeSet::new();
    let mut exempt_groups = std::collections::BTreeSet::new();

    for ex in exemptions {
        let g = &ex.group;
        if !seen_exemptions.insert(g.clone()) {
            raw_violations.push((
                g.clone(),
                format!("group `{g}`: duplicate coverage exemption"),
            ));
        }
        exempt_groups.insert(g.clone());
        if ex.owner.trim().is_empty() {
            raw_violations.push((g.clone(), format!("group `{g}`: owner cannot be empty")));
        } else if !crates.contains(&ex.owner) {
            raw_violations.push((
                g.clone(),
                format!(
                    "group `{g}`: owner `{}` is not a crate in layers.toml [crates]",
                    ex.owner
                ),
            ));
        }
        if ex.reason.trim().is_empty() {
            raw_violations.push((g.clone(), format!("group `{g}`: reason cannot be empty")));
        }
        if !top_level.contains(g) {
            raw_violations.push((
                g.clone(),
                format!("group `{g}`: exemption for unknown top-level group"),
            ));
        }
        match chrono::NaiveDate::parse_from_str(&ex.review_by, "%Y-%m-%d") {
            Err(_) => raw_violations.push((
                g.clone(),
                format!("group `{g}`: unparseable review_by date `{}`", ex.review_by),
            )),
            Ok(date) if date < today => raw_violations.push((
                g.clone(),
                format!(
                    "group `{g}`: exemption expired on {} (past review_by)",
                    ex.review_by
                ),
            )),
            Ok(date) if date <= today + chrono::Duration::days(30) => raw_warnings.push((
                g.clone(),
                format!(
                    "group `{g}`: exemption review_by is {} (expires within 30 days)",
                    ex.review_by
                ),
            )),
            Ok(_) => {}
        }
        let tier = effective
            .get(g)
            .copied()
            .unwrap_or(RepresentationTier::None);
        if tier != RepresentationTier::None {
            raw_violations.push((
                g.clone(),
                format!("group `{g}`: stale exemption; group is represented with tier `{tier:?}`"),
            ));
        }
    }

    for group in top_level {
        let tier = effective
            .get(group)
            .copied()
            .unwrap_or(RepresentationTier::None);
        if tier == RepresentationTier::None && !exempt_groups.contains(group) {
            raw_violations.push((group.clone(), format!("group `{group}`: unrepresented CLI group with no coverage exemption (add a GUI surface, compile a verb into the GUI build, or add an owner+reason exemption to contracts/gui/coverage-exemptions.v1.yaml)")));
        }
    }
    raw_violations.sort();
    raw_warnings.sort();
    GateReport {
        violations: raw_violations.into_iter().map(|(_, msg)| msg).collect(),
        warnings: raw_warnings.into_iter().map(|(_, msg)| msg).collect(),
    }
}

/// Produces the honesty summary line for GUI coverage reporting.
pub(crate) fn honesty_line(
    dedicated: usize,
    generic: usize,
    exempt: usize,
    partial: &[(String, usize, usize)],
) -> String {
    let partial_str = if partial.is_empty() {
        "none".to_string()
    } else {
        partial
            .iter()
            .map(|(g, c, t)| format!("{g} {c}/{t}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "gui-coverage: {dedicated} dedicated, {generic} derived generic_form, {exempt} exempt (most groups pass by derivation; partially compiled: {partial_str})"
    )
}

/// Counts for [`honesty_line`]: (dedicated, derived generic_form, exempt, partially compiled
/// derived groups as `(group, compiled, total)`). A group is "derived" when its hand tier is
/// `none` but at least one verb is compiled into the GUI build.
pub(crate) fn coverage_summary(
    hand: &SurfaceRegistry,
    effective: &BTreeMap<String, RepresentationTier>,
    ops: &[OperationRow],
    exemptions: &[Exemption],
) -> (usize, usize, usize, Vec<(String, usize, usize)>) {
    let compiled = compiled_rows_by_group(ops);
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    for cli in ops.iter().filter_map(|o| o.cli.as_ref()) {
        if let Some(first) = cli.path.first() {
            *totals.entry(first.clone()).or_insert(0) += 1;
        }
    }
    let hand_none: std::collections::BTreeSet<&str> = hand
        .surfaces
        .iter()
        .filter(|s| s.representation_tier == RepresentationTier::None)
        .filter_map(|s| s.cli_group.as_deref())
        .collect();
    let (mut dedicated, mut generic) = (0, 0);
    let mut partial = Vec::new();
    for (g, tier) in effective {
        match tier {
            RepresentationTier::LiveBackend | RepresentationTier::CuratedDecorator => {
                dedicated += 1
            }
            RepresentationTier::GenericForm => {
                generic += 1;
                let (c, t) = (
                    compiled.get(g).copied().unwrap_or(0),
                    totals.get(g).copied().unwrap_or(0),
                );
                if hand_none.contains(g.as_str()) && c < t {
                    partial.push((g.clone(), c, t));
                }
            }
            RepresentationTier::None => {}
        }
    }
    (dedicated, generic, exemptions.len(), partial)
}

pub(crate) fn exemption_ids(exemptions: &[Exemption]) -> std::collections::BTreeSet<String> {
    exemptions.iter().map(|e| e.group.clone()).collect()
}

pub(crate) fn dedicated_ids(
    tiers: &BTreeMap<String, RepresentationTier>,
) -> std::collections::BTreeSet<String> {
    tiers
        .iter()
        .filter(|(_, tier)| {
            matches!(
                tier,
                RepresentationTier::LiveBackend | RepresentationTier::CuratedDecorator
            )
        })
        .map(|(group, _)| group.clone())
        .collect()
}

pub(crate) fn tighten_baselines(repo_root: &std::path::Path) -> anyhow::Result<()> {
    use super::capability_index::ratchet::{
        DEDICATED_FLOOR, EXEMPTION, FloorBaseline, RatchetBaseline, load_baseline, tighten,
        tighten_floor,
    };
    use super::gui_surface_registry::{backfill, load_registry, missing_groups, top_level_groups};
    use super::operations_catalog::write_or_verify_text;

    let parsed = load_registry(repo_root)?;
    let top_level = top_level_groups(repo_root)?;
    let missing = missing_groups(&parsed, &top_level);
    let hand = backfill(parsed, &missing);
    let ops = super::operations_catalog::read_catalog(repo_root)?.operations;
    let effective = resolve_effective(&hand, &ops);
    let exemptions = super::gui_coverage_exemptions::load_exemptions(repo_root)?;
    let tiers: BTreeMap<String, RepresentationTier> = effective
        .surfaces
        .iter()
        .filter_map(|s| Some((s.cli_group.clone()?, s.representation_tier)))
        .collect();

    let live_exemption = exemption_ids(&exemptions);
    let path_ex = repo_root.join(EXEMPTION.baseline_rel);
    let current_ex: Option<RatchetBaseline> = if path_ex.is_file() {
        Some(load_baseline(
            repo_root,
            &EXEMPTION,
            "vox ci gui-surface-registry --tighten",
        )?)
    } else {
        None
    };
    let next_ex = tighten(&live_exemption, current_ex.as_ref(), &EXEMPTION)?;
    write_or_verify_text(
        &path_ex,
        &next_ex.render()?,
        true,
        "run `vox ci gui-surface-registry --tighten`",
    )?;

    let live_floor = dedicated_ids(&tiers);
    let path_fl = repo_root.join(DEDICATED_FLOOR.baseline_rel);
    let current_fl: Option<FloorBaseline> = if path_fl.is_file() {
        Some(load_baseline(
            repo_root,
            &DEDICATED_FLOOR,
            "vox ci gui-surface-registry --tighten",
        )?)
    } else {
        None
    };
    let next_fl = tighten_floor(&live_floor, current_fl.as_ref(), &DEDICATED_FLOOR)?;
    write_or_verify_text(
        &path_fl,
        &next_fl.render()?,
        true,
        "run `vox ci gui-surface-registry --tighten`",
    )?;

    println!("gui-surface-registry: tightened baselines (exemption and dedicated floor)");
    Ok(())
}

#[cfg(test)]
mod tests;
