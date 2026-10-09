//! Subsystem boot outcome and the typed denial shared by stdio MCP and HTTP.

use std::collections::{BTreeMap, BTreeSet};

use crate::Subsystem;

/// Which subsystems the daemon actually brought up. Plan 03 fills this from the conditioned boot;
/// until then hosts use an "everything booted" report.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BootReport {
    pub booted: BTreeSet<Subsystem>,
    pub failed: BTreeMap<Subsystem, String>,
}

impl BootReport {
    pub fn is_up(&self, s: Subsystem) -> bool {
        self.booted.contains(&s)
    }

    /// First requirement (in `requires` order) that is not up.
    pub fn missing_for(&self, requires: &[Subsystem]) -> Option<Subsystem> {
        requires.iter().copied().find(|s| !self.is_up(*s))
    }
}

/// Typed denial, shared by stdio MCP and HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileDenial {
    ToolNotInProfile,
    SubsystemNotBooted(Subsystem),
    ExposureDenied,
    InsufficientRole,
}

impl ProfileDenial {
    pub const fn code(&self) -> &'static str {
        match self {
            ProfileDenial::ToolNotInProfile => "tool_not_in_profile",
            ProfileDenial::SubsystemNotBooted(_) => "subsystem_not_booted",
            ProfileDenial::ExposureDenied => "exposure_denied",
            ProfileDenial::InsufficientRole => "insufficient_role",
        }
    }

    pub const fn http_status(&self) -> u16 {
        match self {
            ProfileDenial::ToolNotInProfile => 404,
            ProfileDenial::SubsystemNotBooted(_) => 503,
            ProfileDenial::ExposureDenied | ProfileDenial::InsufficientRole => 403,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Subsystem;

    #[test]
    fn default_report_has_nothing_up() {
        let r = BootReport::default();
        assert!(!r.is_up(Subsystem::Db));
        assert_eq!(r.missing_for(&[]), None);
        assert_eq!(r.missing_for(&[Subsystem::Db]), Some(Subsystem::Db));
    }

    #[test]
    fn missing_for_returns_first_not_up_in_requires_order() {
        let r = BootReport {
            booted: [Subsystem::Db].into_iter().collect(),
            failed: Default::default(),
        };
        assert!(r.is_up(Subsystem::Db));
        assert_eq!(
            r.missing_for(&[Subsystem::Db, Subsystem::SearchIndex, Subsystem::Memory]),
            Some(Subsystem::SearchIndex)
        );
        assert_eq!(r.missing_for(&[Subsystem::Db]), None);
    }

    #[test]
    fn failed_subsystem_is_not_up() {
        let r = BootReport {
            booted: Default::default(),
            failed: [(Subsystem::Db, "connect refused".to_string())]
                .into_iter()
                .collect(),
        };
        assert!(!r.is_up(Subsystem::Db));
    }

    #[test]
    fn report_serializes_with_snake_case_subsystem_ids() {
        let r = BootReport {
            booted: [Subsystem::LlmFacade, Subsystem::ModelRegistry]
                .into_iter()
                .collect(),
            failed: [(Subsystem::Db, "x".to_string())].into_iter().collect(),
        };
        let v = serde_json::to_value(&r).expect("serialize");
        assert_eq!(
            v["booted"],
            serde_json::json!(["model_registry", "llm_facade"])
        );
        assert_eq!(v["failed"]["db"], "x");
    }

    /// Amendment "From plan 03": stdio `vox mcp` deserializes the daemon's `orch.boot_report`.
    #[test]
    fn report_round_trips_through_json() {
        let r = BootReport {
            booted: [Subsystem::LlmFacade].into_iter().collect(),
            failed: [(Subsystem::Db, "no store".to_string())]
                .into_iter()
                .collect(),
        };
        let back: BootReport =
            serde_json::from_value(serde_json::to_value(&r).expect("ser")).expect("de");
        assert_eq!(back, r);
    }

    #[test]
    fn denial_codes_and_statuses_match_the_contract() {
        let cases = [
            (
                ProfileDenial::ToolNotInProfile,
                "tool_not_in_profile",
                404u16,
            ),
            (
                ProfileDenial::SubsystemNotBooted(Subsystem::SearchIndex),
                "subsystem_not_booted",
                503,
            ),
            (ProfileDenial::ExposureDenied, "exposure_denied", 403),
            (ProfileDenial::InsufficientRole, "insufficient_role", 403),
        ];
        for (d, code, status) in cases {
            assert_eq!(d.code(), code);
            assert_eq!(d.http_status(), status);
        }
    }
}
