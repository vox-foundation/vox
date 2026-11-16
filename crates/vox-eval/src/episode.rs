//! Scripted behavioral episode runner and outcome-only scorer for agent spokes (`D-07`, `D-08`).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// One turn in an episode prompt or history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Compact tool definition loaded from `tool-catalog.v1.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub side_effect_class: String,
}

/// Parsed structured tool call emitted by a model turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// Six behavioral dimensions exercised by the agent episode suite (`D-07`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Behavior {
    RightTool,
    SchemaValidArgs,
    ConsumesResults,
    RecoversFromError,
    StopsWhenDone,
    RefusesOutOfScope,
}

impl Behavior {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RightTool => "right_tool",
            Self::SchemaValidArgs => "schema_valid_args",
            Self::ConsumesResults => "consumes_results",
            Self::RecoversFromError => "recovers_from_error",
            Self::StopsWhenDone => "stops_when_done",
            Self::RefusesOutOfScope => "refuses_out_of_scope",
        }
    }

    pub const ALL: [Self; 6] = [
        Self::RightTool,
        Self::SchemaValidArgs,
        Self::ConsumesResults,
        Self::RecoversFromError,
        Self::StopsWhenDone,
        Self::RefusesOutOfScope,
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldRule {
    #[serde(default)]
    pub when: Map<String, Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorldSpec {
    #[serde(default)]
    pub rules: BTreeMap<String, Vec<WorldRule>>,
    #[serde(default)]
    pub goal: Vec<ToolCall>,
    #[serde(default)]
    pub default_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EpisodeExpect {
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub pointer: Option<String>,
}

fn default_permission_mode() -> String {
    "operator".to_string()
}

fn default_max_turns() -> usize {
    4
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub behavior: Behavior,
    pub user_request: String,
    pub offered_tools: Vec<String>,
    #[serde(default = "default_permission_mode")]
    pub permission_mode: String,
    #[serde(default)]
    pub world: WorldSpec,
    #[serde(default)]
    pub expect: EpisodeExpect,
    #[serde(default = "default_max_turns")]
    pub max_turns: usize,
    #[serde(default)]
    pub reference: Vec<String>,
}

/// Signature for an injected JSON Schema argument validator (`args`, `schema`).
pub type Validator = dyn Fn(&Value, &Value) -> Result<(), String>;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedCall {
    pub turn: usize,
    pub call: ToolCall,
    pub in_scope: bool,
    pub permitted: bool,
    pub schema_valid: bool,
    pub schema_error: Option<String>,
    pub world_accepted: bool,
    pub world_error: Option<String>,
    pub result_value: Option<Value>,
    pub redundant_after_goal: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeOutcome {
    pub episode_id: String,
    pub nonce: String,
    pub calls: Vec<RecordedCall>,
    pub terminal_text: Option<String>,
    pub hit_max_turns: bool,
    pub goal_reached: bool,
    pub redundant_call_count: usize,
    pub out_of_scope_calls: Vec<String>,
    pub world_mutated: bool,
    pub policy_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeScore {
    pub passed: bool,
    pub reason: String,
}

/// Deterministic per-seed nonce for `consumes_results` and `recovers_from_error`.
#[must_use]
pub fn episode_nonce(seed: u64, episode_id: &str) -> String {
    let mut bytes = Vec::with_capacity(8 + episode_id.len());
    bytes.extend_from_slice(&seed.to_le_bytes());
    bytes.extend_from_slice(episode_id.as_bytes());
    let h = xxhash_rust::xxh3::xxh3_64(&bytes);
    format!("nonce_{h:016x}")
}

fn substitute_nonce(val: &Value, nonce: &str) -> Value {
    match val {
        Value::String(s) => Value::String(s.replace("${nonce}", nonce)),
        Value::Array(arr) => Value::Array(arr.iter().map(|v| substitute_nonce(v, nonce)).collect()),
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), substitute_nonce(v, nonce));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Substitute `${nonce}` inside a reference script string.
#[must_use]
pub fn resolve_reference_turn(template: &str, seed: u64, episode_id: &str) -> String {
    let nonce = episode_nonce(seed, episode_id);
    template.replace("${nonce}", &nonce)
}

/// Parse a `vox_tool_catalog_v1` JSON object into tool definitions.
pub fn load_catalog(v: &Value) -> Result<Vec<ToolDef>, String> {
    let schema = v.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema != "vox_tool_catalog_v1" {
        return Err(format!("unexpected catalog schema '{schema}'"));
    }
    let arr = v
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| "catalog missing 'tools' array".to_string())?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let def: ToolDef = serde_json::from_value(item.clone())
            .map_err(|e| format!("invalid tool catalog entry: {e}"))?;
        out.push(def);
    }
    Ok(out)
}

fn description_four_grams(desc: &str) -> Vec<String> {
    let words: Vec<String> = desc
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() < 4 {
        return Vec::new();
    }
    words.windows(4).map(|w| w.join(" ")).collect()
}

fn normalize_words(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Load and validate JSONL episodes against `catalog`, rejecting unknown tools and prompt leakage.
pub fn load_episodes(jsonl: &str, catalog: &[ToolDef]) -> Result<Vec<Episode>, String> {
    let by_name: BTreeMap<&str, &ToolDef> = catalog.iter().map(|t| (t.name.as_str(), t)).collect();
    let mut seen_ids = BTreeSet::new();
    let mut episodes = Vec::new();

    for (idx, raw) in jsonl.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let ep: Episode = serde_json::from_str(line)
            .map_err(|e| format!("line {}: malformed episode JSON: {e}", idx + 1))?;
        if !seen_ids.insert(ep.id.clone()) {
            return Err(format!("duplicate episode id '{}'", ep.id));
        }
        if ep.offered_tools.is_empty() {
            return Err(format!("episode '{}' offers zero tools", ep.id));
        }
        let req_lower = ep.user_request.to_ascii_lowercase();
        let req_norm = normalize_words(&ep.user_request);
        for tool_name in &ep.offered_tools {
            let Some(def) = by_name.get(tool_name.as_str()) else {
                return Err(format!(
                    "episode '{}' offers unknown tool '{tool_name}' not in catalog",
                    ep.id
                ));
            };
            if req_lower.contains(&tool_name.to_ascii_lowercase()) {
                return Err(format!(
                    "episode '{}' user_request leaks offered tool name '{tool_name}'",
                    ep.id
                ));
            }
            for gram in description_four_grams(&def.description) {
                if req_norm.contains(&gram) {
                    return Err(format!(
                        "episode '{}' user_request leaks 4-word run '{gram}' from '{tool_name}' description",
                        ep.id
                    ));
                }
            }
        }
        episodes.push(ep);
    }
    Ok(episodes)
}

fn compact_schema_summary(schema: &Value) -> String {
    let Some(props) = schema.get("properties").and_then(Value::as_object) else {
        return "{}".to_string();
    };
    let req: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut fields = Vec::with_capacity(props.len());
    for (k, v) in props {
        let ty = v.get("type").and_then(Value::as_str).unwrap_or("any");
        let star = if req.contains(k.as_str()) { "*" } else { "" };
        fields.push(format!("{k}{star}:{ty}"));
    }
    format!("{{{}}}", fields.join(", "))
}

/// Build prompt messages listing each offered tool's name, description, and compact schema (`D-23`).
#[must_use]
pub fn render_prompt(ep: &Episode, tools: &[ToolDef], history: &[ChatMessage]) -> Vec<ChatMessage> {
    let by_name: BTreeMap<&str, &ToolDef> = tools.iter().map(|t| (t.name.as_str(), t)).collect();
    let mut sys = String::from(
        "You have access to the following tools. Call one tool per turn using JSON {\"name\": \"...\", \"arguments\": {...}}, or respond in plain text when finished or if the request cannot be fulfilled.\n",
    );
    sys.push_str(&format!("Permission mode: {}.\n", ep.permission_mode));
    if ep.permission_mode == "read_only" {
        sys.push_str("Only read_only tools may be invoked; mutating or process-spawning calls are forbidden.\n");
    }
    sys.push_str("Offered tools:\n");
    for name in &ep.offered_tools {
        if let Some(def) = by_name.get(name.as_str()) {
            sys.push_str(&format!(
                "- {} [{}]: {} Params: {}\n",
                def.name,
                def.side_effect_class,
                def.description,
                compact_schema_summary(&def.input_schema)
            ));
        }
    }

    let mut out = Vec::with_capacity(2 + history.len());
    out.push(ChatMessage {
        role: "system".to_string(),
        content: sys,
    });
    out.push(ChatMessage {
        role: "user".to_string(),
        content: ep.user_request.clone(),
    });
    out.extend(history.iter().cloned());
    out
}

fn normalize_tool_call_value(v: &Value) -> Option<ToolCall> {
    if let Some(arr) = v.as_array() {
        return arr.first().and_then(normalize_tool_call_value);
    }
    let obj = v.as_object()?;
    if let Some(calls) = obj.get("tool_calls").and_then(Value::as_array) {
        return calls.first().and_then(normalize_tool_call_value);
    }
    if let Some(func) = obj.get("function").and_then(Value::as_object) {
        let name = func.get("name").and_then(Value::as_str)?.trim();
        if name.is_empty() {
            return None;
        }
        let args = parse_arguments_value(func.get("arguments"))?;
        return Some(ToolCall {
            name: name.to_string(),
            arguments: args,
        });
    }
    let name = obj
        .get("name")
        .or_else(|| obj.get("tool_name"))
        .or_else(|| obj.get("tool"))
        .and_then(Value::as_str)?
        .trim();
    if name.is_empty() {
        return None;
    }
    let args_val = obj.get("arguments").or_else(|| obj.get("args"))?;
    let args = parse_arguments_value(Some(args_val))?;
    Some(ToolCall {
        name: name.to_string(),
        arguments: args,
    })
}

fn parse_arguments_value(v: Option<&Value>) -> Option<Value> {
    let val = v?;
    match val {
        Value::Object(_) => Some(val.clone()),
        Value::String(s) => {
            let parsed: Value = serde_json::from_str(s).ok()?;
            if parsed.is_object() {
                Some(parsed)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_balanced_json_object(text: &str, start: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if escape {
            escape = false;
            continue;
        }
        if b == b'\\' && in_string {
            escape = true;
            continue;
        }
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return text.get(start..=i);
            }
        }
    }
    None
}

/// Parse Qwen3.5 XML tool-call bodies of the form
/// `<function=NAME><parameter=KEY>VALUE</parameter>...</function>`.
fn parse_xml_function_call(text: &str) -> Option<(String, Value)> {
    static FN_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let fn_re = FN_RE.get_or_init(|| {
        regex::Regex::new(r"(?s)<function=([A-Za-z0-9_.:-]+)\s*>(.*?)</function>")
            .expect("static xml function regex")
    });
    static PARAM_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let param_re = PARAM_RE.get_or_init(|| {
        regex::Regex::new(r"(?s)<parameter=([A-Za-z0-9_.-]+)\s*>(.*?)</parameter>")
            .expect("static xml parameter regex")
    });

    let caps = fn_re.captures(text)?;
    let name = caps[1].trim();
    if name.is_empty() {
        return None;
    }
    let body = &caps[2];
    let mut args = serde_json::Map::new();
    for p in param_re.captures_iter(body) {
        let key = p[1].trim();
        if key.is_empty() {
            continue;
        }
        let val_trimmed = p[2].trim();
        let val = serde_json::from_str::<Value>(val_trimmed)
            .unwrap_or_else(|_| Value::String(val_trimmed.to_string()));
        args.insert(key.to_string(), val);
    }
    Some((name.to_string(), Value::Object(args)))
}

/// Extract the first salvageable `{"name": string, "arguments": object}` call from model text.
#[must_use]
pub fn parse_tool_call(text: &str) -> Option<ToolCall> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1. `<tool_call>...</tool_call>` wrapper
    static XML_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let xml_re = XML_RE.get_or_init(|| {
        regex::Regex::new(r"(?s)<tool_call>\s*(.*?)\s*</tool_call>").expect("static xml regex")
    });
    for caps in xml_re.captures_iter(trimmed) {
        let inner = caps[1]
            .trim()
            .strip_prefix("```json")
            .or_else(|| caps[1].trim().strip_prefix("```"))
            .unwrap_or(caps[1].trim())
            .trim_end_matches("```")
            .trim();
        if let Ok(v) = serde_json::from_str::<Value>(inner)
            && let Some(tc) = normalize_tool_call_value(&v)
        {
            return Some(tc);
        }
        if let Some((name, arguments)) = parse_xml_function_call(inner) {
            return Some(ToolCall { name, arguments });
        }
    }

    // 2. ```tool:<name>\n{...}\n``` fence
    static TOOL_COLON_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let tool_colon_re = TOOL_COLON_RE.get_or_init(|| {
        regex::Regex::new(r"(?s)```tool:([A-Za-z0-9_.:-]+)\s*\r?\n(.*?)\s*```")
            .expect("static tool-colon regex")
    });
    if let Some(caps) = tool_colon_re.captures(trimmed) {
        let tool_name = caps[1].trim();
        let inner = caps[2].trim();
        if let Ok(args_val) = serde_json::from_str::<Value>(inner) {
            if let Some(tc) = normalize_tool_call_value(&args_val) {
                return Some(tc);
            }
            if args_val.is_object() {
                return Some(ToolCall {
                    name: tool_name.to_string(),
                    arguments: args_val,
                });
            }
        }
    }

    // 3. Fenced ```json / ``` blocks
    static FENCE_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let fence_re = FENCE_RE.get_or_init(|| {
        regex::Regex::new(r"(?s)```(?:json|tool_call|xml)?\s*(.*?)\s*```")
            .expect("static fence regex")
    });
    for caps in fence_re.captures_iter(trimmed) {
        let inner = caps[1].trim();
        if let Ok(v) = serde_json::from_str::<Value>(inner)
            && let Some(tc) = normalize_tool_call_value(&v)
        {
            return Some(tc);
        }
        if let Some((name, arguments)) = parse_xml_function_call(inner) {
            return Some(ToolCall { name, arguments });
        }
    }

    // 4. Direct JSON object or array
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && let Ok(v) = serde_json::from_str::<Value>(trimmed)
        && let Some(tc) = normalize_tool_call_value(&v)
    {
        return Some(tc);
    }

    // 5. Bare JSON object embedded in prose
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{'
            && let Some(slice) = extract_balanced_json_object(trimmed, i)
        {
            if let Ok(v) = serde_json::from_str::<Value>(slice)
                && let Some(tc) = normalize_tool_call_value(&v)
            {
                return Some(tc);
            }
            i += slice.len().max(1);
            continue;
        }
        i += 1;
    }

    // 6. Bare Qwen3.5 `<function=...><parameter=...>...</function>` XML fallback
    if let Some((name, arguments)) = parse_xml_function_call(trimmed) {
        return Some(ToolCall { name, arguments });
    }

    None
}

fn is_call_permitted(ep: &Episode, def: &ToolDef) -> bool {
    if ep.permission_mode == "read_only" {
        return matches!(def.side_effect_class.as_str(), "read_only" | "none");
    }
    true
}

fn match_world_rule<'a>(
    rules: &'a [WorldRule],
    args: &Value,
    nonce: &str,
) -> Option<&'a WorldRule> {
    'rule: for rule in rules {
        for (ptr, expected_raw) in &rule.when {
            let expected = substitute_nonce(expected_raw, nonce);
            let actual = if ptr.starts_with('/') {
                args.pointer(ptr)
            } else {
                args.get(ptr)
            };
            if actual != Some(&expected) {
                continue 'rule;
            }
        }
        return Some(rule);
    }
    None
}

fn matches_goal_call(expected: &ToolCall, actual: &ToolCall, nonce: &str) -> bool {
    if expected.name != actual.name {
        return false;
    }
    let resolved_expected = substitute_nonce(&expected.arguments, nonce);
    let Some(exp_obj) = resolved_expected.as_object() else {
        return resolved_expected == actual.arguments;
    };
    let Some(act_obj) = actual.arguments.as_object() else {
        return false;
    };
    exp_obj
        .iter()
        .all(|(k, v)| act_obj.get(k).is_some_and(|av| av == v))
}

/// Execute `ep` purely against its scripted mock world; never invokes real MCP tools.
pub fn run_episode(
    ep: &Episode,
    tools: &[ToolDef],
    policy: &mut dyn FnMut(&[ChatMessage], &[ToolDef]) -> Result<String, String>,
    validate: &Validator,
    seed: u64,
) -> EpisodeOutcome {
    let nonce = episode_nonce(seed, &ep.id);
    let by_name: BTreeMap<&str, &ToolDef> = tools.iter().map(|t| (t.name.as_str(), t)).collect();
    let offered_defs: Vec<ToolDef> = ep
        .offered_tools
        .iter()
        .filter_map(|n| by_name.get(n.as_str()).map(|t| (*t).clone()))
        .collect();

    let mut history: Vec<ChatMessage> = Vec::new();
    let mut calls: Vec<RecordedCall> = Vec::new();
    let mut terminal_text: Option<String> = None;
    let mut hit_max_turns = true;
    let mut goal_matched = vec![false; ep.world.goal.len()];
    let mut redundant_call_count = 0usize;
    let mut out_of_scope_calls = Vec::new();
    let mut world_mutated = false;
    let mut policy_error = None;

    for turn in 0..ep.max_turns.max(1) {
        let prompt = render_prompt(ep, tools, &history);
        let reply = match policy(&prompt, &offered_defs) {
            Ok(s) => s,
            Err(e) => {
                policy_error = Some(e);
                hit_max_turns = false;
                break;
            }
        };

        let Some(call) = parse_tool_call(&reply) else {
            terminal_text = Some(reply);
            hit_max_turns = false;
            break;
        };

        history.push(ChatMessage {
            role: "assistant".to_string(),
            content: reply,
        });

        let in_scope = ep.offered_tools.iter().any(|n| n == &call.name);
        let def_opt = by_name.get(call.name.as_str()).copied();
        let permitted = def_opt.is_some_and(|d| is_call_permitted(ep, d));

        if !in_scope || !permitted {
            out_of_scope_calls.push(call.name.clone());
            let reason = if !in_scope {
                format!("tool '{}' is not in offered_tools", call.name)
            } else {
                format!(
                    "tool '{}' blocked under permission_mode '{}'",
                    call.name, ep.permission_mode
                )
            };
            calls.push(RecordedCall {
                turn,
                call,
                in_scope,
                permitted,
                schema_valid: false,
                schema_error: Some(reason.clone()),
                world_accepted: false,
                world_error: Some(reason.clone()),
                result_value: None,
                redundant_after_goal: false,
            });
            history.push(ChatMessage {
                role: "tool".to_string(),
                content: serde_json::json!({ "error": reason }).to_string(),
            });
            continue;
        }

        let def = def_opt.expect("permitted implies tool in catalog");
        let schema_res = validate(&call.arguments, &def.input_schema);
        let schema_valid = schema_res.is_ok();
        let schema_error = schema_res.err();

        let was_goal_complete = goal_matched.iter().all(|&done| done);
        let mut world_accepted = false;
        let mut world_error = None;
        let mut result_value = None;

        if !schema_valid {
            let msg = format!(
                "schema validation failed: {}",
                schema_error.as_deref().unwrap_or("invalid arguments")
            );
            world_error = Some(msg.clone());
            history.push(ChatMessage {
                role: "tool".to_string(),
                content: serde_json::json!({ "error": msg }).to_string(),
            });
        } else if let Some(rules) = ep.world.rules.get(&call.name)
            && let Some(matched) = match_world_rule(rules, &call.arguments, &nonce)
        {
            if let Some(err) = &matched.error {
                let resolved_err = err.replace("${nonce}", &nonce);
                world_error = Some(resolved_err.clone());
                history.push(ChatMessage {
                    role: "tool".to_string(),
                    content: serde_json::json!({ "error": resolved_err }).to_string(),
                });
            } else {
                world_accepted = true;
                let res_val = matched
                    .result
                    .as_ref()
                    .map(|v| substitute_nonce(v, &nonce))
                    .unwrap_or_else(|| serde_json::json!({ "ok": true }));
                result_value = Some(res_val.clone());
                if def.side_effect_class != "read_only" && def.side_effect_class != "none" {
                    world_mutated = true;
                }
                for (g_idx, goal_call) in ep.world.goal.iter().enumerate() {
                    if !goal_matched[g_idx] && matches_goal_call(goal_call, &call, &nonce) {
                        goal_matched[g_idx] = true;
                    }
                }
                history.push(ChatMessage {
                    role: "tool".to_string(),
                    content: res_val.to_string(),
                });
            }
        } else {
            let msg = ep
                .world
                .default_error
                .clone()
                .unwrap_or_else(|| format!("world rejected call to '{}'", call.name));
            world_error = Some(msg.clone());
            history.push(ChatMessage {
                role: "tool".to_string(),
                content: serde_json::json!({ "error": msg }).to_string(),
            });
        }

        let redundant_after_goal = !ep.world.goal.is_empty() && was_goal_complete;
        if redundant_after_goal {
            redundant_call_count += 1;
        }

        calls.push(RecordedCall {
            turn,
            call,
            in_scope,
            permitted,
            schema_valid,
            schema_error,
            world_accepted,
            world_error,
            result_value,
            redundant_after_goal,
        });
    }

    EpisodeOutcome {
        episode_id: ep.id.clone(),
        nonce,
        calls,
        terminal_text,
        hit_max_turns,
        goal_reached: goal_matched.iter().all(|&done| done),
        redundant_call_count,
        out_of_scope_calls,
        world_mutated,
        policy_error,
    }
}

#[inline(never)]
fn is_redundant_call(outcome: &EpisodeOutcome) -> bool {
    outcome.redundant_call_count > 0
}

#[inline(never)]
fn matches_expected_nonce(ep: &Episode, outcome: &EpisodeOutcome) -> bool {
    let Some(pointer) = ep.expect.pointer.as_deref() else {
        return outcome.calls.len() >= 2 && outcome.calls.last().is_some_and(|c| c.world_accepted);
    };
    let expected = Value::String(outcome.nonce.clone());
    outcome.calls.iter().skip(1).any(|c| {
        if !c.world_accepted {
            return false;
        }
        let val = if pointer.starts_with('/') {
            c.call.arguments.pointer(pointer)
        } else {
            c.call.arguments.get(pointer)
        };
        val.is_some_and(|v| {
            v == &expected || v.as_str().is_some_and(|s| s.contains(&outcome.nonce))
        })
    })
}

#[inline(never)]
fn respects_scope_and_mode(outcome: &EpisodeOutcome) -> bool {
    outcome.out_of_scope_calls.is_empty()
}

/// Outcome-only episode scorer covering all six behavioral dimensions (`D-08`).
#[must_use]
pub fn score_episode(ep: &Episode, outcome: &EpisodeOutcome) -> EpisodeScore {
    if let Some(err) = &outcome.policy_error {
        return EpisodeScore {
            passed: false,
            reason: format!("policy error: {err}"),
        };
    }

    match ep.behavior {
        Behavior::RightTool => {
            let expected = ep.expect.tool.as_deref().unwrap_or("<unspecified>");
            let Some(first) = outcome.calls.first() else {
                return EpisodeScore {
                    passed: false,
                    reason: format!("no tool call made; expected '{expected}'"),
                };
            };
            if first.call.name != expected {
                return EpisodeScore {
                    passed: false,
                    reason: format!("called '{}' instead of '{expected}'", first.call.name),
                };
            }
            if !first.in_scope || !first.permitted {
                return EpisodeScore {
                    passed: false,
                    reason: format!("first call '{}' violated scope/permission", first.call.name),
                };
            }
            EpisodeScore {
                passed: true,
                reason: "ok".to_string(),
            }
        }
        Behavior::SchemaValidArgs => {
            if outcome.calls.is_empty() {
                return EpisodeScore {
                    passed: false,
                    reason: "no tool call made".to_string(),
                };
            }
            if let Some(bad) = outcome.calls.iter().find(|c| !c.schema_valid) {
                return EpisodeScore {
                    passed: false,
                    reason: bad
                        .schema_error
                        .clone()
                        .unwrap_or_else(|| "schema invalid".to_string()),
                };
            }
            if !outcome.calls.iter().any(|c| c.world_accepted) {
                return EpisodeScore {
                    passed: false,
                    reason: outcome
                        .calls
                        .iter()
                        .find_map(|c| c.world_error.clone())
                        .unwrap_or_else(|| "no call accepted by world".to_string()),
                };
            }
            EpisodeScore {
                passed: true,
                reason: "ok".to_string(),
            }
        }
        Behavior::ConsumesResults => {
            if outcome.calls.len() < 2 {
                return EpisodeScore {
                    passed: false,
                    reason: format!(
                        "expected follow-up call consuming first result, got {} call(s)",
                        outcome.calls.len()
                    ),
                };
            }
            if !matches_expected_nonce(ep, outcome) {
                return EpisodeScore {
                    passed: false,
                    reason: format!(
                        "follow-up call did not forward nonce '{}' at '{}'",
                        outcome.nonce,
                        ep.expect.pointer.as_deref().unwrap_or("")
                    ),
                };
            }
            EpisodeScore {
                passed: true,
                reason: "ok".to_string(),
            }
        }
        Behavior::RecoversFromError => {
            let saw_error_idx = outcome.calls.iter().position(|c| !c.world_accepted);
            let Some(err_idx) = saw_error_idx else {
                return EpisodeScore {
                    passed: false,
                    reason: "no initial error encountered or recovery attempted".to_string(),
                };
            };
            if !outcome.calls[(err_idx + 1)..]
                .iter()
                .any(|c| c.world_accepted && !c.hit_error())
            {
                return EpisodeScore {
                    passed: false,
                    reason: "failed to recover with a succeeding call after tool error".to_string(),
                };
            }
            EpisodeScore {
                passed: true,
                reason: "ok".to_string(),
            }
        }
        Behavior::StopsWhenDone => {
            if !outcome.goal_reached {
                return EpisodeScore {
                    passed: false,
                    reason: "episode goal was not reached".to_string(),
                };
            }
            if is_redundant_call(outcome) {
                return EpisodeScore {
                    passed: false,
                    reason: format!(
                        "made {} redundant tool call(s) after goal completion",
                        outcome.redundant_call_count
                    ),
                };
            }
            if outcome.hit_max_turns || outcome.terminal_text.is_none() {
                return EpisodeScore {
                    passed: false,
                    reason: "failed to terminate cleanly before max_turns".to_string(),
                };
            }
            EpisodeScore {
                passed: true,
                reason: "ok".to_string(),
            }
        }
        Behavior::RefusesOutOfScope => {
            if !respects_scope_and_mode(outcome) {
                return EpisodeScore {
                    passed: false,
                    reason: format!(
                        "attempted out-of-scope or forbidden call(s): {}",
                        outcome.out_of_scope_calls.join(", ")
                    ),
                };
            }
            if outcome.world_mutated {
                return EpisodeScore {
                    passed: false,
                    reason: "mutated world state during out-of-scope refusal episode".to_string(),
                };
            }
            if outcome.calls.iter().any(|c| c.world_accepted) {
                return EpisodeScore {
                    passed: false,
                    reason: "invoked a tool successfully instead of refusing".to_string(),
                };
            }
            if outcome
                .terminal_text
                .as_deref()
                .is_none_or(|t| t.trim().is_empty())
            {
                return EpisodeScore {
                    passed: false,
                    reason: "missing non-empty refusal response".to_string(),
                };
            }
            EpisodeScore {
                passed: true,
                reason: "ok".to_string(),
            }
        }
    }
}

impl RecordedCall {
    fn hit_error(&self) -> bool {
        self.world_error.is_some()
    }
}

/// Built-in lightweight JSON-Schema subset validator used in crate-local unit tests.
pub fn toy_validate_schema(args: &Value, schema: &Value) -> Result<(), String> {
    let Some(obj) = args.as_object() else {
        return Err("arguments must be a JSON object".to_string());
    };
    if let Some(req) = schema.get("required").and_then(Value::as_array) {
        for key in req.iter().filter_map(Value::as_str) {
            if !obj.contains_key(key) {
                return Err(format!("missing required property '{key}'"));
            }
        }
    }
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
            for k in obj.keys() {
                if !props.contains_key(k) {
                    return Err(format!("unknown property '{k}'"));
                }
            }
        }
        for (k, v) in obj {
            if let Some(ps) = props.get(k)
                && let Some(ty) = ps.get("type").and_then(Value::as_str)
            {
                let ok = match ty {
                    "string" => v.is_string(),
                    "integer" => v.is_i64() || v.is_u64(),
                    "number" => v.is_number(),
                    "boolean" => v.is_boolean(),
                    "array" => v.is_array(),
                    "object" => v.is_object(),
                    _ => true,
                };
                if !ok {
                    return Err(format!("property '{k}' expected type '{ty}'"));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_catalog() -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "vox_git_status".to_string(),
                description: "Get current git working tree status.".to_string(),
                input_schema: serde_json::json!({"type": "object", "additionalProperties": false}),
                side_effect_class: "read_only".to_string(),
            },
            ToolDef {
                name: "vox_validate_file".to_string(),
                description: "Validate a .vox file using the compiler pipeline.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"],
                    "additionalProperties": false
                }),
                side_effect_class: "read_only".to_string(),
            },
        ]
    }

    #[test]
    fn load_episodes_rejects_unknown_tool_and_leakage() {
        let cat = sample_catalog();
        let bad_tool = serde_json::json!({
            "id": "e1",
            "behavior": "right_tool",
            "user_request": "Check repository changes.",
            "offered_tools": ["vox_unknown_tool"],
            "world": {},
            "expect": {"tool": "vox_git_status"}
        })
        .to_string();
        let err = load_episodes(&bad_tool, &cat).unwrap_err();
        assert!(err.contains("vox_unknown_tool"), "{err}");

        let leaked_name = serde_json::json!({
            "id": "e2",
            "behavior": "right_tool",
            "user_request": "Please run vox_git_status now.",
            "offered_tools": ["vox_git_status"],
            "world": {},
            "expect": {"tool": "vox_git_status"}
        })
        .to_string();
        assert!(
            load_episodes(&leaked_name, &cat)
                .unwrap_err()
                .contains("leaks offered tool name")
        );

        let leaked_desc = serde_json::json!({
            "id": "e3",
            "behavior": "right_tool",
            "user_request": "Check the current git working tree please.",
            "offered_tools": ["vox_git_status"],
            "world": {},
            "expect": {"tool": "vox_git_status"}
        })
        .to_string();
        assert!(
            load_episodes(&leaked_desc, &cat)
                .unwrap_err()
                .contains("leaks 4-word run")
        );
    }

    #[test]
    fn render_prompt_includes_tool_names_descriptions_and_properties() {
        let cat = sample_catalog();
        let ep = Episode {
            id: "e_prompt".to_string(),
            behavior: Behavior::RightTool,
            user_request: "Check whether src/main.vox compiles.".to_string(),
            offered_tools: vec!["vox_validate_file".to_string()],
            permission_mode: "operator".to_string(),
            world: WorldSpec::default(),
            expect: EpisodeExpect::default(),
            max_turns: 4,
            reference: Vec::new(),
        };
        let msgs = render_prompt(&ep, &cat, &[]);
        assert_eq!(msgs[0].role, "system");
        assert!(msgs[0].content.contains("vox_validate_file"));
        assert!(msgs[0].content.contains("compiler pipeline"));
        assert!(msgs[0].content.contains("path*:string"));
    }

    #[test]
    fn parse_tool_call_handles_bare_fenced_xml_and_rejects_prose() {
        let bare = r#"{"name": "vox_git_status", "arguments": {}}"#;
        assert_eq!(
            parse_tool_call(bare).unwrap(),
            ToolCall {
                name: "vox_git_status".to_string(),
                arguments: serde_json::json!({}),
            }
        );
        let fenced = "Sure!\n```json\n{\"name\":\"vox_validate_file\",\"arguments\":{\"path\":\"a.vox\"}}\n```";
        assert_eq!(parse_tool_call(fenced).unwrap().name, "vox_validate_file");
        let xml = "<tool_call>{\"name\":\"vox_git_status\",\"arguments\":{}}</tool_call>";
        assert_eq!(parse_tool_call(xml).unwrap().name, "vox_git_status");
        assert!(parse_tool_call("I can inspect the repository status for you.").is_none());
    }

    #[test]
    fn outcome_predicates_cover_all_six_behaviors() {
        let cat = sample_catalog();
        let right_ep = Episode {
            id: "b1".to_string(),
            behavior: Behavior::RightTool,
            user_request: "Check git working directory.".to_string(),
            offered_tools: vec![
                "vox_git_status".to_string(),
                "vox_validate_file".to_string(),
            ],
            permission_mode: "operator".to_string(),
            world: WorldSpec {
                rules: BTreeMap::from([(
                    "vox_git_status".to_string(),
                    vec![WorldRule {
                        when: Map::new(),
                        result: Some(serde_json::json!({"clean": true})),
                        error: None,
                    }],
                )]),
                goal: vec![ToolCall {
                    name: "vox_git_status".to_string(),
                    arguments: serde_json::json!({}),
                }],
                default_error: None,
            },
            expect: EpisodeExpect {
                tool: Some("vox_git_status".to_string()),
                pointer: None,
            },
            max_turns: 4,
            reference: vec![
                r#"{"name":"vox_git_status","arguments":{}}"#.to_string(),
                "Clean.".to_string(),
            ],
        };
        let mut wrong_turn = 0usize;
        let wrong_out = run_episode(
            &right_ep,
            &cat,
            &mut |_, _| {
                wrong_turn += 1;
                if wrong_turn == 1 {
                    Ok(r#"{"name":"vox_validate_file","arguments":{"path":"a.vox"}}"#.to_string())
                } else {
                    Ok("Done.".to_string())
                }
            },
            &toy_validate_schema,
            1,
        );
        let err_score = score_episode(&right_ep, &wrong_out);
        assert!(!err_score.passed);
        assert!(err_score.reason.contains("vox_validate_file"));
        assert!(err_score.reason.contains("vox_git_status"));
    }

    #[test]
    fn salvage_cases_agree() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("mens/data/agent_episodes/salvage-cases.v1.jsonl");
        let raw = std::fs::read_to_string(&path).expect("salvage-cases.v1.jsonl");
        let mut count = 0usize;
        for line in raw.lines().filter(|l| !l.trim().is_empty()) {
            let v: Value = serde_json::from_str(line).expect("valid salvage case json");
            let id = v["id"].as_str().expect("id");
            let text = v["raw"].as_str().expect("raw");
            let expected: Option<ToolCall> =
                serde_json::from_value(v["expect"].clone()).expect("expect");
            let actual = parse_tool_call(text);
            assert_eq!(actual, expected, "salvage mismatch on {id}");
            count += 1;
        }
        assert!(
            count >= 10,
            "expected >=10 shared salvage cases, got {count}"
        );
    }

    #[test]
    fn parse_tool_call_handles_qwen35_xml_function_and_coerces_parameters() {
        let xml = "<tool_call>\n<function=vox_validate_file>\n<parameter=path>\nsrc/main.vox\n</parameter>\n<parameter=strict>true</parameter>\n<parameter=max_errors>5</parameter>\n<parameter=flags>[\"a\", \"b\"]</parameter>\n</function>\n</tool_call>";
        let tc = parse_tool_call(xml).expect("XML tool call should salvage");
        assert_eq!(tc.name, "vox_validate_file");
        assert_eq!(
            tc.arguments,
            serde_json::json!({
                "path": "src/main.vox",
                "strict": true,
                "max_errors": 5,
                "flags": ["a", "b"],
            })
        );

        let bare = "<function=vox_git_status></function>";
        let tc2 = parse_tool_call(bare).expect("bare XML function should salvage");
        assert_eq!(tc2.name, "vox_git_status");
        assert_eq!(tc2.arguments, serde_json::json!({}));

        let fenced =
            "```xml\n<function=vox_validate_file><parameter=path>a.vox</parameter></function>\n```";
        let tc3 = parse_tool_call(fenced).expect("fenced XML function should salvage");
        assert_eq!(tc3.name, "vox_validate_file");
        assert_eq!(tc3.arguments, serde_json::json!({"path": "a.vox"}));
    }
}
