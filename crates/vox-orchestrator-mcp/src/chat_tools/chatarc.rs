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
pub const CHATARC_VERSION: &str = "chatarc.v2";
/// Shortlist size, including the trailing `none` option.
pub(crate) const K_MAX: usize = 10;
/// Minimum calibrated choice confidence before ChatARC acts. chatarc.v1 used
/// 0.80; its held-out risk at 0.80 was 0.167 (> 0.10), so v2 fails closed: no
/// confidence reaches 1.01 and ChatARC never acts (322-EVAL-REPORT.md).
pub(crate) const TAU_CHOICE: f32 = 1.01;
/// `(margin lower bound, precision)`, ascending by bound. The margin is
/// `(s1 - s2) / s1` over the top two BM25 scores; the precision is the
/// Laplace top-1 precision of that bin on the calib split
/// (`tests/fixtures/chatarc/calib.v1.jsonl`; correct of fired 3/8, 5/8, 14/16).
pub(crate) const CHOICE_BINS: [(f32, f32); 3] = [(0.0, 0.40), (0.25, 0.60), (0.5, 0.83)];

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

/// `decide`, but only gathers candidates when ChatARC is enabled and its
/// Choice can act: no confidence reaches a `tau` above 1.0 (chatarc.v2's
/// fail-closed [`TAU_CHOICE`]), so it then gathers nothing either (no skill
/// list, tool selection, command catalog or BM25 build).
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
    if tau > 1.0 {
        return None;
    }
    decide(&gather(), query, bins, tau)
}

/// This turn's pick. `user_message` is the user's own words (the caller passes
/// the message before `@mention` expansion and canonicalization), never the
/// assembled prompt, which carries history, files and retrieved text.
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

/// Supplies the host's command hints. vox-orchestrator-mcp (L3) cannot depend
/// on vox-cli (L5), so the binary hosting the chat server registers one.
pub type CommandHintProvider = fn() -> Vec<CommandHint>;

#[cfg(not(test))]
static COMMAND_HINT_PROVIDER: std::sync::RwLock<Option<CommandHintProvider>> =
    std::sync::RwLock::new(None);

// Tests keep the provider per thread, so parallel tests never share it.
#[cfg(test)]
thread_local! {
    static TEST_COMMAND_HINT_PROVIDER: std::cell::Cell<Option<CommandHintProvider>> =
        const { std::cell::Cell::new(None) };
}

/// Install the host's command-hint provider (replaces any earlier one).
pub fn register_command_hint_provider(provider: CommandHintProvider) {
    #[cfg(test)]
    TEST_COMMAND_HINT_PROVIDER.with(|p| p.set(Some(provider)));
    #[cfg(not(test))]
    {
        *COMMAND_HINT_PROVIDER
            .write()
            .unwrap_or_else(|p| p.into_inner()) = Some(provider);
    }
}

/// The registered provider's hints; none when no host registered one or the
/// lock is poisoned (fail open).
pub(crate) fn registered_command_hints() -> Vec<CommandHint> {
    #[cfg(test)]
    let provider = TEST_COMMAND_HINT_PROVIDER.with(std::cell::Cell::get);
    #[cfg(not(test))]
    let provider = COMMAND_HINT_PROVIDER.read().ok().and_then(|g| *g);
    provider.map(|p| p()).unwrap_or_default()
}

/// Appends one `Relevant vox command` line for a command pick (newlines
/// flattened, at most 200 chars); any other pick leaves the prompt unchanged.
pub(crate) fn append_command_hint(system_prompt: String, pick: Option<&ChatArcPick>) -> String {
    let Some(pick) = pick.filter(|p| p.kind == ActionKind::Command) else {
        return system_prompt;
    };
    let line: String = format!("Relevant vox command for this turn: {}", pick.text)
        .replace(['\r', '\n'], " ")
        .chars()
        .take(200)
        .collect();
    format!("{system_prompt}\n{line}")
}

/// The registry's static name for a tool pick; none for unknown ids and
/// non-tool picks.
pub(crate) fn tool_pin_for(
    pick: Option<&ChatArcPick>,
    registry: &'static [McpToolRegistryEntry],
) -> Option<&'static str> {
    let pick = pick.filter(|p| p.kind == ActionKind::Tool)?;
    registry.iter().find(|e| e.name == pick.id).map(|e| e.name)
}

#[cfg(test)]
#[path = "chatarc_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatarc_eval_tests.rs"]
mod eval_tests;

#[cfg(test)]
mod seam_tests {
    use super::*;

    fn hints_x() -> Vec<CommandHint> {
        vec![CommandHint {
            invocation: "vox x".to_string(),
            about: "x".to_string(),
        }]
    }

    fn hints_xy() -> Vec<CommandHint> {
        let mut h = hints_x();
        h.push(CommandHint {
            invocation: "vox y".to_string(),
            about: "y".to_string(),
        });
        h
    }

    fn pick(kind: ActionKind, id: &str, text: &str) -> ChatArcPick {
        ChatArcPick {
            kind,
            id: id.to_string(),
            text: text.to_string(),
            confidence: 1.0,
        }
    }

    #[test]
    fn provider_unset_yields_no_command_candidates() {
        assert!(registered_command_hints().is_empty());
        assert!(command_candidates(&registered_command_hints()).is_empty());
    }

    #[test]
    fn registered_provider_is_read_each_pass() {
        register_command_hint_provider(hints_x);
        assert_eq!(registered_command_hints().len(), 1);
        register_command_hint_provider(hints_xy);
        let invocations: Vec<String> = registered_command_hints()
            .into_iter()
            .map(|h| h.invocation)
            .collect();
        assert!(invocations.contains(&"vox y".to_string()));
    }

    #[test]
    fn append_command_hint_is_identity_without_command_pick() {
        let prompt = "system\nprompt".to_string();
        assert_eq!(append_command_hint(prompt.clone(), None), prompt);
        for kind in [ActionKind::Skill, ActionKind::Tool] {
            let p = pick(kind, "x", "x text");
            assert_eq!(append_command_hint(prompt.clone(), Some(&p)), prompt);
        }
    }

    #[test]
    fn append_command_hint_adds_one_line() {
        let prompt = "system\nprompt".to_string();
        let hint = &command_candidates(&[CommandHint {
            invocation: "vox graph query".to_string(),
            about: "query the graph\nfor a symbol".to_string(),
        }])[0];
        let p = pick(ActionKind::Command, &hint.id, &hint.text);
        let out = append_command_hint(prompt.clone(), Some(&p));
        assert_eq!(out.lines().count(), prompt.lines().count() + 1, "{out}");
        let last = out.lines().last().unwrap_or_default();
        assert!(out.starts_with(&prompt));
        assert!(
            last.contains("vox graph query") && last.contains("for a symbol"),
            "{last}"
        );
        let long = pick(ActionKind::Command, "vox z", &"z".repeat(500));
        let out = append_command_hint(String::new(), Some(&long));
        assert!(out.lines().last().unwrap_or_default().chars().count() <= 200);
    }

    #[test]
    fn tool_pin_for_maps_only_registered_tools() {
        let name = TOOL_REGISTRY[0].name;
        assert_eq!(
            tool_pin_for(Some(&pick(ActionKind::Tool, name, "")), TOOL_REGISTRY),
            Some(name)
        );
        let unknown = pick(ActionKind::Tool, "vox_no_such_tool", "");
        assert_eq!(tool_pin_for(Some(&unknown), TOOL_REGISTRY), None);
        let skill = pick(ActionKind::Skill, name, "");
        assert_eq!(tool_pin_for(Some(&skill), TOOL_REGISTRY), None);
        assert_eq!(tool_pin_for(None, TOOL_REGISTRY), None);
    }
}
