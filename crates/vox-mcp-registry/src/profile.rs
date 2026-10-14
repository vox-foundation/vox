//! Capability profiles: named tool + subsystem slices generated from `contracts/profiles/*.v1.yaml`.

use std::collections::BTreeSet;

use crate::{
    BootReport, Exposure, McpToolRegistryEntry, ProfileDenial, Subsystem, registry_entry,
    subsystem_closure,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerRole {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallContext {
    pub role: CallerRole,
    pub loopback: bool,
}

impl CallContext {
    pub const LOCAL_WRITE: CallContext = CallContext {
        role: CallerRole::Write,
        loopback: true,
    };
}

/// The single profile decision. Check order is part of the contract (00-contracts.md).
pub fn authorize_tool(
    profile: &ActiveProfile,
    boot: &BootReport,
    ctx: CallContext,
    tool: &str,
) -> Result<&'static McpToolRegistryEntry, ProfileDenial> {
    let entry = registry_entry(tool)
        .filter(|_| profile.contains_tool(tool))
        .ok_or(ProfileDenial::ToolNotInProfile)?;
    if !ctx.loopback && entry.exposure == Exposure::Internal {
        return Err(ProfileDenial::ExposureDenied);
    }
    if ctx.role == CallerRole::Read && entry.exposure == Exposure::Operator {
        return Err(ProfileDenial::ExposureDenied);
    }
    if ctx.role == CallerRole::Read && !entry.http_read_role_eligible {
        return Err(ProfileDenial::InsufficientRole);
    }
    if let Some(s) = boot.missing_for(entry.requires) {
        return Err(ProfileDenial::SubsystemNotBooted(s));
    }
    // A listed requirement can be up while a dependency it cannot run without
    // is down (search_index depends on db). Deny on the first missing closure member.
    let closure: Vec<Subsystem> = subsystem_closure(entry.requires).into_iter().collect();
    if let Some(s) = boot.missing_for(&closure) {
        return Err(ProfileDenial::SubsystemNotBooted(s));
    }
    Ok(entry)
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
            "vox_scientia_publication_claim_review",
            "vox_scientia_publication_nanopublish",
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

#[cfg(test)]
mod authorize_tests {
    use super::*;
    use crate::{
        BootReport, Exposure, McpToolRegistryEntry, ProfileDenial, Subsystem, TOOL_REGISTRY,
    };
    use std::collections::BTreeSet;

    const READ_REMOTE: CallContext = CallContext {
        role: CallerRole::Read,
        loopback: false,
    };
    const READ_LOCAL: CallContext = CallContext {
        role: CallerRole::Read,
        loopback: true,
    };
    const WRITE_REMOTE: CallContext = CallContext {
        role: CallerRole::Write,
        loopback: false,
    };

    fn profile_with(tools: &[&'static str]) -> ActiveProfile {
        ActiveProfile {
            ids: vec!["test"],
            tools: tools.iter().copied().collect(),
            subsystems: BTreeSet::new(),
            axis_surfaces: BTreeSet::new(),
            max_exposure: Exposure::Internal,
        }
    }

    fn full_boot() -> BootReport {
        BootReport {
            booted: Subsystem::ALL.iter().copied().collect(),
            failed: Default::default(),
        }
    }

    fn find(
        pred: impl Fn(&McpToolRegistryEntry) -> bool,
        what: &str,
    ) -> &'static McpToolRegistryEntry {
        TOOL_REGISTRY
            .iter()
            .find(|e| pred(e))
            .unwrap_or_else(|| panic!("registry has no {what} tool (plan 01 classification)"))
    }

    #[test]
    fn step1_unknown_tool_is_not_in_profile() {
        let r = authorize_tool(
            &ActiveProfile::dev(),
            &full_boot(),
            CallContext::LOCAL_WRITE,
            "vox_does_not_exist",
        );
        assert_eq!(r, Err(ProfileDenial::ToolNotInProfile));
    }

    #[test]
    fn step1_precedes_step2() {
        let internal = find(|e| e.exposure == Exposure::Internal, "internal");
        let r = authorize_tool(
            &profile_with(&[]),
            &BootReport::default(),
            READ_REMOTE,
            internal.name,
        );
        assert_eq!(r, Err(ProfileDenial::ToolNotInProfile));
    }

    #[test]
    fn step2_internal_off_loopback_is_exposure_denied_before_role_checks() {
        let internal = find(
            |e| e.exposure == Exposure::Internal && !e.http_read_role_eligible,
            "internal non-read-eligible",
        );
        let p = profile_with(&[internal.name]);
        assert_eq!(
            authorize_tool(&p, &BootReport::default(), WRITE_REMOTE, internal.name),
            Err(ProfileDenial::ExposureDenied)
        );
        assert_eq!(
            authorize_tool(&p, &BootReport::default(), READ_REMOTE, internal.name),
            Err(ProfileDenial::ExposureDenied)
        );
    }

    #[test]
    fn internal_on_loopback_is_allowed_for_write() {
        let internal = find(|e| e.exposure == Exposure::Internal, "internal");
        let p = profile_with(&[internal.name]);
        assert!(authorize_tool(&p, &full_boot(), CallContext::LOCAL_WRITE, internal.name).is_ok());
    }

    #[test]
    fn step3_operator_with_read_role_is_exposure_denied_before_step4() {
        let op = find(
            |e| e.exposure == Exposure::Operator && !e.http_read_role_eligible,
            "operator non-read-eligible",
        );
        let p = profile_with(&[op.name]);
        assert_eq!(
            authorize_tool(&p, &BootReport::default(), READ_LOCAL, op.name),
            Err(ProfileDenial::ExposureDenied)
        );
    }

    #[test]
    fn step4_public_not_read_eligible_is_insufficient_role_before_step5() {
        let e = find(
            |e| {
                e.exposure == Exposure::Public
                    && !e.http_read_role_eligible
                    && !e.requires.is_empty()
            },
            "public, non-read-eligible, with requires",
        );
        let p = profile_with(&[e.name]);
        assert_eq!(
            authorize_tool(&p, &BootReport::default(), READ_LOCAL, e.name),
            Err(ProfileDenial::InsufficientRole)
        );
    }

    #[test]
    fn step5_first_missing_subsystem_is_reported() {
        let e = find(|e| !e.requires.is_empty(), "with requires");
        let p = profile_with(&[e.name]);
        assert_eq!(
            authorize_tool(&p, &BootReport::default(), CallContext::LOCAL_WRITE, e.name),
            Err(ProfileDenial::SubsystemNotBooted(e.requires[0]))
        );
    }

    #[test]
    fn down_dependency_of_an_up_requirement_is_not_booted() {
        let e = registry_entry("vox_kb_search").expect("vox_kb_search");
        assert!(
            subsystem_closure(e.requires).contains(&Subsystem::Db),
            "vox_kb_search requires must close over db"
        );
        let boot = BootReport {
            booted: subsystem_closure(e.requires)
                .into_iter()
                .filter(|s| *s != Subsystem::Db)
                .collect(),
            failed: [(Subsystem::Db, "down".to_string())].into_iter().collect(),
        };
        assert_eq!(
            authorize_tool(
                &ActiveProfile::dev(),
                &boot,
                CallContext::LOCAL_WRITE,
                e.name
            ),
            Err(ProfileDenial::SubsystemNotBooted(Subsystem::Db))
        );
    }

    #[test]
    fn all_checks_pass_returns_the_registry_entry() {
        let e = find(
            |e| e.exposure == Exposure::Public && e.http_read_role_eligible,
            "public read-eligible",
        );
        let p = profile_with(&[e.name]);
        let got = authorize_tool(&p, &full_boot(), READ_REMOTE, e.name).expect("authorized");
        assert_eq!(got.name, e.name);
    }

    #[test]
    fn local_write_constant_is_write_on_loopback() {
        assert_eq!(
            CallContext::LOCAL_WRITE,
            CallContext {
                role: CallerRole::Write,
                loopback: true,
            }
        );
    }

    /// Controller: browser tools are never served to a read-role caller.
    #[test]
    fn read_role_is_denied_every_browser_tool() {
        let boot = full_boot();
        let browser: Vec<_> = TOOL_REGISTRY
            .iter()
            .filter(|e| e.name.starts_with("vox_browser_"))
            .collect();
        assert!(!browser.is_empty(), "registry has no browser tools");
        for e in browser {
            let p = profile_with(&[e.name]);
            for ctx in [READ_REMOTE, READ_LOCAL] {
                let err = authorize_tool(&p, &boot, ctx, e.name).expect_err(e.name);
                assert!(
                    matches!(
                        err,
                        ProfileDenial::ExposureDenied | ProfileDenial::InsufficientRole
                    ),
                    "{} served to read role: {err:?}",
                    e.name
                );
            }
        }
    }

    /// Controller: these two stay off the read-role allowlist.
    #[test]
    fn skill_use_and_search_structural_are_not_read_role_eligible() {
        for name in ["vox_skill_use", "vox_search_structural"] {
            let e = find(|e| e.name == name, name);
            assert!(
                !e.http_read_role_eligible,
                "{name} must not be http_read_role_eligible"
            );
            let p = profile_with(&[e.name]);
            assert_eq!(
                authorize_tool(&p, &full_boot(), READ_LOCAL, e.name),
                Err(ProfileDenial::InsufficientRole)
            );
        }
    }
}
