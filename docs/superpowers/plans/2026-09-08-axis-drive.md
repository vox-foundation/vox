---
title: "Axis Drive Implementation Plan"
description: "Task-by-task plan for vox gui drive: contract, session, loopback listener, DriveBus, and headless plane."
category: "roadmap"
status: "roadmap"
training_eligible: true
---

# Axis Drive Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Agents can start a dedicated debug Axis, set every send-changing composer knob, send a turn through `handleLoquelaSubmit` → `chat_turn`, and read picker/catalog truth from the terminal — without touching the user's Axis window.

**Architecture:** Loopback HTTP/JSON drive bus inside a second `vox-gui --drive` process (hidden unless `--show`). Isolation is skip-localStorage-pin-read + journey-store override — **not** `$HOME` / `VOX_GUI_DATA_DIR` for WKWebView. CLI is `vox gui drive start|set|send|state|wait|stop|show|headless`. Headless is `vox-gui --drive-headless` stdin/stdout JSON with `plane: "headless"` and `claims.*.false`. Spec: [docs/superpowers/specs/2026-09-08-axis-drive-design.md](docs/superpowers/specs/2026-09-08-axis-drive-design.md).

**Tech Stack:** Existing `vox-cli` (`gui` feature), `vox-gui` binary crate + React 19 / vitest. No new crate. No new HTTP framework — `std::net::TcpListener` on `127.0.0.1:0` via `tauri::async_runtime::spawn`. Drive Console strip is unchanged.

## Implementation status

Landed on `main` / this branch: Tasks 1–8 of the audited redesign (contract, clap, bearer-ping session + Windows lock parity, listener, honesty, detached launch, AxisDriveHost, headless claims including `events: false`, catalog SSOT). Per-task bodies below retain historical TDD sketches; **Global Constraints** and the design spec are authoritative when a sketch conflicts (no `kill(0)` sole liveness, no interim `ready=true` at bind, no env bearer).

## Global Constraints

- Spec name is **Axis Drive**. Do not rename or extend Drive Console. `clutch`/`risk` must parity-test against `drive-console.v1.yaml`.
- Never attach to the user's Axis. `start` refuses (exit 2) if a **bearer ping** succeeds. `pid` is a hint only — no `libc::kill(0)` as sole liveness.
- Bind `127.0.0.1` only. Never print the bearer token; never put it in the child env. Logs may name `token_path`.
- Token: 32 CSPRNG bytes, hex, `0600`. Fail closed. No timestamp fallback.
- `set` calls App + Loquela **setters**. `send` goes through App's `onSubmit` wrapper → `handleLoquelaSubmit` / `buildChatTurn` / `chat_turn`. Do not call `:9745` / `orch.tool_call`.
- Headless responses must include `"plane":"headless"` and `claims: { picker_ui: false, composer_knobs: false, bubbles: false, events: false }`. Live: `"plane":"live"`.
- `ready` stays false until AxisDriveHost registers.
- `DriveArgs` live in `cli_args.rs` only. `vox gui --command <view>` must keep working.
- Hand-author `gui` + `gui.drive.*` catalog rows **before** `operations-sync --target cli --write`. Clap does not invent rows.
- Register every new `VOX_*` env var in registry + `CONFIG_KEYS` + `env-vars.v1.yaml`.
- Mount via `AxisDriveHost.tsx`; App only renders the host.
- No new crate, no crate-edge exceptions, no Playwright drive spec, no `cargo test -p vox-gui --lib`.
- Format: `cargo fmt -p vox-cli` and `cargo fmt -p vox-gui`, never `cargo fmt --all`.
- `vox-cli` GUI code is behind `--features gui`. Root parse tests: `#[cfg(feature = "gui")]`.
- Test-first. Typed YAML deserialize (not substring grep). Mutation-check the 409 selectable guard. Live mens serve is not required for unit tests.

## File map

**Create**

| File | Responsibility |
|---|---|
| `contracts/gui/axis-drive.v1.yaml` | Verb/knob/error SSOT |
| `crates/vox-gui/src/drive/mod.rs` | Module root |
| `crates/vox-gui/src/drive/protocol.rs` | Serde types + `apply_set` / `empty_state` |
| `crates/vox-gui/src/drive/listener.rs` | Loopback HTTP + token check |
| `crates/vox-gui/src/drive/headless.rs` | Stdin JSON → handlers → stdout |
| `crates/vox-gui/src/drive/flags.rs` | `--drive` / `--drive-headless` / `--show` argv |
| `crates/vox-cli/src/commands/gui/mod.rs` | Dispatch launch vs drive (replaces `gui.rs`) |
| `crates/vox-cli/src/commands/gui/launch.rs` | Today's `vox gui` spawn |
| `crates/vox-cli/src/commands/gui/drive.rs` | Clap `DriveArgs` + `run` |
| `crates/vox-cli/src/commands/gui/session.rs` | Session file, pid, token, exit 2 |
| `crates/vox-cli/src/commands/gui/client.rs` | HTTP client for live verbs |
| `crates/vox-gui/ui/src/lib/axisDrive.ts` | TS types + `applySet` / `snapshotState` |
| `crates/vox-gui/ui/src/lib/axisDrive.test.ts` | Vitest for apply/snapshot |
| `crates/vox-gui/ui/src/lib/useDriveBus.ts` | Live-plane event bridge |
| `crates/vox-gui/ui/src/lib/useDriveBus.test.ts` | Setter injection + selectable=false |
| `crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx` | Thin App mount; wires setters |

**Modify**

| File | Change |
|---|---|
| `crates/vox-cli/src/cli_args.rs` | `GuiArgs.cmd: Option<GuiCmd>` |
| `crates/vox-cli/src/commands/gui.rs` | Delete after move to `gui/` |
| `crates/vox-gui/src/main.rs` | `mod drive`; flags; window visibility; data dir |
| `crates/vox-gui/ui/src/App.tsx` | Render `<AxisDriveHost />` only |
| `crates/vox-cli/tests/vox_cli_root_parsing.rs` | `vox gui drive start` + `--command` still parse |

**Do not create** a new crate. **Do not** add `contracts/gui/axis-drive.v1.schema.json` unless a gate already requires one for every YAML (drive-console has none).

---

### Task 1: Contract + protocol types

**Files:**
- Create: `contracts/gui/axis-drive.v1.yaml`
- Create: `crates/vox-gui/src/drive/mod.rs`
- Create: `crates/vox-gui/src/drive/protocol.rs`
- Modify: `crates/vox-gui/src/main.rs` (add `mod drive;` near other `mod` lines)
- Test: `crates/vox-gui/src/drive/protocol.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: nothing
- Produces: `DriveVerb`, `DrivePlane`, `DriveSet`, `DriveState`, `DriveErrorCode`, `apply_set(&mut DriveState, DriveSet) -> Result<(), DriveApplyError>`, `ALLOWED_SET_KEYS`

- [ ] **Step 1: Write the failing test**

Add to `crates/vox-gui/src/drive/protocol.rs` (file may not compile until Step 3 — write test module first in this file):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_yaml_lists_required_verbs_and_keys() {
        let yaml = include_str!("../../../../contracts/gui/axis-drive.v1.yaml");
        for needle in ["health", "set", "send", "state", "show", "model_override", "pin_policy"] {
            assert!(yaml.contains(needle), "missing {needle}");
        }
        assert!(!yaml.contains("eval"));
    }

    #[test]
    fn unknown_set_key_is_rejected() {
        let mut state = DriveState::empty_live();
        let set = DriveSet {
            extra: [("nope".into(), serde_json::json!(true))].into_iter().collect(),
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
        assert_eq!(state.knobs.model_override.as_deref(), Some("mens/e2e-smoke"));
        assert_eq!(state.plane, DrivePlane::Live);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-gui contract_yaml_lists_required_verbs_and_keys -- --nocapture`

Expected: FAIL (module/file missing or `include_str!` path missing)

- [ ] **Step 3: Write minimal implementation**

`contracts/gui/axis-drive.v1.yaml`:

```yaml
version: 1
name: axis-drive
listener_verbs: [health, set, send, state, show]
cli_only_verbs: [wait, start, stop, headless]
planes: [live, headless]
bind: 127.0.0.1
health_service: vox-gui-drive
set_keys:
  model_override: { type: string, alias: pin }
  pin_policy: { type: enum, values: [fail, coerce], default: fail }
  execution: { type: enum, values: [sync, background, plan] }
  tier: { type: enum, values: [local, mesh, cloud, auto] }
  clutch: { type: enum, values: [free, efficiency, balanced, genius] }
  risk: { type: enum, values: [high, moderate, low] }
  grounding_check_enabled: { type: bool }
  active_skill: { type: string, nullable: true }
  skill_exclusions: { type: string_list }
  session_id: { type: string }
  chat_session_id: { type: string }
  priority: { type: string }
  dry_run: { type: bool }
  allow_duplicate: { type: bool }
  mode: { type: enum, values: [plan, act, verify] }
  context_files: { type: string_list }
  refresh_catalog: { type: bool, action: true }
errors:
  unknown_key: 400
  empty_text: 400
  model_not_selectable: 409
  unauthorized: 401
```

`crates/vox-gui/src/drive/mod.rs`:

```rust
pub mod protocol;
```

`crates/vox-gui/src/drive/protocol.rs` — implement the types the tests import. Required shape:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrivePlane { Live, Headless }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DriveExecution { Sync, Background, Plan }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveErrorCode { UnknownKey, EmptyText, ModelNotSelectable, Unauthorized }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveApplyError {
    pub code: DriveErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriveSet {
    pub model_override: Option<String>,
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
pub struct DriveState {
    pub plane: DrivePlane,
    pub knobs: DriveKnobs,
    pub pin: Option<String>,
    pub catalog: Vec<DriveCatalogRow>,
    pub probe: DriveProbe,
    pub bubbles: Vec<serde_json::Value>,
    pub last_error: Option<String>,
    pub orch_fresh: bool,
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
        }
    }
}

pub fn apply_set(state: &mut DriveState, set: DriveSet) -> Result<(), DriveApplyError> {
    if !set.extra.is_empty() {
        let key = set.extra.keys().next().cloned().unwrap_or_default();
        return Err(DriveApplyError {
            code: DriveErrorCode::UnknownKey,
            message: format!("unknown_key:{key}"),
        });
    }
    if let Some(model) = set.model_override {
        state.knobs.model_override = Some(model.clone());
        state.pin = Some(model);
    }
    if let Some(execution) = set.execution {
        state.knobs.execution = Some(execution);
    }
    if let Some(tier) = set.tier { state.knobs.tier = Some(tier); }
    if let Some(clutch) = set.clutch { state.knobs.clutch = Some(clutch); }
    if let Some(risk) = set.risk { state.knobs.risk = Some(risk); }
    if let Some(v) = set.grounding_check_enabled { state.knobs.grounding_check_enabled = Some(v); }
    if let Some(v) = set.active_skill { state.knobs.active_skill = v; }
    if let Some(v) = set.skill_exclusions { state.knobs.skill_exclusions = v; }
    if let Some(v) = set.session_id { state.knobs.session_id = Some(v); }
    if let Some(v) = set.chat_session_id { state.knobs.chat_session_id = Some(v); }
    if let Some(v) = set.priority { state.knobs.priority = Some(v); }
    if let Some(v) = set.dry_run { state.knobs.dry_run = Some(v); }
    if let Some(v) = set.allow_duplicate { state.knobs.allow_duplicate = Some(v); }
    if let Some(v) = set.mode { state.knobs.mode = Some(v); }
    if let Some(v) = set.context_files { state.knobs.context_files = v; }
    Ok(())
}
```

If `include_str!` relative path is wrong from `src/drive/`, count `../` until you hit repo-root `contracts/`. From `crates/vox-gui/src/drive/protocol.rs` the path is `../../../../contracts/gui/axis-drive.v1.yaml`.

Add `mod drive;` in `crates/vox-gui/src/main.rs` next to existing `mod commands;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vox-gui contract_yaml_lists_required_verbs_and_keys set_model_and_execution_round_trip unknown_set_key_is_rejected -- --nocapture`

Expected: PASS

- [ ] **Step 5: Format and commit**

```bash
cargo fmt -p vox-gui
git add contracts/gui/axis-drive.v1.yaml crates/vox-gui/src/drive crates/vox-gui/src/main.rs
git commit -m "$(cat <<'EOF'
feat(gui): add Axis Drive contract and protocol types

EOF
)"
```

---

### Task 2: CLI clap (`vox gui drive …`) without launching Axis

**Files:**
- Create: `crates/vox-cli/src/commands/gui/mod.rs`
- Create: `crates/vox-cli/src/commands/gui/launch.rs` (move body of today's `gui.rs`)
- Create: `crates/vox-cli/src/commands/gui/drive.rs`
- Delete: `crates/vox-cli/src/commands/gui.rs` (after move — cannot keep both)
- Modify: `crates/vox-cli/src/cli_args.rs`
- Modify: `crates/vox-cli/tests/vox_cli_root_parsing.rs`
- Test: `crates/vox-cli/src/commands/gui/drive.rs`

**Interfaces:**
- Consumes: existing `GuiArgs.command: Option<String>`
- Produces: `GuiCmd::Drive(DriveArgs)`, `DriveCmd::{Start,Stop,Show,Set,Send,State,Wait,Headless}`, `pub async fn run_drive(args: DriveArgs) -> Result<()>` (stub `todo!` or `bail!("not implemented")` until Task 3)

Preserve every function and test from `gui.rs` (`run`, `resolve_or_build_gui`, `gui_missing_message_tests`) in `launch.rs`. `mod.rs` calls `launch::run` when `cmd` is `None`.

- [ ] **Step 1: Write the failing parse tests**

In `crates/vox-cli/src/commands/gui/drive.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn drive_start_parses_show_and_profile() {
        let args = DriveArgs::try_parse_from([
            "drive", "start", "--show", "--profile", "agent-1",
        ])
        .expect("parse");
        match args.cmd {
            DriveCmd::Start(s) => {
                assert!(s.show);
                assert_eq!(s.profile.as_deref(), Some("agent-1"));
            }
            other => panic!("expected Start, got {other:?}"),
        }
    }

    #[test]
    fn drive_set_accepts_repeated_knob() {
        let args = DriveArgs::try_parse_from([
            "drive", "set",
            "--knob", "model_override=mens/e2e-smoke",
            "--knob", "execution=sync",
        ])
        .expect("parse");
        match args.cmd {
            DriveCmd::Set(s) => {
                assert_eq!(s.knob.len(), 2);
                assert_eq!(s.knob[0], "model_override=mens/e2e-smoke");
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn drive_send_requires_text() {
        assert!(DriveArgs::try_parse_from(["drive", "send"]).is_err());
    }
}
```

In `crates/vox-cli/tests/vox_cli_root_parsing.rs` (inside `#[cfg(feature = "gui")]` if that is how `Cli::Gui` is gated; otherwise skip the feature cfg and use the same `with_cli_parse_stack` helper as `parse_db_explain_subcommand`):

```rust
#[test]
fn parse_gui_command_flag_still_works() {
    with_cli_parse_stack(|| {
        VoxCliRoot::try_parse_from(["vox", "gui", "--command", "chat"])
            .expect("gui --command");
    });
}

#[test]
fn parse_gui_drive_start() {
    with_cli_parse_stack(|| {
        VoxCliRoot::try_parse_from(["vox", "gui", "drive", "start"])
            .expect("gui drive start");
    });
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vox-cli --features gui drive_start_parses_show_and_profile -- --nocapture`

Expected: FAIL (`DriveArgs` not found)

- [ ] **Step 3: Implement clap + move launch**

`cli_args.rs` — replace `GuiArgs` with:

```rust
#[derive(clap::Args, Clone, Debug)]
pub struct GuiArgs {
    /// Open directly to a specific command panel (`vox gui --command chat`).
    #[arg(long, value_name = "COMMAND", help = "Open to a specific command panel")]
    pub command: Option<String>,
    #[command(subcommand)]
    pub cmd: Option<GuiCmd>,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum GuiCmd {
    /// Drive a dedicated debug Axis from the terminal (never the user's window).
    Drive(crate::commands::gui::drive::DriveArgs),
}
```

Define `DriveArgs` / `DriveCmd` in `cli_args.rs` next to `GuiArgs` (avoids `cli_args` → `commands` cycle). Tests in `drive.rs` import those types. `GuiCmd::Drive(DriveArgs)` stays in `cli_args.rs`.

`DriveCmd` shape:

```rust
#[derive(clap::Parser, Clone, Debug)]
pub struct DriveArgs {
    #[command(subcommand)]
    pub cmd: DriveCmd,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum DriveCmd {
    Start(DriveStartArgs),
    Stop,
    Show,
    Set(DriveSetArgs),
    Send(DriveSendArgs),
    State,
    Wait(DriveWaitArgs),
    Headless(DriveHeadlessArgs),
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveStartArgs {
    #[arg(long)]
    pub show: bool,
    #[arg(long)]
    pub profile: Option<String>,
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveSetArgs {
    #[arg(long, required = true)]
    pub knob: Vec<String>,
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveSendArgs {
    #[arg(long)]
    pub text: String,
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveWaitArgs {
    #[arg(long)]
    pub until: String,
    #[arg(long, default_value = "90s")]
    pub timeout: String,
}

#[derive(clap::Args, Clone, Debug)]
pub struct DriveHeadlessArgs {
    #[command(subcommand)]
    pub cmd: DriveHeadlessCmd,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum DriveHeadlessCmd {
    Set(DriveSetArgs),
    Send(DriveSendArgs),
    State,
}
```

`gui/mod.rs`:

```rust
pub mod drive;
pub mod launch;

pub async fn run(args: crate::cli_args::GuiArgs) -> anyhow::Result<()> {
    match args.cmd {
        Some(crate::cli_args::GuiCmd::Drive(d)) => drive::run(d).await,
        None => launch::run(args).await,
    }
}
```

`drive::run` for this task:

```rust
pub async fn run(_args: DriveArgs) -> anyhow::Result<()> {
    anyhow::bail!("axis drive is not implemented yet")
}
```

Copy `gui.rs` → `launch.rs` unchanged except `pub async fn run`.

- [ ] **Step 4: Run tests**

```bash
cargo test -p vox-cli --features gui drive_start_parses_show_and_profile drive_set_accepts_repeated_knob drive_send_requires_text parse_gui_command_flag_still_works parse_gui_drive_start -- --nocapture
cargo test -p vox-cli --features gui leads_with_actual_status_not_installed_instruction -- --nocapture
```

Expected: PASS. Launch-missing-GUI test still passes.

- [ ] **Step 5: Format and commit**

```bash
cargo fmt -p vox-cli
git add crates/vox-cli/src/cli_args.rs crates/vox-cli/src/commands/gui crates/vox-cli/src/commands/gui.rs crates/vox-cli/tests/vox_cli_root_parsing.rs
git commit -m "$(cat <<'EOF'
feat(cli): parse vox gui drive without breaking --command

EOF
)"
```

If `gui.rs` delete is not staged, the build will fail (`mod gui` vs both file and dir). Stage the deletion.

---

### Task 3: Session file + start/stop against a fake child

**Files:**
- Create: `crates/vox-cli/src/commands/gui/session.rs`
- Modify: `crates/vox-cli/src/commands/gui/drive.rs` (`Start`/`Stop` call session)
- Test: `crates/vox-cli/src/commands/gui/session.rs`

**Interfaces:**
- Consumes: `DriveStartArgs`
- Produces: `DriveSession { schema_version, pid, port, token_path, store_path, show }`, `preflight_start` (bearer ping refuse), `ping_drive`, `acquire_lock` (Unix `flock` / Windows exclusive `share_mode(0)`), `generate_token` via `OsRng` (32 bytes hex; **no** timestamp fallback / **no** `kill(0)` sole liveness)

Session path: `$VOX_HOME/run/gui-drive.json` if `VOX_HOME` set, else `~/.vox/run/gui-drive.json`. Tests set `VOX_HOME` to a tempdir. Token never in child env (`VOX_GUI_DRIVE_TOKEN_PATH` only).

- [x] **Step 1: Write failing tests** — `start_refuses_when_ping_succeeds`, `start_replaces_dead_session`, `token_file_is_0600_and_not_in_session_json_value`
- [x] **Step 2: Run — expect FAIL** (historical TDD)
- [x] **Step 3: Implement** bearer-ping preflight + lock + OsRng token + 0600 file
- [x] **Step 4: Run — expect PASS**
- [x] **Step 5: Commit** (landed)

---

### Task 4: Loopback listener (Rust)

**Files:**
- Create: `crates/vox-gui/src/drive/listener.rs`
- Modify: `crates/vox-gui/src/drive/mod.rs` (`pub mod listener;`)
- Test: `crates/vox-gui/src/drive/listener.rs`

**Interfaces:**
- Consumes: `protocol::{DriveSet, DriveState}`
- Produces: `fn bind_loopback(token: &str) -> io::Result<ListenerHandle>` with `.addr() -> SocketAddr`, `.set_ready(bool)`, `.set_handler(Arc<dyn Fn(DriveHttpRequest) -> DriveHttpResponse + Send + Sync>)`

HTTP subset: `GET /health` (no auth) → `{"service":"vox-gui-drive","ready":<bool>}`. `POST /v1/{set,send,state,show}` requires `Authorization: Bearer <token>`. Else 401. Unknown path 404. Bind `127.0.0.1:0` only.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn health_names_service_without_token() {
    let h = bind_loopback("secret-token").unwrap();
    let url = format!("http://{}/health", h.addr());
    let body = ureq_or_std_get(&url);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["service"], "vox-gui-drive");
    assert_eq!(v["ready"], false);
    h.shutdown();
}

#[test]
fn missing_token_is_401() {
    let h = bind_loopback("secret-token").unwrap();
    let status = post_no_auth(format!("http://{}/v1/state", h.addr()));
    assert_eq!(status, 401);
    h.shutdown();
}

#[test]
fn valid_token_state_is_200() {
    let h = bind_loopback("secret-token").unwrap();
    h.set_ready(true);
    h.set_handler(Arc::new(|req| {
        assert_eq!(req.verb, "state");
        DriveHttpResponse { status: 200, body: r#"{"plane":"live"}"#.into() }
    }));
    let (status, body) = post_auth(
        format!("http://{}/v1/state", h.addr()),
        "secret-token",
        "{}",
    );
    assert_eq!(status, 200);
    assert!(body.contains("live"));
    h.shutdown();
}
```

Do **not** add `ureq` if it is not already a `vox-gui` dep. Use `std::net::TcpStream` + write `GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n` and read the response. Put a 20-line `http_exchange` helper in the test module.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p vox-gui health_names_service_without_token -- --nocapture`

- [ ] **Step 3: Implement listener**

Spawn a dedicated `std::thread` (do not take Tauri’s runtime). Parse method, path, headers, optional body (`Content-Length`). Reject if local addr is not 127.0.0.1 (it cannot be if you bind that). Compare bearer with constant-time `bool` via `vox_crypto` if a helper exists; otherwise `token.as_bytes() == provided.as_bytes()` is acceptable for v1 (loopback).

Default handler for `/v1/state` before DriveBus connects: `200` + `DriveState::empty_live()` JSON so CLI `start` can wait on `/health` `ready` flipping true later.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p vox-gui health_names_service_without_token missing_token_is_401 valid_token_state_is_200 -- --nocapture`

- [ ] **Step 5: Format and commit**

```bash
cargo fmt -p vox-gui
git add crates/vox-gui/src/drive
git commit -m "$(cat <<'EOF'
feat(gui): serve Axis Drive on loopback with bearer auth

EOF
)"
```

---

### Task 5: DriveBus apply/snapshot (TypeScript)

**Files:**
- Create: `crates/vox-gui/ui/src/lib/axisDrive.ts`
- Create: `crates/vox-gui/ui/src/lib/axisDrive.test.ts`
- Test: that file

**Interfaces:**
- Consumes: `isModelSelectable` from `crates/vox-gui/ui/src/lib/modelPicker.ts`; `CHAT_TURN_KEYS` / `ChatTurnSource` from `buildChatTurn.ts`; `ClutchId` / `RiskId` from `driveConsole.ts`
- Produces: `export type DrivePlane = 'live' | 'headless'`, `export interface DriveSet`, `export interface DriveState`, `export function applySet(state: DriveState, set: DriveSet): DriveState`, `export function snapshotCatalog(models: PickerModel[], statuses: ProviderStatus[]): DriveCatalogRow[]`, `export function parseKnobPairs(pairs: string[]): DriveSet`

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, expect, it } from 'vitest';
import { applySet, emptyLiveState, parseKnobPairs, snapshotCatalog } from './axisDrive';
import type { PickerModel, ProviderStatus } from './modelPicker';

describe('axisDrive', () => {
  it('applies execution and model knobs', () => {
    const next = applySet(emptyLiveState(), {
      model_override: 'mens/e2e-smoke',
      execution: 'sync',
    });
    expect(next.plane).toBe('live');
    expect(next.knobs.model_override).toBe('mens/e2e-smoke');
    expect(next.knobs.execution).toBe('sync');
    expect(next.pin).toBe('mens/e2e-smoke');
  });

  it('rejects unknown keys', () => {
    expect(() => applySet(emptyLiveState(), { nope: true } as never)).toThrow(/unknown_key/);
  });

  it('marks local models unselectable when probe is down', () => {
    const models: PickerModel[] = [
      { id: 'mens/e2e-smoke-metal', label: 'metal', provider: 'VoxLocal', providerType: 'local' },
    ];
    const statuses: ProviderStatus[] = [
      {
        provider: 'VoxLocal',
        key_present: false,
        is_local: true,
        local_reachable: false,
        local_models: [],
      },
    ];
    const rows = snapshotCatalog(models, statuses);
    expect(rows[0]?.selectable).toBe(false);
    expect(rows[0]?.reason).toMatch(/reachable|local/i);
  });

  it('parseKnobPairs splits key=value', () => {
    const set = parseKnobPairs([
      'model_override=mens/e2e-smoke',
      'execution=sync',
      'grounding_check_enabled=true',
    ]);
    expect(set.model_override).toBe('mens/e2e-smoke');
    expect(set.execution).toBe('sync');
    expect(set.grounding_check_enabled).toBe(true);
  });
});
```

- [ ] **Step 2: Run — expect FAIL**

Run (from `crates/vox-gui/ui`): `pnpm vitest run src/lib/axisDrive.test.ts`

Expected: FAIL (module not found)

- [ ] **Step 3: Implement `axisDrive.ts`**

`applySet` copies known keys only. If a key is not in the allow-list (`model_override`, `pin`→`model_override`, `pin_policy`, `execution`, `tier`, `clutch`, `risk`, `grounding_check_enabled`, `active_skill`, `skill_exclusions`, `session_id`, `chat_session_id`, `priority`, `dry_run`, `allow_duplicate`, `mode`, `context_files`, `refresh_catalog`), throw `Error('unknown_key:' + key)`.

`snapshotCatalog` maps each model through `isModelSelectable`. `reason` when false: `'local_unreachable'` if `is_local && local_reachable !== true`, else `'not_listed'` / `'key_missing'`.

If `coercePinnedLocalModel` exists on this branch, **do not** call it inside `applySet` unless `pin_policy === 'coerce'`. Default `fail`.

- [ ] **Step 4: Run — expect PASS**

Run: `pnpm vitest run src/lib/axisDrive.test.ts`

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/lib/axisDrive.ts crates/vox-gui/ui/src/lib/axisDrive.test.ts
git commit -m "$(cat <<'EOF'
feat(gui): snapshot Axis Drive knobs and picker honesty

EOF
)"
```

---

### Task 6: Launch dedicated Axis (`start` / `show` / window / data dir)

**Files:**
- Create: `crates/vox-gui/src/drive/flags.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Modify: `crates/vox-cli/src/commands/gui/launch.rs` (export spawn helper that does **not** `.wait()`)
- Modify: `crates/vox-cli/src/commands/gui/drive.rs` (`Start`/`Show`/`Stop`)
- Modify: `crates/vox-cli/src/commands/gui/session.rs`
- Test: `crates/vox-gui/src/drive/flags.rs`; `crates/vox-cli/src/commands/gui/session.rs` (env assembly)

**Interfaces:**
- Consumes: Task 3 session + Task 4 listener
- Produces: `DriveFlags`, store root via `VOX_GUI_DRIVE_STORE_ROOT` → `~/.vox/gui-drive/<profile>/` (DB at `.vox/store.db` under that root). Child env: `VOX_GUI_DRIVE=1`, `VOX_GUI_DRIVE_TOKEN_PATH` (never bare `VOX_GUI_DRIVE_TOKEN`), session path, show flag. Isolation is store override + skip-localStorage — **not** `$HOME` / `VOX_GUI_DATA_DIR`.

Child (`vox-gui`):
- If `VOX_GUI_DRIVE=1` or `--drive`: parse flags; set webview/app data dir to `VOX_GUI_DATA_DIR` (create dir); bind listener with token; write session JSON (`pid`, `port`, paths, `show`); window title `Axis (drive)`; `visible = show`.
- Tauri 2 window: after `Builder`, in setup, `window.set_title("Axis (drive)")?; if !show { window.hide()?; }`.
- Data dir: if Tauri has no one-liner, set `WEBKIT_DISABLE_DMABUF_RENDERER` is **not** the isolation knob. Prefer `std::env::set_var("XDG_DATA_HOME", data_dir)` on Linux and, on macOS, pass a custom `tauri::path` base if the builder supports it; if the current Tauri version only uses the bundle identifier, isolate by setting `HOME` to `data_dir` **only inside the child** (CLI sets `HOME=data_dir` for the spawned process). Document that in a comment. Do not change the parent’s `HOME`.

CLI `start`:
1. `preflight_start` (exit 2 if alive)
2. Generate token, write `0600`
3. Spawn the same binary resolution as `launch.rs` plus args `--drive` and env above. **Do not wait.**
4. Poll session file for `port != 0` then `GET /health` until `ready` or 15s. On timeout: SIGTERM child, delete session, fail.
5. Print **only** `started pid=… port=… session=…` (no token).

`ready` stays **false** at bind. Task 7 / AxisDriveHost flips it true after mount. `start` waits on `/health` `ready:true` (do **not** set ready at bind).

- [ ] **Step 1: Write failing flag tests**

```rust
#[test]
fn parse_drive_and_show() {
    let f = parse_drive_flags(&[
        "vox-gui".into(),
        "--drive".into(),
        "--show".into(),
    ]);
    assert!(f.drive);
    assert!(f.show);
    assert!(!f.drive_headless);
}

#[test]
fn parse_headless_is_not_live() {
    let f = parse_drive_flags(&["vox-gui".into(), "--drive-headless".into()]);
    assert!(f.drive_headless);
    assert!(!f.drive);
}
```

CLI test: `drive_env_does_not_contain_user_home_as_data_dir` — `data_dir` ends with `gui-drive/<profile>`.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p vox-gui parse_drive_and_show -- --nocapture`

- [ ] **Step 3: Implement flags + spawn + hide**

- [ ] **Step 4: Run flag tests — expect PASS**

Run: `cargo test -p vox-gui parse_drive_and_show parse_headless_is_not_live -- --nocapture`

`cargo test -p vox-cli --features gui` session env test.

Manual (not CI): `vox gui drive start --show` then `vox gui drive stop`. Confirm the everyday Axis window (if open) is untouched.

- [ ] **Step 5: Format and commit**

```bash
cargo fmt -p vox-gui
cargo fmt -p vox-cli
git add crates/vox-gui/src/drive crates/vox-gui/src/main.rs crates/vox-cli/src/commands/gui
git commit -m "$(cat <<'EOF'
feat(gui): launch a hidden dedicated Axis Drive process

EOF
)"
```

---

### Task 7: Wire DriveBus to submit + listener

**Files:**
- Create: `crates/vox-gui/ui/src/lib/useDriveBus.ts`
- Create: `crates/vox-gui/ui/src/lib/useDriveBus.test.ts`
- Modify: `crates/vox-gui/ui/src/App.tsx`
- Modify: `crates/vox-gui/src/main.rs` (Tauri command `get_drive_mode` → `"off" | "live"`; events `drive://request` / `drive://response`)
- Modify: `crates/vox-gui/src/drive/listener.rs` (forward POST body to event; wait for response)
- Test: `useDriveBus.test.ts`

**Interfaces:**
- Consumes: `handleLoquelaSubmit` (inject as `submit`), `applySet` / `snapshotCatalog`, listen `drive://request`
- Produces: `export function handleDriveRequest(args: { state, setCatalog, submit, req }): Promise<DriveHttpLike>`

Event payload:

```typescript
type DriveRequest = { id: string; verb: 'set' | 'send' | 'state' | 'show'; body: unknown };
```

- [ ] **Step 1: Write the failing tests**

```typescript
import { describe, expect, it, vi } from 'vitest';
import { handleDriveRequest } from './useDriveBus';
import { emptyLiveState } from './axisDrive';

describe('handleDriveRequest', () => {
  it('send calls submit with description and not a daemon client', async () => {
    const submit = vi.fn(async () => undefined);
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [],
      statuses: [],
      submit,
      req: { id: '1', verb: 'send', body: { text: 'ping' } },
    });
    expect(submit).toHaveBeenCalledTimes(1);
    expect(submit.mock.calls[0]?.[0]).toMatchObject({ description: 'ping' });
    expect(JSON.stringify(res)).not.toMatch(/tool_call|9745/);
    expect(res.plane).toBe('live');
  });

  it('empty send is empty_text', async () => {
    const submit = vi.fn();
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [],
      statuses: [],
      submit,
      req: { id: '1', verb: 'send', body: { text: '' } },
    });
    expect(submit).not.toHaveBeenCalled();
    expect(res.error).toBe('empty_text');
  });

  it('fail pin_policy does not coerce an unselectable model', async () => {
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [
        {
          id: 'mens/e2e-smoke-metal',
          label: 'metal',
          provider: 'VoxLocal',
          providerType: 'local',
        },
      ],
      statuses: [
        {
          provider: 'VoxLocal',
          key_present: false,
          is_local: true,
          local_reachable: false,
          local_models: [],
        },
      ],
      submit: vi.fn(),
      req: {
        id: '1',
        verb: 'set',
        body: { model_override: 'mens/e2e-smoke-metal', pin_policy: 'fail' },
      },
    });
    expect(res.error).toBe('model_not_selectable');
    expect(res.status).toBe(409);
  });
});
```

`submit` argument type: the Loquela payload subset `{ description: string; execution_mode?: 'chat' | 'task' | 'plan'; model_override?: string | null; … }` — same fields `handleLoquelaSubmit` already accepts (`ChatPayload`). Map `execution: sync` → `execution_mode: 'chat'`, `background` → `'task'`, `plan` → `'plan'`.

- [ ] **Step 2: Run — expect FAIL**

Run: `pnpm vitest run src/lib/useDriveBus.test.ts` (cwd `crates/vox-gui/ui`)

- [ ] **Step 3: Implement hook + App mount**

`useDriveBus.ts` exports `handleDriveRequest` (pure, tested) and `useDriveBus({ enabled, submit, getModels, getStatuses })` which:
- no-ops when `enabled` is false
- listens with the existing Tauri event helper used for `vox://orch-status` (same `listen` import as `App.tsx`)
- on each request, calls `handleDriveRequest`, then `emit('drive://response', { id, ...res })`

`App.tsx`: `const driveMode = …` from `invoke('get_drive_mode')` once on mount. When `'live'`, call `useDriveBus({ enabled: true, submit: handleLoquelaSubmit, … })`. Pass current picker models/statuses already loaded for the composer.

Listener thread: on POST, `app_handle.emit("drive://request", …)` and wait up to 30s on a oneshot keyed by `id` that setup registers when `drive://response` arrives. Timeout → 504.

`show` verb: `window.show()` + `set_title("Axis (drive)")` on the Rust side (do not ask React to show the window).

- [ ] **Step 4: Run — expect PASS**

Run: `pnpm vitest run src/lib/useDriveBus.test.ts src/lib/axisDrive.test.ts`

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/lib/useDriveBus.ts crates/vox-gui/ui/src/lib/useDriveBus.test.ts crates/vox-gui/ui/src/App.tsx crates/vox-gui/src
git commit -m "$(cat <<'EOF'
feat(gui): apply Axis Drive verbs through the composer submit path

EOF
)"
```

---

### Task 8: Headless plane + CLI client (`set`/`send`/`state`/`wait`)

**Files:**
- Create: `crates/vox-gui/src/drive/headless.rs`
- Create: `crates/vox-cli/src/commands/gui/client.rs`
- Modify: `crates/vox-cli/src/commands/gui/drive.rs`
- Modify: `crates/vox-gui/src/main.rs` (if `--drive-headless`, skip Tauri; `headless::run_stdio()`)
- Test: `crates/vox-gui/src/drive/headless.rs`; `crates/vox-cli/src/commands/gui/client.rs`

**Interfaces:**
- Consumes: `DriveSet` / `DriveState`; existing `chat_turn` function (call it only when a test injects a stub — production headless may return `last_error: "chat_turn requires tauri runtime"` for send **or** construct the same `ChatTurnInput` and call `chat_turn` if you can obtain `AppHandle` without a window). Prefer: headless `send` builds `ChatTurnInput` and calls `chat_turn` with a test `AppState` if tests already do that in `chat_turn.rs`. If `chat_turn` requires a full daemon, headless `send` still returns `plane: "headless"` and forwards the `chat_turn` error string in `last_error` — do not open a window.
- Produces: `pub fn handle_headless(req: HeadlessRequest) -> HeadlessResponse` where every response has `plane: Headless`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn headless_send_empty_text() {
    let res = handle_headless(HeadlessRequest {
        verb: HeadlessVerb::Send,
        text: Some("".into()),
        set: DriveSet::default(),
    });
    assert_eq!(res.plane, DrivePlane::Headless);
    assert_eq!(res.error.as_deref(), Some("empty_text"));
}

#[test]
fn headless_state_declares_plane() {
    let res = handle_headless(HeadlessRequest {
        verb: HeadlessVerb::State,
        text: None,
        set: DriveSet::default(),
    });
    assert_eq!(res.plane, DrivePlane::Headless);
    let json = serde_json::to_string(&res).unwrap();
    assert!(json.contains("\"headless\""));
}
```

CLI client test with a thread listener from Task 4:

```rust
#[test]
fn client_state_uses_session_port_and_token() {
    let token = "t";
    let h = vox_gui_listener_bind(token); // or duplicate a tiny echo server in the test
    // …
}
```

If importing `vox-gui`’s listener from `vox-cli` would add a crate edge, **do not**. Write the client test against a 30-line echo `TcpListener` in the CLI test module.

`wait`: poll `GET`/`POST /v1/state` every 200ms until `until` matches or timeout. `until=reply` → last bubble `role==assistant` or `last_error` set. `until=error` → `last_error` Some. `until=selectable=ID` → catalog row selectable.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p vox-gui headless_send_empty_text -- --nocapture`

- [ ] **Step 3: Implement headless + client**

`main.rs` top (next to `--print-action-manifest-json`):

```rust
if args.iter().any(|a| a == "--drive-headless") {
    if let Err(err) = drive::headless::run_stdio() {
        eprintln!("{err}");
        std::process::exit(1);
    }
    return;
}
```

`run_stdio`: read all stdin, parse `HeadlessRequest`, print `serde_json::to_string(&handle_headless(req))?`.

CLI live verbs: load session; if missing, exit 1 with spec copy. `client::post(verb, body)` adds bearer from `token_path`. Print response JSON to stdout (still no token).

`headless` CLI: spawn resolved `vox-gui` with `--drive-headless`, write request JSON to stdin, inherit no TTY, read stdout JSON. Do not set `VOX_GUI_DRIVE=1` (that would start the live listener).

- [ ] **Step 4: Run — expect PASS**

```bash
cargo test -p vox-gui headless_send_empty_text headless_state_declares_plane -- --nocapture
cargo test -p vox-cli --features gui --lib
```

- [ ] **Step 5: Command catalog + format + commit**

```bash
cargo run -q -p vox-cli --features gui -- ci operations-sync --target cli --write
cargo run -q -p vox-cli -- ci command-sync --write
cargo fmt -p vox-cli
cargo fmt -p vox-gui
git add crates/vox-gui/src/drive crates/vox-gui/src/main.rs crates/vox-cli/src/commands/gui contracts/operations/catalog.v1.yaml contracts/cli/command-registry.yaml docs/src/reference/cli-command-surface.generated.md
git commit -m "$(cat <<'EOF'
feat(cli): add vox gui drive client and headless plane

EOF
)"
```

If operations-sync rewrites a huge catalog, include it. Do not hand-edit generated markdown.

---

## Plan self-review

**Spec coverage**

| Spec section | Task |
|---|---|
| Dedicated debug Axis, never attach | 3, 6 |
| Hidden / `--show` | 2, 6, 7 (`show` verb) |
| All send-changing knobs + picker truth | 1, 5, 7 |
| Loopback + token | 4, 6 |
| `handleLoquelaSubmit` send path | 7 |
| Headless `plane` tag | 8 |
| `vox gui --command` preserved | 2 |
| No Playwright drive plane | Global constraint |
| Catalog sync | 8 Step 5 |

**Placeholders:** none. Windows Drive lock uses exclusive `share_mode(0)` (flock parity). Liveness is bearer ping, not pid.

**Types:** `DriveSet` / `DriveState` / `DrivePlane` names are identical in Rust and TS. `empty_text` / `unknown_key` / `model_not_selectable` match the YAML.

---

## Done when

```bash
cargo test -p vox-gui contract_yaml_lists_required_verbs_and_keys health_names_service_without_token headless_state_declares_plane -- --nocapture
cargo test -p vox-cli --features gui drive_start_parses_show_and_profile start_refuses_when_pid_alive -- --nocapture
pnpm --dir crates/vox-gui/ui vitest run src/lib/axisDrive.test.ts src/lib/useDriveBus.test.ts
```

Manual (operator): `vox gui drive start --show`, `set` `mens/e2e-smoke`, `send --text ping`, `state`, `stop`. User Axis pin unchanged.
