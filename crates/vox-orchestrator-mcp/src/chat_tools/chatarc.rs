//! ChatARC (phase 322, CAT-01/CAT-02): one lexical shortlist over the actions a
//! chat turn could take, and one calibrated Choice over it, including `none`.
//!
//! Candidates come from three live sources and nowhere else: the skill registry
//! (`state.orchestrator.skill_registry`, minus the turn's exclusions), the MCP
//! tools `select_tools_for_turn` admits for a plain chat turn, and the command
//! hints the hosting binary registers. ChatARC keeps no registry of its own and
//! rebuilds its candidates on every turn, so a newly added skill, tool or command
//! is reachable on the next pass. Actions are open strings; [`ActionKind`] only
//! names the source.
//!
//! Fail open: the flag off (`VOX_CHATARC`), an empty shortlist, `none`, or a
//! confidence below [`TAU_CHOICE`] all yield no pick, which is today's turn.

use vox_mcp_registry::{McpToolRegistryEntry, TOOL_REGISTRY};
use vox_skills::{SkillManifest, SkillRegistry};

use crate::llm_bridge::tool_selection::{TurnContext, select_tools_for_turn};
use crate::server_state::ServerState;
use crate::skill_search_index::SkillSearchIndex;

/// Bumped whenever the scorer, the bins or the threshold change meaning.
pub const CHATARC_VERSION: &str = "chatarc.v1";
/// Shortlist size, including the trailing `none` option.
pub(crate) const K_MAX: usize = 10;
/// Minimum calibrated choice confidence before ChatARC acts.
pub(crate) const TAU_CHOICE: f32 = 0.80;
/// `(margin lower bound, precision)`, ascending by bound. The margin is
/// `(s1 - s2) / s1` over the top two BM25 scores; the precision is the
/// Laplace top-1 precision of that bin on the calib split.
pub(crate) const CHOICE_BINS: [(f32, f32); 3] = [(0.0, 0.5), (0.25, 0.5), (0.5, 0.5)];

/// Which live source a candidate came from (not a closed set of actions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionKind {
    Skill,
    Tool,
    Command,
}

impl ActionKind {
    fn tag(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Tool => "tool",
            Self::Command => "command",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionCandidate {
    pub kind: ActionKind,
    pub id: String,
    pub text: String,
}

impl ActionCandidate {
    /// Index document id; the kind prefix keeps equal ids from different
    /// sources apart.
    fn doc_id(&self) -> String {
        format!("{}:{}", self.kind.tag(), self.id)
    }
}

pub(crate) fn skill_candidates(manifests: &[SkillManifest]) -> Vec<ActionCandidate> {
    manifests
        .iter()
        .map(|m| ActionCandidate {
            kind: ActionKind::Skill,
            id: m.id.clone(),
            text: format!("{} {} {}", m.name, m.tags.join(" "), m.description),
        })
        .collect()
}

pub(crate) fn tool_candidates(entries: &[&McpToolRegistryEntry]) -> Vec<ActionCandidate> {
    entries
        .iter()
        .map(|e| ActionCandidate {
            kind: ActionKind::Tool,
            id: e.name.to_string(),
            text: format!("{} {}", e.name.replace('_', " "), e.description),
        })
        .collect()
}

pub(crate) fn command_candidates(hints: &[CommandHint]) -> Vec<ActionCandidate> {
    hints
        .iter()
        .map(|h| ActionCandidate {
            kind: ActionKind::Command,
            id: h.invocation.clone(),
            text: format!("{} — {}", h.invocation, h.about),
        })
        .collect()
}

/// The MCP tools a plain chat turn may be offered: the same permission, lane,
/// skill and `vox_chat_` exclusion rules `run_agent_turn` applies. Only the cap
/// is lifted, for candidate gathering.
pub(crate) fn gather_tool_candidates(
    registry: &'static [McpToolRegistryEntry],
    skill_registry: &SkillRegistry,
) -> Vec<ActionCandidate> {
    let ctx = TurnContext {
        exclude_name_prefixes: vec!["vox_chat_"],
        max_tools: usize::MAX,
        ..Default::default()
    };
    tool_candidates(&select_tools_for_turn(registry, skill_registry, &ctx))
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ChatArcOption {
    Action {
        candidate: ActionCandidate,
        score: u64,
    },
    None,
}

/// Lexical top-(K_MAX - 1) actions by BM25, then `none` last.
pub(crate) fn shortlist(cands: &[ActionCandidate], query: &str) -> Vec<ChatArcOption> {
    // ponytail: the index is rebuilt per turn (a few hundred short docs behind a
    // default-off flag); cache it keyed on the sources' generation if profiling shows it.
    let index = SkillSearchIndex::from_docs(cands.iter().map(|c| (c.doc_id(), c.text.clone())));
    let by_id: std::collections::HashMap<String, &ActionCandidate> =
        cands.iter().map(|c| (c.doc_id(), c)).collect();
    let mut options: Vec<ChatArcOption> = index
        .search(query, K_MAX - 1)
        .into_iter()
        .filter_map(|hit| {
            by_id.get(&hit.id).map(|c| ChatArcOption::Action {
                candidate: (*c).clone(),
                score: hit.score,
            })
        })
        .collect();
    options.push(ChatArcOption::None);
    options
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChatArcPick {
    pub kind: ActionKind,
    pub id: String,
    pub text: String,
    pub confidence: f32,
}

impl ChatArcPick {
    /// The skill to inject, for skill picks only.
    pub(crate) fn skill_id(&self) -> Option<&str> {
        (self.kind == ActionKind::Skill).then_some(self.id.as_str())
    }
}

/// `(s1 - s2) / s1` over the top two scores; `s2` is 0 for a single hit.
pub(crate) fn margin(s1: u64, s2: u64) -> f32 {
    if s1 == 0 {
        return 0.0;
    }
    s1.saturating_sub(s2) as f32 / s1 as f32
}

/// Precision of the highest bin whose lower bound is at most `margin`.
pub(crate) fn bin_index(margin: f32, bins: &[(f32, f32)]) -> Option<usize> {
    bins.iter().rposition(|(lo, _)| *lo <= margin)
}

/// The top action when its calibrated confidence reaches `tau`, else none.
pub(crate) fn choose_with(
    options: &[ChatArcOption],
    bins: &[(f32, f32)],
    tau: f32,
) -> Option<ChatArcPick> {
    let mut actions = options.iter().filter_map(|o| match o {
        ChatArcOption::Action { candidate, score } => Some((candidate, *score)),
        ChatArcOption::None => None,
    });
    let (top, s1) = actions.next()?;
    let s2 = actions.next().map_or(0, |(_, s)| s);
    let confidence = bin_index(margin(s1, s2), bins).map_or(0.0, |i| bins[i].1);
    if confidence >= tau {
        return Some(ChatArcPick {
            kind: top.kind,
            id: top.id.clone(),
            text: top.text.clone(),
            confidence,
        });
    }
    None
}

pub(crate) fn decide(
    cands: &[ActionCandidate],
    query: &str,
    bins: &[(f32, f32)],
    tau: f32,
) -> Option<ChatArcPick> {
    choose_with(&shortlist(cands, query), bins, tau)
}

/// `decide`, but only gathers candidates when ChatARC is enabled.
pub(crate) fn decide_if_enabled(
    enabled: bool,
    gather: impl FnOnce() -> Vec<ActionCandidate>,
    query: &str,
    bins: &[(f32, f32)],
    tau: f32,
) -> Option<ChatArcPick> {
    if !enabled {
        return None;
    }
    decide(&gather(), query, bins, tau)
}

/// This turn's pick from the user's own message (never the assembled prompt,
/// which carries history, files and retrieved text).
pub(crate) fn pick_for_turn(
    state: &ServerState,
    user_message: &str,
    excluded_skills: &[String],
) -> Option<ChatArcPick> {
    decide_if_enabled(
        vox_config::inference::chatarc_enabled(),
        || {
            let skills = super::filter_excluded_skills(
                state.orchestrator.skill_registry.list(None),
                excluded_skills,
            );
            let mut cands = skill_candidates(&skills);
            cands.extend(gather_tool_candidates(TOOL_REGISTRY, &state.skill_registry));
            cands.extend(command_candidates(&registered_command_hints()));
            cands
        },
        user_message,
        &CHOICE_BINS,
        TAU_CHOICE,
    )
}

/// The skill to inject and its trigger source: an explicit pin always wins.
pub(crate) fn skill_to_inject<'a>(
    pinned: Option<&'a str>,
    chatarc: Option<&'a str>,
) -> Option<(&'a str, &'static str)> {
    pinned
        .map(|p| (p, "pinned"))
        .or_else(|| chatarc.map(|c| (c, "chatarc")))
}

/// One CLI command a host can suggest. Text only: ChatARC never executes it.
#[derive(Debug, Clone)]
pub struct CommandHint {
    pub invocation: String,
    pub about: String,
}

/// No provider seam yet (Task 2): the provider-unset behaviour.
pub(crate) fn registered_command_hints() -> Vec<CommandHint> {
    Vec::new()
}

#[cfg(test)]
#[path = "chatarc_tests.rs"]
mod tests;
