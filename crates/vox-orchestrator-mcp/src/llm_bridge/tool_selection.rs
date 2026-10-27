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
//! Filters are applied in a fixed order: permission -> profile availability ->
//! active-skill allowlist -> name exclusion, then ordering (pins, intent-triggered
//! families, lane-selected tools) and the cap. See [`select_tools_for_turn`].

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

/// Tools every agent chat turn pins inside the cap: the delegation tools, which sit
/// past the alphabetical cut, and `vox_tool_search`, the progressive-disclosure hatch
/// whose hits [`admit_discovered_tools`] adds to the turn.
pub const CHAT_TURN_PINS: &[&str] = &[
    "vox_spawn_agent",
    "vox_submit_task",
    "vox_task_status",
    "vox_tool_search",
];

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
    /// The user's request for this turn. When it matches an [`IntentFamily`]'s
    /// triggers, that family's core tools are offered ahead of the lane-selected
    /// set even when their lane is not in `lanes` (browser and desktop tools live
    /// in `interop`, which a plain chat turn does not otherwise see). `None`
    /// disables intent matching.
    pub query: Option<String>,
    /// Tools the caller's active capability profile can actually dispatch; `None`
    /// means unrestricted. Offering a tool the profile refuses only teaches the
    /// model to call something that fails, so this filter runs before the cap.
    pub available: Option<std::collections::BTreeSet<&'static str>>,
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
            available: None,
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
/// 2. Availability filter — keeps only `ctx.available` tools (no-op if `None`).
/// 3. Active-skill filter — drops any tool [`check_skill_tool_permission`]
///    denies for `ctx.active_skill_id` (skill-infrastructure tools such as
///    `vox_skill_list`/`vox_chat_*` are never denied by that function, so
///    they always survive this step regardless of the active skill).
/// 4. Name-prefix exclusion — drops any tool whose name starts with one of
///    `ctx.exclude_name_prefixes` (no-op if empty).
/// 5. Ordering — pinned tools first, then the core tools of every
///    [`IntentFamily`] `ctx.query` triggers (in family order), then the
///    remaining survivors whose `product_lane` is in `ctx.lanes`, by relevance then registry
///    order. The lane filter applies only to that last group: a triggered
///    family is an explicit relevance signal, the lane list a default.
///    When `ctx.query` is set, that last group also skips tools owned (by
///    [`IntentFamily::tool_prefix`]) by a family the request did not trigger, so
///    an in-lane browser tool is not offered for a desktop or code request.
/// 6. Cap — truncates to the first `ctx.max_tools`.
///
/// Every filter runs BEFORE the cap: excluded or unavailable tools never consume
/// a cap slot, and pinned and triggered tools are always inside it.
///
/// Apart from pins and intent families, the lane group is ranked by relevance to
/// `ctx.query`: each candidate is scored with the `vox_tool_search` scorer
/// (`crate::tool_search::score_entry`) over the turn's terms
/// (`crate::tool_search::turn_terms`), best first, ties and zero scores in
/// registry (alphabetical) order. So a turn with no signal keeps the plain lane
/// order, and one about files or repositories is offered the tools that name
/// them instead of whatever sorts first. Up to [`MAX_OFF_LANE_RELEVANT`]
/// read-only tools outside `ctx.lanes` whose *name* matches a turn term join the
/// ranking (e.g. `vox_repo_query_file`, lane `platform`, for "does the file X
/// exist"). Tools outside the offered set stay reachable through `vox_tool_search`
/// (see [`admit_discovered_tools`]).
#[must_use]
pub fn select_tools_for_turn(
    registry: &'static [McpToolRegistryEntry],
    skill_registry: &SkillRegistry,
    ctx: &TurnContext,
) -> Vec<&'static McpToolRegistryEntry> {
    let candidates: Vec<&'static McpToolRegistryEntry> = registry
        .iter()
        .filter(|entry| admissible(skill_registry, ctx, entry))
        .collect();
    let families: Vec<&'static IntentFamily> = ctx
        .query
        .as_deref()
        .map(triggered_families)
        .unwrap_or_default();
    // Without a request to judge (non-chat callers) lanes alone decide.
    let untriggered_prefixes: Vec<&'static str> = match ctx.query {
        Some(_) => INTENT_FAMILIES
            .iter()
            .filter(|f| !families.iter().any(|t| t.id == f.id))
            .map(|f| f.tool_prefix)
            .collect(),
        None => Vec::new(),
    };
    let mut out: Vec<&'static McpToolRegistryEntry> = Vec::with_capacity(ctx.max_tools);
    let mut push = |entry: &'static McpToolRegistryEntry| {
        if out.len() < ctx.max_tools && !out.iter().any(|e| e.name == entry.name) {
            out.push(entry);
        }
    };
    for entry in candidates
        .iter()
        .filter(|e| ctx.pin_names.contains(&e.name))
    {
        push(entry);
    }
    for name in families.iter().flat_map(|f| f.core_tools.iter().copied()) {
        if let Some(entry) = candidates.iter().find(|e| e.name == name) {
            push(entry);
        }
    }
    // Relevance: score every candidate the request leaves in play against the
    // turn's terms, so the cap cuts the least relevant tools instead of the
    // alphabetically last. A tool outside `ctx.lanes` is admitted only when it
    // names a term outright, is read-only, and ranks within the top
    // `MAX_OFF_LANE_RELEVANT` such tools. Zero scores keep registry order.
    let terms = ctx.query.as_deref().map(turn_terms).unwrap_or_default();
    let mut ranked: Vec<(u32, &'static McpToolRegistryEntry)> = Vec::new();
    let mut off_lane: Vec<(u32, &'static McpToolRegistryEntry)> = Vec::new();
    for entry in candidates
        .iter()
        .filter(|e| !untriggered_prefixes.iter().any(|p| e.name.starts_with(p)))
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
    ranked.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    for (_, entry) in ranked {
        push(entry);
    }
    out
}

/// Whether a tool name is acceptable on a provider wire (`[A-Za-z0-9_-]{1,128}`).
/// One bad name fails the entire request, so such tools are never offered.
fn is_wire_safe_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Every filter except lanes and the cap: the conditions under which a tool may be
/// offered at all in this turn.
fn admissible(
    skill_registry: &SkillRegistry,
    ctx: &TurnContext,
    entry: &McpToolRegistryEntry,
) -> bool {
    is_wire_safe_name(entry.name)
        && (!ctx.is_read_only() || entry.http_read_role_eligible)
        && ctx
            .available
            .as_ref()
            .is_none_or(|set| set.contains(entry.name))
        && check_skill_tool_permission(skill_registry, ctx.active_skill_id.as_deref(), entry.name)
            .is_none()
        && !ctx
            .exclude_name_prefixes
            .iter()
            .any(|prefix| entry.name.starts_with(prefix))
}

/// Tools a `vox_tool_search` call surfaced that this turn may add to its offered
/// set: registry entries passing every [`select_tools_for_turn`] filter except the
/// lane list, minus anything already offered, at most `limit`.
#[must_use]
pub fn admit_discovered_tools(
    registry: &'static [McpToolRegistryEntry],
    skill_registry: &SkillRegistry,
    ctx: &TurnContext,
    discovered: &[&str],
    already_offered: &[&str],
    limit: usize,
) -> Vec<&'static McpToolRegistryEntry> {
    let mut out: Vec<&'static McpToolRegistryEntry> = Vec::new();
    for name in discovered {
        if out.len() >= limit {
            break;
        }
        if already_offered.contains(name) || out.iter().any(|e| e.name == *name) {
            continue;
        }
        if let Some(entry) = registry.iter().find(|e| e.name == *name)
            && admissible(skill_registry, ctx, entry)
        {
            out.push(entry);
        }
    }
    out
}

/// A tool family the default lane list hides but a turn can explicitly ask for.
#[derive(Debug)]
pub struct IntentFamily {
    pub id: &'static str,
    /// Whole lowercase words; any one triggers the family.
    pub trigger_words: &'static [&'static str],
    /// Lowercase phrases matched on word boundaries.
    pub trigger_phrases: &'static [&'static str],
    /// Whether a URL-like token (`https://…`, `www.…`, `example.com`) triggers it.
    pub triggered_by_url: bool,
    /// Offered, in this order, when the family triggers.
    pub core_tools: &'static [&'static str],
    /// Every tool whose name starts with this prefix belongs to the family and is
    /// offered only on turns that trigger it (still reachable via `vox_tool_search`).
    pub tool_prefix: &'static str,
}

/// Families whose tools a plain chat turn would otherwise never see.
///
/// Triggers are deliberately concrete nouns: a false positive costs a handful of
/// extra offered tools (and every desktop action still parks for approval), a
/// false negative leaves the capability unreachable for the turn. The trigger
/// corpus in [`TRIGGER_CASES`] pins both directions.
pub const INTENT_FAMILIES: &[IntentFamily] = &[
    IntentFamily {
        id: "browser",
        tool_prefix: "vox_browser_",
        trigger_words: &[
            "browser",
            "website",
            "websites",
            "webpage",
            "webpages",
            "chrome",
            "chromium",
            "url",
            "urls",
            "localhost",
            "playwright",
            "captcha",
        ],
        trigger_phrases: &[
            "web page",
            "web site",
            "web app",
            "web form",
            "sign in to",
            "log in to",
            "login page",
            "checkout page",
            "in a tab",
        ],
        triggered_by_url: true,
        core_tools: &[
            "vox_browser_open",
            "vox_browser_goto",
            "vox_browser_snapshot",
            "vox_browser_click_ref",
            "vox_browser_fill_ref",
            "vox_browser_press",
            "vox_browser_text",
            "vox_browser_screenshot_viewport",
            "vox_browser_events",
            "vox_browser_handle_dialog",
            "vox_browser_wait_for",
            "vox_browser_list_pages",
            "vox_browser_close",
        ],
    },
    IntentFamily {
        id: "computer",
        tool_prefix: "vox_computer_",
        trigger_words: &[
            "desktop",
            "finder",
            "dock",
            "menubar",
            "calculator",
            "mouse",
            "gui",
            "macos",
            "screen",
            "onscreen",
        ],
        trigger_phrases: &[
            "menu bar",
            "system settings",
            "system preferences",
            "native app",
            "mac app",
            "desktop app",
            "computer use",
            "my mac",
            "my computer",
            "this computer",
            "control panel",
            "file explorer",
            "click on the",
            "on screen",
        ],
        triggered_by_url: false,
        core_tools: &[
            "vox_computer_snapshot",
            "vox_computer_act",
            "vox_computer_screenshot",
            "vox_computer_list_apps",
            "vox_computer_list_windows",
            "vox_computer_permissions",
        ],
    },
];

/// The [`INTENT_FAMILIES`] a request triggers, in table order.
#[must_use]
pub fn triggered_families(query: &str) -> Vec<&'static IntentFamily> {
    let lower = query.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .filter(|w| !w.is_empty())
        .collect();
    let normalized = format!(" {} ", words.join(" "));
    let has_url = lower.split_whitespace().any(looks_like_url);
    INTENT_FAMILIES
        .iter()
        .filter(|f| {
            (f.triggered_by_url && has_url)
                || f.trigger_words.iter().any(|t| words.contains(t))
                || f.trigger_phrases
                    .iter()
                    .any(|p| normalized.contains(&format!(" {p} ")))
        })
        .collect()
}

fn looks_like_url(token: &str) -> bool {
    const TLDS: &[&str] = &[
        "com", "org", "net", "io", "dev", "app", "ai", "co", "gov", "edu",
    ];
    let t = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != ':');
    if t.contains("://") || t.starts_with("www.") {
        return true;
    }
    let host = t.split('/').next().unwrap_or("");
    let mut labels = host.rsplit('.');
    let tld = labels.next().unwrap_or("");
    labels.next().is_some_and(|l| !l.is_empty()) && TLDS.contains(&tld) && !host.ends_with(".rs")
}

/// One labelled request for tool-family trigger checks.
#[derive(Debug, Clone, Copy)]
pub struct TriggerCase {
    /// Stable identifier, used as the live eval task id.
    pub id: &'static str,
    pub prompt: &'static str,
    /// The family whose tools must be offered (and that a model should pick
    /// first), or `None` when no browser/desktop tool should be offered.
    pub family: Option<&'static str>,
}

const fn case(id: &'static str, prompt: &'static str, family: Option<&'static str>) -> TriggerCase {
    TriggerCase { id, prompt, family }
}

/// Positive, negative, and confusable requests shared by the selection unit tests,
/// the hermetic `vox harness eval` gate, and the live first-tool-choice eval.
pub const TRIGGER_CASES: &[TriggerCase] = &[
    // Web pages.
    case(
        "browser-explicit-url",
        "Open https://example.com and tell me what the main heading says.",
        Some("browser"),
    ),
    case(
        "browser-bare-host",
        "Go to news.ycombinator.com and list the top three story titles.",
        Some("browser"),
    ),
    case(
        "browser-web-app-login",
        "Log in to the staging web app and check whether the dashboard loads.",
        Some("browser"),
    ),
    case(
        "browser-localhost-form",
        "Fill out the signup form on the website at localhost:3000 with test data.",
        Some("browser"),
    ),
    case(
        "browser-named",
        "In the browser, click the Accept cookies button.",
        Some("browser"),
    ),
    // Native desktop apps.
    case(
        "computer-calculator",
        "Use the Calculator app to compute 17 times 23.",
        Some("computer"),
    ),
    case(
        "computer-finder-folder",
        "Open Finder and create a new folder called Invoices on the desktop.",
        Some("computer"),
    ),
    case(
        "computer-system-settings",
        "Turn on dark mode in System Settings.",
        Some("computer"),
    ),
    case(
        "computer-screen-overview",
        "Take a screenshot of my screen and tell me which apps are open.",
        Some("computer"),
    ),
    case(
        "computer-notes-menu",
        "Click on the Save button in the menu bar of the Notes app.",
        Some("computer"),
    ),
    // Code and chat work: neither family.
    case(
        "none-source-path",
        "Fix the failing test in crates/vox-cli/src/main.rs.",
        None,
    ),
    case("none-readme", "Summarize the README for me.", None),
    case(
        "none-window-function",
        "Refactor the window function in the SQL query builder.",
        None,
    ),
    case(
        "none-app-state-machine",
        "Explain how the app's state machine handles retries.",
        None,
    ),
    case("none-arithmetic", "What is 17 times 23?", None),
];

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
            available: None,
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
            available: None,
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
            available: None,
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
            available: None,
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
            available: None,
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

    fn chat_turn(query: &str) -> TurnContext {
        TurnContext {
            pin_names: CHAT_TURN_PINS.to_vec(),
            exclude_name_prefixes: vec!["vox_chat_"],
            query: Some(query.to_string()),
            ..TurnContext::default()
        }
    }

    fn family_prefix(id: &str) -> &'static str {
        match id {
            "browser" => "vox_browser_",
            "computer" => "vox_computer_",
            other => panic!("unknown family {other}"),
        }
    }

    /// Exposure + selection level of the trigger corpus: a relevant request is
    /// offered the family's whole core tool set inside the cap, alongside the
    /// pinned delegation tools; an unrelated request is offered none of it.
    #[test]
    fn trigger_corpus_offers_exactly_the_expected_families() {
        let reg = new_registry_arc();
        for case in TRIGGER_CASES {
            let ctx = chat_turn(case.prompt);
            let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
            assert!(selected.len() <= ctx.max_tools, "{}", case.prompt);
            let names: HashSet<&str> = selected.iter().map(|t| t.name).collect();
            for tool in CHAT_TURN_PINS {
                assert!(names.contains(tool), "{tool} lost for {:?}", case.prompt);
            }
            for family in INTENT_FAMILIES {
                let offered = names
                    .iter()
                    .any(|n| n.starts_with(family_prefix(family.id)));
                if case.family == Some(family.id) {
                    for tool in family.core_tools {
                        assert!(
                            names.contains(tool),
                            "{tool} not offered for {:?}",
                            case.prompt
                        );
                    }
                } else {
                    assert!(
                        !offered,
                        "{} tools offered for unrelated request {:?}",
                        family.id, case.prompt
                    );
                }
            }
        }
    }

    #[test]
    fn every_core_tool_is_a_registered_tool_of_its_family() {
        for family in INTENT_FAMILIES {
            for tool in family.core_tools {
                assert!(tool.starts_with(family_prefix(family.id)), "{tool}");
                assert!(
                    TOOL_REGISTRY.iter().any(|e| e.name == *tool),
                    "{tool} missing from TOOL_REGISTRY"
                );
            }
        }
    }

    #[test]
    fn triggered_families_respect_read_only_mode() {
        let reg = new_registry_arc();
        let ctx = TurnContext {
            permission_mode: Some(READ_ONLY_PERMISSION_MODE.to_string()),
            ..chat_turn("Use the Calculator app to compute 2+2")
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        assert!(selected.iter().all(|t| t.http_read_role_eligible));
        assert!(!selected.iter().any(|t| t.name == "vox_computer_act"));
    }

    #[test]
    fn availability_filter_drops_tools_the_profile_cannot_dispatch() {
        let reg = new_registry_arc();
        let available: std::collections::BTreeSet<&'static str> = TOOL_REGISTRY
            .iter()
            .map(|e| e.name)
            .filter(|n| !n.starts_with("vox_computer_"))
            .collect();
        let ctx = TurnContext {
            available: Some(available.clone()),
            ..chat_turn("Open Finder on my desktop")
        };
        let selected = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx);
        assert!(!selected.is_empty());
        assert!(selected.iter().all(|t| available.contains(t.name)));
        assert_eq!(
            selected.len(),
            ctx.max_tools,
            "unavailable tools must not eat cap slots"
        );
    }

    #[test]
    fn triggered_family_fits_under_a_small_cap_after_pins() {
        let reg = new_registry_arc();
        let ctx = TurnContext {
            max_tools: 5,
            ..chat_turn("Open https://example.com")
        };
        let names: Vec<&str> = select_tools_for_turn(TOOL_REGISTRY, &reg, &ctx)
            .iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(
            names,
            [
                "vox_spawn_agent",
                "vox_submit_task",
                "vox_task_status",
                "vox_tool_search",
                "vox_browser_open",
            ]
        );
    }

    #[test]
    fn discovered_tools_are_admitted_once_and_bypass_only_the_lane_filter() {
        let reg = new_registry_arc();
        let ctx = chat_turn("anything");
        let admitted = admit_discovered_tools(
            TOOL_REGISTRY,
            &reg,
            &ctx,
            &[
                "vox_browser_open",
                "vox_browser_open",
                "vox_chat_message",
                "vox_not_a_tool",
                "vox_git_status",
                "vox_computer_snapshot",
            ],
            &["vox_git_status"],
            10,
        );
        let names: Vec<&str> = admitted.iter().map(|t| t.name).collect();
        assert_eq!(names, ["vox_browser_open", "vox_computer_snapshot"]);
        let limited = admit_discovered_tools(
            TOOL_REGISTRY,
            &reg,
            &ctx,
            &["vox_browser_open", "vox_computer_snapshot"],
            &[],
            1,
        );
        assert_eq!(limited.len(), 1);
    }

    #[test]
    fn url_detection_separates_hosts_from_source_paths() {
        for url in [
            "https://a.b/c",
            "www.example.org",
            "news.ycombinator.com",
            "(example.dev).",
        ] {
            assert!(looks_like_url(url), "{url}");
        }
        for not_url in [
            "main.rs",
            "crates/vox-cli/src/main.rs",
            "std::io",
            "e.g.",
            "v1.2.3",
            "README.md",
        ] {
            assert!(!looks_like_url(not_url), "{not_url}");
        }
    }

    #[test]
    fn trigger_case_ids_are_unique() {
        let ids: HashSet<&str> = TRIGGER_CASES.iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), TRIGGER_CASES.len());
    }

    fn offered_names(query: &str) -> Vec<&'static str> {
        let reg = new_registry_arc();
        select_tools_for_turn(TOOL_REGISTRY, &reg, &chat_turn(query))
            .iter()
            .map(|t| t.name)
            .collect()
    }

    /// Turns that ask about a file's existence or contents must be offered a
    /// read-only tool that can answer them. Before relevance ranking the offered
    /// set was the alphabetical head of the `ai`/`app` lanes, and
    /// `vox_repo_query_file` (lane `platform`) was never in it.
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

    /// Relevance ranking must not disturb the guarantees around it: the cap, the
    /// pins and the family withholding all still hold for a file question.
    #[test]
    fn file_question_selection_keeps_cap_pins_and_family_gating() {
        let names = offered_names("does the file Cargo.toml exist in the repo root?");
        assert!(names.len() <= DEFAULT_MAX_TOOLS);
        for pin in CHAT_TURN_PINS {
            assert!(names.contains(pin), "pin {pin} lost: {names:?}");
        }
        assert!(
            !names.iter().any(|n| n.starts_with("vox_browser_")
                || n.starts_with("vox_computer_")
                || n.starts_with("vox_chat_")),
            "{names:?}"
        );
    }

    /// An off-lane tool may be pulled in by relevance only if it is read-only, and
    /// only a handful of them: a turn must not turn into a platform-wide dump.
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
                .filter(|e| !CHAT_TURN_PINS.contains(&e.name))
                .collect();
            assert!(
                off_lane.len() <= MAX_OFF_LANE_RELEVANT,
                "{prompt:?}: {:?}",
                off_lane.iter().map(|e| e.name).collect::<Vec<_>>()
            );
            assert!(
                off_lane
                    .iter()
                    .all(|e| e.side_effect_class == vox_mcp_registry::SideEffectClass::ReadOnly),
                "{prompt:?}: {:?}",
                off_lane.iter().map(|e| e.name).collect::<Vec<_>>()
            );
        }
    }

    /// A turn with no signal keeps the plain lane order: same set as an empty turn.
    #[test]
    fn unrelated_turns_keep_the_plain_lane_selection() {
        let plain = offered_names("");
        for prompt in ["hello", "thanks!", "good morning, how are you?"] {
            assert_eq!(offered_names(prompt), plain, "{prompt:?}");
        }
    }

    #[test]
    fn turn_terms_drop_filler_and_expand_paths() {
        let terms =
            crate::tool_search::turn_terms("How many lines are in crates/vox-cli/Cargo.toml?");
        assert!(terms.contains(&"file".to_string()), "{terms:?}");
        assert!(terms.contains(&"line".to_string()), "{terms:?}");
        for filler in ["how", "many", "are", "in", "vox"] {
            assert!(
                !terms.contains(&filler.to_string()),
                "{filler} in {terms:?}"
            );
        }
    }

    /// The file tool must rank near the top of the offered set, not merely scrape
    /// in at the cap: a model weighs earlier tools, and a looser cap would hide it.
    #[test]
    fn file_tool_ranks_near_the_front_of_the_offered_set() {
        let names = offered_names("how many lines are in crates/vox-cli/Cargo.toml?");
        let pos = names
            .iter()
            .position(|n| *n == "vox_repo_query_file")
            .expect("offered");
        assert!(pos < CHAT_TURN_PINS.len() + 12, "ranked {pos}: {names:?}");
    }

    #[test]
    fn path_like_turns_do_not_pull_in_graph_path_tools() {
        let names = offered_names("how many lines are in crates/vox-cli/Cargo.toml?");
        assert!(!names.contains(&"vox_search_path"), "{names:?}");
    }

    /// Providers (Bedrock via OpenRouter) reject a whole request when any tool name
    /// is outside `[A-Za-z0-9_-]{1,128}`. `vox_compiler::ast_inspect` is such a
    /// name; relevance ranking would otherwise surface it for any turn that talks
    /// about parsing a file.
    #[test]
    fn offered_tools_always_have_provider_safe_names() {
        let reg = new_registry_arc();
        let mut prompts: Vec<&str> = TRIGGER_CASES.iter().map(|c| c.prompt).collect();
        prompts.extend([
            "parse the .vox file and show me its AST to inspect the compiler tree",
            "does the file Cargo.toml exist in the repo root?",
            "What is 17 times 23?",
        ]);
        for prompt in prompts {
            for name in offered_names(prompt) {
                assert!(is_wire_safe_name(name), "{name} offered for {prompt:?}");
            }
        }
        let admitted = admit_discovered_tools(
            TOOL_REGISTRY,
            &reg,
            &chat_turn("x"),
            &["vox_compiler::ast_inspect", "vox_git_status"],
            &[],
            10,
        );
        let names: Vec<&str> = admitted.iter().map(|t| t.name).collect();
        assert_eq!(names, ["vox_git_status"]);
    }
}
