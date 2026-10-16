//! Unified Autonomy Oversight Resolver, Session Registry, Background/Chat Consumers & Attention Inbox
//! (Phases 13–17: `OVR-01..06`, `STATE-01..02`, `FLOOR-04..04b`, `BG-01..02b`, `CHAT-01..03`, `ATTN-01..06`).

use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::attention::ApprovalTier;
use crate::mode::ClutchProfile;

/// Canonical 5-stop Oversight scale (`1..=5`, strictly monotonic in safety: more = safer).
/// Maps 3-stop UI (`Relaxed`, `Standard`, `Strict`) and 5-stop CLI/daemon (`Relaxed`, `Trusted`,
/// `Standard`, `Cautious`, `Strict`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum OversightLevel {
    /// Ordinal 1: Auto-approves chat file edits (`AcceptEdits`), advisory grounding.
    Relaxed = 1,
    /// Ordinal 2: Auto-approves chat file edits (`AcceptEdits`), enforces background grounding.
    Trusted = 2,
    /// Ordinal 3 (Default): Asks on chat file edits (`Ask`), enforces grounding, daemon reply check.
    #[default]
    Standard = 3,
    /// Ordinal 4: Review floor in inbox, Socrates enforced, bounded self-check.
    Cautious = 4,
    /// Ordinal 5: Strictest; disables trust auto-approve and surfaces all prompts.
    Strict = 5,
}

impl OversightLevel {
    pub const ALL: [OversightLevel; 5] = [
        Self::Relaxed,
        Self::Trusted,
        Self::Standard,
        Self::Cautious,
        Self::Strict,
    ];

    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Relaxed => "relaxed",
            Self::Trusted => "trusted",
            Self::Standard => "standard",
            Self::Cautious => "cautious",
            Self::Strict => "strict",
        }
    }

    #[must_use]
    pub const fn ordinal(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub fn from_id(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "relaxed" | "1" | "autonomous" | "light" => Some(Self::Relaxed),
            "trusted" | "2" => Some(Self::Trusted),
            "standard" | "3" | "balanced" => Some(Self::Standard),
            "cautious" | "4" | "close" => Some(Self::Cautious),
            "strict" | "5" | "supervised" => Some(Self::Strict),
            _ => None,
        }
    }
}

/// Tool auto-approval policy emitted by the pure resolver (`OVR-03`).
/// Deliberately excludes `AcceptAll` so no oversight level ever disables the safety floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAutoApprove {
    AcceptEdits = 1,
    Ask = 2,
}

/// Grounding enforcement level (`OVR-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundingEnforcement {
    Advisory = 1,
    Enforce = 2,
}

/// Socrates gate enforcement level (`OVR-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocratesEnforcement {
    Shadow = 1,
    Enforce = 2,
}

/// Human-readable effect row generated from the same resolver table as `EffectivePolicy` (`OVR-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EffectRow {
    pub id: &'static str,
    pub label: &'static str,
    pub value: &'static str,
    pub consumer: &'static str,
}

/// Effective autonomy policy resolved for a session or task (`OVR-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EffectivePolicy {
    pub level: OversightLevel,
    pub approval_floor: ApprovalTier,
    pub trust_auto_approve: bool,
    pub trust_relax_gates: bool,
    pub tool_auto_approve: ToolAutoApprove,
    pub grounding: GroundingEnforcement,
    pub socrates: SocratesEnforcement,
    pub reply_verification: bool,
    pub admission_offset_bps: i32,
    pub per_agent_quota_15m: u32,
    pub effects: Vec<EffectRow>,
    pub hard_floors: Vec<&'static str>,
}

/// Pure resolver mapping an `OversightLevel` to its `EffectivePolicy` and human-facing `EffectRow`s (`OVR-01..03`).
#[must_use]
pub fn resolve(level: OversightLevel) -> EffectivePolicy {
    let hard_floors = vec![
        "force_push_history_rewrite",
        "allowlist_policy_grant",
        "deploy_external_publish",
        "secrets_write",
    ];
    match level {
        OversightLevel::Relaxed => EffectivePolicy {
            level,
            approval_floor: ApprovalTier::AutoApprove,
            trust_auto_approve: true,
            trust_relax_gates: true,
            tool_auto_approve: ToolAutoApprove::AcceptEdits,
            grounding: GroundingEnforcement::Advisory,
            socrates: SocratesEnforcement::Shadow,
            reply_verification: false,
            admission_offset_bps: 1000,
            per_agent_quota_15m: 2,
            effects: vec![
                EffectRow {
                    id: "chat_file_edits",
                    label: "Chat file edits",
                    value: "Auto-approved (AcceptEdits)",
                    consumer: "sync_chat",
                },
                EffectRow {
                    id: "background_gate",
                    label: "Background task approval floor",
                    value: "AutoApprove (trust graduation active)",
                    consumer: "background_tasks",
                },
                EffectRow {
                    id: "reply_check",
                    label: "Daemon reply verification",
                    value: "Advisory only",
                    consumer: "sync_chat",
                },
            ],
            hard_floors,
        },
        OversightLevel::Trusted => EffectivePolicy {
            level,
            approval_floor: ApprovalTier::Confirm,
            trust_auto_approve: true,
            trust_relax_gates: true,
            tool_auto_approve: ToolAutoApprove::AcceptEdits,
            grounding: GroundingEnforcement::Enforce,
            socrates: SocratesEnforcement::Shadow,
            reply_verification: false,
            admission_offset_bps: 500,
            per_agent_quota_15m: 3,
            effects: vec![
                EffectRow {
                    id: "chat_file_edits",
                    label: "Chat file edits",
                    value: "Auto-approved (AcceptEdits)",
                    consumer: "sync_chat",
                },
                EffectRow {
                    id: "background_gate",
                    label: "Background task approval floor",
                    value: "Confirm (grounding enforced)",
                    consumer: "background_tasks",
                },
                EffectRow {
                    id: "reply_check",
                    label: "Daemon reply verification",
                    value: "Advisory only",
                    consumer: "sync_chat",
                },
            ],
            hard_floors,
        },
        OversightLevel::Standard => EffectivePolicy {
            level,
            approval_floor: ApprovalTier::Confirm,
            trust_auto_approve: true,
            trust_relax_gates: true,
            tool_auto_approve: ToolAutoApprove::Ask,
            grounding: GroundingEnforcement::Enforce,
            socrates: SocratesEnforcement::Shadow,
            reply_verification: true,
            admission_offset_bps: 0,
            per_agent_quota_15m: 4,
            effects: vec![
                EffectRow {
                    id: "chat_file_edits",
                    label: "Chat file edits",
                    value: "Asks before writing (Ask)",
                    consumer: "sync_chat",
                },
                EffectRow {
                    id: "background_gate",
                    label: "Background task approval floor",
                    value: "Confirm (grounding enforced)",
                    consumer: "background_tasks",
                },
                EffectRow {
                    id: "reply_check",
                    label: "Daemon reply verification",
                    value: "Enabled (non-blocking badge)",
                    consumer: "sync_chat",
                },
            ],
            hard_floors,
        },
        OversightLevel::Cautious => EffectivePolicy {
            level,
            approval_floor: ApprovalTier::Review,
            trust_auto_approve: true,
            trust_relax_gates: false,
            tool_auto_approve: ToolAutoApprove::Ask,
            grounding: GroundingEnforcement::Enforce,
            socrates: SocratesEnforcement::Enforce,
            reply_verification: true,
            admission_offset_bps: -500,
            per_agent_quota_15m: 8,
            effects: vec![
                EffectRow {
                    id: "chat_file_edits",
                    label: "Chat file edits",
                    value: "Asks before writing (Ask)",
                    consumer: "sync_chat",
                },
                EffectRow {
                    id: "background_gate",
                    label: "Background task approval floor",
                    value: "Review in inbox (Socrates enforced)",
                    consumer: "background_tasks",
                },
                EffectRow {
                    id: "reply_check",
                    label: "Daemon reply verification",
                    value: "Enabled + bounded self-check",
                    consumer: "sync_chat",
                },
            ],
            hard_floors,
        },
        OversightLevel::Strict => EffectivePolicy {
            level,
            approval_floor: ApprovalTier::Review,
            trust_auto_approve: false,
            trust_relax_gates: false,
            tool_auto_approve: ToolAutoApprove::Ask,
            grounding: GroundingEnforcement::Enforce,
            socrates: SocratesEnforcement::Enforce,
            reply_verification: true,
            admission_offset_bps: -10000,
            per_agent_quota_15m: 1000,
            effects: vec![
                EffectRow {
                    id: "chat_file_edits",
                    label: "Chat file edits",
                    value: "Asks before writing (Ask)",
                    consumer: "sync_chat",
                },
                EffectRow {
                    id: "background_gate",
                    label: "Background task approval floor",
                    value: "Review in inbox (no trust auto-approve)",
                    consumer: "background_tasks",
                },
                EffectRow {
                    id: "reply_check",
                    label: "Daemon reply verification",
                    value: "Strict daemon verification",
                    consumer: "sync_chat",
                },
            ],
            hard_floors,
        },
    }
}

/// Resolve effective autonomy combining session level, default level, and escalate-only hint (`OVR-03`).
#[must_use]
pub fn resolve_effective_autonomy(
    session_level: Option<OversightLevel>,
    default_level: OversightLevel,
    hint_escalation: Option<OversightLevel>,
) -> EffectivePolicy {
    let base = session_level.unwrap_or(default_level);
    let effective = match hint_escalation {
        Some(hint) => base.max(hint),
        None => base,
    };
    resolve(effective)
}

/// Decoupled Mode axis resolver (`OVR-04`): choosing any `OversightLevel` leaves model selection axes unchanged.
#[must_use]
pub fn effective_mode_axes(clutch: ClutchProfile, _oversight: OversightLevel) -> (u8, u8, u8) {
    clutch.resolve().axes
}

/// One-way migration from legacy inverted `risk` (`high|moderate|low`) + `check_replies_on` (`OVR-05`, `STATE-03`).
// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="legacy inverted risk key" canonical="OversightLevel"
#[must_use]
pub fn migrate_legacy_risk(
    legacy_risk: &str,
    check_replies_on: bool,
) -> (OversightLevel, &'static str) {
    let mapped = match legacy_risk.trim().to_ascii_lowercase().as_str() {
        "high" => OversightLevel::Relaxed,
        "moderate" => OversightLevel::Standard,
        "low" => OversightLevel::Strict,
        _ => OversightLevel::Standard,
    };
    let final_level = if check_replies_on {
        mapped.max(OversightLevel::Standard)
    } else {
        mapped
    };
    (
        final_level,
        "DEPRECATED: `risk` is inverted and deprecated; use `oversight` (`relaxed|standard|strict`).",
    )
}

// ── Phase 14: Daemon Session State, Auth & Floored Park Guard ───────────────

/// Daemon-held per-session oversight registry (`STATE-01`, `FLOOR-04`, `FLOOR-04b`).
#[derive(Debug, Clone)]
pub struct OversightSessionRegistry {
    default_level: OversightLevel,
    sessions: BTreeMap<String, OversightLevel>,
}

impl Default for OversightSessionRegistry {
    fn default() -> Self {
        Self {
            default_level: OversightLevel::Standard,
            sessions: BTreeMap::new(),
        }
    }
}

impl OversightSessionRegistry {
    #[must_use]
    pub fn new(default_level: OversightLevel) -> Self {
        Self {
            default_level,
            sessions: BTreeMap::new(),
        }
    }

    /// Rehydrate from `user_preferences` (`oversight.default` / `oversight.session.<id>`).
    /// Missing or corrupt values fail-safe to `max(default, Standard)` (`STATE-01`).
    #[must_use]
    pub fn rehydrate_level(&self, raw: Option<&str>) -> OversightLevel {
        match raw.and_then(OversightLevel::from_id) {
            Some(lvl) => lvl,
            None => self.default_level.max(OversightLevel::Standard),
        }
    }

    #[must_use]
    pub fn get_session_level(&self, session_id: Option<&str>) -> OversightLevel {
        session_id
            .and_then(|id| self.sessions.get(id).copied())
            .unwrap_or(self.default_level)
    }

    #[must_use]
    pub fn get_session_policy(&self, session_id: Option<&str>) -> EffectivePolicy {
        resolve(self.get_session_level(session_id))
    }

    /// `orch.oversight_set` handler (`FLOOR-04`): lowering a level is permitted ONLY over the
    /// authenticated human transport (`authenticated_human_channel && !agent_origin`).
    /// Unauthenticated or agent-origin callers may only raise strictness.
    pub fn set_session_level(
        &mut self,
        session_id: Option<&str>,
        target: OversightLevel,
        authenticated_human_channel: bool,
        agent_origin: bool,
    ) -> Result<EffectivePolicy, &'static str> {
        let current = self.get_session_level(session_id);
        let is_lowering = target < current;
        if is_lowering && (!authenticated_human_channel || agent_origin) {
            return Err("FLOOR-04: only an authenticated human RPC can lower oversight level");
        }
        match session_id {
            Some(id) => {
                self.sessions.insert(id.to_string(), target);
            }
            None => {
                self.default_level = target;
            }
        }
        Ok(resolve(target))
    }
}

/// Closes Phase 12 residual `T-12-25` (`FLOOR-04b`): only an authenticated human channel with
/// `agent_origin == false` can approve or modify a safety-floored park.
pub fn verify_floored_park_resolution(
    is_floored_park: bool,
    authenticated_human_channel: bool,
    agent_origin: Option<bool>,
) -> Result<(), &'static str> {
    if is_floored_park {
        if !authenticated_human_channel {
            return Err("FLOOR-04b: unauthenticated caller cannot resolve a floored park");
        }
        if agent_origin.unwrap_or(true) {
            return Err(
                "FLOOR-04b: agent-origin or missing-origin caller cannot resolve a floored park",
            );
        }
    }
    Ok(())
}

// ── Phase 15: Background Task Consumers (`BG-01`, `BG-02`, `BG-02b`, `STATE-02`) ──

/// Oversight metadata stamped on a background task at enqueue time (`STATE-02`, `BG-02b`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundTaskOversightStamp {
    pub stamped_level: OversightLevel,
    pub is_floor_lifted: bool,
}

/// Resolved gate policy for a background task (`BG-01`, `BG-02`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveBackgroundGate {
    pub effective_level: OversightLevel,
    pub approval_tier: ApprovalTier,
    pub grounding: GroundingEnforcement,
    pub socrates: SocratesEnforcement,
    pub trust_relax_gates: bool,
    pub routes_to_approval_inbox: bool,
}

/// Resolves background task gate policy (`BG-01`, `BG-02`, `STATE-02`):
/// tightening mid-turn applies immediately (`max(stamped, current)`), loosening applies only to new tasks.
#[must_use]
pub fn resolve_background_gate(
    stamp: BackgroundTaskOversightStamp,
    current_session_level: OversightLevel,
    floor_tier: ApprovalTier,
    trust_graduated_auto_approve: bool,
) -> EffectiveBackgroundGate {
    let effective_level = stamp.stamped_level.max(current_session_level);
    let policy = resolve(effective_level);

    let base_tier = if trust_graduated_auto_approve && policy.trust_auto_approve {
        ApprovalTier::AutoApprove
    } else if trust_graduated_auto_approve && !policy.trust_auto_approve {
        ApprovalTier::Confirm
    } else {
        policy.approval_floor
    };

    let approval_tier = floor_tier
        .max_strictness(policy.approval_floor)
        .max_strictness(base_tier);

    let routes_to_approval_inbox =
        matches!(approval_tier, ApprovalTier::Review | ApprovalTier::Blocked);

    EffectiveBackgroundGate {
        effective_level,
        approval_tier,
        grounding: policy.grounding,
        socrates: policy.socrates,
        trust_relax_gates: policy.trust_relax_gates && !stamp.is_floor_lifted,
        routes_to_approval_inbox,
    }
}

/// Closes Phase 12 residual `T-12-23` (`BG-02b`): a floor-lifted Review task cannot be satisfied
/// by agent self-attestation (`checks_passed`); only a human inbox resolution can complete it.
#[must_use]
pub fn can_complete_background_review(
    stamp: BackgroundTaskOversightStamp,
    agent_self_attested_checks_passed: bool,
    human_inbox_resolved: bool,
) -> bool {
    if stamp.is_floor_lifted {
        human_inbox_resolved
    } else {
        human_inbox_resolved || agent_self_attested_checks_passed
    }
}

// ── Phase 16: Sync Chat Consumers (`CHAT-01`, `CHAT-02`, `CHAT-03`) ─────────

/// Outcome of resolving `OversightLevel` for a synchronous chat turn (`CHAT-01..03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncChatOversightOutcome {
    pub effective_level: OversightLevel,
    pub tool_auto_approve: ToolAutoApprove,
    pub reply_confidence_check: bool,
    pub requires_human_approval: bool,
    pub event_kind: &'static str,
}

/// Resolves synchronous chat tool dispatch & daemon reply verification (`CHAT-01..03`).
#[must_use]
pub fn resolve_sync_chat_turn(
    session_level: OversightLevel,
    transport_mode: Option<ToolAutoApprove>,
    is_hard_floor_action: bool,
    is_file_edit: bool,
) -> SyncChatOversightOutcome {
    let policy = resolve(session_level);
    let tool_auto_approve = match transport_mode {
        Some(m) => policy.tool_auto_approve.max(m),
        None => policy.tool_auto_approve,
    };
    let requires_human_approval =
        is_hard_floor_action || (is_file_edit && tool_auto_approve == ToolAutoApprove::Ask);

    SyncChatOversightOutcome {
        effective_level: session_level,
        tool_auto_approve,
        reply_confidence_check: policy.reply_verification,
        requires_human_approval,
        event_kind: "oversight_applied",
    }
}

// ── Phase 17: Cross-Agent Attention Admission & Inbox (`ATTN-01..04`, `ATTN-06`) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxSurface {
    NeedsYou,
    WithheldDigest,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WhyReachedYou {
    pub level: OversightLevel,
    pub action_class: String,
    pub trust_score: f32,
    pub floor_or_tier: String,
    pub originating_agent: String,
    pub reason_code: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmissionCandidate {
    pub item_id: String,
    pub agent_id: String,
    pub action_class: String,
    pub is_hard_floor: bool,
    pub is_blocking: bool,
    pub utility_score_bps: i32,
    pub trust_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AdmissionVerdict {
    pub item_id: String,
    pub surface: InboxSurface,
    pub why: WhyReachedYou,
}

/// Unified cross-agent Attention Inbox with fair-share quotas, aging, digest batching,
/// and anti-rubber-stamp trust protection (`ATTN-01..04`, `ATTN-06`).
#[derive(Debug, Clone, Default)]
pub struct AttentionInbox {
    pub needs_you: VecDeque<AdmissionVerdict>,
    pub withheld_digest: VecDeque<AdmissionVerdict>,
    per_agent_counts_15m: BTreeMap<String, u32>,
}

impl AttentionInbox {
    /// Admit a prompt candidate (`ATTN-01..04`):
    /// 1. Hard floors always surface (`NeedsYou`), bypassing quota and thresholds.
    /// 2. Blocking approvals (`ToolApproval`, `TaskReview`) are never deferred (`NeedsYou`).
    /// 3. Non-blocking items check per-agent fair-share quota (`per_agent_quota_15m`) and level threshold.
    pub fn admit(
        &mut self,
        candidate: AdmissionCandidate,
        policy: &EffectivePolicy,
    ) -> AdmissionVerdict {
        if candidate.is_hard_floor {
            let verdict = AdmissionVerdict {
                item_id: candidate.item_id,
                surface: InboxSurface::NeedsYou,
                why: WhyReachedYou {
                    level: policy.level,
                    action_class: candidate.action_class,
                    trust_score: candidate.trust_score,
                    floor_or_tier: "hard_floor".to_string(),
                    originating_agent: candidate.agent_id,
                    reason_code: "hard_floor_always_surfaces",
                },
            };
            self.needs_you.push_back(verdict.clone());
            return verdict;
        }

        if candidate.is_blocking {
            let verdict = AdmissionVerdict {
                item_id: candidate.item_id,
                surface: InboxSurface::NeedsYou,
                why: WhyReachedYou {
                    level: policy.level,
                    action_class: candidate.action_class,
                    trust_score: candidate.trust_score,
                    floor_or_tier: format!("{:?}", policy.approval_floor),
                    originating_agent: candidate.agent_id,
                    reason_code: "blocking_approval_never_deferred",
                },
            };
            self.needs_you.push_back(verdict.clone());
            return verdict;
        }

        let count = self
            .per_agent_counts_15m
            .entry(candidate.agent_id.clone())
            .or_insert(0);
        let threshold_bps = 5000 + policy.admission_offset_bps;
        if *count < policy.per_agent_quota_15m && candidate.utility_score_bps >= threshold_bps {
            *count += 1;
            let verdict = AdmissionVerdict {
                item_id: candidate.item_id,
                surface: InboxSurface::NeedsYou,
                why: WhyReachedYou {
                    level: policy.level,
                    action_class: candidate.action_class,
                    trust_score: candidate.trust_score,
                    floor_or_tier: "utility_above_threshold".to_string(),
                    originating_agent: candidate.agent_id,
                    reason_code: "admitted_within_agent_quota",
                },
            };
            self.needs_you.push_back(verdict.clone());
            verdict
        } else {
            let reason_code = if *count >= policy.per_agent_quota_15m {
                "withheld_agent_quota_exceeded"
            } else {
                "withheld_below_level_threshold"
            };
            let verdict = AdmissionVerdict {
                item_id: candidate.item_id,
                surface: InboxSurface::WithheldDigest,
                why: WhyReachedYou {
                    level: policy.level,
                    action_class: candidate.action_class,
                    trust_score: candidate.trust_score,
                    floor_or_tier: "digest".to_string(),
                    originating_agent: candidate.agent_id,
                    reason_code,
                },
            };
            self.withheld_digest.push_back(verdict.clone());
            verdict
        }
    }

    /// Promote withheld items into a single digest batch at natural breaks (`ATTN-04`).
    pub fn promote_withheld_digest(&mut self) -> Vec<AdmissionVerdict> {
        self.withheld_digest.drain(..).collect()
    }

    /// Age per-agent interruption counters so a previously chatty agent recovers fair share over time (`ATTN-03`).
    pub fn age_quotas(&mut self) {
        for count in self.per_agent_counts_15m.values_mut() {
            *count = count.saturating_sub(1);
        }
    }
}

/// Anti-rubber-stamp trust filter (`ATTN-06`): rapid repeated approvals (`< 1500 ms` dwell time)
/// do not raise agent trust score.
#[must_use]
pub fn should_credit_trust_on_approval(
    approval_dwell_ms: u64,
    consecutive_fast_approvals: u32,
) -> bool {
    approval_dwell_ms >= 1500 && consecutive_fast_approvals == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversight_contract_yaml_parity_and_monotonic_strictness() {
        let yaml_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("contracts/orchestration/oversight-levels.v1.yaml");
        let text = std::fs::read_to_string(&yaml_path).expect("read oversight-levels.v1.yaml");
        let doc: serde_yaml::Value = serde_yaml::from_str(&text).expect("parse yaml");

        assert_eq!(
            doc["invariants"]["strictness_monotonic"].as_bool(),
            Some(true)
        );
        assert_eq!(
            doc["invariants"]["modifies_mode_axes"].as_bool(),
            Some(false)
        );
        assert_eq!(
            doc["invariants"]["allows_accept_all"].as_bool(),
            Some(false)
        );

        let levels = doc["levels"].as_sequence().expect("levels sequence");
        assert_eq!(levels.len(), OversightLevel::ALL.len());

        let mut prev_floor = ApprovalTier::AutoApprove;
        let mut prev_tool = ToolAutoApprove::AcceptEdits;
        for (idx, lvl) in OversightLevel::ALL.into_iter().enumerate() {
            let yaml_lvl = &levels[idx];
            assert_eq!(yaml_lvl["id"].as_str(), Some(lvl.id()));
            let pol = resolve(lvl);
            assert_eq!(
                pol.approval_floor.max_strictness(prev_floor),
                pol.approval_floor
            );
            assert!(pol.tool_auto_approve >= prev_tool);
            prev_floor = pol.approval_floor;
            prev_tool = pol.tool_auto_approve;

            let yaml_effects = yaml_lvl["effects"].as_sequence().expect("effects");
            assert_eq!(yaml_effects.len(), pol.effects.len());
            for (y_eff, r_eff) in yaml_effects.iter().zip(pol.effects.iter()) {
                assert_eq!(y_eff["id"].as_str(), Some(r_eff.id));
                assert_eq!(y_eff["value"].as_str(), Some(r_eff.value));
            }
        }
    }

    #[test]
    fn oversight_never_changes_mode_axes_and_migrates_legacy_risk() {
        for lvl in OversightLevel::ALL {
            assert_eq!(
                effective_mode_axes(ClutchProfile::Efficiency, lvl),
                ClutchProfile::Efficiency.resolve().axes
            );
        }
        let (lvl, warn) = migrate_legacy_risk("high", false);
        assert_eq!(lvl, OversightLevel::Relaxed);
        assert!(warn.contains("DEPRECATED"));

        let (lvl_lifted, _) = migrate_legacy_risk("high", true);
        assert_eq!(lvl_lifted, OversightLevel::Standard);
    }

    #[test]
    fn daemon_session_registry_enforces_authenticated_lowering_and_floored_park_guard() {
        let mut reg = OversightSessionRegistry::new(OversightLevel::Standard);
        assert_eq!(
            reg.rehydrate_level(Some("corrupt-value")),
            OversightLevel::Standard
        );

        // Agent or unauthenticated caller cannot lower from Standard to Relaxed
        assert!(
            reg.set_session_level(Some("s1"), OversightLevel::Relaxed, false, false)
                .is_err()
        );
        assert!(
            reg.set_session_level(Some("s1"), OversightLevel::Relaxed, true, true)
                .is_err()
        );

        // Agent CAN escalate from Standard to Strict
        assert!(
            reg.set_session_level(Some("s1"), OversightLevel::Strict, false, true)
                .is_ok()
        );
        assert_eq!(reg.get_session_level(Some("s1")), OversightLevel::Strict);

        // Authenticated human can lower to Relaxed
        assert!(
            reg.set_session_level(Some("s1"), OversightLevel::Relaxed, true, false)
                .is_ok()
        );
        assert_eq!(reg.get_session_level(Some("s1")), OversightLevel::Relaxed);

        // FLOOR-04b: floored park resolution
        assert!(verify_floored_park_resolution(true, false, Some(false)).is_err());
        assert!(verify_floored_park_resolution(true, true, None).is_err());
        assert!(verify_floored_park_resolution(true, true, Some(true)).is_err());
        assert!(verify_floored_park_resolution(true, true, Some(false)).is_ok());
    }

    #[test]
    fn background_and_sync_chat_consumers_maintain_parity_and_floor_guards() {
        // STATE-02: tightening mid-turn applies immediately; loosening leaves queued task at stamped level
        let stamp = BackgroundTaskOversightStamp {
            stamped_level: OversightLevel::Cautious,
            is_floor_lifted: true,
        };
        let gate_loosened =
            resolve_background_gate(stamp, OversightLevel::Relaxed, ApprovalTier::Review, true);
        assert_eq!(gate_loosened.effective_level, OversightLevel::Cautious);
        assert!(gate_loosened.routes_to_approval_inbox);

        // BG-02b: floor-lifted review cannot be satisfied by agent self-attestation
        assert!(!can_complete_background_review(stamp, true, false));
        assert!(can_complete_background_review(stamp, false, true));

        // CHAT-01..03: Relaxed auto-approves non-floored file edits; Standard/Strict ask; floor always asks
        let chat_relaxed = resolve_sync_chat_turn(OversightLevel::Relaxed, None, false, true);
        assert!(!chat_relaxed.requires_human_approval);
        assert!(!chat_relaxed.reply_confidence_check);

        let chat_standard = resolve_sync_chat_turn(OversightLevel::Standard, None, false, true);
        assert!(chat_standard.requires_human_approval);
        assert!(chat_standard.reply_confidence_check);

        let chat_floored = resolve_sync_chat_turn(OversightLevel::Relaxed, None, true, false);
        assert!(chat_floored.requires_human_approval);
    }

    #[test]
    fn attention_inbox_enforces_hard_floors_blocking_fair_share_and_rubber_stamp_guard() {
        let mut inbox = AttentionInbox::default();
        let policy = resolve(OversightLevel::Relaxed); // quota = 2 per 15m

        // 1. Hard floor always surfaces even with low utility
        let v_floor = inbox.admit(
            AdmissionCandidate {
                item_id: "floor-1".to_string(),
                agent_id: "agent-noisy".to_string(),
                action_class: "deploy".to_string(),
                is_hard_floor: true,
                is_blocking: false,
                utility_score_bps: 0,
                trust_score: 0.99,
            },
            &policy,
        );
        assert_eq!(v_floor.surface, InboxSurface::NeedsYou);

        // 2. Noisy agent exhausts its 2-item non-blocking quota
        for idx in 0..2 {
            let v = inbox.admit(
                AdmissionCandidate {
                    item_id: format!("n-{idx}"),
                    agent_id: "agent-noisy".to_string(),
                    action_class: "clarification".to_string(),
                    is_hard_floor: false,
                    is_blocking: false,
                    utility_score_bps: 8000,
                    trust_score: 0.5,
                },
                &policy,
            );
            assert_eq!(v.surface, InboxSurface::NeedsYou);
        }
        let v_over = inbox.admit(
            AdmissionCandidate {
                item_id: "n-over".to_string(),
                agent_id: "agent-noisy".to_string(),
                action_class: "clarification".to_string(),
                is_hard_floor: false,
                is_blocking: false,
                utility_score_bps: 8000,
                trust_score: 0.5,
            },
            &policy,
        );
        assert_eq!(v_over.surface, InboxSurface::WithheldDigest);

        // 3. Another agent still gets its fair share!
        let v_peer = inbox.admit(
            AdmissionCandidate {
                item_id: "peer-1".to_string(),
                agent_id: "agent-quiet".to_string(),
                action_class: "clarification".to_string(),
                is_hard_floor: false,
                is_blocking: false,
                utility_score_bps: 8000,
                trust_score: 0.5,
            },
            &policy,
        );
        assert_eq!(v_peer.surface, InboxSurface::NeedsYou);

        // 4. Digest flush returns withheld items
        let digest = inbox.promote_withheld_digest();
        assert_eq!(digest.len(), 1);
        assert_eq!(digest[0].item_id, "n-over");

        // 5. Rubber-stamp approvals (<1500ms) do not credit trust
        assert!(!should_credit_trust_on_approval(400, 1));
        assert!(should_credit_trust_on_approval(2200, 0));
    }

    #[test]
    fn autonomy_claims_ledger_covers_all_consequences_and_forbids_retired_claims() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let levels_text =
            std::fs::read_to_string(root.join("contracts/orchestration/oversight-levels.v1.yaml"))
                .expect("read oversight-levels.v1.yaml");
        let claims_text =
            std::fs::read_to_string(root.join("contracts/orchestration/autonomy-claims.v1.yaml"))
                .expect("read autonomy-claims.v1.yaml");
        let resolver_src =
            std::fs::read_to_string(root.join("crates/vox-orchestrator/src/oversight/resolver.rs"))
                .expect("read resolver.rs");
        let floor_src =
            std::fs::read_to_string(root.join("crates/vox-orchestrator/src/oversight/floor.rs"))
                .expect("read floor.rs");

        let levels_doc: serde_yaml::Value =
            serde_yaml::from_str(&levels_text).expect("parse oversight-levels.v1.yaml");
        let claims_doc: serde_yaml::Value =
            serde_yaml::from_str(&claims_text).expect("parse autonomy-claims.v1.yaml");

        // VERIF-02: retired claims must be absent from oversight-levels.v1.yaml and EffectRow strings
        let levels_lower = levels_text.to_ascii_lowercase();
        for retired in claims_doc["retired_claims"]
            .as_sequence()
            .expect("retired_claims")
        {
            for banned in retired["banned_substrings"]
                .as_sequence()
                .expect("banned_substrings")
            {
                let sub = banned.as_str().expect("string").to_ascii_lowercase();
                assert!(
                    !levels_lower.contains(&sub),
                    "Retired claim substring '{sub}' found in oversight-levels.v1.yaml"
                );
                for lvl in OversightLevel::ALL {
                    for eff in resolve(lvl).effects {
                        assert!(
                            !eff.value.to_ascii_lowercase().contains(&sub)
                                && !eff.label.to_ascii_lowercase().contains(&sub),
                            "Retired claim substring '{sub}' found in EffectRow for {:?}",
                            lvl
                        );
                    }
                }
            }
        }

        // VERIF-01: every hard floor in oversight-levels.v1.yaml has a backing claim & existing test
        let floor_claims = claims_doc["hard_floor_claims"]
            .as_sequence()
            .expect("hard_floor_claims");
        for floor in levels_doc["hard_floors"]
            .as_sequence()
            .expect("hard_floors")
        {
            let fid = floor["id"].as_str().expect("floor id");
            let flabel = floor["label"].as_str().expect("floor label");
            let matched = floor_claims
                .iter()
                .find(|c| {
                    c["floor_id"].as_str() == Some(fid) && c["label"].as_str() == Some(flabel)
                })
                .unwrap_or_else(|| panic!("Missing hard_floor_claim for {fid} ({flabel})"));
            let test_path = matched["behavioral_test"]
                .as_str()
                .expect("behavioral_test");
            let test_fn = test_path.rsplit("::").next().expect("fn name");
            assert!(
                resolver_src.contains(&format!("fn {test_fn}"))
                    || floor_src.contains(&format!("fn {test_fn}")),
                "Backing test '{test_fn}' for floor '{fid}' not found in oversight module"
            );
        }

        // VERIF-01 / CLAIMS-01: every (level, effect_id, consequence) in resolve(level) is backed
        let effect_claims = claims_doc["level_effect_claims"]
            .as_sequence()
            .expect("level_effect_claims");
        let mut total_effects = 0usize;
        for lvl in OversightLevel::ALL {
            let pol = resolve(lvl);
            for eff in &pol.effects {
                total_effects += 1;
                let matched = effect_claims
                    .iter()
                    .find(|c| {
                        c["level_id"].as_str() == Some(lvl.id())
                            && c["effect_id"].as_str() == Some(eff.id)
                            && c["consequence"].as_str() == Some(eff.value)
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "Missing level_effect_claim for ({}, {}, {:?})",
                            lvl.id(),
                            eff.id,
                            eff.value
                        )
                    });
                let test_path = matched["behavioral_test"]
                    .as_str()
                    .expect("behavioral_test");
                let test_fn = test_path.rsplit("::").next().expect("fn name");
                assert!(
                    resolver_src.contains(&format!("fn {test_fn}"))
                        || floor_src.contains(&format!("fn {test_fn}")),
                    "Backing test '{test_fn}' for ({}, {}) not found in oversight module",
                    lvl.id(),
                    eff.id
                );
            }
        }
        assert_eq!(effect_claims.len(), total_effects);
    }
}
