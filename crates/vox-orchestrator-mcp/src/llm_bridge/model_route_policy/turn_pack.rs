//! DEC-01 turn decision pack (phase 322): deterministic, versioned answers about
//! the user's own message that fill routing inputs the caller left unset, ahead
//! of the existing [`super::resolve::build_selection_request`] → `decide()` path.
//!
//! The pack only ever feeds `task`, `complexity`, `prefer_local` and
//! `allow_free_in_performance_mode`. It never emits a model id, and it never
//! touches selection axes, the free-floor list, the selection policy, SIZE-02
//! (size selection) or MESH-01 (mesh selection). Off by default
//! (`VOX_TURN_DECISION_PACK`); with it off, the request is byte-identical to the
//! pre-pack one.

use vox_orchestrator::config::CostPreference;
use vox_orchestrator::models::{Capability, ModelSelectionRequest};
use vox_orchestrator::types::TaskCategory;

use super::resolve::{VOX_LOCAL_PREFERRED_TASKS, build_selection_request};
use super::types::McpChatModelResolution;

/// Version of the rule table and its thresholds; bump on any change to either.
pub(super) const PACK_VERSION: &str = "turn-pack.v1";
/// Minimum confidence for an answer to patch `task`, `complexity` or `prefer_local`.
pub(super) const TAU_APPLY: f32 = 0.80;
/// Minimum confidence for the allow-free-tier answer to widen scorer eligibility.
pub(super) const TAU_FREE: f32 = 0.85;
/// Prior carried by every rule until 322-02 replaces it with the calib-split
/// precision. Below both τ, so an uncalibrated rule can never patch.
pub(super) const UNCALIBRATED: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Question {
    TaskShape,
    Complexity,
    Stakes,
    AllowFreeTier,
    PrivacyLocal,
}

#[derive(Debug)]
pub(super) struct Rule {
    pub id: &'static str,
    pub question: Question,
    /// Lowercase keywords, already normalized (alphanumerics, single spaces).
    pub any_of: &'static [&'static str],
    /// TaskShape: 0 code_edit, 1 research, 2 plan, 3 chat. Complexity and
    /// Stakes: 0 low, 1 medium, 2 high. AllowFreeTier and PrivacyLocal: 1 true,
    /// 0 false.
    pub answer: u8,
    pub confidence: f32,
}

// ponytail: keyword rules, ceiling is lexical recall; upgrade path is a labeled classifier head.
#[rustfmt::skip]
pub(super) const RULES: &[Rule] = &[
    Rule { id: "task_shape.0.1", question: Question::TaskShape, answer: 0, confidence: UNCALIBRATED,
        any_of: &["fix", "bug", "refactor", "implement", "compile error", "stack trace", "failing test", "rename", "patch"] },
    Rule { id: "task_shape.1.1", question: Question::TaskShape, answer: 1, confidence: UNCALIBRATED,
        any_of: &["what is", "explain why", "compare", "sources", "papers", "latest", "research"] },
    Rule { id: "task_shape.2.1", question: Question::TaskShape, answer: 2, confidence: UNCALIBRATED,
        any_of: &["plan", "roadmap", "break down", "steps to", "design doc", "milestone"] },
    Rule { id: "task_shape.3.1", question: Question::TaskShape, answer: 3, confidence: UNCALIBRATED,
        any_of: &["thanks", "thank you", "hello", "hi", "good morning", "how are you"] },
    Rule { id: "complexity.0.1", question: Question::Complexity, answer: 0, confidence: UNCALIBRATED,
        any_of: &["typo", "one line", "quick", "trivial"] },
    Rule { id: "complexity.2.1", question: Question::Complexity, answer: 2, confidence: UNCALIBRATED,
        any_of: &["architecture", "across crates", "redesign", "migrate", "concurrency", "race condition"] },
    Rule { id: "stakes.0.1", question: Question::Stakes, answer: 0, confidence: UNCALIBRATED,
        any_of: &["explain", "show me", "read"] },
    Rule { id: "stakes.2.1", question: Question::Stakes, answer: 2, confidence: UNCALIBRATED,
        any_of: &["delete", "drop table", "force push", "production", "deploy", "rm rf", "credentials", "secret"] },
    Rule { id: "allow_free_tier.1.1", question: Question::AllowFreeTier, answer: 1, confidence: UNCALIBRATED,
        any_of: &["cheap", "free model", "free tier", "quick question", "any model", "no rush"] },
    Rule { id: "allow_free_tier.0.1", question: Question::AllowFreeTier, answer: 0, confidence: UNCALIBRATED,
        any_of: &["best model", "highest quality", "most capable", "critical"] },
    Rule { id: "privacy_local.1.1", question: Question::PrivacyLocal, answer: 1, confidence: UNCALIBRATED,
        any_of: &["offline", "local only", "locally", "private", "confidential", "on device", "do not send", "air gapped"] },
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Answer {
    pub value: u8,
    pub confidence: f32,
    pub rule_id: &'static str,
}

/// One answer per question. Carries no string but rule ids, so no model id can
/// ride through it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TurnPack {
    pub task_shape: Option<Answer>,
    pub complexity: Option<Answer>,
    pub stakes: Option<Answer>,
    pub allow_free_tier: Option<Answer>,
    pub privacy_local: Option<Answer>,
}

impl TurnPack {
    fn slot_mut(&mut self, q: Question) -> &mut Option<Answer> {
        match q {
            Question::TaskShape => &mut self.task_shape,
            Question::Complexity => &mut self.complexity,
            Question::Stakes => &mut self.stakes,
            Question::AllowFreeTier => &mut self.allow_free_tier,
            Question::PrivacyLocal => &mut self.privacy_local,
        }
    }
}

pub(super) fn evaluate(prompt: &str) -> TurnPack {
    evaluate_with_rules(prompt, RULES)
}

/// Normalizes `prompt` to lowercase alphanumeric words joined by single spaces
/// (padded with one space each side), fires every rule with a space-padded
/// keyword substring, and keeps, per question, the highest-confidence firing
/// rule; ties go to the lexicographically smallest rule id, so the answer never
/// depends on table order.
pub(super) fn evaluate_with_rules(prompt: &str, rules: &[Rule]) -> TurnPack {
    let mut text = String::from(" ");
    for word in prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        text.push_str(&word.to_lowercase());
        text.push(' ');
    }
    let mut pack = TurnPack::default();
    for rule in rules {
        if !rule.any_of.iter().any(|k| text.contains(&format!(" {k} "))) {
            continue;
        }
        let slot = pack.slot_mut(rule.question);
        let wins = slot.is_none_or(|cur| {
            rule.confidence > cur.confidence
                || (rule.confidence == cur.confidence && rule.id < cur.rule_id)
        });
        if wins {
            *slot = Some(Answer {
                value: rule.answer,
                confidence: rule.confidence,
                rule_id: rule.id,
            });
        }
    }
    pack
}

/// The four `build_selection_request` inputs the pack may fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SelectionInputs {
    pub task: TaskCategory,
    pub complexity: u8,
    pub prefer_local: bool,
    pub allow_free: bool,
}

impl SelectionInputs {
    /// Today's inputs: the caller's values, and no free-tier widening.
    pub(super) fn from_caller(res: &McpChatModelResolution, derived_prefer_local: bool) -> Self {
        Self {
            task: res.task_category,
            complexity: res.complexity,
            prefer_local: derived_prefer_local,
            allow_free: false,
        }
    }
}

pub(super) fn confident(a: &Answer, tau: f32) -> bool {
    a.confidence >= tau
}

/// Complexity answer 0/1/2 → the `complexity` hint `build_selection_request` takes.
const COMPLEXITY_SCORE_TO_U8: [u8; 3] = [2, 5, 8];

/// TaskShape answer 0..=3 → task category.
const TASK_SHAPES: [TaskCategory; 4] = [
    TaskCategory::CodeGen,
    TaskCategory::Research,
    TaskCategory::Planning,
    TaskCategory::Chat,
];

pub(super) fn selection_inputs(
    pack: Option<&TurnPack>,
    res: &McpChatModelResolution,
    derived_prefer_local: bool,
) -> SelectionInputs {
    let caller = SelectionInputs::from_caller(res, derived_prefer_local);
    let Some(pack) = pack else {
        return caller;
    };
    let sure = |a: Option<Answer>, tau: f32| a.filter(|a| confident(a, tau)).map(|a| a.value);
    let may_set_task = res.task_unset;
    let may_set_complexity = res.complexity_unset;
    // prefer_local is derived from the caller's task (resolve.rs
    // VOX_LOCAL_PREFERRED_TASKS), so it is unset exactly when task is.
    let may_set_prefer_local = res.task_unset;
    // `res` is post source-policy: resolve.rs writes any policy clutch into
    // `res.clutch` before the decide() site, so a policy-derived clutch counts as set.
    let may_set_allow_free = res.clutch.is_none();

    let task = match sure(pack.task_shape, TAU_APPLY) {
        Some(v) if may_set_task => TASK_SHAPES
            .get(usize::from(v))
            .copied()
            .unwrap_or(caller.task),
        _ => caller.task,
    };
    let complexity = match sure(pack.complexity, TAU_APPLY) {
        Some(v) if may_set_complexity => COMPLEXITY_SCORE_TO_U8
            .get(usize::from(v))
            .copied()
            .unwrap_or(caller.complexity),
        _ => caller.complexity,
    };
    let private = sure(pack.privacy_local, TAU_APPLY) == Some(1);
    let prefer_local = if may_set_prefer_local {
        VOX_LOCAL_PREFERRED_TASKS.contains(&task) || private
    } else {
        caller.prefer_local
    };
    // Scorer eligibility only: lets `:free` specs compete under Performance
    // (orchestrator ranking). It selects no policy, touches no axes, and never
    // reads or reorders the free-floor list.
    let allow_free = may_set_allow_free
        && sure(pack.allow_free_tier, TAU_FREE) == Some(1)
        && sure(pack.stakes, TAU_APPLY) != Some(2)
        && !private;
    SelectionInputs {
        task,
        complexity,
        prefer_local,
        allow_free,
    }
}

/// The decide() request for this turn: `pack` (or `None` with the flag off)
/// fills only caller-unset inputs, then the existing builder runs unchanged.
pub(super) fn turn_selection_request(
    pack: Option<&TurnPack>,
    res: &McpChatModelResolution,
    derived_prefer_local: bool,
    preference: CostPreference,
    required_capabilities: Vec<Capability>,
) -> ModelSelectionRequest {
    let inputs = selection_inputs(pack, res, derived_prefer_local);
    build_selection_request(
        inputs.task,
        inputs.complexity,
        inputs.prefer_local,
        inputs.allow_free,
        res.free_tier_fill_in_middle,
        preference,
        required_capabilities,
        res.clutch,
        res.risk,
    )
}
