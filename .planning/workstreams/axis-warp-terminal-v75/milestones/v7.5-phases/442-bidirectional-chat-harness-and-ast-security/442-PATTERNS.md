# Phase 442: Bidirectional Chat Harness & AST Security - Pattern Map

**Mapped:** 2026-11-14
**Files analyzed:** 32 (8 new Rust modules + 21 modified Rust/TS files + 3 modified YAML contracts)
**Analogs found:** 31 / 32 (1 novel heuristic module with partial codebase analogs)

## File Classification

| New / Modified File | Wave | Role | Data Flow | Current LoC (Total / Pre-Test) & `god_object` Status | Closest Analog | Match Quality |
|---|---|---|---|---|---|---|
| `crates/vox-redact/src/lib.rs` (mod) | W1 | utility | transform | 204 / 119 (~90 LoC) — Safe | `crates/vox-redact/src/lib.rs` (self) | exact |
| `crates/vox-secrets/src/redact.rs` (mod) | W1 | utility | transform | 68 / 68 (~55 LoC) — Safe | `crates/vox-secrets/src/redact.rs` + `crates/vox-secrets/src/spec/mod.rs` | exact |
| `crates/vox-secrets/src/lib.rs` (mod) | W1 | barrel / re-export | transform | 875 / 670 — Watch LoC (1-line re-export only) | `crates/vox-secrets/src/lib.rs:34-47` | exact |
| `crates/vox-terminal-core/src/block.rs` (mod) | W1 | model / utility | transform | 161 / 161 (~130 LoC) — Safe | `crates/vox-terminal-core/src/block.rs` (self) | exact |
| `crates/vox-terminal-core/src/corpus/redact.rs` (mod) | W1 | utility | transform | 37 / 6 (~5 LoC) — Safe | `crates/vox-redact/src/lib.rs` | exact |
| `crates/vox-orchestrator/src/oversight/exec_policy.rs` (**new**) | W1 | policy / parser | transform | 0 (Target `< 320` LoC, `<= 12` methods/impl) | `crates/vox-cli-research/src/check_terminal.rs` + `crates/vox-terminal-core/src/host.rs:77-310` | role + data-flow |
| `crates/vox-orchestrator/src/oversight/floor.rs` (mod) | W1 | policy | transform | 771 / 502 (~380 LoC) — Watch `< 500` ceiling | `crates/vox-orchestrator/src/oversight/floor.rs:89,210-216,400-421` | exact |
| `crates/vox-orchestrator/src/oversight/mod.rs` (mod) | W1 | barrel | N/A | 6 / 6 — Safe | `crates/vox-orchestrator/src/oversight/mod.rs` | exact |
| `crates/vox-cli-research/src/check_terminal.rs` (mod) | W1 | CLI validator | file-I/O + transform | 764 / 514 (~410 LoC) — Shrinks via delegation | `crates/vox-cli-research/src/check_terminal.rs:98-153,338-508` | exact |
| `crates/vox-terminal-core/src/stall_detector.rs` (**new**) | W2 | state-machine | event-driven | 0 (Target `< 220` LoC, `<= 8` methods/impl) | `crates/vox-terminal-core/src/session.rs` + `vt_shadow.rs` | partial (novel algorithm) |
| `crates/vox-terminal-core/src/agent_exec.rs` (**new**) | W2 | service | request-response + stream | 0 (Target `< 380` LoC, `<= 10` methods/impl) | `crates/vox-terminal-core/src/host.rs:446-858` | exact |
| `crates/vox-terminal-core/src/shell_init.rs` (mod) | W2 | config / shims | transform | 1004 / 625 (raw string constants) — Safe (0 new fns) | `crates/vox-terminal-core/src/shell_init.rs:282-624` | exact |
| `crates/vox-terminal-core/src/session.rs` (mod) | W2 | model / event-bus | event-driven | 641 / 564 (~450 LoC, 18 methods) — **OVER METHOD CAP**: add enum variants + minimal delegators only | `crates/vox-terminal-core/src/session.rs:96-143` | exact |
| `crates/vox-terminal-core/src/host.rs` (mod) | W2 | service / host | event-driven | 1249 / 972 (~780 LoC, 19 methods on `LocalHost`) — **OVER GOD-OBJECT CAP**: extract L77-310 & delegate to `agent_exec.rs` | `crates/vox-terminal-core/src/host.rs` (self) | exact |
| `crates/vox-terminal-core/src/daemon_host.rs` (mod) | W2 | client adapter | RPC + stream | 731 / 545 (~430 LoC) — Watch `< 500` ceiling | `crates/vox-terminal-core/src/daemon_host.rs:21-28,135-254` | exact |
| `crates/vox-terminal-core/src/lib.rs` (mod) | W2 | barrel | N/A | 55 / 55 — Safe | `crates/vox-terminal-core/src/lib.rs` | exact |
| `crates/vox-orchestrator/src/terminal_bridge.rs` (**new**) | W2 | trait + RPC client | request-response | 0 (Target `< 280` LoC, `<= 8` methods/impl) | `crates/vox-orchestrator/src/orch_daemon/client.rs` | exact |
| `crates/vox-orchestrator/src/lib.rs` (mod) | W2 | barrel | N/A | 439 / 429 — Safe | `crates/vox-orchestrator/src/lib.rs:415-429` | exact |
| `crates/vox-orchestrator-d/src/term_bridge_impl.rs` (**new**) | W2 | adapter | request-response | 0 (Target `< 220` LoC, `<= 8` methods/impl) | `crates/vox-orchestrator-d/src/term_dispatch.rs` | exact |
| `crates/vox-orchestrator-d/src/term_dispatch.rs` (mod) | W2 | RPC router | request-response + stream | 923 / 350 (~280 LoC) — Safe (delegate bodies to `term_bridge_impl`) | `crates/vox-orchestrator-d/src/term_dispatch.rs:175-286` | exact |
| `crates/vox-orchestrator-d/src/lib.rs` (mod) | W2 | barrel | N/A | 7 / 7 — Safe (fixes `#[path]` split) | `crates/vox-orchestrator-d/src/lib.rs` | exact |
| `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` (mod) | W2 | daemon entrypoint | startup wiring | 425 / 323 (~260 LoC) — Safe | `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:14-15,272-298` | exact |
| `contracts/operations/catalog.v1.yaml` (mod) | W3 | contract SSOT | declarative | 37087 — Targeted 4-row insert at L35681 | `contracts/operations/catalog.v1.yaml:64-84,35656-35682` | exact |
| `contracts/mcp/http-read-role-governance.yaml` (mod) | W3 | contract SSOT | declarative | 47 — Targeted 2-row insert at L41 | `contracts/mcp/http-read-role-governance.yaml:38-46` | exact |
| `contracts/orchestration/permission-modes.v2.yaml` (mod) | W3 | contract SSOT | declarative | 293 — Targeted 2-row + 1-tool insert | `contracts/orchestration/permission-modes.v2.yaml:76-86,267-278` | exact |
| `crates/vox-orchestrator-mcp/src/server_state.rs` (mod) | W3 | state container | shared state | 1074 / 976 — Existing state struct (3 constructors) | `crates/vox-orchestrator-mcp/src/server_state.rs:143,453,577,939` | exact |
| `crates/vox-orchestrator-mcp/src/terminal_schemas.rs` (**new**) | W3 | JSON schema | declarative | 0 (Target `< 180` LoC) | `crates/vox-orchestrator-mcp/src/input_schemas/browser_and_gui.rs` | exact |
| `crates/vox-orchestrator-mcp/src/input_schemas.rs` (mod) | W3 | schema router | transform | 816 / 816 (mostly `#[cfg(any())]` match) — Safe | `crates/vox-orchestrator-mcp/src/input_schemas.rs:16-45` | exact |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` (**new**) | W3 | MCP handler | request-response | 0 (Target `< 320` LoC, `<= 8` fns) | `crates/vox-orchestrator-mcp/src/dispatch_domains/browser_gui_ops.rs` | exact |
| `crates/vox-orchestrator-mcp/src/dispatch.rs` + `dispatch_timeout.rs` + `permission_modes.rs` + `floor_gate.rs` + `llm_bridge/tool_selection.rs` (mod) | W3 | MCP pipeline / gates | middleware + routing | 1227/858, 368/209, 334/208, 428/142, 997/461 — Targeted table rows | Self-tables in `crates/vox-orchestrator-mcp/src/` | exact |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` (**new**) + `mentions.rs` + `message.rs` + `mod.rs` (mod) | W3 | prompt expander | async RPC + transform | `mentions.rs` 500/413, `message.rs` 693/678 — Keep new logic in `term_mentions.rs` (`< 240` LoC) | `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs:13-67,340-412` | exact |
| `crates/vox-gui/src/commands/pty.rs` + `console_a2a.rs` + `main.rs` + `ui/src/transport.ts` (mod) | W3 | Tauri IPC + DTOs | request-response | `pty.rs` 627/452 (~355 LoC), `console_a2a.rs` 140/68 — Safe (`< 500` LoC) | `crates/vox-gui/src/commands/pty.rs:250-451`, `console_a2a.rs:36-67` | exact |

---

## Pattern Assignments

### Wave 1: Scrubber Hardening + AST `ExecPolicyV1`

#### 1. `crates/vox-redact/src/lib.rs` (modified — 204 total lines, 119 pre-test lines)

**Analog:** Self (`crates/vox-redact/src/lib.rs:8-74`)

**Imports & `OnceLock<Regex>` pattern (`L8-15`):**
```rust
use regex::Regex;
use std::sync::OnceLock;

static RE_EMAIL: OnceLock<Regex> = OnceLock::new();
static RE_API_KEY: OnceLock<Regex> = OnceLock::new();
static RE_TOKEN: OnceLock<Regex> = OnceLock::new();
static RE_IPV4: OnceLock<Regex> = OnceLock::new();
static RE_HOME: OnceLock<Regex> = OnceLock::new();
```

**Core anchor (`L40-74`) & Lint Guard Gotcha:**
```rust
fn re_home() -> &'static Regex {
    RE_HOME.get_or_init(|| {
        // vox-arch-check: allow abs-path (or keep leading `(?:` so `"(/(tmp|...|home)\b` never matches)
        Regex::new(r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)").expect("valid home regex")
    })
}

pub fn redact_owned(text: &str) -> String {
    let s = re_pem().replace_all(text, "[REDACTED_PEM]");
    let s = re_url_cred().replace_all(&s, "${1}[REDACTED_CRED]@");
    let s = re_email().replace_all(&s, "[REDACTED_EMAIL]");
    let s = re_api_key().replace_all(&s, "$1=[REDACTED]");
    let s = re_token().replace_all(&s, "[REDACTED_KEY]");
    let s = re_ipv4().replace_all(&s, "[REDACTED_IP]");
    let s = re_home().replace_all(&s, "~[REDACTED_PATH]");
    s.into_owned()
}
```
- **Critical Gotcha 1 (`[no-hardcoded-abs-path]`):** `vox-arch-check` bans string literals matching `"(/(tmp|usr|etc|var|home|opt|bin|root)\b|[A-Za-z]:\\)`. Starting the raw regex literal with `(?:` (`r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)"`) avoids both the leading `"/home` prefix and a literal `"C:\` prefix. In unit tests that construct `/Users/alice` or `C:\Users\alice` literals, add `// vox-arch-check: allow abs-path` on the line immediately preceding the test assertion (same convention as `crates/vox-gui/src/commands/pty.rs:474`).
- **Critical Gotcha 2 (Existing replacement tokens):** Preserve `[REDACTED_KEY]` for `re_token()` and `~[REDACTED_PATH]` for `re_home()` so existing callers (`crates/vox-redact/src/lib.rs:124-203` and `crates/vox-terminal-core/src/corpus/redact.rs:8-36`) stay 100% green. Expand `re_token()` (`L47-64`) with `AIza[0-9A-Za-z\-_]{35}`, `hf_[A-Za-z0-9]{30,}`, `npm_[A-Za-z0-9]{30,}`, `sk-ant-[A-Za-z0-9\-_]{20,}`, and `sk-(?:proj-|svcacct-)?[A-Za-z0-9_\-]{20,}`. Add `RE_PEM` (`(?s)-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?-----END [A-Z0-9 ]*PRIVATE KEY-----`) and `RE_URL_CRED` (`([a-zA-Z][a-zA-Z0-9+.\-]*://[^/\s:@]+:)([^@\s/]+)@`).

---

#### 2. `crates/vox-secrets/src/redact.rs` (68 lines) + `crates/vox-secrets/src/lib.rs` (875 lines)

**Analog:** `crates/vox-secrets/src/redact.rs:1-34` + `crates/vox-secrets/src/spec/mod.rs:24-32,452-455` + `crates/vox-secrets/src/spec/types.rs:101-136`

**Imports & Aho-Corasick pattern (`crates/vox-secrets/src/redact.rs:1-28`):**
```rust
use aho_corasick::{AhoCorasick, MatchKind};
use serde_json::Value;
```

**Exact Clavis metadata lookup pattern (CRITICAL compile fix vs pseudocode):**
`SecretSpec` (`crates/vox-secrets/src/spec/types.rs:243-253`) does **not** carry `taxonomy_class` or `material_kind` fields directly — those live on `SecretMetadata` returned by `spec.id.metadata()` (`crates/vox-secrets/src/spec/mod.rs:454`):
```rust
/// Scrub resolved user-facing secret material (>= 6 chars, non-config/non-URL/non-username)
/// from arbitrary UTF-8 text using a `LeftmostLongest` Aho-Corasick automaton.
#[must_use]
pub fn scrub_resolved_secrets(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut needles: Vec<String> = Vec::new();
    for spec in crate::all_specs() {
        let meta = spec.id.metadata();
        if meta.taxonomy_class.is_config_only()
            || matches!(
                meta.material_kind,
                crate::spec::SecretMaterialKind::ConfigValue
                    | crate::spec::SecretMaterialKind::EndpointUrl
                    | crate::spec::SecretMaterialKind::Username
                    | crate::spec::SecretMaterialKind::DelegationRef
            )
        {
            continue;
        }
        let resolved = crate::resolve_secret(spec.id);
        if let Some(val) = resolved.expose() {
            let trimmed = val.trim();
            if trimmed.len() >= 6 && !needles.iter().any(|n| n == trimmed) {
                needles.push(trimmed.to_string());
            }
        }
    }
    if needles.is_empty() {
        return text.to_string();
    }
    let Ok(ac) = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .build(&needles)
    else {
        return text.to_string();
    };
    let replacements: Vec<&str> = vec!["[REDACTED:CLAVIS]"; needles.len()];
    ac.replace_all(text, &replacements)
}
```
- **Re-export anchor (`crates/vox-secrets/src/lib.rs:7,40`):** `pub mod redact;` is already declared at `L7`, and `L40` already has `pub use redact::{contains_secret_material, redact_secrets_from_value};` — append `scrub_resolved_secrets` to that `pub use` list.
- **Secret-env-guard rule:** Never read `std::env::var` in `scrub_resolved_secrets`; always go through `crate::resolve_secret(spec.id)`.

---

#### 3. `crates/vox-terminal-core/src/block.rs` (161 lines) + `crates/vox-terminal-core/src/corpus/redact.rs` (37 lines)

**Analog:** `crates/vox-terminal-core/src/block.rs:14-21,57-85,113-161`

**`BlockStatus` & `Block` anchors (`crates/vox-terminal-core/src/block.rs:14-85`):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockStatus {
    Running,
    AwaitingInput,
    Ok,
    Failed,
    Cancelled,
}
// ...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub kind: BlockKind,
    pub status: BlockStatus,
    #[serde(default)]
    pub origin: BlockOrigin,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub credential_prompt: bool,
    pub input: String,
    pub output: Vec<OutputChunk>,
    pub exit_code: Option<i32>,
    pub started_ms: u64,
    pub ended_ms: Option<u64>,
}
```
- **Critical Gotcha (`secret-env-guard`):** Name the boolean field `credential_prompt: bool` (never `password_prompt` or `secret_prompt` — `vox ci secret-env-guard` scans Rust identifiers for `password`/`secret` tokens outside `vox-secrets`). Initialize `pinned: false, credential_prompt: false` in `Block::new` (`L73-85`).
- **ANSI/OSC/DCS normalizer (`crates/vox-terminal-core/src/block.rs:113-161`):** Expose `pub fn strip_ansi_and_normalize(raw: &str) -> String` (called by `Block::plain_output(&self)` at `L113-117`) that runs `normalize_cr_overwrites(raw)` and strips CSI (`ESC [ ... 0x40..=0x7E`), OSC (`ESC ] ... BEL/ST`), DCS/SOS/PM/APC (`ESC P | X | ^ | _ ... BEL/ST`), charset designators (`ESC ( X`, `ESC ) X`), 2-byte ESC (`ESC 0x30..=0x7E`), and non-printable C0 controls (`< 0x20` except `\n` and `\t`, plus `0x7F`).
- **Scrubber + budget helper (`crates/vox-terminal-core/src/corpus/redact.rs:1-6`):**
```rust
pub use vox_redact::redact_owned;

/// Strip ANSI/OSC/DCS control sequences, normalize `\r` progress overwrites, and apply
/// `vox_redact::redact_owned` so downstream crates (`vox-gui`, `vox-orchestrator-d`) do
/// not need direct `vox-redact` Cargo edges.
#[must_use]
pub fn scrub_terminal_text(raw: &str) -> String {
    let plain = crate::block::strip_ansi_and_normalize(raw);
    vox_redact::redact_owned(&plain)
}
```
Also expose `pub fn truncate_head_tail(text: &str, head_lines: usize, tail_lines: usize, max_bytes: usize) -> (String, bool, usize)` here (default 40 head + 160 tail, 16,384 bytes) so `agent_exec.rs`, `term_bridge_impl.rs`, and `term_mentions.rs` share a single UTF-8-safe (`floor_char_boundary` / `ceil_char_boundary`) truncation helper.

---

#### 4. `crates/vox-orchestrator/src/oversight/exec_policy.rs` (**new**) + `floor.rs` (771/502 lines) + `mod.rs` (6 lines) + `crates/vox-cli-research/src/check_terminal.rs` (764/514 lines)

**Analogs:**
- `crates/vox-cli-research/src/check_terminal.rs:98-128,431-508` (`ExecPolicyV1`, `ExecCheckViolation`, `ExecCheckReport`)
- `crates/vox-terminal-core/src/host.rs:77-310` (`normalize_cmd_basename`, `extract_segment_basenames` shell-wrapper unwrapping)
- `crates/vox-orchestrator/src/oversight/floor.rs:89,210-216,400-421` (`SHELL_COMMAND_TOOLS`, `classify_floor`, `command_rewrites_history`)
- `contracts/terminal/exec-policy.v1.yaml:1-39` (embedded via `include_str!("../../../../contracts/terminal/exec-policy.v1.yaml")`)

**Contract struct & evaluation API (`crates/vox-orchestrator/src/oversight/exec_policy.rs`):**
```rust
use std::collections::{BTreeMap, HashSet};
use serde::{Deserialize, Serialize};

pub const EMBEDDED_EXEC_POLICY_V1_YAML: &str =
    include_str!("../../../../contracts/terminal/exec-policy.v1.yaml");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecPolicyV1 {
    pub version: u32,
    #[serde(default)]
    pub allowed_cmdlets: Vec<String>,
    #[serde(default)]
    pub allowed_binaries: Vec<String>,
    #[serde(default)]
    pub blocked_parameters: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub network_fetch_commands: Vec<String>,
    #[serde(default)]
    pub network_fetch_domains: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecCheckViolation {
    pub rule: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}
```
- **AST/Lexical requirements for `evaluate_command_ast(command: &str, policy: &ExecPolicyV1) -> Vec<ExecCheckViolation>`:**
  1. Quote-aware lexical scanner handling single quotes (`'...'` literal), double quotes (`"..."` scanning inner `$()` / `` `...` ``), backslash escapes, comments (`#`), and newlines (`\n`).
  2. Compound operator splitting (`|`, `||`, `&&`, `;`, `&`) and background/redirection detection (`>`, `>>`, `2>`, `&>`, `<<<`, `<<`).
  3. Recursive subshell extraction for `$(...)`, `` `...` ``, PowerShell subexpressions `$(...)` / `& { ... }`, process substitution `<(...)` / `>(...)`, and nested shell/wrapper payloads:
     - `bash -c <payload>`, `sh -c <payload>`, `zsh -c <payload>`, `fish -c <payload>`
     - `pwsh -Command/-c <payload>`, `powershell -Command/-c <payload>` (reject `-EncodedCommand` / `-ec` / `-e` unconditionally with rule `"encoded-command-forbidden"`!)
     - Prefix wrappers: `env [K=V ...] <cmd>`, `command <cmd>`, `exec <cmd>`, `nohup <cmd>`, `timeout [FLAGS] <dur> <cmd>`, `nice [FLAGS] <cmd>`, `sudo [FLAGS] <cmd>`, `xargs [FLAGS] <cmd>`, `find ... -exec <cmd>`.
  4. Dynamic invocation rejection: `Invoke-Expression`, `iex`, `eval`, `& $var`, `. $var` → violation `"dynamic-eval-forbidden"`.
  5. Binary / cmdlet allowlist check (case-insensitive for PowerShell `Verb-Noun` cmdlets in `allowed_cmdlets`, basename-normalized stripping `.exe`/`.cmd`/`.bat` for `allowed_binaries`), `blocked_parameters` (`"*"` + per-command parameter names matching `-<Param>` / `--<param>` case-insensitively), and `network_fetch_commands` (`curl`, `wget`, `Invoke-WebRequest`, `Invoke-RestMethod`, `iwr`, `irm`) URL host allowlist against `network_fetch_domains`.
  6. Also expose `pub fn extract_effective_commands(command: &str) -> Vec<Vec<String>>` (each unwrapped leaf command's argv tokens) so `floor.rs` can inspect nested commands!

- **`crates/vox-orchestrator/src/oversight/mod.rs` (`L1-6`):**
  Add `pub mod exec_policy;` unconditionally alongside `pub mod floor; pub mod resolver;`.

- **`crates/vox-orchestrator/src/oversight/floor.rs` (`L89, L210-216, L400-421`):**
  - `L89`: Change `pub const SHELL_COMMAND_TOOLS: &[&str] = &["vox_run_shell", "vox_skill_run", "vox_terminal_exec"];` (and update `contracts/orchestration/permission-modes.v2.yaml:267-271` in Wave 3 — **CRITICAL**: `floor_categories_yaml_matches_rust_floor` in `crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs:626-636` asserts `yaml_tools == tools_for(FloorCategory::IrreversibleHistoryRewrite)`! Either update `permission-modes.v2.yaml:267-271` in the **same wave** as `SHELL_COMMAND_TOOLS`, or add `"vox_terminal_exec"` to both in Wave 1!).
  - `L211`: `if let Some(cmd) = input.args.get("command").or_else(|| input.args.get("cmd")).and_then(Value::as_str)`
  - `L400-421`: Upgrade `command_rewrites_history(cmd: &str) -> bool` to inspect both raw `cmd` and every unwrapped argv segment from `super::exec_policy::extract_effective_commands(cmd)` so `bash -c "git reset --hard"` and `env FOO=1 git push --force` trigger `FloorCategory::IrreversibleHistoryRewrite`.
  - **LoC watch:** `floor.rs` is at 502 pre-test lines (~380 non-blank LoC) — keep edits to `<= 25` lines so it stays well below `< 500` non-blank LoC.

- **`crates/vox-cli-research/src/check_terminal.rs` (`L98-153, L431-508`):**
  - Re-use `vox_orchestrator::oversight::exec_policy::{ExecPolicyV1, ExecCheckViolation, evaluate_command_ast}` inside `run_check_rust_fallback` (`L431-508`), while keeping `validate_policy_yaml_against_schema` (`L130-153`, which uses `vox_jsonschema_util`) inside `check_terminal.rs` so `vox-orchestrator` adds zero new crate dependencies.

---

### Wave 2: Stateful `agent_exec`, `stall_detector`, `TerminalBridge`, Daemon `term.*` RPCs

#### 5. `crates/vox-terminal-core/src/stall_detector.rs` (**new** — target `< 220` LoC)

**Partial Analogs:** `crates/vox-terminal-core/src/session.rs:260-380` (OSC 633 block state transitions) + `crates/vox-terminal-core/src/vt_shadow.rs` (alt-screen tracking)

**Core types & state machine (`crates/vox-terminal-core/src/stall_detector.rs`):**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptKind {
    YesNo,
    Credential,
    Selection,
    PressEnter,
    InteractiveTui,
    GenericQuestion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StallSignal {
    pub prompt_kind: PromptKind,
    pub prompt_tail: String,
    pub credential_prompt: bool,
    pub silence_ms: u64,
}
```
- **Two-tier detection contract:**
  - Never fire while `OSC 633;D` has already arrived (`status != BlockStatus::Running`).
  - If `alt_screen == true` while an agent command is running (e.g., `vim`, `top`, `less` if pager guard bypassed) for `>= 500 ms` → `PromptKind::InteractiveTui`, `credential_prompt: false`.
  - Tier 1 (`silence_ms >= 500`): inspect last non-empty normalized output line (with **no** trailing `\n` or trailing `? ` / `: `) for:
    - Credential prompts (case-insensitive): `password:`, `passphrase`, `enter pin`, `token:`, `secret key:`, `[sudo] password for`, `verification code:` → `PromptKind::Credential`, `credential_prompt: true`.
    - Yes/No prompts: `[y/n]`, `(y/n)`, `(yes/no)`, `continue?`, `proceed?`, `are you sure` → `PromptKind::YesNo`.
    - Press-Enter / Selection: `press enter`, `press any key`, `select an option`, `choose [`, `? ` menu prompt → `PromptKind::PressEnter` / `PromptKind::Selection`.
  - Tier 2 (`silence_ms >= 2_500`): trailing `:` or `?` at end of partial line (no trailing `\n`) → `PromptKind::GenericQuestion` (excluding compiler/log prefixes like `error[E`, `warning:`, `-->`, timestamps `12:34:56`, Rust paths `foo::bar`).

---

#### 6. `crates/vox-terminal-core/src/shell_init.rs` (1,004 total / 625 pre-test lines) + `pty.rs` (592 / 514 lines)

**Analog:** `crates/vox-terminal-core/src/shell_init.rs:282-624` (`ZSH_SHIM_ZSHRC`, `BASH_SHIM_RC`, `FISH_SHIM_INIT`, `PWSH_SHIM_INIT`, `NU_SHIM_CONFIG`)

**Insertion anchors in the 5 shell shim raw string constants:**
- **Zsh (`L282-365`, inside `ZSH_SHIM_ZSHRC` before `add-zsh-hook precmd __vox_precmd` at `L360`):**
  Define `__vox_pager_enter()` saving `PAGER`, `GIT_PAGER`, `SYSTEMD_PAGER`, `BAT_PAGER`, `GH_PAGER`, `DELTA_PAGER`, `MANPAGER`, `DEBIAN_FRONTEND`, `CI`, `GIT_TERMINAL_PROMPT` and exporting `cat` / `noninteractive` / `1` / `0`; restore saved/unset state at the top of `__vox_precmd()` when `__VOX_PAGER_ACTIVE=1`.
- **Bash (`L367-475`, inside `BASH_SHIM_RC` around `__vox_prompt_cmd` at `L424`):**
  Same `__vox_pager_enter()` function + restore at the top of `__vox_prompt_cmd()`.
- **Fish (`L477-528`, inside `FISH_SHIM_INIT`):**
  `function __vox_pager_enter` + restore inside `function __vox_postexec --on-event fish_postexec`.
- **PowerShell (`L530-583`, inside `PWSH_SHIM_INIT`):**
  `function global:__vox_pager_enter` saving `$env:PAGER`, `$env:GIT_PAGER`, `$env:GH_PAGER`, `$env:BAT_PAGER`, `$env:CI`, `$env:GIT_TERMINAL_PROMPT` and restoring at the top of `function global:prompt`.
- **Nushell (`L585-624`, inside `NU_SHIM_CONFIG`):**
  `def --env __vox_pager_enter []` (or inline `with-env { PAGER: "cat", GIT_PAGER: "cat", ... } { ... }` wrapper emitted by `format_agent_command_for_shell` in `agent_exec.rs` since Nushell functions require `--env` to mutate caller env).
- **Shell name retention (`crates/vox-terminal-core/src/pty.rs:335-341,442`):**
  `PtyHandle` currently stores `writer`, `master`, `child`, `_shim_guard`. Add `pub fn shell_name(&self) -> &str` (storing `normalized_shell: String` from `shell_init::normalize_shell_name(shell)` in `spawn_pty_with_callback` at `L442`) so `LocalHost` / `agent_exec.rs` knows whether to wrap an agent command for `zsh`/`bash`/`fish`, `pwsh`, `nu`, or bare `cmd`.

---

#### 7. `crates/vox-terminal-core/src/session.rs` (641/564 lines) + `host.rs` (1,249/972 lines) + `agent_exec.rs` (**new**) + `daemon_host.rs` (731/545 lines) + `lib.rs` (55 lines)

**CRITICAL `arch/god_object` BUDGET ALERT:**
- `crates/vox-terminal-core/src/host.rs` is currently **972 pre-test lines** (~780 non-blank LoC) and `impl LocalHost` (`L446-858`) already has **19 methods** (`> 12` ceiling).
- `crates/vox-terminal-core/src/session.rs` is currently **564 pre-test lines** (~450 non-blank LoC) and `impl Session` already has **18 methods**.
- **Required decomposition pattern:**
  1. **Extract** `normalize_cmd_basename` + `extract_segment_basenames` + `TermSecurityLevel::allows_agent_command` (`host.rs:77-310`, 233 lines) into `crates/vox-terminal-core/src/agent_exec.rs` (keeping `TermSecurityLevel` and `TermControlMode` enum definitions in `host.rs` or re-exporting them from `host.rs` + `lib.rs` so existing imports don't break). That immediately **shrinks** `host.rs` by ~220 lines!
  2. **Do NOT add new methods to `impl LocalHost` in `host.rs`!** Make `pub(crate) entries: Mutex<HashMap<String, Entry>>` (and fields on `pub(crate) struct Entry` at `host.rs:430-439`, adding `pub(crate) agent_exec_lock: Arc<tokio::sync::Mutex<()>>` and `pub(crate) shell_name: String`) `pub(crate)` so `crates/vox-terminal-core/src/agent_exec.rs` implements a dedicated `pub struct AgentExecutor` (or `impl LocalHost` block inside `agent_exec.rs` with `<= 8` methods — `vox-arch-check` checks per-`impl`-block method count and per-file LoC!).
  3. **`SessionEvent` variants (`crates/vox-terminal-core/src/session.rs:96-143`):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionEvent {
    BlockOpened { block: Block },
    BlockOutput { id: BlockId, chunk: OutputChunk },
    BlockAwaitingInput {
        id: BlockId,
        prompt_tail: String,
        prompt_kind: crate::stall_detector::PromptKind,
        credential_prompt: bool,
    },
    BlockClosed {
        id: BlockId,
        status: BlockStatus,
        exit_code: Option<i32>,
        ended_ms: u64,
    },
    BlockPinned { id: BlockId, pinned: bool },
    SecurityLevelChanged { security_level: crate::host::TermSecurityLevel },
    ControlModeChanged { control_mode: crate::host::TermControlMode },
    AgentAttached { agent_id: String },
    AgentDetached { agent_id: String },
    AltScreenChanged { active: bool },
    CwdChanged {
        cwd: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        git_branch: Option<String>,
    },
    SnapshotResync {
        blocks: Vec<Block>,
        resync_required: bool,
        alt_screen: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        git_branch: Option<String>,
    },
}
```
  4. **`daemon_host.rs` (`L135-254` `route()`):** Add match arms in `route(v, out_tx, ev_tx)` for `"block_awaiting_input"`, `"block_pinned"`, `"security_changed"`, and `"control_changed"`, mirroring `term_dispatch.rs::forward_session_event` (`crates/vox-orchestrator-d/src/term_dispatch.rs:301-349`).
  5. **`agent_exec.rs` (**new**) core execution flow:**
     - Acquire per-session `agent_exec_lock` (`tokio::sync::Mutex::try_lock_owned()` or bounded wait; reject if `control_mode == TermControlMode::UserHasControl` or `security_level == TermSecurityLevel::Deny` or `alt_screen == true`).
     - In `TermSecurityLevel::Sandbox`, validate against the caller-passed policy callback or `TermSecurityLevel::allows_agent_command` (full `ExecPolicyV1` AST validation runs in `term_bridge_impl.rs` + `terminal_tools.rs` where `vox-orchestrator` is in scope, since `vox-terminal-core` is L1 and does **not** depend on `vox-orchestrator`).
     - Subscribe `events_rx = session.subscribe_events()` **before** writing to PTY so fast sub-millisecond commands cannot race `BlockClosed`.
     - Open an agent-originated block (`BlockOrigin::Agent { agent_id }`), write pager-guarded line (`__vox_pager_enter; <cmd>\r`), and `tokio::select!` between `events_rx.recv()`, a `250 ms` `StallDetector` tick, and `timeout_ms` (hard-capped at `120_000 ms`):
       - On `BlockClosed { id, status, exit_code, .. }` for `block_id` → scrub + head-40/tail-160 truncate → return `AgentExecOutcome` with `status: Completed` (`Ok`/`Failed`/`Cancelled`).
       - On `StallDetector` firing `StallSignal` → transition block to `BlockStatus::AwaitingInput`, set `block.credential_prompt = signal.credential_prompt`, broadcast `SessionEvent::BlockAwaitingInput`, and **return immediately** (`status: AwaitingInput`) so the LLM can either answer non-credential prompts via `vox_terminal_send_input` or ask the user in chat when `credential_prompt == true`.
       - On timeout → send `\x03` (`SIGINT`) to PTY when `interrupt_on_timeout` (default `true`), wait up to `500 ms` for `OSC 633;D`, else mark `Cancelled` / `TimedOut` and return bounded partial output.
     - Also provide `agent_send_input(session_id, block_id, text, append_newline, wait_ms)` (**hard-refuse** if `block.credential_prompt == true` with `"CREDENTIAL_PROMPT_USER_ONLY: credential prompts must be answered directly by the operator in the terminal pane"`!), `pin_block(session_id, block_id, pinned)`, and `promote_bg(session_id)` (clones `cwd` + `shell_name`, opens a fresh session in that `cwd`, sends `\x03` to foreground or leaves background session running, returns new `session_id`).

---

#### 8. `crates/vox-orchestrator/src/terminal_bridge.rs` (**new**) + `crates/vox-orchestrator/src/lib.rs` (439 lines)

**Analog:** `crates/vox-orchestrator/src/orch_daemon/client.rs:57-233` (`OrchDaemonClient::from_env_or_token_file`, `call_with_deadline`)

**Trait & Daemon RPC Client (`crates/vox-orchestrator/src/terminal_bridge.rs`):**
```rust
use std::time::{Duration, Instant};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::orch_daemon::OrchDaemonClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermExecRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_session_id: Option<String>,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_agent_id")]
    pub agent_id: String,
}

fn default_timeout_ms() -> u64 { 30_000 }
fn default_agent_id() -> String { "mcp-agent".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermExecResponse {
    pub session_id: String,
    pub block_id: u64,
    pub status: String, // "ok" | "failed" | "awaiting_input" | "timed_out" | "cancelled"
    pub exit_code: Option<i32>,
    pub output: String,
    pub truncated: bool,
    pub total_bytes: usize,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub credential_prompt: bool,
    pub prompt_kind: Option<String>,
    pub duration_ms: u64,
}
// + TermSendInputRequest, TermReadRequest, TermBlockSnapshot, TermReadResponse, TermSessionSummary

#[async_trait]
pub trait TerminalBridge: Send + Sync {
    async fn exec(&self, req: TermExecRequest) -> Result<TermExecResponse, String>;
    async fn send_input(&self, req: TermSendInputRequest) -> Result<TermExecResponse, String>;
    async fn read(&self, req: TermReadRequest) -> Result<TermReadResponse, String>;
    async fn list_sessions(&self) -> Result<Vec<TermSessionSummary>, String>;
}
```
- **`OrchDaemonTerminalBridge` implementation:** Wraps `OrchDaemonClient` (`OrchDaemonTerminalBridge::from_env_or_token_file() -> Option<Self>` calling `OrchDaemonClient::from_env_or_token_file()`) and invokes:
  - `self.client.call_with_deadline("term.agent_exec", serde_json::to_value(&req)?, Instant::now() + Duration::from_millis(req.timeout_ms.min(120_000) + 15_000)).await`
  - `self.client.call("term.agent_input", ...)`
  - `self.client.call("term.read", ...)`
  - `self.client.call("term.list", json!({}))`
- **Registration anchor (`crates/vox-orchestrator/src/lib.rs:425`):** Add `pub mod terminal_bridge;` in the unconditional module block (`L415-429`). Notice this adds **zero** new Cargo dependencies (`vox-orchestrator` has no edge to `vox-terminal-core`).

---

#### 9. `crates/vox-orchestrator-d/src/term_bridge_impl.rs` (**new**) + `term_dispatch.rs` (923/350 lines) + `lib.rs` (7 lines) + `bin/vox_orchestrator_d.rs` (425/323 lines)

**Analog:** `crates/vox-orchestrator-d/src/term_dispatch.rs:175-349` + `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:14-15,272-298`

**CRITICAL `lib.rs` vs `[[bin]]` Wiring Fix:**
Currently `crates/vox-orchestrator-d/src/lib.rs` (`L1-7`) only exports `boot_plan` and `daemon_steps`, while `bin/vox_orchestrator_d.rs:14-15` uses `#[path = "../term_dispatch.rs"] mod term_dispatch;` (which means `cargo test -p vox-orchestrator-d --lib` never compiles `term_dispatch.rs`!).
1. Update `crates/vox-orchestrator-d/src/lib.rs`:
```rust
pub mod boot_plan;
pub mod daemon_steps;
pub mod term_bridge_impl;
pub mod term_dispatch;
```
2. Update `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs`:
   - Replace `L14-15` (`#[path = "../term_dispatch.rs"] mod term_dispatch;`) with `use vox_orchestrator_d::{term_bridge_impl, term_dispatch};`.
   - At `L288-296` (right after `state.set_boot_report(outcome.report);`):
```rust
    let local_host = Arc::new(vox_terminal_core::LocalHost::default());
    let term_bridge = Arc::new(term_bridge_impl::LocalHostTerminalBridge::new(
        Arc::clone(&local_host),
        state.workspace_root.clone(),
    ));
    state.set_terminal_bridge(term_bridge.clone());
    let term_router = Arc::new(term_dispatch::TermDispatch::with_bridge(
        local_host,
        term_bridge,
    ));
```
3. **`term_bridge_impl.rs` (**new**):** Implements `vox_orchestrator::terminal_bridge::TerminalBridge` over `Arc<vox_terminal_core::LocalHost>` + sticky `parking_lot::Mutex<HashMap<String, String>>` (`chat_session_id -> term_session_id`, creating/reusing a default daemon PTY session on demand when `req.session_id` is `None`), runs `ExecPolicyV1` AST validation when the session is in `TermSecurityLevel::Sandbox`, and scrubs every block output + `replay_tail` through **all 3 stages**:
   - Stage 1+2: `vox_terminal_core::corpus::redact::scrub_terminal_text(&raw)` (ANSI/OSC/DCS + `\r` overwrite + `vox_redact::redact_owned`)
   - Stage 3: `vox_secrets::redact::scrub_resolved_secrets(&stage2)` (Clavis resolved secrets — note check `crates/vox-orchestrator-d/Cargo.toml` already depends on `vox-secrets`; if not, call `vox_secrets::redact::scrub_resolved_secrets` in `vox-orchestrator-mcp` where `vox-secrets` is already a direct dependency! Let's check: `vox-orchestrator-mcp` definitely depends on `vox-secrets` (`secrets_tools.rs`), and `vox-orchestrator-d` depends on `vox-secrets` too — verify in `Cargo.toml` before importing, and NEVER edit `Cargo.lock`!).
4. **`term_dispatch.rs` (`L175-286` & `L301-349`):**
   - Add RPC arms in `try_handle`: `"term.agent_exec"`, `"term.agent_input"`, `"term.read"`, `"term.pin"`, `"term.promote_bg"` delegating directly to `self.bridge` / `self.host` so `term_dispatch.rs` stays `< 400` pre-test LoC.
   - Add `SessionEvent::BlockAwaitingInput`, `BlockPinned`, `SecurityLevelChanged`, `ControlModeChanged` arms in `forward_session_event` (`L301-349`).

---

### Wave 3: MCP Catalog, Schemas, Dispatch, HITL, `@term` Citations & `vox-gui` Backend

#### 10. Contract SSOT Edits (`contracts/**`)

1. **`contracts/operations/catalog.v1.yaml` (Insert between `L35681` and `L35682`):**
   - `L35656-35681` is `- id: term` (`cli: [term]`, `mcp: null`); `L35682` is `- id: test`.
   - Insert 4 sorted entries (`terminal.exec`, `terminal.read`, `terminal.send_input`, `terminal.sessions`) right after `L35681` matching the exact schema of `L64-84`:
```yaml
- id: terminal.exec
  title: Terminal Exec
  description: Execute a shell command inside a stateful Vox Axis PTY session with block tracking, stall detection, and scrubbed output.
  description_human: null
  product_lane: ai
  intent_tags: []
  side_effect_class: local_mutation
  scope_kind: null
  reversible: false
  requires_repo: null
  preferred_for_models: true
  human_takeover_friendly: true
  mens_planner_visible: true
  canonical_name: null
  latin_aliases: null
  mcp:
    name: vox_terminal_exec
    http_read_role_eligible: false
    exposure: model
    requires: []
  cli: null
```
   - For `terminal.read` and `terminal.sessions`: set `side_effect_class: read_only`, `reversible: true`, `http_read_role_eligible: true`, `exposure: model`.
   - For `terminal.send_input`: set `side_effect_class: local_mutation`, `reversible: false`, `http_read_role_eligible: false`, `exposure: model`.
   - Then run `cargo run -p vox-cli -- ci operations-sync --write` + `cargo run -p vox-cli -- ci command-sync --write` to regenerate `crates/vox-orchestrator-mcp/src/registry.rs` and `docs/src/reference/mcp-tools.generated.md` (never hand-edit `registry.rs`).

2. **`contracts/mcp/http-read-role-governance.yaml` (Insert between `L41` and `L42`):**
   - `L41` is `- vox_task_status`; `L42` is `- vox_test_decision`. Insert:
```yaml
- vox_terminal_read
- vox_terminal_sessions
```

3. **`contracts/orchestration/permission-modes.v2.yaml` (`L76-86` & `L267-271`):**
   - After `vox_run_shell` (`L76-81`), add:
```yaml
  - tool: vox_terminal_exec
    class: destructive
    reversible: false
    always_requires_approval: false
    floor: null
  - tool: vox_terminal_send_input
    class: destructive
    reversible: false
    always_requires_approval: false
    floor: null
```
   - In `floor_categories` → `irreversible_history_rewrite` → `tools:` (`L267-270`), add `- vox_terminal_exec` alongside `vox_run_shell` and `vox_skill_run` (must match `SHELL_COMMAND_TOOLS` in `floor.rs`!).

---

#### 11. `crates/vox-orchestrator-mcp/src/server_state.rs` (1,074/976 lines) + `terminal_schemas.rs` (**new**) + `input_schemas.rs` (816 lines)

- **`server_state.rs` anchors:**
  - Struct field (`L143`, right below `pub boot_report`):
    `pub terminal_bridge: Arc<PrRwLock<Option<Arc<dyn vox_orchestrator::terminal_bridge::TerminalBridge>>>>,`
  - Initialize `terminal_bridge: Arc::new(PrRwLock::new(None))` in **all 3 constructors**:
    1. `ServerState::new_full` (`L453`)
    2. `ServerState::new_for_daemon_gated` (`L577`)
    3. `ServerState::hermetic_stub` (`L939`)
  - Add helper methods `pub fn set_terminal_bridge(&self, bridge: Arc<dyn TerminalBridge>)` and `pub fn resolve_terminal_bridge(&self) -> Option<Arc<dyn TerminalBridge>>` (returns injected in-process bridge if `Some`, otherwise falls back lazily to `OrchDaemonTerminalBridge::from_env_or_token_file().map(|b| Arc::new(b) as Arc<dyn TerminalBridge>)` so stdio `vox mcp` automatically reaches `vox-orchestrator-d`!).

- **`terminal_schemas.rs` (**new**) + `input_schemas.rs` (`L16-45`):**
  - Analog: `crates/vox-orchestrator-mcp/src/input_schemas/browser_and_gui.rs:1-40`.
  - Declare `pub(crate) mod terminal_schemas;` in `crates/vox-orchestrator-mcp/src/lib.rs` (around `L148`) and chain `.or_else(|| crate::terminal_schemas::schema(name))` in `input_schemas.rs:32-40`.
  - **CRITICAL Parity Gate (`input_schemas.rs:42-600`):** `operations_catalog.rs` brace-scans the `#[cfg(any())] match name { ... }` block in `input_schemas.rs`! You **MUST** add:
```rust
        "vox_terminal_exec" => {}
        "vox_terminal_read" => {}
        "vox_terminal_send_input" => {}
        "vox_terminal_sessions" => {}
```
    inside `#[cfg(any())] match name { ... }` in `crates/vox-orchestrator-mcp/src/input_schemas.rs` or `vox ci operations-verify` will fail!

---

#### 12. `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` (**new**) + `dispatch.rs` + `dispatch_timeout.rs` + `permission_modes.rs` + `llm_bridge/tool_selection.rs`

**5 Mandatory Lockstep Wiring Points:**
1. **`crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs` (`L1-7`):** Add `pub mod terminal_tools;`.
2. **`crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` (**new**):**
   - Analog: `crates/vox-orchestrator-mcp/src/dispatch_domains/browser_gui_ops.rs:22-98`.
   - `pub fn handles(name: &str) -> bool` with literal `"vox_terminal_exec" | "vox_terminal_read" | "vox_terminal_send_input" | "vox_terminal_sessions"`.
   - `pub async fn dispatch(state: &ServerState, name: &str, args: serde_json::Value) -> Result<String, anyhow::Error>`:
     - Resolve bridge via `state.resolve_terminal_bridge()` (return `ToolResult::<Value>::err_with_remediation("No active Vox terminal host reachable...", "Start vox-orchestrator-d or launch Vox Axis.")` when `None`).
     - For `vox_terminal_exec`: validate `ExecPolicyV1` via `vox_orchestrator::oversight::exec_policy::evaluate_command_ast` before calling `bridge.exec(req)` (unless caller explicitly ran through approved HITL override, though blocked PowerShell `-EncodedCommand` / `eval` stays blocked in Sandbox), then scrub returned `output` through `vox_secrets::redact::scrub_resolved_secrets` as defense-in-depth!
     - For `vox_terminal_read` / `vox_terminal_send_input` / `vox_terminal_sessions`: scrub output fields through `vox_secrets::redact::scrub_resolved_secrets` before wrapping in `ToolResult::ok(...)`.
3. **`crates/vox-orchestrator-mcp/src/dispatch.rs`:**
   - `L731`: Add `if crate::dispatch_domains::terminal_tools::handles(name) { return crate::dispatch_domains::terminal_tools::dispatch(state, name, args).await; }`.
   - `L758` (`tool_call_kind_for`) & `L843` (`subsystem_from_tool`): classify `name.starts_with("vox_terminal")`.
   - **`L867-940` (`SKIP_DISPATCH_PROBE`):** Add `"vox_terminal_exec"` and `"vox_terminal_send_input"` so `every_registry_tool_has_static_dispatch` (`L981`) does not park 300s waiting for HITL approval!
   - **`L955-962` (`yaml_registry_tools_have_dispatch_match_arms`):** Add `include_str!("dispatch_domains/terminal_tools.rs"),` inside `concat!(...)`!
4. **`crates/vox-orchestrator-mcp/src/dispatch_timeout.rs` (`L171-195` `EXPLICIT_TIMEOUTS`):**
   - Add `("vox_terminal_exec", Duration::from_secs(150)),` (above `DEFAULT_TIMEOUT = 120s` so a `120_000 ms` PTY command + scrubber finishes cleanly, below `ORCH_CLIENT_READ_DEADLINE = 195s`).
5. **`crates/vox-orchestrator-mcp/src/permission_modes.rs` (`L80-86` `RISK_CLASSES`) & `llm_bridge/tool_selection.rs` (`L252-285`):**
   - Add `vox_terminal_exec` and `vox_terminal_send_input` (`SafetyClass::Destructive, reversible: false, always_requires_approval: false`) to `RISK_CLASSES` (`permission_modes.rs:80-86`) — verified by `pending_approvals_tests::yaml_matches_compiled_table` (`L559`).
   - Add `"vox_terminal_exec"`, `"vox_terminal_read"`, `"vox_terminal_sessions"` to `spoke_priority_tools` (`StructuredShell` at `L261-267`, `RustWorkspace` at `L252-260`, `AgentHarness` at `L277-285`) and `CORE_DAILY_DRIVER_TOOLS` (`L317-334`) in `tool_selection.rs`. Do **not** edit `FROZEN_PREFIX_CORE_TOOLS`.

---

#### 13. `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` (**new**) + `mentions.rs` (500/413 lines) + `message.rs` (693/678 lines) + `mod.rs` (19 lines)

**Analogs:**
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs:13-67` (`RULE_MENTION_RE` + pre-`MENTION_RE` stripping)
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:118-167` (`resolve_mentions_with_rules` → `raw_prompt_for_research` → `canonicalize_prompt`)

**Syntax & Resolution Pattern (`term_mentions.rs`):**
- Supported citation tokens (matched via `LazyLock<Regex>` `r"@term(?::([A-Za-z0-9_\-]+))?(?::block:(\d+)|:last(?::(\d+))?)?"`):
  - `@term` or `@term:last` → active/latest session's most recent block (plus 40-line replay tail if 0 blocks exist)
  - `@term:last:<N>` → last `N` (`1..=10`) blocks of the session
  - `@term:block:<id>` or `@term:<session_id>:block:<id>` → exact block `id`
  - `@term:<session_id>` → latest block + cwd/branch header of `<session_id>`
- **Why strip BEFORE `MENTION_RE` and BEFORE `canonicalize_prompt`:**
  1. In `mentions.rs:376`, `MENTION_RE` (`r"@([A-Za-z0-9_.:/\\-]+)"`) matches `@term`, `@term:last`, `@term:block:3` and would wastefully scan the workspace file index if not stripped first! Add `if filename == "term" || filename.starts_with("term:") { continue; }` at `mentions.rs:376` as a belt-and-suspenders guard, AND run `super::term_mentions::resolve_term_mentions(state, &params.prompt, params.session_id.as_deref()).await` in `message.rs` right before `resolve_mentions_with_rules` (`message.rs:118`), while keeping `let raw_prompt_for_research = params.prompt.clone();` (`message.rs:137`) bound to the **original unexpanded** `params.prompt` so terminal output words never pollute `classify_research_intent`!
  2. Every resolved `<terminal_context session="..." cwd="..." branch="...">...</terminal_context>` block is scrubbed via `vox_secrets::redact::scrub_resolved_secrets` (on top of `term_bridge_impl`'s ANSI + `vox_redact` pass) and capped to `16 KiB` / head-40 + tail-160 lines. If no terminal session exists or daemon is offline, inject a compact `<terminal_context status="unavailable">No active terminal session found for @term citation.</terminal_context>` note rather than failing the chat turn.
- Declare `pub mod term_mentions;` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs` (`L13`).

---

#### 14. `crates/vox-gui/src/commands/pty.rs` (627/452 lines) + `console_a2a.rs` (140/68 lines) + `main.rs` (483 lines) + `ui/src/transport.ts`

**Analogs:**
- `crates/vox-gui/src/commands/pty.rs:250-451` (`term_session_*` Tauri commands)
- `crates/vox-gui/src/commands/console_a2a.rs:36-67` (`send_to_agent_impl` + `#[tauri::command] pub async fn send_to_agent`)
- `crates/vox-gui/src/remote/gate.rs:22-35` (**CRITICAL MODULE GATE**)

**Critical Module Classification Rule (`crates/vox-gui/src/remote/gate.rs`):**
`axis_invoke_handler!` (`crates/vox-gui/src/main.rs:9-18`) inspects `manifest::module_class(module)` for every `commands::<module>::<cmd>`. `commands::pty` (`ModuleClass::Host`) and `commands::console_a2a` (`ModuleClass::Daemon`) are **already classified**. Keep the new commands inside those two modules rather than creating an unclassified `commands::<new_mod>`!

1. **`crates/vox-gui/src/commands/pty.rs` (currently ~355 non-blank pre-test LoC; stays `< 420` LoC with 4 thin wrappers):**
   Add helper `PtyManager::daemon_client(&self, daemon: &Arc<PersistentDaemon>) -> Option<OrchDaemonClient>` (reusing `L164-169`) and 4 Tauri commands right after `term_session_close` (`L451`):
   - `term_session_pin(manager, daemon, session_id: String, block_id: u64, pinned: bool) -> Result<(), String>` → calls daemon `"term.pin"` or `manager.local_host` fallback
   - `term_session_promote_bg(manager, daemon, session_id: String) -> Result<String, String>` → calls daemon `"term.promote_bg"` or `manager.local_host` fallback
   - `term_session_set_security(manager, daemon, session_id: String, level: vox_terminal_core::TermSecurityLevel) -> Result<(), String>` → calls daemon `"term.security"` or `manager.local_host.set_security_level(&session_id, level)`
   - `term_session_set_control(manager, daemon, session_id: String, mode: vox_terminal_core::TermControlMode) -> Result<(), String>` → calls daemon `"term.control"` or `manager.local_host.set_control_mode(&session_id, mode)`

2. **`crates/vox-gui/src/commands/console_a2a.rs` (`L59-68`):**
   Add `send_terminal_block_to_agent_impl<B: BackendCalls>(b: &B, agent_id: String, session_id: String, block_id: u64, command: String, raw_output: String, exit_code: Option<i32>, note: Option<String>) -> Result<String, String>` + `#[tauri::command] pub async fn send_terminal_block_to_agent(...)`:
   - Scrub `command` and `raw_output` through `vox_terminal_core::corpus::redact::scrub_terminal_text` + `vox_secrets::redact::scrub_resolved_secrets` (note: check `crates/vox-gui/Cargo.toml` for `vox-secrets`; `vox-gui` already depends on `vox-secrets` for user/secret settings, and `vox-terminal-core` for PTY!), apply `truncate_head_tail(..., 40, 160, 16_384)`, format structured markdown/XML block envelope, and delegate to `send_to_agent_impl(b, agent_id, formatted_body).await`!

3. **`crates/vox-gui/src/main.rs` (`L199-207` inside `axis_invoke_handler!`):**
```rust
            commands::pty::term_session_pin,
            commands::pty::term_session_promote_bg,
            commands::pty::term_session_set_security,
            commands::pty::term_session_set_control,
            commands::console_a2a::send_to_agent,
            commands::console_a2a::send_terminal_block_to_agent,
```

4. **`crates/vox-gui/ui/src/transport.ts` (`L861-885`):**
   - `L861`: Widen `export type TermBlockStatus = 'Running' | 'AwaitingInput' | 'Ok' | 'Failed' | 'Cancelled';`
   - `L869-879` (`TermBlockDto`): Add optional `pinned?: boolean; credential_prompt?: boolean;`
   - Export thin `safeInvoke` wrappers `termSessionPin`, `termSessionPromoteBg`, `termSessionSetSecurity`, `termSessionSetControl`, `sendTerminalBlockToAgent` so Phase 442 backend contract matches TypeScript types without touching Phase 443 React components.

---

## Shared Patterns

### 1. 3-Stage Scrubbing & Static Audit Guard Compliance
- **Pipeline:** Every terminal string crossing into an LLM tool response, `@term` chat prompt preamble, or `send_terminal_block_to_agent` A2A payload **must** pass through:
  1. `vox_terminal_core::block::strip_ansi_and_normalize` (OSC/CSI/DCS/APC stripping + `\r` progress-bar overwrite collapse)
  2. `vox_redact::redact_owned` (PEM blocks, URL `user:pass@`, emails, `KEY=val`, provider tokens `ghp_`/`sk-`/`sk-ant-`/`AIza`/`hf_`/`npm_`/`AKIA`/JWT, IPv4, `/Users/<name>` + `/home/<name>` + `C:\Users\<name>`)
  3. `vox_secrets::redact::scrub_resolved_secrets` (Aho-Corasick `LeftmostLongest` over `spec.id.metadata()` non-config resolved Clavis secrets `>= 6` chars → `[REDACTED:CLAVIS]`)
- **Audit guards:**
  - Never name struct fields or variables `password_*` or `secret_*` outside `vox-secrets` (`credential_prompt: bool` passes `vox ci secret-env-guard`).
  - Never write string literals starting with `"/home`, `"/tmp`, `"/usr`, or `"C:\` without `(?:` regex grouping or `// vox-arch-check: allow abs-path` (`[no-hardcoded-abs-path]`).

### 2. Zero-Edge `TerminalBridge` & `[lib]`/`[[bin]]` Sharing
- `vox-orchestrator` defines pure DTOs + `#[async_trait] pub trait TerminalBridge` + `OrchDaemonTerminalBridge` (over `OrchDaemonClient`), adding **zero** new Cargo dependency edges (`vox-arch-check` clean).
- `vox-orchestrator-d` exports `pub mod term_dispatch; pub mod term_bridge_impl;` from `src/lib.rs`, wires a single `Arc<LocalHost>` into both `TermDispatch` and `ServerState::set_terminal_bridge(...)` in `src/bin/vox_orchestrator_d.rs`, and makes `term_dispatch` + `term_bridge_impl` directly unit-testable via `cargo test -p vox-orchestrator-d --lib`.

### 3. `arch/god_object` (`< 500` LoC / `<= 12` Methods) Budget Discipline
- Every new file (`exec_policy.rs`, `stall_detector.rs`, `agent_exec.rs`, `terminal_bridge.rs`, `term_bridge_impl.rs`, `terminal_schemas.rs`, `terminal_tools.rs`, `term_mentions.rs`) must stay strictly `< 400` non-blank pre-test LoC and `<= 12` methods per `impl` block.
- `crates/vox-terminal-core/src/host.rs` (currently 972 pre-test lines, 19 methods on `impl LocalHost`) **must shrink**, not grow: move `L77-310` command-allowlist helpers + new agent execution methods into `crates/vox-terminal-core/src/agent_exec.rs`.

### 4. 5-Gate MCP Tool Registration & HITL Parity Synchronization
Adding `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions` requires updating all 5 gates in the same wave so `cargo test -p vox-orchestrator-mcp` stays green:
1. `contracts/operations/catalog.v1.yaml` + `vox ci operations-sync --write`
2. `contracts/mcp/http-read-role-governance.yaml` (`vox_terminal_read`, `vox_terminal_sessions`)
3. `contracts/orchestration/permission-modes.v2.yaml` + `permission_modes.rs::RISK_CLASSES` + `floor.rs::SHELL_COMMAND_TOOLS`
4. `input_schemas.rs` (`tool_input_schema` chain **AND** `#[cfg(any())] match name`)
5. `dispatch.rs` (`handle_tool_call_inner` chain + `SKIP_DISPATCH_PROBE` + `include_str!("dispatch_domains/terminal_tools.rs")`) + `dispatch_timeout.rs` (`150s` for `vox_terminal_exec`)

---

## No Analog Found

| File | Role | Data Flow | Reason & Recommended Approach |
|---|---|---|---|
| `crates/vox-terminal-core/src/stall_detector.rs` | heuristic state-machine | event-driven timer + tail scanner | No existing PTY interactive-prompt / credential-stall detector exists in the workspace. Implement as a pure, deterministic state machine (`StallDetector::observe_chunk` / `StallDetector::check_elapsed(now_ms, alt_screen, block_status)`) borrowing `BlockStatus` / `AltScreen` idioms from `session.rs` and `vt_shadow.rs`, so every prompt heuristic (`YesNo`, `Credential`, `Selection`, `PressEnter`, `InteractiveTui`, `GenericQuestion`) is 100% unit-testable without spawning OS PTYs. |

---

## Metadata

**Analog search scope:**
- `crates/vox-redact/`, `crates/vox-secrets/`, `crates/vox-terminal-core/`
- `crates/vox-orchestrator/`, `crates/vox-orchestrator-d/`, `crates/vox-orchestrator-mcp/`
- `crates/vox-cli-research/`, `crates/vox-gui/`, `crates/vox-foundation/`
- `contracts/terminal/`, `contracts/operations/`, `contracts/mcp/`, `contracts/orchestration/`

**Files scanned:** 35 tracked repository files (verified via `git ls-files`)
**Pattern extraction date:** 2026-11-14
