use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrivePlane {
    Live,
    Headless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DriveExecution {
    Sync,
    Background,
    Plan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveErrorCode {
    UnknownKey,
    EmptyText,
    ModelNotSelectable,
    Unauthorized,
    SendInFlight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveApplyError {
    pub code: DriveErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriveSet {
    pub model_override: Option<String>,
    pub pin: Option<String>,
    pub pin_policy: Option<String>,
    pub execution: Option<DriveExecution>,
    pub tier: Option<String>,
    pub clutch: Option<String>,
    pub risk: Option<String>,
    pub grounding_check_enabled: Option<bool>,
    pub active_skill: Option<Option<String>>,
    pub skill_exclusions: Option<Vec<String>>,
    pub session_id: Option<String>,
    pub chat_session_id: Option<String>,
    pub priority: Option<String>,
    pub dry_run: Option<bool>,
    pub allow_duplicate: Option<bool>,
    pub mode: Option<String>,
    pub context_files: Option<Vec<String>>,
    pub refresh_catalog: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriveKnobs {
    pub model_override: Option<String>,
    pub pin_policy: Option<String>,
    pub execution: Option<DriveExecution>,
    pub tier: Option<String>,
    pub clutch: Option<String>,
    pub risk: Option<String>,
    pub grounding_check_enabled: Option<bool>,
    pub active_skill: Option<String>,
    pub skill_exclusions: Vec<String>,
    pub session_id: Option<String>,
    pub chat_session_id: Option<String>,
    pub priority: Option<String>,
    pub dry_run: Option<bool>,
    pub allow_duplicate: Option<bool>,
    pub mode: Option<String>,
    pub context_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveCatalogRow {
    pub id: String,
    pub selectable: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveProbe {
    pub reachable: bool,
    pub base_url: Option<String>,
    pub service: Option<String>,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveClaims {
    pub picker_ui: bool,
    pub composer_knobs: bool,
    pub bubbles: bool,
}

impl DriveClaims {
    pub fn live() -> Self {
        Self {
            picker_ui: true,
            composer_knobs: true,
            bubbles: true,
        }
    }

    pub fn headless() -> Self {
        Self {
            picker_ui: false,
            composer_knobs: false,
            bubbles: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveState {
    pub plane: DrivePlane,
    pub knobs: DriveKnobs,
    pub pin: Option<String>,
    pub catalog: Vec<DriveCatalogRow>,
    pub probe: DriveProbe,
    pub bubbles: Vec<serde_json::Value>,
    pub last_error: Option<String>,
    pub orch_fresh: bool,
    pub claims: DriveClaims,
}

impl DriveState {
    pub fn empty_live() -> Self {
        Self {
            plane: DrivePlane::Live,
            knobs: DriveKnobs::default(),
            pin: None,
            catalog: Vec::new(),
            probe: DriveProbe {
                reachable: false,
                base_url: None,
                service: None,
                models: Vec::new(),
            },
            bubbles: Vec::new(),
            last_error: None,
            orch_fresh: false,
            claims: DriveClaims::live(),
        }
    }

    pub fn empty_headless() -> Self {
        let mut state = Self::empty_live();
        state.plane = DrivePlane::Headless;
        state.claims = DriveClaims::headless();
        state
    }
}

#[allow(dead_code)]
pub const ALLOWED_SET_KEYS: &[&str] = &[
    "model_override",
    "pin",
    "pin_policy",
    "execution",
    "tier",
    "clutch",
    "risk",
    "grounding_check_enabled",
    "active_skill",
    "skill_exclusions",
    "session_id",
    "chat_session_id",
    "priority",
    "dry_run",
    "allow_duplicate",
    "mode",
    "context_files",
    "refresh_catalog",
];

pub fn apply_set(state: &mut DriveState, set: DriveSet) -> Result<(), DriveApplyError> {
    if !set.extra.is_empty() {
        let key = set.extra.keys().next().cloned().unwrap_or_default();
        return Err(DriveApplyError {
            code: DriveErrorCode::UnknownKey,
            message: format!("unknown_key:{key}"),
        });
    }
    let model = set.model_override.or(set.pin);
    if let Some(model) = model {
        state.knobs.model_override = Some(model.clone());
        state.pin = Some(model);
    }
    if let Some(policy) = set.pin_policy {
        state.knobs.pin_policy = Some(policy);
    }
    if let Some(execution) = set.execution {
        state.knobs.execution = Some(execution);
    }
    if let Some(tier) = set.tier {
        state.knobs.tier = Some(tier);
    }
    if let Some(clutch) = set.clutch {
        state.knobs.clutch = Some(clutch);
    }
    if let Some(risk) = set.risk {
        state.knobs.risk = Some(risk);
    }
    if let Some(v) = set.grounding_check_enabled {
        state.knobs.grounding_check_enabled = Some(v);
    }
    if let Some(v) = set.active_skill {
        state.knobs.active_skill = v;
    }
    if let Some(v) = set.skill_exclusions {
        state.knobs.skill_exclusions = v;
    }
    if let Some(v) = set.session_id {
        state.knobs.session_id = Some(v);
    }
    if let Some(v) = set.chat_session_id {
        state.knobs.chat_session_id = Some(v);
    }
    if let Some(v) = set.priority {
        state.knobs.priority = Some(v);
    }
    if let Some(v) = set.dry_run {
        state.knobs.dry_run = Some(v);
    }
    if let Some(v) = set.allow_duplicate {
        state.knobs.allow_duplicate = Some(v);
    }
    if let Some(v) = set.mode {
        state.knobs.mode = Some(v);
    }
    if let Some(v) = set.context_files {
        state.knobs.context_files = v;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[cfg_attr(not(test), allow(dead_code))]
struct AxisDriveContract {
    version: u32,
    name: String,
    listener_verbs: Vec<String>,
    cli_only_verbs: Vec<String>,
    planes: Vec<String>,
    bind: String,
    health_service: String,
    set_keys: BTreeMap<String, serde_yaml::Value>,
    errors: BTreeMap<String, u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_contract() -> AxisDriveContract {
        serde_yaml::from_str(include_str!("../../../../contracts/gui/axis-drive.v1.yaml"))
            .expect("axis-drive.v1.yaml must deserialize")
    }

    #[test]
    fn contract_yaml_typed_verbs_and_keys() {
        let contract = load_contract();
        assert_eq!(contract.version, 1);
        assert_eq!(contract.name, "axis-drive");
        assert_eq!(contract.bind, "127.0.0.1");
        assert_eq!(contract.health_service, "vox-gui-drive");
        for verb in ["health", "ping", "set", "send", "state", "show"] {
            assert!(
                contract.listener_verbs.iter().any(|v| v == verb),
                "missing listener verb {verb}"
            );
        }
        for verb in ["wait", "start", "stop", "headless"] {
            assert!(
                contract.cli_only_verbs.iter().any(|v| v == verb),
                "missing cli-only verb {verb}"
            );
        }
        assert!(contract.planes.iter().any(|p| p == "live"));
        assert!(contract.planes.iter().any(|p| p == "headless"));
        for key in [
            "model_override",
            "pin_policy",
            "execution",
            "clutch",
            "risk",
        ] {
            assert!(contract.set_keys.contains_key(key), "missing set key {key}");
        }
        assert!(!contract.listener_verbs.iter().any(|v| v == "eval"));
        assert_eq!(contract.errors.get("unknown_key"), Some(&400));
        assert_eq!(contract.errors.get("model_not_selectable"), Some(&409));
        assert_eq!(contract.errors.get("unauthorized"), Some(&401));
    }

    #[test]
    fn clutch_risk_match_drive_console() {
        let contract = load_contract();
        let console: serde_yaml::Value = serde_yaml::from_str(include_str!(
            "../../../../contracts/gui/drive-console.v1.yaml"
        ))
        .expect("drive-console.v1.yaml");
        let clutch: Vec<String> = console["clutch"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_str().unwrap().to_string())
            .collect();
        let risk: Vec<String> = console["risk"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_str().unwrap().to_string())
            .collect();
        let drive_clutch: Vec<String> = contract.set_keys["clutch"]["values"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let drive_risk: Vec<String> = contract.set_keys["risk"]["values"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(drive_clutch, clutch);
        assert_eq!(drive_risk, risk);
    }

    #[test]
    fn unknown_set_key_is_rejected() {
        let mut state = DriveState::empty_live();
        let set = DriveSet {
            extra: [("nope".into(), serde_json::json!(true))]
                .into_iter()
                .collect(),
            ..DriveSet::default()
        };
        let err = apply_set(&mut state, set).expect_err("unknown key");
        assert_eq!(err.code, DriveErrorCode::UnknownKey);
    }

    #[test]
    fn set_model_and_execution_round_trip() {
        let mut state = DriveState::empty_live();
        apply_set(
            &mut state,
            DriveSet {
                model_override: Some("mens/e2e-smoke".into()),
                execution: Some(DriveExecution::Sync),
                ..DriveSet::default()
            },
        )
        .unwrap();
        assert_eq!(
            state.knobs.model_override.as_deref(),
            Some("mens/e2e-smoke")
        );
        assert_eq!(state.knobs.execution, Some(DriveExecution::Sync));
        assert_eq!(state.plane, DrivePlane::Live);
    }
}
