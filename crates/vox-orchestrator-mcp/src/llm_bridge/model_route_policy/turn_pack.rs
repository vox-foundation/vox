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

/// Version of the rule table, thresholds and vetoes; bump on any change (v3: vetoes).
pub(super) const PACK_VERSION: &str = "turn-pack.v3";
/// Minimum confidence for an answer to patch `task`, `complexity` or `prefer_local`.
pub(super) const TAU_APPLY: f32 = 0.80;
/// Minimum confidence for the allow-free-tier answer to widen scorer eligibility.
pub(super) const TAU_FREE: f32 = 0.85;

/// Above 1.0, so no answer reaches it: the question is disabled (fails closed).
pub(super) const TAU_DISABLED: f32 = 1.01;

/// The threshold each question's answer must reach to act in `selection_inputs`
/// (and the one the DEC-04 eval reports risk at).
pub(super) const fn tau_for(q: Question) -> f32 {
    match q {
        // turn-pack.v2: held-out risk at τ was 0.25 for both (322-EVAL-REPORT.md),
        // above the 0.10 gate. Disabled rather than refit on the eval split.
        Question::TaskShape | Question::Stakes => TAU_DISABLED,
        Question::AllowFreeTier => TAU_FREE,
        _ => TAU_APPLY,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Question {
    TaskShape,
    Complexity,
    Stakes,
    AllowFreeTier,
    PrivacyLocal,
}

#[derive(Debug, Clone)]
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

/// confidence = Laplace precision on tests/fixtures/turn_pack/calib.v1.jsonl,
/// checked by rule_confidences_match_calib_split. A rule that never fires on
/// calib keeps 0.5 (Laplace of 0/0), below both τ.
// ponytail: keyword rules, ceiling is lexical recall; upgrade path is a labeled classifier head.
#[rustfmt::skip]
pub(super) const RULES: &[Rule] = &[
    Rule { id: "task_shape.0.1", question: Question::TaskShape, answer: 0, confidence: 0.67,
        any_of: &["fix", "bug", "refactor", "implement", "compile error", "stack trace", "failing test", "rename", "patch"] },
    Rule { id: "task_shape.1.1", question: Question::TaskShape, answer: 1, confidence: 0.83,
        any_of: &["what is", "what does", "how does", "look up", "investigate", "explain why", "compare", "sources", "papers", "latest", "research"] },
    Rule { id: "task_shape.2.1", question: Question::TaskShape, answer: 2, confidence: 0.86,
        any_of: &["plan", "roadmap", "break down", "steps to", "design doc", "milestone"] },
    Rule { id: "task_shape.3.1", question: Question::TaskShape, answer: 3, confidence: 0.75,
        any_of: &["thanks", "thank you", "hello", "hi", "hey", "good morning", "how are you"] },
    Rule { id: "complexity.0.1", question: Question::Complexity, answer: 0, confidence: 0.40,
        any_of: &["typo", "one line", "quick", "trivial"] },
    Rule { id: "complexity.2.1", question: Question::Complexity, answer: 2, confidence: 0.50,
        any_of: &["architecture", "across crates", "redesign", "migrate", "concurrency", "race condition"] },
    Rule { id: "stakes.0.1", question: Question::Stakes, answer: 0, confidence: 0.50,
        any_of: &["explain", "show me", "read"] },
    Rule { id: "stakes.2.1", question: Question::Stakes, answer: 2, confidence: 0.80,
        any_of: &["delete", "drop table", "force push", "production", "deploy", "rm rf", "credentials", "secret"] },
    Rule { id: "allow_free_tier.1.1", question: Question::AllowFreeTier, answer: 1, confidence: 0.86,
        any_of: &["cheap", "quick one", "small change", "free model", "free tier", "quick question", "any model", "no rush"] },
    Rule { id: "allow_free_tier.0.1", question: Question::AllowFreeTier, answer: 0, confidence: 0.50,
        any_of: &["best model", "highest quality", "most capable", "critical"] },
    Rule { id: "privacy_local.1.1", question: Question::PrivacyLocal, answer: 1, confidence: 0.80,
        any_of: &["offline", "local only", "locally", "keep it local", "private", "confidential", "on device", "do not send", "don t send", "air gapped"] },
];

/// Negated free-tier requests ("don't use a free model") veto instead of allowing.
// ponytail: lexical negation, ceiling is phrasings not listed; upgrade path is the classifier head.
#[rustfmt::skip]
const FREE_NEGATIONS: &[&str] = &["don t use", "dont use", "do not use", "never use", "avoid",
    "no free", "not free", "not a free", "not the free", "not cheap", "not a cheap"];

/// Credential words; `vox_redact` catches token shapes (`sk-…`, `ghp_…`, `AKIA…`, JWTs).
#[rustfmt::skip]
const SECRET_CUES: &[&str] = &["api key", "apikey", "password", "passwd", "passphrase", "token",
    "private key", "secret", "credential", "credentials", "ssn", "sk", "bearer"];

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
    /// Vetoes free eligibility (from cue firings, never from an answer).
    pub free_veto: bool,
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
/// depends on table order. Vetoes come from firings, never from answers.
pub(super) fn evaluate_with_rules(prompt: &str, rules: &[Rule]) -> TurnPack {
    let mut text = String::from(" ");
    for word in prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        text.push_str(&word.to_lowercase());
        text.push(' ');
    }
    let fires = |any_of: &[&str]| any_of.iter().any(|k| text.contains(&format!(" {k} ")));
    let fired = |q: Question, answer: u8| {
        rules
            .iter()
            .any(|r| r.question == q && r.answer == answer && fires(r.any_of))
    };
    let mut pack = TurnPack::default();
    for rule in rules {
        if !fires(rule.any_of) {
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
    // CR-02: a deny-side rule (it loses the argmax) or a negated free cue vetoes.
    pack.free_veto = fired(Question::AllowFreeTier, 0) || fires(FREE_NEGATIONS);
    // WR-02: secret-shaped text never becomes free-tier eligible.
    pack.free_veto |= fires(SECRET_CUES) || vox_redact::redact_owned(prompt) != prompt;
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

/// `tau` is the per-question threshold table: [`tau_for`] in production.
pub(super) fn selection_inputs(
    tau: fn(Question) -> f32,
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

    let task = match sure(pack.task_shape, tau(Question::TaskShape)) {
        Some(v) if may_set_task => TASK_SHAPES
            .get(usize::from(v))
            .copied()
            .unwrap_or(caller.task),
        _ => caller.task,
    };
    let complexity = match sure(pack.complexity, tau(Question::Complexity)) {
        Some(v) if may_set_complexity => COMPLEXITY_SCORE_TO_U8
            .get(usize::from(v))
            .copied()
            .unwrap_or(caller.complexity),
        _ => caller.complexity,
    };
    let private = sure(pack.privacy_local, tau(Question::PrivacyLocal)) == Some(1);
    let prefer_local = if may_set_prefer_local {
        VOX_LOCAL_PREFERRED_TASKS.contains(&task) || private
    } else {
        caller.prefer_local
    };
    // Stakes only vetoes free eligibility, so a disabled Stakes fails closed by
    // vetoing on any stakes-2 firing, whatever its confidence.
    let stakes_tau = tau(Question::Stakes);
    let high_stakes = pack
        .stakes
        .is_some_and(|a| a.value == 2 && (stakes_tau > 1.0 || confident(&a, stakes_tau)));
    // Scorer eligibility only: lets `:free` specs compete under Performance
    // (orchestrator ranking). It selects no policy, touches no axes, and never
    // reads or reorders the free-floor list.
    let allow_free = may_set_allow_free
        && sure(pack.allow_free_tier, tau(Question::AllowFreeTier)) == Some(1)
        && !pack.free_veto
        && !high_stakes
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
    turn_selection_request_at(
        tau_for,
        pack,
        res,
        derived_prefer_local,
        preference,
        required_capabilities,
    )
}

/// [`turn_selection_request`] at an explicit per-question threshold table.
pub(super) fn turn_selection_request_at(
    tau: fn(Question) -> f32,
    pack: Option<&TurnPack>,
    res: &McpChatModelResolution,
    derived_prefer_local: bool,
    preference: CostPreference,
    required_capabilities: Vec<Capability>,
) -> ModelSelectionRequest {
    let inputs = selection_inputs(tau, pack, res, derived_prefer_local);
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
