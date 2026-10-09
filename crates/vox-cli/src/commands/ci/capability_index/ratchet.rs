//! Unresolved-owner ratchet (D-05): the number of cli/mcp rows with no resolvable owner may only
//! fall. The baseline file is written by `tighten` alone (removal-only), never by `--write`.

use std::collections::BTreeSet;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use vox_capability_registry::{CapabilityIndex, RowKind};

pub(crate) const BASELINE_REL: &str = "contracts/capabilities/owner-ratchet.v1.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnerRatchet {
    pub schema_version: u32,
    pub max_unresolved: usize,
    pub unresolved: Vec<String>,
}

impl OwnerRatchet {
    fn from_set(ids: &BTreeSet<String>) -> Self {
        Self {
            schema_version: 1,
            max_unresolved: ids.len(),
            unresolved: ids.iter().cloned().collect(),
        }
    }

    /// Exact on-disk text: pretty JSON, ids sorted, trailing newline.
    pub(crate) fn render(&self) -> Result<String> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// Ids of the cli/mcp rows whose owner is `None`.
pub(crate) fn unresolved_ids(index: &CapabilityIndex) -> BTreeSet<String> {
    index
        .rows
        .iter()
        .filter(|r| matches!(r.kind, RowKind::Cli | RowKind::McpTool) && r.owner.is_none())
        .map(|r| r.id.clone())
        .collect()
}

/// Fail when a row lost (or never had) its owner and is not already in the baseline.
pub(crate) fn check(live: &BTreeSet<String>, baseline: &OwnerRatchet) -> Result<()> {
    let known: BTreeSet<&str> = baseline.unresolved.iter().map(String::as_str).collect();
    let added: Vec<&str> = live
        .iter()
        .map(String::as_str)
        .filter(|id| !known.contains(id))
        .collect();
    if !added.is_empty() {
        bail!(
            "{} cli/mcp row(s) have no resolvable owner and are not in {BASELINE_REL}: {}. \
             Fix the source, not the baseline: add the group to `cli_groups` (with an `owner`) in \
             contracts/capabilities/domains.v1.yaml, or set `owner_crate` on the catalog op",
            added.len(),
            added.join(", ")
        );
    }
    if live.len() > baseline.max_unresolved {
        bail!(
            "{} unresolved owners exceed max_unresolved {} in {BASELINE_REL}",
            live.len(),
            baseline.max_unresolved
        );
    }
    Ok(())
}

/// Shrink the baseline to the rows still unresolved. `None` bootstraps it from `live`.
/// Additions are refused: raising the baseline is a visible hand edit, never automation.
pub(crate) fn tighten(
    live: &BTreeSet<String>,
    baseline: Option<&OwnerRatchet>,
) -> Result<OwnerRatchet> {
    let Some(base) = baseline else {
        return Ok(OwnerRatchet::from_set(live));
    };
    let known: BTreeSet<&str> = base.unresolved.iter().map(String::as_str).collect();
    let added: Vec<&str> = live
        .iter()
        .map(String::as_str)
        .filter(|id| !known.contains(id))
        .collect();
    if !added.is_empty() {
        bail!(
            "--tighten would ADD {} unresolved owner(s) (tighten is removal-only): {}",
            added.len(),
            added.join(", ")
        );
    }
    Ok(OwnerRatchet::from_set(live))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    fn baseline(ids: &[&str]) -> OwnerRatchet {
        OwnerRatchet::from_set(&set(ids))
    }

    #[test]
    fn new_unresolved_id_fails() {
        let err = check(&set(&["a", "b"]), &baseline(&["a"])).expect_err("b is new");
        let msg = err.to_string();
        assert!(
            msg.contains('b') && msg.contains("domains.v1.yaml") && msg.contains("owner_crate"),
            "got: {msg}"
        );
    }

    #[test]
    fn count_above_max_fails() {
        let mut b = baseline(&["a", "b"]);
        b.max_unresolved = 1;
        let err = check(&set(&["a", "b"]), &b).expect_err("count above max");
        assert!(err.to_string().contains("max_unresolved"), "got: {err}");
    }

    #[test]
    fn shrink_passes() {
        check(&set(&["a"]), &baseline(&["a", "b"])).expect("a shrinking set is fine");
        check(&set(&[]), &baseline(&["a"])).expect("fully resolved is fine");
    }

    #[test]
    fn tighten_refuses_additions() {
        let err = tighten(&set(&["a", "b"]), Some(&baseline(&["a"]))).expect_err("must refuse");
        assert!(
            err.to_string().contains("tighten is removal-only"),
            "got: {err}"
        );
    }

    #[test]
    fn tighten_removes_resolved() {
        let out = tighten(&set(&["a"]), Some(&baseline(&["a", "b"]))).expect("tighten");
        assert_eq!(out.unresolved, vec!["a".to_string()]);
        assert_eq!(out.max_unresolved, 1);
    }

    #[test]
    fn tighten_bootstraps_from_live() {
        let out = tighten(&set(&["x", "y"]), None).expect("bootstrap");
        assert_eq!(out, baseline(&["x", "y"]));
    }

    #[test]
    fn baseline_render_is_sorted_and_stable() {
        let one = baseline(&["b", "a"]).render().expect("render");
        let two = baseline(&["a", "b"]).render().expect("render");
        assert_eq!(one, two);
        assert!(one.ends_with("}\n"));
        assert!(one.find("\"a\"").expect("a") < one.find("\"b\"").expect("b"));
        assert!(one.contains("\"max_unresolved\": 2"));
    }
}
