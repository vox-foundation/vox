#[allow(unused_imports)]
use std::collections::BTreeMap;
use std::collections::HashMap;

use serde::Deserialize;

use crate::forbidden_patterns::ForbiddenPatternRule;

#[derive(Debug, Deserialize)]
pub(crate) struct LayersConfig {
    pub(crate) crates: HashMap<String, CrateEntry>,
    #[serde(default)]
    pub(crate) known_inversions: Vec<KnownInversion>,
    #[serde(default)]
    pub(crate) forbidden_deps: Vec<ForbiddenDepRule>,
    /// Rule 11 (P3-T7): forbidden code patterns with optional allow annotations.
    #[serde(default)]
    pub(crate) forbidden_pattern: Vec<ForbiddenPatternRule>,
    #[serde(default)]
    pub(crate) guards: GuardsConfig,
    #[serde(default)]
    pub(crate) arch_check: ArchCheckConfig,
    /// Rule 12: crates that are documented (in WTL / layers.toml) but not yet on
    /// disk. Entries here suppress the parity warning; each must point to the plan
    /// doc that owns the work via `plan = "..."`.
    #[serde(default)]
    pub(crate) planned: HashMap<String, PlannedEntry>,
    /// Rule 20: named build profiles (e.g. [profiles.lean]) with forbidden-crate lists.
    #[serde(default)]
    pub(crate) profiles: HashMap<String, ProfileConfig>,
}

/// A named build profile with an optional forbidden-crate list for Rule 20.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProfileConfig {
    #[allow(dead_code)]
    pub(crate) build: Option<String>,
    #[serde(default)]
    pub(crate) forbidden: Vec<String>,
}

/// A crate that is planned but not yet landed on disk. Used by Rule 12 to suppress
/// WTL/layers.toml parity warnings for in-flight designs.
#[derive(Debug, Deserialize)]
pub(crate) struct PlannedEntry {
    /// Path to the architecture doc that owns this planned crate.
    #[allow(dead_code)]
    pub(crate) plan: String,
    /// Intended layer when the crate lands.
    #[allow(dead_code)]
    pub(crate) layer: Option<u8>,
}

/// Optional knobs for `vox-arch-check` itself (see `[arch_check.walk_prune]` in layers.toml).
#[derive(Debug, Default, Deserialize)]
pub(crate) struct ArchCheckConfig {
    #[serde(default)]
    pub(crate) walk_prune: WalkPruneConfig,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct WalkPruneConfig {
    /// Extra directory *names* (not full paths) to skip when recursing.
    #[serde(default)]
    pub(crate) extra_skip_dir_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CrateEntry {
    pub(crate) layer: u8,
    #[serde(default = "default_kind")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) max_dependents: Option<usize>,
    #[serde(default)]
    pub(crate) max_loc: Option<usize>,
    /// Opt out of Rule 8 staleness check for intentionally stable crates.
    #[serde(default)]
    pub(crate) staleness_exempt: bool,
    /// Opt out of Rule 4 orphan check — for libraries consumed by generated
    /// code or Tauri app scaffolding that has no in-tree Rust dep edge.
    #[serde(default)]
    pub(crate) orphan_exempt: bool,
    /// Rule 15: cap on normal workspace-member deps for this crate.
    #[serde(default)]
    pub(crate) max_workspace_deps: Option<usize>,
    /// Other crates this one is structurally a sibling of. Consumed by
    /// `vox-drift-check`'s `sweep/duplicate-body` rule to tolerate intentional
    /// code duplication across vendor splits and extraction migrations. Edges
    /// are undirected (closure over the declared graph).
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) sibling_of: Vec<String>,
    /// Rule 18: this crate is intended for public crates.io release.
    /// Its entire workspace-crate dependency closure must also be publishable.
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) publishable: bool,
    /// Track A classifier advisory: this statically-linked capability should
    /// migrate behind a plugin extension point (separable, optional at runtime).
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) plugin_candidate: bool,
}

pub(crate) fn default_kind() -> String {
    "library".to_string()
}

#[derive(Debug, Deserialize)]
pub(crate) struct KnownInversion {
    pub(crate) from: String,
    pub(crate) to: String,
    #[allow(dead_code)]
    pub(crate) reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ForbiddenDepRule {
    #[serde(rename = "crate")]
    pub(crate) krate: String,
    pub(crate) forbidden: Vec<String>,
    #[allow(dead_code)]
    pub(crate) reason: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct GuardsConfig {
    /// "error" or "warn"; defaults to "warn" for all but layer ordering.
    #[serde(default)]
    pub(crate) fan_in: Option<String>,
    #[serde(default)]
    pub(crate) loc_budget: Option<String>,
    #[serde(default)]
    pub(crate) orphan: Option<String>,
    #[serde(default)]
    pub(crate) docstring: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) where_things_live: Option<String>,
    #[serde(default)]
    pub(crate) staleness: Option<String>,
    #[serde(default)]
    pub(crate) generated_file_drift: Option<String>,
    #[serde(default)]
    pub(crate) forbidden_deps: Option<String>,
    #[serde(default)]
    pub(crate) forbidden_pattern: Option<String>,
    /// Rule 12: WTL / layers.toml / disk three-way parity.
    #[serde(default)]
    pub(crate) wtl_parity: Option<String>,
    /// Rule 13: LoC delta regression check.
    #[serde(default)]
    pub(crate) loc_delta: Option<String>,
    /// Rule 14: cdylib-as-normal-dep guard (default: error).
    #[serde(default)]
    pub(crate) cdylib_dep: Option<String>,
    /// Rule 15: workspace-dep budget guard (default: warn).
    #[serde(default)]
    pub(crate) workspace_deps: Option<String>,
}
