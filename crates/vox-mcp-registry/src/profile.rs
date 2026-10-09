//! Capability profiles: named tool + subsystem slices generated from `contracts/profiles/*.v1.yaml`.

use std::collections::BTreeSet;

use crate::{Exposure, Subsystem, subsystem_closure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileManifest {
    pub id: &'static str,
    pub description: &'static str,
    pub requires: &'static [Subsystem],
    /// Fully resolved at build time (globs/lanes expanded, clipped to `max_exposure`), sorted.
    pub tools: &'static [&'static str],
    pub max_exposure: Exposure,
    pub axis_surfaces: &'static [&'static str],
}

pub const DEV_PROFILE_ID: &str = "dev";

const PROFILE_ENV: &str = "VOX_PROFILE";

include!(concat!(env!("OUT_DIR"), "/profiles.rs"));

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("profile_unknown: {0}")]
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ActiveProfile {
    pub ids: Vec<&'static str>,
    pub tools: BTreeSet<&'static str>,
    pub subsystems: BTreeSet<Subsystem>,
    pub axis_surfaces: BTreeSet<&'static str>,
    pub max_exposure: Exposure,
}

fn manifest(id: &str) -> Option<&'static ProfileManifest> {
    PROFILES.iter().find(|p| p.id == id)
}

impl ActiveProfile {
    pub fn dev() -> Self {
        let dev = manifest(DEV_PROFILE_ID).expect("generated PROFILES always contains dev");
        Self::from_manifests(&[dev])
    }

    /// Union of the named profiles; empty (or all-blank) ⇒ `dev`; any `dev` id ⇒ `dev`.
    pub fn resolve(ids: &[&str]) -> Result<Self, ProfileError> {
        let mut picked: Vec<&'static ProfileManifest> = Vec::new();
        for raw in ids {
            let id = raw.trim();
            if id.is_empty() {
                continue;
            }
            picked.push(manifest(id).ok_or_else(|| ProfileError::Unknown(id.to_string()))?);
        }
        if picked.is_empty() || picked.iter().any(|m| m.id == DEV_PROFILE_ID) {
            return Ok(Self::dev());
        }
        Ok(Self::from_manifests(&picked))
    }

    /// Reads `VOX_PROFILE` (comma-separated ids); unset or empty ⇒ `dev`.
    pub fn from_env() -> Result<Self, ProfileError> {
        Self::from_raw(std::env::var(PROFILE_ENV).ok().as_deref())
    }

    fn from_raw(raw: Option<&str>) -> Result<Self, ProfileError> {
        let ids: Vec<&str> = raw.unwrap_or("").split(',').collect();
        Self::resolve(&ids)
    }

    pub fn contains_tool(&self, name: &str) -> bool {
        self.tools.contains(name)
    }

    /// Canonical id list: sorted, unique, joined with `,`.
    pub fn id_string(&self) -> String {
        self.ids.join(",")
    }

    fn from_manifests(manifests: &[&'static ProfileManifest]) -> Self {
        let ids: BTreeSet<&'static str> = manifests.iter().map(|m| m.id).collect();
        let roots: Vec<Subsystem> = manifests
            .iter()
            .flat_map(|m| m.requires.iter().copied())
            .collect();
        Self {
            ids: ids.into_iter().collect(),
            tools: manifests
                .iter()
                .flat_map(|m| m.tools.iter().copied())
                .collect(),
            subsystems: subsystem_closure(&roots),
            axis_surfaces: manifests
                .iter()
                .flat_map(|m| m.axis_surfaces.iter().copied())
                .collect(),
            max_exposure: manifests
                .iter()
                .map(|m| m.max_exposure)
                .max()
                .unwrap_or(Exposure::Public),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Exposure, Subsystem, TOOL_REGISTRY, registry_entry, subsystem_closure};
    use std::collections::BTreeSet;

    const FIRST_WAVE: &[&str] = &[
        "attention",
        "browser",
        "knowledge",
        "lang",
        "mesh",
        "models",
        "orchestrator",
        "repo-read",
        "research",
        "research-publish",
        "skills",
        "speech",
    ];

    #[test]
    fn profiles_are_first_wave_plus_dev() {
        let mut ids: Vec<&str> = PROFILES.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        let mut want: Vec<&str> = FIRST_WAVE.to_vec();
        want.push(DEV_PROFILE_ID);
        want.sort_unstable();
        assert_eq!(ids, want);
    }

    #[test]
    fn dev_is_every_tool_every_subsystem_internal_all_surfaces() {
        let dev = ActiveProfile::dev();
        assert_eq!(dev.ids, vec![DEV_PROFILE_ID]);
        assert_eq!(dev.tools.len(), TOOL_REGISTRY.len());
        for e in TOOL_REGISTRY {
            assert!(dev.contains_tool(e.name), "dev missing {}", e.name);
        }
        let all: BTreeSet<Subsystem> = Subsystem::ALL.iter().copied().collect();
        assert_eq!(dev.subsystems, all);
        assert_eq!(dev.max_exposure, Exposure::Internal);
        assert!(dev.axis_surfaces.contains("*"));
    }

    #[test]
    fn named_profiles_respect_max_exposure_and_subsystem_closure() {
        for p in PROFILES.iter().filter(|p| p.id != DEV_PROFILE_ID) {
            assert!(
                p.max_exposure < Exposure::Internal,
                "{} may not be internal",
                p.id
            );
            assert!(!p.tools.is_empty(), "{} resolves to no tools", p.id);
            let closure = subsystem_closure(p.requires);
            for name in p.tools {
                let e = registry_entry(name).unwrap_or_else(|| panic!("{}: unknown {name}", p.id));
                assert!(
                    e.exposure <= p.max_exposure,
                    "{}: {name} above max_exposure",
                    p.id
                );
                for r in e.requires {
                    assert!(
                        closure.contains(r),
                        "{}: {name} requires {r:?} outside closure",
                        p.id
                    );
                }
            }
        }
    }

    #[test]
    fn manifest_tools_are_sorted_and_unique() {
        for p in PROFILES {
            let mut sorted = p.tools.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted, p.tools, "{} tools not sorted/unique", p.id);
        }
    }

    #[test]
    fn browser_profile_is_only_browser_tools() {
        let b = ActiveProfile::resolve(&["browser"]).expect("browser");
        assert!(b.tools.iter().all(|t| t.starts_with("vox_browser_")));
    }

    #[test]
    fn research_excludes_publish_and_approve_which_live_in_research_publish() {
        let r = ActiveProfile::resolve(&["research"]).expect("research");
        let p = ActiveProfile::resolve(&["research-publish"]).expect("research-publish");
        for t in [
            "vox_scientia_publication_publish",
            "vox_scientia_publication_approve",
            "vox_news_approve",
        ] {
            assert!(!r.contains_tool(t), "research must not contain {t}");
        }
        assert!(r.tools.is_disjoint(&p.tools));
    }

    #[test]
    fn empty_resolve_is_dev() {
        assert_eq!(
            ActiveProfile::resolve(&[]).expect("empty"),
            ActiveProfile::dev()
        );
        assert_eq!(
            ActiveProfile::resolve(&["", "  "]).expect("blank"),
            ActiveProfile::dev()
        );
    }

    #[test]
    fn composition_is_union_and_canonical() {
        let m = ActiveProfile::resolve(&["models"]).expect("models");
        let k = ActiveProfile::resolve(&["knowledge"]).expect("knowledge");
        let mk = ActiveProfile::resolve(&[" models ", "knowledge"]).expect("models,knowledge");
        assert_eq!(
            mk,
            ActiveProfile::resolve(&["knowledge", "models", "models"]).expect("dup")
        );
        assert_eq!(mk.ids, vec!["knowledge", "models"]);
        assert_eq!(mk.id_string(), "knowledge,models");
        assert_eq!(
            mk.tools,
            m.tools.union(&k.tools).copied().collect::<BTreeSet<_>>()
        );
        let roots: Vec<Subsystem> = m.subsystems.union(&k.subsystems).copied().collect();
        assert_eq!(mk.subsystems, subsystem_closure(&roots));
        assert!(mk.axis_surfaces.contains("models") && mk.axis_surfaces.contains("knowledge"));
    }

    #[test]
    fn dev_absorbs_other_ids() {
        assert_eq!(
            ActiveProfile::resolve(&["models", "dev"]).expect("dev"),
            ActiveProfile::dev()
        );
    }

    #[test]
    fn unknown_id_is_profile_unknown() {
        let err = ActiveProfile::resolve(&["models", "nope"]).expect_err("unknown");
        assert_eq!(err, ProfileError::Unknown("nope".to_string()));
        assert_eq!(err.to_string(), "profile_unknown: nope");
    }

    #[test]
    fn max_exposure_is_max_over_composed_profiles() {
        assert_eq!(
            ActiveProfile::resolve(&["repo-read"])
                .expect("rr")
                .max_exposure,
            Exposure::Public
        );
        assert_eq!(
            ActiveProfile::resolve(&["repo-read", "models"])
                .expect("rr,m")
                .max_exposure,
            Exposure::Operator
        );
    }

    /// Decision D1 (00-contracts Amendments): `models` boots the DB; `attention` and `skills` stay DB-free.
    #[test]
    fn models_requires_db_attention_and_skills_are_db_free() {
        assert!(
            ActiveProfile::resolve(&["models"])
                .expect("models")
                .subsystems
                .contains(&Subsystem::Db)
        );
        for id in ["attention", "skills"] {
            let p = ActiveProfile::resolve(&[id]).expect(id);
            assert!(
                !p.subsystems.contains(&Subsystem::Db),
                "{id} must not require db"
            );
        }
    }

    #[test]
    fn contains_tool_matches_resolved_set() {
        let m = ActiveProfile::resolve(&["models"]).expect("models");
        assert!(m.contains_tool("vox_list_models"));
        assert!(!m.contains_tool("vox_git_status"));
        assert!(!m.contains_tool("vox_does_not_exist"));
    }

    #[test]
    fn from_raw_parses_comma_list() {
        assert_eq!(
            ActiveProfile::from_raw(None).expect("none"),
            ActiveProfile::dev()
        );
        assert_eq!(
            ActiveProfile::from_raw(Some("")).expect("empty"),
            ActiveProfile::dev()
        );
        assert_eq!(
            ActiveProfile::from_raw(Some("models, knowledge"))
                .expect("list")
                .id_string(),
            "knowledge,models"
        );
        assert!(ActiveProfile::from_raw(Some("models,nope")).is_err());
    }

    #[test]
    fn from_env_reads_vox_profile() {
        let raw = std::env::var("VOX_PROFILE").ok();
        assert_eq!(
            ActiveProfile::from_env(),
            ActiveProfile::from_raw(raw.as_deref())
        );
    }
}
