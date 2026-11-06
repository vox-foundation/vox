//! Task 1.2 (harness implementation spec,
//! `docs/src/architecture/vox-harness-implementation-spec-2026-07-31.md` §3.2):
//! deterministic, unit-testable tool-selection filter for a single chat turn.
//!
//! Vox's MCP tool registry ([`vox_mcp_registry::TOOL_REGISTRY`]) has 330
//! entries. Published research on tool-selection accuracy (cited in the
//! implementation spec) documents degradation once a model is offered more
//! than roughly 30-50 tools in one turn — 330 is 6.6x that. This module does
//! NOT send anything to a model; it only narrows `TOOL_REGISTRY` down to a
//! small relevant subset for a turn, which a *future* task (1.3, the agent
//! loop) will use to build the wire-format tool list actually sent.
//!
//! Filters are applied in a fixed order: permission -> lane -> active-skill
//! allowlist -> cap. See [`select_tools_for_turn`].

use vox_mcp_registry::{McpToolRegistryEntry, SideEffectClass};
use vox_skills::SkillRegistry;

use crate::skill_permissions::check_skill_tool_permission;
use crate::tool_search::{score_entry, turn_terms};

/// Thin re-export of `vox_skills::new_registry_arc` so `vox-cli`'s
/// `tool-cap-never-exceeds-cap` golden task (`vox harness eval`) can build a
/// fixture [`SkillRegistry`] for [`select_tools_for_turn`] without taking a
/// direct (optional, `ars`-feature-gated) dependency on `vox-skills` itself —
/// `vox-orchestrator-mcp` already depends on it unconditionally.
#[must_use]
pub fn new_registry_arc_for_eval() -> std::sync::Arc<SkillRegistry> {
    vox_skills::new_registry_arc()
}

/// Default cap on the number of tools offered per turn.
///
/// Chosen well under the ~30-50-tool degradation threshold cited in the
/// implementation spec, while leaving headroom for the always-on skill
/// infrastructure tools (`vox_skill_*`, `vox_chat_*`) that
/// [`check_skill_tool_permission`] never filters out. 40 sits comfortably
/// inside that range without hugging its upper edge.
pub const DEFAULT_MAX_TOOLS: usize = 40;

/// Most read-only tools from outside `ctx.lanes` that relevance may pull into one turn.
pub const MAX_OFF_LANE_RELEVANT: usize = 3;

/// Lowest relevance score at which a tool outside `ctx.lanes` may be offered: one
/// exact tool-name segment hit (see `tool_search::score_entry`).
const OFF_LANE_MIN_SCORE: u32 = 8;

/// Wire-string used by [`crate::permission_modes::PermissionMode::from_wire`]
/// for its read-only/planning mode. `PermissionMode::Plan` is documented there
/// as "Read-only/planning mode" — the closest existing canonical concept to
/// "read-only" in this crate's permission vocabulary, so this filter reuses
/// the literal `"plan"` rather than inventing a new string. Note that for the
/// *dispatch* HITL gate `Plan` behaves identically to `Ask` (auto-approves
/// nothing); here we additionally interpret it as "restrict the tool surface
/// to `http_read_role_eligible` tools only". If task 1.3 (the agent loop)
/// establishes a different / more granular read-only vocabulary, this
/// constant should be reconciled with it then.
pub const READ_ONLY_PERMISSION_MODE: &str = "plan";

/// Everything needed to filter [`vox_mcp_registry::TOOL_REGISTRY`] down to a
/// small, relevant subset for one chat turn.
#[derive(Debug, Clone)]
pub struct TurnContext {
    /// Wire-level permission mode string (see
    /// `DispatchRequest::permission_mode` / `PermissionMode::from_wire`).
    /// `Some(READ_ONLY_PERMISSION_MODE)` restricts selection to
    /// `http_read_role_eligible` tools only. Any other value (including
    /// `None`) is a no-op for this filter — permission-mode enforcement for
    /// non-read-only modes already happens later, in `dispatch.rs`'s HITL
    /// gate, not here.
    pub permission_mode: Option<String>,
    /// Which `product_lane`s are relevant for this turn (e.g. a plain chat
    /// turn might want `["ai", "app"]` but not `"platform"`/`"interop"`).
    /// An empty vec means "no lane restriction" (all lanes pass) rather than
    /// "nothing passes" — callers that want zero tools should not call this
    /// function at all.
    pub lanes: Vec<&'static str>,
    /// The currently active skill, if any, for the per-skill allowlist
    /// filter (see [`check_skill_tool_permission`]).
    pub active_skill_id: Option<String>,
    /// Hard cap on the number of tools returned. See [`DEFAULT_MAX_TOOLS`].
    pub max_tools: usize,
    /// Tool-name prefixes to exclude from the candidate set entirely, applied
    /// BEFORE the `max_tools` cap (unlike a post-hoc filter, this guarantees
    /// excluded tools never consume a cap slot). Callers that must never offer
    /// a given family of tools — e.g. the agent loop excluding `vox_chat_*` to
    /// prevent unbounded re-entrant recursion into `run_agent_turn` — should
    /// list those prefixes here rather than filtering the already-capped
    /// result, which can silently shrink the effective tool count below
    /// `max_tools` and can drop a genuinely useful tool that only lost its cap
    /// slot to an excluded one. Empty by default (no exclusion).
    pub exclude_name_prefixes: Vec<&'static str>,
    /// Tool names hoisted to the front of the candidate list, applied BEFORE
    /// the `max_tools` cap. Registry order is alphabetical, so a plain
    /// truncation silently makes late-alphabet tools unreachable no matter how
    /// relevant they are — `vox_spawn_agent` sits at index ~166 of the ~188
    /// candidates a default chat turn produces, well past the 40-tool cut.
    /// Listing a name here guarantees it a cap slot.
    ///
    /// Pins are subject to every other filter (read-only permission, lane,
    /// active-skill allowlist, prefix exclusion) — a pin is a priority hint,
    /// not a permission bypass. Names not present in the registry, or filtered
    /// out by an earlier step, are simply ignored.
    pub pin_names: Vec<&'static str>,
    /// The user text for this turn. `None` keeps registry order inside the lanes
    /// (callers that are not a chat turn). `Some` ranks survivors with
    /// [`turn_terms`] / [`score_entry`] and may pull in a few read-only tools
    /// from outside `ctx.lanes`.
    pub query: Option<String>,
}

impl Default for TurnContext {
    /// A plain chat turn: `ai` + `app` lanes, no active skill, default cap,
    /// no permission restriction, no name-prefix exclusions.
    fn default() -> Self {
        Self {
            permission_mode: None,
            lanes: vec!["ai", "app"],
            active_skill_id: None,
            max_tools: DEFAULT_MAX_TOOLS,
            exclude_name_prefixes: Vec::new(),
            pin_names: Vec::new(),
            query: None,
        }
    }
}

impl TurnContext {
    fn is_read_only(&self) -> bool {
        self.permission_mode.as_deref() == Some(READ_ONLY_PERMISSION_MODE)
    }

    fn lane_allowed(&self, lane: &str) -> bool {
        self.lanes.is_empty() || self.lanes.contains(&lane)
    }
}

/// Select the tool subset for one chat turn.
///
/// Applies, in order:
/// 1. Permission filter — read-only mode keeps only `http_read_role_eligible`
///    tools.
/// 2. Lane filter — keeps only tools whose `product_lane` is in
///    `ctx.lanes` (no-op if `ctx.lanes` is empty).
/// 3. Active-skill filter — drops any tool [`check_skill_tool_permission`]
///    denies for `ctx.active_skill_id` (skill-infrastructure tools such as
///    `vox_skill_list`/`vox_chat_*` are never denied by that function, so
///    they always survive this step regardless of the active skill).
/// 4. Name-prefix exclusion — drops any tool whose name starts with one of
///    `ctx.exclude_name_prefixes` (no-op if empty).
/// 5. Pin hoist — moves any in-lane survivor named in `ctx.pin_names` to the
///    front, preserving registry order (no-op if empty).
/// 6. Relevance — the remaining in-lane survivors are ordered by
///    [`score_entry`] over [`turn_terms`] of `ctx.query`, best first. Ties and
///    a missing query keep registry order. Up to [`MAX_OFF_LANE_RELEVANT`]
///    read-only tools outside `ctx.lanes` whose name matches a turn term join
///    that ranking.
/// 7. Cap — truncates to the first `ctx.max_tools` survivors.
///
/// Filters, the pin hoist, and the ranking all run BEFORE the cap: excluded
/// tools never consume a cap slot, and pinned tools are always inside it.
#[must_use]
pub fn select_tools_for_turn(
    registry: &'static [McpToolRegistryEntry],
    skill_registry: &SkillRegistry,
    ctx: &TurnContext,
) -> Vec<&'static McpToolRegistryEntry> {
    let candidates: Vec<_> = registry
        .iter()
        .filter(|entry| admissible(skill_registry, ctx, entry))
        .collect();
    let pinned: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|entry| ctx.pin_names.contains(&entry.name) && ctx.lane_allowed(entry.product_lane))
        .collect();
    let terms = ctx.query.as_deref().map(turn_terms).unwrap_or_default();
    let mut ranked: Vec<(u32, &'static McpToolRegistryEntry)> = Vec::new();
    let mut off_lane: Vec<(u32, &'static McpToolRegistryEntry)> = Vec::new();
    for entry in candidates
        .iter()
        .copied()
        .filter(|entry| !ctx.pin_names.contains(&entry.name))
    {
        let score = score_entry(&terms, entry);
        if ctx.lane_allowed(entry.product_lane) {
            ranked.push((score, entry));
        } else if score >= OFF_LANE_MIN_SCORE
            && entry.side_effect_class == SideEffectClass::ReadOnly
        {
            off_lane.push((score, entry));
        }
    }
    off_lane.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    off_lane.truncate(MAX_OFF_LANE_RELEVANT);
    ranked.extend(off_lane);
    // `sort_by_key` is stable, so equal scores stay in registry order.
    ranked.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    pinned
        .into_iter()
        .chain(ranked.into_iter().map(|(_, entry)| entry))
        .take(ctx.max_tools)
        .collect()
}

/// Whether a tool name is acceptable on a provider wire (`[A-Za-z0-9_-]{1,128}`).
/// One bad name fails the entire request, so such tools are never offered.
fn is_wire_safe_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Every filter except lanes and the cap.
fn admissible(
    skill_registry: &SkillRegistry,
    ctx: &TurnContext,
    entry: &McpToolRegistryEntry,
) -> bool {
    is_wire_safe_name(entry.name)
        && (!ctx.is_read_only() || entry.http_read_role_eligible)
        && check_skill_tool_permission(skill_registry, ctx.active_skill_id.as_deref(), entry.name)
            .is_none()
        && !ctx
            .exclude_name_prefixes
            .iter()
            .any(|prefix| entry.name.starts_with(prefix))
}

/// Compact default tool-schema budget when spoke-conditioned BM25 + intent ranking is active (`SOTA-TOOLS-03`).
/// Reduces prompt tool-schema overhead by 50% compared to [`DEFAULT_MAX_TOOLS`] (40 -> 20) while
/// surfacing 100% of late-alphabet spoke priority tools.
pub const DEFAULT_RANKED_MAX_TOOLS: usize = 20;

/// Canonical high-signal MCP tools for each [`vox_corpus::training::contract::OrganicSpokeKind`] (`SOTA-TOOLS-02`).
#[must_use]
pub fn spoke_priority_tools(
    spoke: vox_corpus::training::contract::OrganicSpokeKind,
) -> &'static [&'static str] {
    use vox_corpus::training::contract::OrganicSpokeKind;
    match spoke {
        OrganicSpokeKind::VoxLang => &[
            "vox_apply_structured_edit",
            "vox_validate_file",
            "vox_validate_source",
            "vox_check",
            "vox_check_workspace",
            "vox_repo_query_file",
            "vox_git_status",
        ],
        OrganicSpokeKind::RustWorkspace => &[
            "vox_apply_structured_edit",
            "vox_run_tests",
            "vox_check_workspace",
            "vox_repo_query_file",
            "vox_repo_query_text",
            "vox_validate_file",
            "vox_git_status",
        ],
        OrganicSpokeKind::StructuredShell => &[
            "vox_repo_status",
            "vox_repo_query_file",
            "vox_repo_query_text",
            "vox_check_workspace",
            "vox_git_status",
        ],
        OrganicSpokeKind::McpToolChain => &[
            "vox_kb_search",
            "vox_search_query",
            "vox_validate_file",
            "vox_validate_mermaid",
            "vox_repo_status",
            "vox_skill_search",
            "vox_git_status",
        ],
        OrganicSpokeKind::AgentHarness => &[
            "vox_apply_structured_edit",
            "vox_spawn_agent",
            "vox_submit_task",
            "vox_task_status",
            "vox_verify_task_claims",
            "vox_repo_status",
            "vox_git_status",
        ],
        OrganicSpokeKind::SpeechToCode => &[
            "vox_apply_structured_edit",
            "vox_validate_source",
            "vox_validate_file",
            "vox_check",
            "vox_repo_query_file",
            "vox_git_status",
        ],
        OrganicSpokeKind::PopuliMesh => &[
            "vox_populi_local_status",
            "vox_spawn_agent",
            "vox_submit_task",
            "vox_task_status",
            "vox_orchestrator_status",
            "vox_git_status",
        ],
        OrganicSpokeKind::SkillCreation => &[
            "vox_apply_structured_edit",
            "vox_skill_search",
            "vox_skill_list",
            "vox_skill_info",
            "vox_skill_parse",
            "vox_skill_use",
            "vox_validate_file",
            "vox_git_status",
        ],
    }
}

/// Core daily-driver tools that receive a baseline relevance boost so generic greetings ("hi")
/// still include git status, delegation, skill activation, and validation tools.
const CORE_DAILY_DRIVER_TOOLS: &[&str] = &[
    "vox_apply_structured_edit",
    "vox_answer_question",
    "vox_resolve_approval",
    "vox_git_status",
    "vox_skill_use",
    "vox_skill_list",
    "vox_skill_search",
    "vox_spawn_agent",
    "vox_submit_task",
    "vox_task_status",
    "vox_validate_file",
    "vox_validate_mermaid",
    "vox_kb_search",
    "vox_repo_status",
    "vox_deep_research",
    "vox_verify_task_claims",
];

/// Score a candidate [`McpToolRegistryEntry`] for a user turn using spoke priority, explicit pins,
/// exact identifier mention, and lexical BM25-style token overlap (`SOTA-TOOLS-01`, `SOTA-WRITE-PRIORITY-01..02`).
#[must_use]
pub fn score_tool_relevance_for_turn(
    entry: &McpToolRegistryEntry,
    user_prompt: &str,
    spoke: vox_corpus::training::contract::OrganicSpokeKind,
    is_pinned: bool,
) -> i32 {
    let mut score = 0i32;
    if is_pinned {
        score += 1_000;
    }
    if spoke_priority_tools(spoke).contains(&entry.name) {
        score += 220;
    }
    if CORE_DAILY_DRIVER_TOOLS.contains(&entry.name) {
        score += 50;
    }

    let prompt_lower = user_prompt.to_ascii_lowercase();
    let name_lower = entry.name.to_ascii_lowercase();
    let desc_lower = entry.description.to_ascii_lowercase();

    if prompt_lower.contains(&name_lower) {
        score += 300;
    }

    let mut has_mutation_intent = false;
    for raw_tok in prompt_lower.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if matches!(
            raw_tok,
            "fix"
                | "add"
                | "create"
                | "update"
                | "implement"
                | "refactor"
                | "write"
                | "patch"
                | "replace"
                | "modify"
        ) {
            has_mutation_intent = true;
        }
        if raw_tok.len() < 3
            || matches!(
                raw_tok,
                "the" | "and" | "for" | "with" | "that" | "this" | "can" | "you" | "how" | "what"
            )
        {
            continue;
        }
        if name_lower.contains(raw_tok) {
            score += 40;
        }
        if desc_lower.contains(raw_tok) {
            score += 15;
        }
    }

    if has_mutation_intent
        && matches!(
            entry.name,
            "vox_apply_structured_edit" | "vox_generate_code" | "vox_repo_query_file"
        )
    {
        score += 120;
    }

    score
}

/// Spoke-conditioned, relevance-ranked tool selector (`SOTA-TOOLS-01..04`).
///
/// Replaces blind alphabetical `.take(max_tools)` truncation with deterministic spoke + lexical
/// relevance ranking so late-alphabet tools (`vox_validate_file`, `vox_kb_search`, `vox_repo_status`,
/// `vox_skill_search`, `vox_spawn_agent`) are always surfaced when needed, while allowing callers
/// to use a smaller `max_tools` window (`DEFAULT_RANKED_MAX_TOOLS = 20`) to save ~50% schema tokens.
#[must_use]
pub fn select_tools_for_turn_ranked(
    registry: &'static [McpToolRegistryEntry],
    skill_registry: &SkillRegistry,
    ctx: &TurnContext,
    user_prompt: &str,
    prior_spoke: Option<vox_corpus::training::contract::OrganicSpokeKind>,
) -> (
    Vec<&'static McpToolRegistryEntry>,
    vox_corpus::training::contract::OrganicSpokeKind,
    vox_orchestrator::models::ConversationalTurnMode,
) {
    let (classified_spoke, _score, _runner_up, _ru_score, _signals, turn_mode, _used_ctx) =
        vox_orchestrator::models::ModelRegistry::score_conversational_turn_across_spokes(
            user_prompt,
            prior_spoke,
        );
    let priority_set = spoke_priority_tools(classified_spoke);

    let mut scored: Vec<(i32, usize, &'static McpToolRegistryEntry)> = registry
        .iter()
        .enumerate()
        .filter(|(_, entry)| admissible(skill_registry, ctx, entry))
        .filter(|(_, entry)| {
            ctx.lane_allowed(entry.product_lane)
                || priority_set.contains(&entry.name)
                || CORE_DAILY_DRIVER_TOOLS.contains(&entry.name)
        })
        .map(|(idx, entry)| {
            let is_pinned = ctx.pin_names.contains(&entry.name);
            let rel =
                score_tool_relevance_for_turn(entry, user_prompt, classified_spoke, is_pinned);
            (rel, idx, entry)
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let selected = scored
        .into_iter()
        .take(ctx.max_tools)
        .map(|(_, _, entry)| entry)
        .collect();

    (selected, classified_spoke, turn_mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use vox_mcp_registry::TOOL_REGISTRY;
    use vox_plugin_api::skill::LoadedSkill;
    use vox_skills::{SkillManifest, new_registry_arc};

    fn install_restrictive_skill(reg: &SkillRegistry) {
        reg.install(LoadedSkill {
            plugin_id: "narrow-skill".to_string(),
            format_version: 1,
            manifest: SkillManifest {
                id: "narrow-skill".to_string(),
                name: "narrow-skill".to_string(),
                version: "1.0.0".to_string(),
                description: "only allows one domain tool".to_string(),
                tools: vec!["vox_git_status".to_string()],
                ..Default::default()
            },
            body: String::new(),
            exposed_tools: vec!["vox_git_status".to_string()],
        });
    }

    #[test]
    fn default_context_on_real_registry_is_nonempty_and_capped() {
        let reg = new_registry_arc();
        let ctx = TurnContext::default();
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        assert!(!selected.is_empty());
        assert!(selected.len() <= ctx.max_tools);
    }

    #[test]
    fn read_only_mode_keeps_only_http_read_role_eligible_tools() {
        let reg = new_registry_arc();
        let ctx = TurnContext {
            permission_mode: Some(READ_ONLY_PERMISSION_MODE.to_string()),
            lanes: vec![],
            active_skill_id: None,
            max_tools: TOOL_REGISTRY.len(),
            exclude_name_prefixes: vec![],
            pin_names: vec![],
            query: None,
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        assert!(!selected.is_empty());
        assert!(selected.iter().all(|t| t.http_read_role_eligible));
    }

    #[test]
    fn lane_filter_excludes_tools_outside_requested_lanes() {
        let reg = new_registry_arc();
        let allowed: HashSet<&'static str> = ["data", "interop"].into_iter().collect();
        let ctx = TurnContext {
            permission_mode: None,
            lanes: allowed.iter().copied().collect(),
            active_skill_id: None,
            max_tools: TOOL_REGISTRY.len(),
            exclude_name_prefixes: vec![],
            pin_names: vec![],
            query: None,
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        assert!(!selected.is_empty());
        assert!(selected.iter().all(|t| allowed.contains(t.product_lane)));
        // Sanity: the registry does contain tools outside {data, interop}
        // (e.g. ai/app/platform/workflow lanes), so this filter is doing real work.
        assert!(
            TOOL_REGISTRY
                .iter()
                .any(|t| !allowed.contains(t.product_lane))
        );
    }

    #[test]
    fn active_skill_allowlist_shrinks_the_selected_set() {
        let reg = new_registry_arc();
        install_restrictive_skill(&reg);
        let ctx_no_skill = TurnContext {
            permission_mode: None,
            lanes: vec![],
            active_skill_id: None,
            max_tools: TOOL_REGISTRY.len(),
            exclude_name_prefixes: vec![],
            pin_names: vec![],
            query: None,
        };
        let ctx_with_skill = TurnContext {
            active_skill_id: Some("narrow-skill".to_string()),
            ..ctx_no_skill.clone()
        };
        let without_skill = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx_no_skill);
        let with_skill = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx_with_skill);
        assert!(with_skill.len() < without_skill.len());
    }

    #[test]
    fn skill_infrastructure_tools_survive_restrictive_active_skill() {
        let reg = new_registry_arc();
        install_restrictive_skill(&reg);
        let ctx = TurnContext {
            permission_mode: None,
            lanes: vec![],
            active_skill_id: Some("narrow-skill".to_string()),
            max_tools: TOOL_REGISTRY.len(),
            exclude_name_prefixes: vec![],
            pin_names: vec![],
            query: None,
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        let selected_names: HashSet<&str> = selected.iter().map(|t| t.name).collect();
        for infra in ["vox_skill_list", "vox_skill_search"] {
            if TOOL_REGISTRY.iter().any(|t| t.name == infra) {
                assert!(
                    selected_names.contains(infra),
                    "expected skill-infrastructure tool {infra} to survive the active-skill filter"
                );
            }
        }
        for name in TOOL_REGISTRY.iter().map(|t| t.name) {
            if name.starts_with("vox_chat_") {
                assert!(
                    selected_names.contains(name),
                    "expected vox_chat_* tool {name} to survive the active-skill filter"
                );
            }
        }
    }

    /// Regression: an excluded prefix (e.g. `vox_chat_*` in the agent loop) must
    /// not consume a cap slot. Before the fix, excluding `vox_chat_*` happened
    /// AFTER `select_tools_for_turn`'s `.take(max_tools)`, so any `vox_chat_*`
    /// entries within the first `max_tools` registry-order entries would occupy a
    /// slot and then be discarded by the caller's post-filter — leaving turns
    /// with fewer than `max_tools` usable tools and potentially pushing a
    /// genuinely useful tool just past the cap boundary out of reach. Now the
    /// exclusion runs inside `select_tools_for_turn`, before the cap, so the
    /// full `max_tools` budget is always spent on genuinely-usable tools (up to
    /// however many survive every other filter).
    #[test]
    fn excluded_prefix_does_not_consume_a_cap_slot() {
        let reg = new_registry_arc();

        // Sanity: the real registry does contain vox_chat_* entries within
        // registry order, so this test exercises the bug it targets.
        assert!(
            TOOL_REGISTRY
                .iter()
                .any(|t| t.name.starts_with("vox_chat_")),
            "test assumption: TOOL_REGISTRY must contain at least one vox_chat_* tool"
        );

        // A small cap chosen so that, in plain registry order without exclusion,
        // at least one vox_chat_* tool would land inside the cap window (proving
        // the old post-filter bug would have shrunk the result below the cap).
        let small_cap = TOOL_REGISTRY
            .iter()
            .position(|t| t.name.starts_with("vox_chat_"))
            .map(|idx| idx + 1)
            .unwrap_or(TOOL_REGISTRY.len())
            .max(1);

        let ctx = TurnContext {
            permission_mode: None,
            lanes: vec![],
            active_skill_id: None,
            max_tools: small_cap,
            exclude_name_prefixes: vec!["vox_chat_"],
            pin_names: vec![],
            query: None,
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);

        assert!(
            selected.iter().all(|t| !t.name.starts_with("vox_chat_")),
            "excluded vox_chat_* tools must never appear in the result"
        );
        assert_eq!(
            selected.len(),
            small_cap,
            "the cap slot vacated by an excluded tool must be backfilled by the \
             next genuinely-usable tool, not left empty — got {} of {small_cap} \
             expected usable tools",
            selected.len()
        );
    }

    /// The three delegation tools the chat agent loop pins.
    const DELEGATION_TOOLS: [&str; 3] = ["vox_spawn_agent", "vox_submit_task", "vox_task_status"];

    /// Regression for the reachability bug this pin hatch exists to fix: on a
    /// default chat turn (`ai`+`app` lanes, cap 40) registry order is
    /// alphabetical, so `vox_spawn_agent` sat at candidate index ~166 of ~188
    /// and was never offered. Fails without `pin_names`.
    #[test]
    fn default_chat_turn_offers_the_delegation_tools_when_pinned() {
        let reg = new_registry_arc();
        let ctx = TurnContext {
            pin_names: DELEGATION_TOOLS.to_vec(),
            exclude_name_prefixes: vec!["vox_chat_"],
            ..TurnContext::default()
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        let names: HashSet<&str> = selected.iter().map(|t| t.name).collect();
        for tool in DELEGATION_TOOLS {
            assert!(
                names.contains(tool),
                "pinned delegation tool {tool} missing from a default chat turn's \
                 offered set ({} tools)",
                selected.len()
            );
        }
    }

    #[test]
    fn pinning_does_not_exceed_the_cap() {
        let reg = new_registry_arc();
        let unpinned = select_tools_for_turn(TOOL_REGISTRY, &reg, &TurnContext::default());
        let ctx = TurnContext {
            pin_names: DELEGATION_TOOLS.to_vec(),
            ..TurnContext::default()
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        assert_eq!(selected.len(), unpinned.len());
        assert_eq!(selected.len(), ctx.max_tools);
    }

    #[test]
    fn pinning_a_tool_already_inside_the_cap_does_not_duplicate_it() {
        let reg = new_registry_arc();
        let base = select_tools_for_turn(TOOL_REGISTRY, &reg, &TurnContext::default());
        let already_in = base[0].name;
        let ctx = TurnContext {
            pin_names: vec![already_in],
            ..TurnContext::default()
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        assert_eq!(
            selected.iter().filter(|t| t.name == already_in).count(),
            1,
            "pinned tool {already_in} appeared more than once"
        );
        assert_eq!(selected.len(), base.len());
    }

    #[test]
    fn pinning_is_applied_before_the_cap() {
        let reg = new_registry_arc();
        let ctx = TurnContext {
            max_tools: 1,
            pin_names: vec!["vox_spawn_agent"],
            ..TurnContext::default()
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "vox_spawn_agent");
    }

    /// The bug itself, pinned down so it cannot silently return.
    ///
    /// Without `pin_names`, a default chat turn's alphabetical truncation drops
    /// every delegation tool — which is why `run_agent_turn` pins them. If this
    /// ever starts failing it means the cap, the registry order, or the
    /// candidate count changed enough to make the pin redundant: verify that,
    /// then delete this test and the pin together. It is a canary for *why* the
    /// pin exists, not a requirement that the truncation stay broken.
    #[test]
    fn unpinned_chat_turn_cannot_reach_the_delegation_tools() {
        let reg = new_registry_arc();
        let ctx = TurnContext {
            exclude_name_prefixes: vec!["vox_chat_"],
            ..TurnContext::default()
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        let names: HashSet<&str> = selected.iter().map(|t| t.name).collect();
        for tool in DELEGATION_TOOLS {
            assert!(
                !names.contains(tool),
                "{tool} is now reachable unpinned — the alphabetical-truncation \
                 bug the pin works around may be gone; re-verify before removing \
                 the pin (offered set was {} tools)",
                selected.len()
            );
        }
    }

    #[test]
    fn delegation_tools_survive_a_restrictive_active_skill() {
        let reg = new_registry_arc();
        install_restrictive_skill(&reg);
        let ctx = TurnContext {
            active_skill_id: Some("narrow-skill".to_string()),
            pin_names: DELEGATION_TOOLS.to_vec(),
            ..TurnContext::default()
        };
        let names: HashSet<&str> = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx)
            .iter()
            .map(|t| t.name)
            .collect();
        for tool in DELEGATION_TOOLS {
            assert!(names.contains(tool), "{tool} removed by the active skill");
        }
    }

    #[test]
    fn ranked_tool_selection_recalls_late_alphabet_tools_across_all_eight_spokes_at_half_cap() {
        use vox_corpus::training::contract::OrganicSpokeKind;

        let reg = new_registry_arc();
        let unpinned_ctx = TurnContext {
            max_tools: DEFAULT_RANKED_MAX_TOOLS, // 20 tools (50% reduction vs 40)
            exclude_name_prefixes: vec!["vox_chat_"],
            pin_names: vec![],
            ..TurnContext::default()
        };

        let cases: [(&str, OrganicSpokeKind, &str); 8] = [
            (
                "Hey! Can you help me plan out the steps we should take first?",
                OrganicSpokeKind::AgentHarness,
                "vox_spawn_agent",
            ),
            (
                "Before we write code, can you check the knowledge base to see if @island is supported?",
                OrganicSpokeKind::McpToolChain,
                "vox_kb_search",
            ),
            (
                "Can you check what example .vox files are in the examples/golden folder and how big they are?",
                OrganicSpokeKind::StructuredShell,
                "vox_repo_status",
            ),
            (
                "I'm dictating this on my phone: um define actor build monitor on record count int to int",
                OrganicSpokeKind::SpeechToCode,
                "vox_validate_source",
            ),
            (
                "In that .vox file, can you add a TelemetryRow table and a record_sample workflow?",
                OrganicSpokeKind::VoxLang,
                "vox_validate_file",
            ),
            (
                "Now we need a fast Rust helper function sliding_window_max over a slice of numbers.",
                OrganicSpokeKind::RustWorkspace,
                "vox_run_tests",
            ),
            (
                "Let's save what we just built as a reusable skill so the rest of the team and other agents can use it.",
                OrganicSpokeKind::SkillCreation,
                "vox_skill_search",
            ),
            (
                "Finally, let's deploy what we just built across 3 GPU worker nodes with 24GB VRAM on our cluster!",
                OrganicSpokeKind::PopuliMesh,
                "vox_populi_local_status",
            ),
        ];

        // Prove baseline alphabetical selector (at 40 tools, unpinned) misses late-alphabet tools
        let alpha_ctx = TurnContext {
            max_tools: DEFAULT_MAX_TOOLS,
            exclude_name_prefixes: vec!["vox_chat_"],
            pin_names: vec![],
            ..TurnContext::default()
        };
        let alpha_selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &alpha_ctx);
        let alpha_names: HashSet<&str> = alpha_selected.iter().map(|t| t.name).collect();
        let mut alpha_hits = 0usize;
        for (_, _, required_tool) in &cases {
            if alpha_names.contains(required_tool) {
                alpha_hits += 1;
            }
        }
        assert_eq!(
            alpha_hits, 0,
            "baseline alphabetical 40-tool cut misses all 8 late-alphabet spoke tools"
        );

        // Prove spoke-ranked selector (at 20 tools, 50% smaller schema payload) hits 8/8 (100% recall)
        for (prompt, expected_spoke, required_tool) in cases {
            let (selected, classified_spoke, _mode) =
                select_tools_for_turn_ranked(TOOL_REGISTRY, &reg, &unpinned_ctx, prompt, None);
            assert_eq!(classified_spoke, expected_spoke);
            assert!(selected.len() <= DEFAULT_RANKED_MAX_TOOLS);
            let names: HashSet<&str> = selected.iter().map(|t| t.name).collect();
            assert!(
                names.contains(required_tool),
                "spoke {:?} ranked selection must include `{required_tool}` within top {} tools (got: {:?})",
                expected_spoke,
                DEFAULT_RANKED_MAX_TOOLS,
                selected.iter().map(|t| t.name).collect::<Vec<_>>()
            );
        }

        // Phase 163 (`SOTA-WRITE-PRIORITY-01..04`): Natural-language mutation prompts without
        // the literal word "edit" must rank `vox_apply_structured_edit` in the primary full-schema tier (`rank < 8`).
        for mutation_prompt in [
            "Fix the lifetime borrow error in crates/vox-orchestrator/src/models/select.rs",
            "Add a /health server route and counter component to src/main.vox",
            "Refactor the SKILL.md frontmatter for our new audit skill",
        ] {
            let (selected, _spoke, _mode) = select_tools_for_turn_ranked(
                TOOL_REGISTRY,
                &reg,
                &unpinned_ctx,
                mutation_prompt,
                None,
            );
            let edit_rank = selected
                .iter()
                .position(|t| t.name == "vox_apply_structured_edit")
                .expect("vox_apply_structured_edit must be included in top 20 tools");
            assert!(
                edit_rank < 8,
                "vox_apply_structured_edit must rank in the primary full-schema tier (< 8) for `{mutation_prompt}`, got rank {edit_rank}"
            );
        }
    }
    fn chat_turn(query: &str) -> TurnContext {
        TurnContext {
            exclude_name_prefixes: vec!["vox_chat_"],
            pin_names: DELEGATION_TOOLS.to_vec(),
            query: Some(query.to_string()),
            ..TurnContext::default()
        }
    }

    fn offered_names(query: &str) -> Vec<&'static str> {
        let reg = new_registry_arc();
        select_tools_for_turn(TOOL_REGISTRY, &reg, &chat_turn(query))
            .iter()
            .map(|t| t.name)
            .collect()
    }

    /// A question about a file must be offered `vox_repo_query_file` (lane
    /// `platform`), which alphabetical truncation of the `ai`/`app` lanes never
    /// reaches.
    #[test]
    fn file_questions_are_offered_a_file_reading_tool() {
        for prompt in [
            "does the file Cargo.toml exist in the repo root?",
            "how many lines are in crates/vox-cli/Cargo.toml?",
            "Use a tool to check whether Cargo.toml exists in the current directory, then report the result.",
            "Use a tool to read Cargo.toml in the current directory and report how many lines it has.",
        ] {
            let names = offered_names(prompt);
            assert!(
                names.contains(&"vox_repo_query_file"),
                "no file-reading tool offered for {prompt:?}: {names:?}"
            );
        }
    }

    #[test]
    fn file_question_selection_keeps_cap_and_pins() {
        let names = offered_names("does the file Cargo.toml exist in the repo root?");
        assert!(names.len() <= DEFAULT_MAX_TOOLS);
        for pin in DELEGATION_TOOLS {
            assert!(names.contains(&pin), "pin {pin} lost: {names:?}");
        }
        assert!(
            names.iter().all(|n| !n.starts_with("vox_chat_")),
            "{names:?}"
        );
    }

    #[test]
    fn relevance_admits_only_a_few_read_only_off_lane_tools() {
        for prompt in [
            "does the file Cargo.toml exist in the repo root?",
            "how many lines are in crates/vox-cli/Cargo.toml?",
            "read the file and the path and the repo and the history and the status",
        ] {
            let names = offered_names(prompt);
            let off_lane: Vec<&McpToolRegistryEntry> = TOOL_REGISTRY
                .iter()
                .filter(|e| names.contains(&e.name))
                .filter(|e| !["ai", "app"].contains(&e.product_lane))
                .filter(|e| !DELEGATION_TOOLS.contains(&e.name))
                .collect();
            assert!(
                off_lane.len() <= MAX_OFF_LANE_RELEVANT,
                "{prompt:?}: {:?}",
                off_lane.iter().map(|e| e.name).collect::<Vec<_>>()
            );
            assert!(
                off_lane
                    .iter()
                    .all(|e| e.side_effect_class == SideEffectClass::ReadOnly),
                "{prompt:?}: {:?}",
                off_lane.iter().map(|e| e.name).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn unrelated_turns_keep_the_plain_lane_selection() {
        let plain = offered_names("");
        for prompt in ["hello", "thanks!", "good morning, how are you?"] {
            assert_eq!(offered_names(prompt), plain, "{prompt:?}");
        }
    }

    #[test]
    fn file_tool_ranks_near_the_front_of_the_offered_set() {
        let names = offered_names("how many lines are in crates/vox-cli/Cargo.toml?");
        let pos = names
            .iter()
            .position(|n| *n == "vox_repo_query_file")
            .expect("offered");
        assert!(pos < DELEGATION_TOOLS.len() + 12, "ranked {pos}: {names:?}");
    }

    #[test]
    fn path_like_turns_do_not_pull_in_graph_path_tools() {
        let names = offered_names("how many lines are in crates/vox-cli/Cargo.toml?");
        assert!(!names.contains(&"vox_search_path"), "{names:?}");
    }

    #[test]
    fn offered_tools_always_have_provider_safe_names() {
        for prompt in [
            "parse the .vox file and show me its AST to inspect the compiler tree",
            "does the file Cargo.toml exist in the repo root?",
            "What is 17 times 23?",
        ] {
            for name in offered_names(prompt) {
                assert!(is_wire_safe_name(name), "{name} offered for {prompt:?}");
            }
        }
    }
}
