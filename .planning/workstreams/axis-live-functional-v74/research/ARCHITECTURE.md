# Architecture Research

**Domain:** Vox Axis Desktop IDE (`crates/vox-gui` + `crates/vox-gui/ui`) — End-to-End Tauri/MCP Control Wiring, Dual-Theme shadcn/Radix Component Layer, `@container` Bento Layouts & Concept-Art CV Flywheel (Milestone v7.4)
**Researched:** 2026-11-13
**Confidence:** HIGH

## Standard Architecture

### System Overview

> **Binary Crate Note:** `vox-gui` is declared in [`docs/src/architecture/layers.toml`](file:///Users/brbrainerd/dev/vox/docs/src/architecture/layers.toml#L305) as an **L5 binary crate** (`kind = "binary"`, `max_loc = 25_000`) — **`crates/vox-gui/src/lib.rs` does not exist**. All Tauri state registration, `axis_invoke_handler!` command tables, and startup lifecycle live in [`crates/vox-gui/src/main.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/main.rs#L1-L483). Furthermore, `vox-gui` intentionally holds **zero persistent `VoxDb` connection pools** ([`main.rs:120-124`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/main.rs#L120-L124) resets the Drive store and immediately drops any handle) so `vox-orchestrator-d` remains the sole local Turso/libSQL writer without SQLite file-lock contention.

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ LAYER A: React 19 Surfaces & @container Bento / Dock Workspaces (crates/vox-gui/ui/src/) │
│  ┌─────────────────────┐  ┌──────────────────────────┐  ┌─────────────────────────────┐  │
│  │ 33 SURFACE_REGISTRY │  │ 14 Chat Companion Docks  │  │ DashboardGrid Mini-Renders  │  │
│  │ Views (SurfaceScroll│  │ (DockWorkspaceShell      │  │ (BentoTile @container       │  │
│  │ Host @container)    │  │  ResizeObserver rAF)     │  │  inline-size containment)   │  │
│  └──────────┬──────────┘  └────────────┬─────────────┘  └──────────────┬──────────────┘  │
├─────────────┴──────────────────────────┴───────────────────────────────┴─────────────────┤
│ LAYER B: Dual-Theme shadcn/Radix UI Primitive Layer (ui/src/components/ui/*)             │
│  ┌───────────────────────────────────────┐  ┌─────────────────────────────────────────┐  │
│  │ RomanSurfaceHeader + SVG Icons.tsx    │  │ Radix Primitives (data-slot="<name>")   │  │
│  │ (0 raw emojis; Cinzel friezes in      │  │ Button, Select, Tabs, Switch, Dialog,   │  │
│  │  arcane/travertine; hidden in slate)  │  │ Popover, DropdownMenu, Sheet, Card      │  │
│  └───────────────────┬───────────────────┘  └────────────────────┬────────────────────┘  │
│                      └─────────────────────┬─────────────────────┘                       │
│     Style Dictionary 4-Sheet Tokens -> Tailwind v4 @theme inline -> shadcn CSS aliases   │
├────────────────────────────────────────────┼─────────────────────────────────────────────┤
│ LAYER C: Single Frontend IPC Boundary (ui/src/transport.ts + transport/t3Wave3Rpc.ts)    │
│  ┌─────────────────────────────────────────▼──────────────────────────────────────────┐  │
│  │ voxTransport (ONLY legal @tauri-apps/api/{core,event} importer; guarded by Vitest) │  │
│  │  • Injects permissionMode on invokeMcpTool(tool, args)                             │  │
│  │  • Browser/offline (!backendAvailable()) -> rejects typed BackendUnavailableError  │  │
│  │  • ZERO catch-and-fake _demo tokens, fake Git OIDs, or hardcoded ws://127.0.0.1    │  │
│  └─────────────────────────────────────────┬──────────────────────────────────────────┘  │
├────────────────────────────────────────────┼─────────────────────────────────────────────┤
│ LAYER D: Tauri v2 IPC Gate & Command Table (crates/vox-gui/src/) [Layer L5]              │
│  ┌─────────────────────────────────────────▼──────────────────────────────────────────┐  │
│  │ axis_invoke_handler! (main.rs:9-18, 190-462) + COMMAND_MODULES                     │  │
│  │  └─► remote::gate::gate_invoke (remote/gate.rs:47-63)                              │  │
│  │       Checks contracts/gui/command-backends.v1.yaml (daemon | host | shell; 0 db)  │  │
│  └──────────────────┬──────────────────────────────────────────────┬──────────────────┘  │
│                     │ class: daemon                                │ class: host / shell │
│                     ▼                                              ▼                     │
│  ┌─────────────────────────────────────────┐  ┌───────────────────────────────────────┐  │
│  │ BackendCalls (remote/calls.rs:14-134)   │  │ Host & Shell Handlers (commands/*.rs) │  │
│  │  • Local:  PersistentDaemon TCP RPC     │  │  • Clavis: vox_secrets (L1, redacted) │  │
│  │  • Remote: GatewayClient (/v1/ws, v2)   │  │  • Config: vox_config::VoxConfig (L2) │  │
│  └──────────────────┬──────────────────────┘  │  • Sidecars: Loquela/Fabrica/Terminal │  │
└─────────────────────┼─────────────────────────┴───────────────────────────────────────┘  │
                      ▼                                                                    │
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ LAYER E: Rust Daemon, MCP Tool Dispatch & Storage Spine [Layers L3 -> L1]                │
│  ┌────────────────────────────────────────────────────────────────────────────────────┐  │
│  │ vox-orchestrator-d + vox-orchestrator-mcp (t3_wave3_tools.rs, dispatch_domains/*)  │  │
│  │  • Persistence: VoxDb (Turso/libSQL — preferences, chat, codex, telemetry, drive)  │  │
│  │  • Live LLM Fan-Out / Routing: vox_actor_runtime::llm (L3; zero vendor HTTP in L5) │  │
│  │  • Secrets Resolution: vox_secrets::resolve_secret (Clavis SSoT; zero std::env)    │  │
│  └────────────────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **Surface & Workspace Containers** | Render domain workbenches across full windows (`900×600`–`1920×1080`), 14 `DockWorkspaceShell` panes, and `DashboardGrid` mini-tiles using `@container` inline-size queries | [`SurfaceScrollHost.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/layout/SurfaceScrollHost.tsx), [`DockWorkspaceShell.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.tsx), new `BentoGrid.tsx` & `SplitWorkbench.tsx` |
| **Dual-Theme shadcn/Radix Layer** | Provide accessible, keyboard-navigable, portal-safe UI primitives (`data-slot="<primitive>"`) adapting automatically between Roman White/Gold (`arcane`, `glacier`/`travertine`) and Minimalist Grey-Blue (`slate`/`void`, `high-contrast`) with zero raw emojis | [`RomanSurfaceHeader.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/ui/RomanSurfaceHeader.tsx), [`Icons.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/ui/Icons.tsx), upgraded [`components/ui/*`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/ui/) |
| **Token & Theme Pipeline** | Compile W3C design tokens into 4 theme stylesheets and expose unified Tailwind v4 `@theme inline` + shadcn semantic CSS variables without un-scoped `!important` button clobbering | [`style-dictionary.config.mjs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/style-dictionary.config.mjs), [`tokens/semantic*.json`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/tokens/semantic.json), [`index.css`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css) |
| **Frontend IPC Hub (`voxTransport`)** | Enforce a single typed boundary around `@tauri-apps/api/core` (`invoke`) and `@tauri-apps/api/event` (`listen`), attach `permissionMode` to MCP tool calls, and surface honest `BackendUnavailableError` rejections when offline | [`transport.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport.ts#L59-L80), [`t3Wave3Rpc.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts), [`backendGuard.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/lib/backendGuard.ts) |
| **Tauri Command Gate & Router** | Register all `#[tauri::command]` handlers, enforce remote-mode fail-closed gating against `command-backends.v1.yaml`, and dispatch `daemon`-class calls through `BackendCalls` | [`main.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/main.rs#L9-L18), [`remote/gate.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/gate.rs#L47-L63), [`remote/manifest.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/manifest.rs#L14-L46), [`remote/calls.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/calls.rs#L14-L134) |
| **Host Command Modules** | Execute strictly host-local capabilities (Clavis OS keyring/vault writes, `VoxConfig` TOML persistence, native file/PTY/sidecar control) while emitting Tauri push events for reactive UI updates | [`commands/secrets.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/secrets.rs), [`commands/llm_settings.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/llm_settings.rs), [`commands/mercatus.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/mercatus.rs) |
| **Daemon & MCP Tool Spine** | Execute stateful mutations against `VoxDb`, VCS/worktrees, guest-share state, and `vox_actor_runtime::llm` consensus/chat pipelines | [`crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs), `vox-orchestrator-d` |
| **Concept-Art CV Flywheel** | Generate reference concept-art plates via `image-generator` and score live rendered surfaces (`CV_STAGE=before` vs. `after`) across DOM geometry, axe-core WCAG, color histogram similarity, and VLM rubric gates | [`e2e/review/vlmVisionFlywheel.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts), [`e2e/review/cv-flywheel.spec.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts) |

---

## Recommended Project Structure

### Exact 4-Point Registration Contract for Tauri IPC Commands

Whenever a `#[tauri::command]` is added or moved in `crates/vox-gui/src/commands/`, **all four artifacts** must be updated in lockstep (enforced by `vox ci gui-catalog-parity` and Rust/Vitest unit tests):

1. **Command Module (`crates/vox-gui/src/commands/<mod>.rs` + [`commands/mod.rs:1-70`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/mod.rs#L1-L70)):** Implement `pub async fn <cmd>(...) -> Result<T, AxisError>`. Never accept or hold `State<'_, VoxDb>`.
2. **Handler Macro ([`crates/vox-gui/src/main.rs:9-18`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/main.rs#L9-L18) & [`190-462`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/main.rs#L190-L462)):** Add `commands::<mod>::<cmd>` to `axis_invoke_handler![...]`. This macro simultaneously expands `tauri::generate_handler!` and populates `COMMAND_MODULES: &remote::gate::CommandTable`.
3. **Backend Classification Catalog ([`contracts/gui/command-backends.v1.yaml:10-80`](file:///Users/brbrainerd/dev/vox/contracts/gui/command-backends.v1.yaml#L10-L80)):** Classify `<mod>` under `modules:` as `daemon`, `host`, or `shell` (`db` is permanently forbidden by [`commandBackendsMigration.test.ts:95-100`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/guards/commandBackendsMigration.test.ts#L95-L100)). Unclassified modules fail closed in remote mode (`AxisError::LocalOnly { class: "unclassified" }`, [`remote/gate.rs:29`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/gate.rs#L29)).
4. **Remote Routing Manifest ([`crates/vox-gui/src/remote/manifest.rs:14-46`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/manifest.rs#L14-L46)):** If `class: daemon`, list `<mod>` in `REMOTE_ROUTED_DAEMON_MODULES` (enforced by `every_daemon_module_has_a_routing_decision` at [`manifest.rs:289-304`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/manifest.rs#L289-L304)) and regenerate `ui/e2e/fixtures/remote-gate-mesh-console.json` if `surfaces:` change ([`manifest.rs:362-389`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/manifest.rs#L362-L389)).

### New vs. Modified Files Layout

```
crates/
├── vox-orchestrator-mcp/src/
│   ├── t3_wave3_tools.rs              # [MODIFIED] Add handle_guest_share_list; align DTO keys for
│   │                                  #   guest_share_mint (token/expires_at_ms), session_list_external,
│   │                                  #   muse_pool_adopt; remove synthetic message fabrication in moderate
│   └── dispatch_domains/
│       └── chat_session.rs            # [MODIFIED] Wire vox_guest_share_list tool dispatch + schema
├── vox-gui/
│   ├── src/
│   │   ├── main.rs                    # [AUDIT/MODIFIED] Register any new host helper in axis_invoke_handler!
│   │   ├── remote/
│   │   │   └── calls.rs               # [MODIFIED] Extend deadline_for_tool (L37-43) for vox_consensus_run
│   │   └── commands/
│   │       └── mercatus.rs            # [MODIFIED] Fix missing fs::create_dir_all before fs::write (L62-67);
│   │                                  #   emit vox://mercatus-config-changed on save
│   └── ui/
│       ├── package.json               # [MODIFIED] Add @radix-ui/react-{select,tabs,switch,popover,
│       │                              #   dropdown-menu,checkbox,slider,accordion,separator} + cva
│       ├── public/assets/romanesque/
│       │   └── v74-concept-*.jpg      # [NEW] Concept-art Bento & dual-theme plates via image-generator
│       ├── e2e/review/
│       │   ├── vlmVisionFlywheel.ts   # [MODIFIED] Register v74 concept-art plates + dual-theme rubric checks
│       │   └── cv-flywheel.spec.ts    # [MODIFIED] Add dual-theme (travertine + slate) & Bento container passes
│       └── src/
│           ├── index.css              # [MODIFIED] @theme inline shadcn aliases + container tokens; scope
│           │                          #   legacy button !important rules to :not([data-slot])
│           ├── transport.ts           # [MODIFIED] Add missing wrappers for the 38 direct-invoke + 8 listen
│           │                          #   bypass files; add subscribe* event helpers
│           ├── transport/
│           │   ├── t3Wave3Rpc.ts      # [MODIFIED] Delete ALL paper/demo fallbacks & hardcoded ws:// URLs;
│           │   │                      #   align DTOs with t3_wave3_tools.rs; propagate BackendUnavailableError
│           │   └── t3Wave3Rpc.test.ts # [MODIFIED] Assert BackendUnavailableError offline (no fake data)
│           ├── guards/
│           │   ├── ipcBoundaries.test.ts       # [MODIFIED] Shrink ALLOW_DIRECT_INVOKE (38 -> 0) & listen (8 -> 0)
│           │   └── noPaperFallbackGuard.test.ts# [NEW] AST/regex guard banning _demo/fake OIDs in src/transport/**
│           ├── components/
│           │   ├── layout/
│           │   │   ├── SurfaceScrollHost.tsx   # [MODIFIED] Add @container inline-size root contract
│           │   │   ├── BentoGrid.tsx           # [NEW] @container BentoGrid + BentoTile layout primitives
│           │   │   └── SplitWorkbench.tsx      # [NEW] @container responsive master-detail split shell
│           │   ├── ui/
│           │   │   ├── Button.tsx              # [MODIFIED] CVA + data-slot="button" + dual-theme variants
│           │   │   ├── Dialog.tsx              # [MODIFIED] data-slot="dialog-*" + portal token safety
│           │   │   ├── Popover.tsx             # [MODIFIED] Replace clipped absolute div with @radix-ui/react-popover Portal
│           │   │   ├── Segment.tsx             # [MODIFIED] Delegate to Radix Tabs (remove hardcoded bg-black/30)
│           │   │   ├── Panel.tsx               # [MODIFIED] Replace legacy var(--vscode-*) with Vox semantic tokens
│           │   │   ├── Icons.tsx               # [MODIFIED] Add GripDots, StatusDot, Romanesque empty-state SVGs
│           │   │   ├── Select.tsx              # [NEW] @radix-ui/react-select (replaces native <select>)
│           │   │   ├── Tabs.tsx                # [NEW] @radix-ui/react-tabs dual-theme pill & underline variants
│           │   │   ├── Switch.tsx              # [NEW] @radix-ui/react-switch (riveted brass / slate sky toggle)
│           │   │   ├── DropdownMenu.tsx        # [NEW] @radix-ui/react-dropdown-menu portal menu
│           │   │   ├── Input.tsx               # [NEW] Dual-theme text/number input with validation slot
│           │   │   ├── Textarea.tsx            # [NEW] Dual-theme auto/resizable textarea primitive
│           │   │   ├── Badge.tsx               # [NEW] Status & telemetry badge (pass/warn/fail/brass/slate)
│           │   │   ├── Card.tsx                # [NEW] RomanTablet / SlateCard container primitive
│           │   │   ├── Sheet.tsx               # [NEW] Slide-over drawer built on @radix-ui/react-dialog
│           │   │   └── Separator.tsx           # [NEW] @radix-ui/react-separator (brass hairline / slate rule)
│           │   ├── dock/DockWorkspaceShell.tsx # [MODIFIED] Sync dock layout v3 with get/setGuiPreference
│           │   ├── dashboard/DashboardGrid.tsx # [MODIFIED] Sync dashboard layout with get/setGuiPreference; SVG grip
│           │   └── surfaces/
│           │       ├── __guards__/honestyScan.ts      # [MODIFIED] Expand scan scope + demo token patterns
│           │       ├── Chat/SessionInteropModal.tsx   # [MODIFIED] Live guest submissions, file-picker import, zero gsub_demo_1
│           │       ├── Chat/ChatExecutionRail.tsx     # [MODIFIED] Live axisBackendStatus routes (no hardcoded 192.168.1.10)
│           │       ├── Mercatus/Mercatus.tsx          # [MODIFIED] Wire Switch/Input/Dialog to mercatusSaveConfig; Bento grid
│           │       └── {Repository,Settings,SkillsPlugins,Loquela,Tasks,CodeRabbit,Models,Scientia,...}
│           │                                          # [MODIFIED] Migrate off direct invoke/listen -> voxTransport + Bento/shadcn
```

### Structure Rationale

- **`crates/vox-orchestrator-mcp/src/` over new `vox-gui` DB commands:** Because `vox-gui` is an L5 binary with zero direct `VoxDb` handles (`CORE-04`), stateful additions like listing pending guest-share submissions belong on `vox_orchestrator_mcp` (`vox_guest_share_list`), automatically working over both local `PersistentDaemon` and remote `GatewayClient` through existing `commands::mcp::invoke_mcp_tool` ([`mcp.rs:148-159`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/mcp.rs#L148-L159)).
- **`ui/src/components/ui/` with `data-slot` attributes:** Keeps shadcn/Radix ergonomics idiomatic for React 19 (ref-as-prop, no legacy `React.forwardRef` wrappers) while giving [`index.css`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css) deterministic attribute hooks (`[data-slot="button"]`) that cleanly bypass legacy catch-all selectors.
- **`ui/src/components/layout/{BentoGrid,SplitWorkbench}.tsx`:** Separates container-query layout geometry from visual card chrome so every surface renders cleanly whether mounted full-window, docked at `320px`, or embedded inside `DashboardGrid`.

---

## Architectural Patterns

### Pattern 1: Single IPC Funnel & `PermissionMode` Propagation (`voxTransport`)

**What:** Every frontend IPC call and Tauri event subscription routes exclusively through [`crates/vox-gui/ui/src/transport.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport.ts#L59-L80), retiring all **38 direct `invoke` files** and **8 direct `listen` files** currently grandfathered in [`ipcBoundaries.test.ts:43-97`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts#L43-L97).
**When to use:** Always — for all Tauri commands, MCP tool invocations, and `vox://*` event listeners.
**Trade-offs:** Adds ~10 lines per new wrapper in `transport.ts`, but guarantees uniform `BackendUnavailableError` handling in browser preview mode and prevents silent security/governance drift such as [`SkillsPluginsView.tsx:48-50`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/SkillsPlugins/SkillsPluginsView.tsx#L48-L50), [`Loquela.tsx:1822`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx#L1822), and [`SettingsView.tsx:203`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx#L203) calling `invoke('invoke_mcp_tool', { tool, args })` without `permissionMode`.

**Example:**
```typescript
// crates/vox-gui/ui/src/transport.ts
export const voxTransport = {
  // Always propagates currentPermissionMode ('AutoApprove' | 'AskForMutations' | 'ReadOnlyPlan')
  invokeMcpTool: <T = unknown>(tool: string, args: Record<string, unknown> = {}): Promise<T> =>
    safeInvoke<T>('invoke_mcp_tool', {
      tool,
      args,
      permissionMode: currentPermissionMode,
    }),

  onLlmConfigChanged: (handler: () => void): Promise<UnlistenFn> =>
    safeListen('vox://llm-config-changed', () => handler()),
};
```

### Pattern 2: Honest Offline & Error Contract (Zero Paper Fallbacks)

**What:** Eliminate all silent `try { ... } catch { /* return fake data */ }` blocks in [`crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts). When `!backendAvailable()` (browser-only preview), read queries return explicit offline/empty descriptors (`{ offline: true, items: [] }`) or reject with `BackendUnavailableError`, and **all mutating operations strictly reject with `BackendUnavailableError`** — never fabricating `vox_share_${sessionId}_demo` ([`t3Wave3Rpc.ts:929`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts#L929)), `'gsub_demo_1'` ([`L995`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts#L995)), fake Git OIDs ([`L78`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts#L78)), or fake Rust diffs ([`L468-487`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts#L468-L487)). When `backendAvailable()` is true, daemon/MCP errors propagate directly to UI toast/inline error boundaries via `sanitizeErrorForToast`.
**When to use:** Every RPC client in `src/transport/` and every surface action handler.
**Trade-offs:** Requires updating standalone unit tests ([`t3Wave3Rpc.test.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.test.ts)) to assert `BackendUnavailableError` when `window.__TAURI_INTERNALS__` is absent, which directly prevents production bugs like `guestShareMint` silently falling through to `_demo` tokens due to DTO field mismatches.

**Example:**
```typescript
// crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts
import { BackendUnavailableError, backendAvailable } from '../lib/backendGuard';
import { voxTransport } from '../transport';

export async function guestShareMint(args: {
  sessionId: string;
  permission: GuestPermission;
  ttlMs?: number;
}): Promise<GuestShareToken> {
  if (!backendAvailable()) {
    throw new BackendUnavailableError('vox_guest_share_mint');
  }
  const raw = await voxTransport.invokeMcpTool< string | Record<string, unknown> >(
    'vox_guest_share_mint',
    {
      thread_id: args.sessionId,
      permission: args.permission,
      ttl_ms: args.ttlMs ?? 3_600_000,
    },
  );
  return parseStrictGuestShareToken(raw); // Throws typed AxisRpcParseError on malformed payload — NEVER fakes _demo
}
```

### Pattern 3: Dual-Theme `data-slot` shadcn/Radix Layering Over Style Dictionary & `RomanSurfaceHeader`

**What:** Layer React 19 Radix primitives (`data-slot="<primitive>"`, `data-variant="<variant>"`) directly atop Style Dictionary's 4 generated CSS token sheets (`tokens.generated.css` for `arcane`, `tokens.travertine.generated.css` for `glacier`/`travertine`, `tokens.slate.generated.css` for `slate`/`void`, and `tokens.contrast.generated.css` for `high-contrast`), while scoping legacy catch-all selectors in [`index.css:311-325`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css#L311-L325) and [`index.css:1076-1082`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css#L1076-L1082) with `:not([data-slot])`.
**When to use:** All interactive controls, form fields, tabs, switches, selects, menus, sheets, and cards under `crates/vox-gui/ui/src/components/ui/`.
**Trade-offs:** Preserves `<RomanSurfaceHeader>` architectural friezes (`flutedPillarCount >= 2` and `Cinzel` display typography in Roman light themes; automatically suppressed via [`index.css:922`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css#L922) in `slate`/`void`/`high-contrast` with `Inter` display typography) while unlocking clean `primary` / `danger` / `outline` / `ghost` / `[data-state="checked"]` states without `!important` specificity wars.

**Example:**
```css
/* crates/vox-gui/ui/src/index.css — Bridge Style Dictionary tokens to Tailwind v4 & shadcn */
@theme inline {
  --color-background: var(--color-bg-base);
  --color-foreground: var(--color-text-primary);
  --color-card: var(--color-bg-surface);
  --color-card-foreground: var(--color-text-primary);
  --color-popover: var(--color-bg-elevated);
  --color-popover-foreground: var(--color-text-primary);
  --color-primary: var(--color-accent-default);
  --color-primary-foreground: var(--color-bg-base);
  --color-muted: var(--color-overlay-subtle);
  --color-muted-foreground: var(--color-text-muted);
  --color-destructive: var(--color-status-fail);
  --color-border: var(--color-border-subtle);
  --color-ring: var(--color-accent-default);

  /* Standardized @container breakpoints for Bento & Split Workbenches */
  --container-tile-sm: 20rem;   /* 320px — Dock pane / mini tile */
  --container-tile-md: 32rem;   /* 512px — 2-col sub-grid threshold */
  --container-bento: 48rem;     /* 768px — 3-col Bento threshold */
  --container-split: 60rem;     /* 960px — Master-detail split threshold */
}

/* Legacy catch-all scoped ONLY to unmigrated buttons so [data-slot] variants win cleanly */
html:not([data-theme="slate"]):not([data-theme="void"]):not([data-theme="high-contrast"])
  :is(main, [role="dialog"])
  button:not([data-slot]):not([role="tab"]):not([data-roman-gauge="true"]):not([disabled]) {
  /* existing riveted brass fallback for unmigrated raw <button> only */
}
```

### Pattern 4: `@container` Inline-Size Bento & Split-Panel Composition

**What:** Establish `container-type: inline-size` (`@container/surface` on [`SurfaceScrollHost.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/layout/SurfaceScrollHost.tsx) and `@container/tile` on `<BentoTile>`) so every surface adapts to its **actual allocated panel width** rather than `window.innerWidth`.
**When to use:** All 33 `SURFACE_REGISTRY` views, 14 Chat companion docks, and `DashboardGrid` mini-renders.
**Trade-offs:** Must use `container-type: inline-size` (`@container`), **never** `container-type: size`, on vertically scrolling children so intrinsic vertical height does not collapse to `0px`, while keeping [`DockWorkspaceShell.tsx:134`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.tsx#L134)'s explicit pixel-height wrapper intact for Dockview.

**Example:**
```tsx
// crates/vox-gui/ui/src/components/layout/BentoGrid.tsx
export function BentoGrid({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div
      data-slot="bento-grid"
      className={cn(
        'grid grid-cols-1 gap-3 @tile-md/surface:grid-cols-2 @bento/surface:grid-cols-12 auto-rows-[minmax(8.5rem,auto)]',
        className,
      )}
    >
      {children}
    </div>
  );
}

export function BentoTile({
  span = 'default',
  children,
  className,
}: {
  span?: 'default' | 'wide' | 'hero' | 'rail';
  children: React.ReactNode;
  className?: string;
}) {
  const spanClass = {
    default: '@bento/surface:col-span-4',
    wide: '@bento/surface:col-span-8',
    hero: '@tile-md/surface:col-span-2 @bento/surface:col-span-12',
    rail: '@bento/surface:col-span-4 @bento/surface:row-span-2',
  }[span];
  return (
    <section
      data-slot="bento-tile"
      className={cn('@container/tile min-w-0 rounded-lg border border-border-subtle bg-bg-surface p-4', spanClass, className)}
    >
      {children}
    </section>
  );
}
```

### Pattern 5: Concept-Art (`image-generator`) + Dual-Theme CV Flywheel Convergence

**What:** Pair `image-generator` concept-art plates (`public/assets/romanesque/v74-concept-*.jpg`, registered in [`vlmVisionFlywheel.ts:78-102`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts#L78-L102)) with multi-pass Playwright + OpenCV/Bhattacharyya histogram + VLM rubric runs across **both** `travertine` (Roman White/Gold) and `slate` (Minimalist Grey-Blue) themes at compact (`900×600`), desktop (`1440×900`), and full (`1920×1080`) viewports.
**When to use:** Phase 6 verification and per-surface Bento sign-off (`CV_STAGE=before` baseline → UI/layout pass → `CV_STAGE=after` delta).

---

## Data Flow

### Request Flow (Live Mutating Control Path)

```
[User Clicks Switch / Save / Approve / Mint in Surface]
    │
    ▼
[shadcn/Radix Primitive (data-slot)] ──(optimistic pending / aria-busy)──┐
    │                                                                    │
    ▼                                                                    │
[voxTransport.<method> / t3Wave3Rpc.<fn>]                                │
    ├── if !backendAvailable() ──► throw BackendUnavailableError ────────┼──► [Toast / Inline Disabled State]
    │                                                                    │
    ▼ (Tauri IPC invoke)                                                 │
[axis_invoke_handler! -> remote::gate::gate_invoke (remote/gate.rs)]     │
    ├── class == "host"   ──► commands::{secrets,llm_settings,mercatus}  │
    │                           ├─► Clavis (vox_secrets) / VoxConfig     │
    │                           └─► app.emit("vox://*-changed") ─────────┼──► [safeListen Re-hydrates Store]
    └── class == "daemon" ──► BackendCalls (remote/calls.rs)             │
                                ├─► Local: PersistentDaemon TCP RPC      │
                                └─► Remote: GatewayClient (/v1/ws)       │
                                      │                                  │
                                      ▼                                  │
                        [vox-orchestrator-d / vox-orchestrator-mcp]      │
                          ├─► VoxDb (Turso) / VCS Worktree / GuestSvc    │
                          └─► vox_actor_runtime::llm (for consensus/chat)│
                                      │                                  │
                                      ▼ (Strict typed JSON DTO)          │
                        [UI Updates Canonical State & Clears Busy] ◄─────┘
```

### State Management

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ 1. Canonical Server/Daemon State (VoxDb via Orchestrator / MCP)              │
│    • Chat transcripts, checkpoints, tasks, runs, approvals, telemetry, codex │
│    • Queried/mutated via voxTransport -> BackendCalls; never cached as fake  │
├──────────────────────────────────────────────────────────────────────────────┤
│ 2. Host Credential & Runtime Config State (Clavis + VoxConfig)               │
│    • Secrets stored/resolved ONLY via vox_secrets (redacted boolean/masked   │
│      status returned to UI via commands::secrets::SecretStatusDto)           │
│    • TOML settings via commands::llm_settings + commands::mercatus           │
├──────────────────────────────────────────────────────────────────────────────┤
│ 3. Two-Tier UI Shell & Layout Preferences (L1 Cache + L2 VoxDb/LocalPrefs)   │
│    • Theme, active surface, sidebar collapse, DashboardGrid layout, and      │
│      DockWorkspaceShell v3 layout:                                           │
│      Read L1 (localStorage) synchronously for 0-CLS first paint -> reconcile │
│      async with voxTransport.getGuiPreference / setGuiPreference             │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Key Data Flows

1. **Wave 3 Session Interop & Guest Share (`SessionInteropModal.tsx` → `t3_wave3_tools.rs`):**
   - *Mint:* `guestShareMint` sends `{ thread_id, permission, ttl_ms }` → `handle_guest_share_mint` returns `{ token, invite_id, thread_id, permission, expires_at_ms, join_url }` (aligned DTO so UI never falls through).
   - *Pending Moderation:* New `vox_guest_share_list` reads real pending messages from `state.guest_share_service` (eliminating hardcoded `'gsub_demo_1'` at [`SessionInteropModal.tsx:108`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Chat/SessionInteropModal.tsx#L108) and removing synthetic message fabrication at [`t3_wave3_tools.rs:673-689`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/crates/../crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs#L673-L689)).
   - *External Import:* Uses `@tauri-apps/plugin-dialog` file picker (via `voxTransport.pickSessionJsonlFile`) or `vox_session_list_external` catalog selection (`file_path`) instead of pre-populated synthetic JSONL (`{"role":"user","content":"hello"}` at [`SessionInteropModal.tsx:31`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Chat/SessionInteropModal.tsx#L31)).
2. **Mercatus Watchlist & Source Controls (`Mercatus.tsx` → `commands::mercatus`):**
   - Toggling a feed `Switch`, editing cadence/cost `Input`, or adding/removing a watchlist item via Radix `Dialog` mutates a typed draft `PriceWatchConfig` and calls `voxTransport.mercatusSaveConfig(nextConfig)` → [`commands/mercatus.rs:62`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/mercatus.rs#L62) calls `fs::create_dir_all(parent)` + atomic write + emits `vox://mercatus-config-changed` → triggers `mercatusRefreshSnapshots`.
3. **Layout Persistence Sync (`DashboardGrid.tsx` & `DockWorkspaceShell.tsx` → `commands::preferences`):**
   - Layout drag/resize writes immediately to `localStorage` for 60fps responsiveness and debounces (`300ms`) `voxTransport.setGuiPreference('gui.dashboard.layout.v1' | 'gui.dock.<surface>.v3', JSON.stringify(layout))`. In local mode, [`commands/preferences.rs:42-50`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/preferences.rs#L42-L50) persists to `VoxDb` via `vox_gui_preference_set`; in remote mode, it persists to host-scoped `AxisLocalPrefs` (`~/.vox/axis-remote-prefs.json`).

---

## Scaling Considerations

| Scale / Dimension | Architecture Adjustments |
|-------------------|--------------------------|
| **Narrow Dock Panes (`280px`–`480px`)** | `@container/surface` and `@container/tile` collapse Bento grids to 1 column (`grid-cols-1`), hide secondary table columns behind Radix `Popover`/`Sheet` drawers, and wrap header toolbars cleanly (`outerOverflowPx == 0`). |
| **Compact Laptop (`900×600`) to 4K (`1920×1080+`)** | `SplitWorkbench` switches from tabbed/sheet detail (`< 960px` container) to side-by-side master-detail (`@split/surface:grid-cols-[minmax(0,1fr)_24rem]`), maintaining `chartPanelFillRatio >= 0.84`. |
| **Long-Running Multi-Model Consensus RPCs** | Extend [`remote/calls.rs:37-43`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/calls.rs#L37-L43) `deadline_for_tool` so `vox_consensus_run` (fan-out across `vox_actor_runtime::llm`) receives `CHAT_MESSAGE_CLIENT_DEADLINE` alongside `vox_chat_message` instead of timing out at `ORCH_CLIENT_READ_DEADLINE`. |
| **Rapid Dock / Window Resize Storms** | Keep [`DockWorkspaceShell.tsx:120-128`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.tsx#L120-L128)'s single-frame `requestAnimationFrame` coalescing around `ResizeObserver` and debounce `setGuiPreference` IPC writes (`300ms`) so resize gestures trigger zero IPC congestion. |

### Scaling Priorities

1. **First bottleneck — Turso/SQLite file-lock contention:** Preserved by keeping `vox-gui` strictly `0` `db`-class command modules (`VoxDb` accessed solely by `vox-orchestrator-d`).
2. **Second bottleneck — Portal z-index & scroll-host clipping:** Solved by migrating inline `absolute z-[99999]` popovers ([`Popover.tsx:14`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/ui/Popover.tsx#L14)) and native `<select>` elements to Radix `<Portal>` primitives anchored outside `overflow-y: auto` scroll containers.

---

## Anti-Patterns

### Anti-Pattern 1: Silent Catch-and-Fake Fallbacks in RPC Clients

**What people do:** Wrap `voxTransport.invokeMcpTool(...)` in `try { ... } catch { /* fall through */ }` and return hardcoded demo objects (`vox_share_*_demo`, `a1b2c3d4...` Git OIDs, `inMemoryQuarantinedPrefixes`) as currently done throughout [`t3Wave3Rpc.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts).
**Why it's wrong:** Masks real Rust/TS wire-schema bugs even when the desktop daemon is healthy (e.g., `guestShareMint` checking `data?.token` when Rust returned `invite_id`, silently serving fake share links in production) and misleads users into thinking destructive VCS/quarantine mutations succeeded.
**Do this instead:** Check `if (!backendAvailable()) throw new BackendUnavailableError(cmd);`, validate wire DTOs strictly, and render explicit offline/error banners + disabled mutation buttons.

### Anti-Pattern 2: Direct `invoke('invoke_mcp_tool')` in React Surfaces

**What people do:** Import `invoke` directly from `@tauri-apps/api/core` inside views ([`SkillsPluginsView.tsx:48`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/SkillsPlugins/SkillsPluginsView.tsx#L48), [`Loquela.tsx:1822`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx#L1822), [`SettingsView.tsx:203`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx#L203)).
**Why it's wrong:** Drops `permissionMode: currentPermissionMode` (bypassing user `ReadOnlyPlan` / `AskForMutations` governance in [`commands/mcp.rs:148-159`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/mcp.rs#L148-L159)) and throws raw `TypeError: Cannot read properties of undefined (reading 'invoke')` in Vite browser runs.
**Do this instead:** Route 100% of calls through `voxTransport` and lock `ALLOW_DIRECT_INVOKE` and `ALLOW_DIRECT_EVENT_LISTEN` in [`ipcBoundaries.test.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts) to empty sets (`new Set()`).

### Anti-Pattern 3: Unscoped Global `button { ... !important }` Theme Overrides

**What people do:** Force theme colors via `:is(main, [role="dialog"]) button:not([role="tab"]) { background-color: ... !important; color: ... !important; }` ([`index.css:311-325`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css#L311-L325), [`1076-1082`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css#L1076-L1082)).
**Why it's wrong:** Flattens `Button` `danger`/`primary`/`ghost` variants into identical appearance, breaks Radix `Switch` `[data-state="checked"]` track colors, and turns icon-only popover/select triggers into heavy bordered buttons.
**Do this instead:** Scope legacy fallback selectors with `:not([data-slot])` and author explicit dual-theme token styles per `[data-slot="button"]`, `[data-slot="switch"]`, `[data-slot="select-trigger"]`, and `[data-slot="tabs-trigger"]`.

### Anti-Pattern 4: Viewport Media Queries (`sm:`, `md:`, `lg:`) Inside Dockable Surfaces

**What people do:** Use `sm:grid-cols-3` or `lg:grid-cols-4` inside surface bodies (e.g., [`Mercatus.tsx:76`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Mercatus/Mercatus.tsx#L76)).
**Why it's wrong:** When a surface is mounted inside a `340px` `DockWorkspaceShell` companion pane on a `1920×1080` monitor, `lg:` still evaluates `true`, crushing 4 columns into `85px` each (`overlappingControlPairs > 0`, `scrollHostOverflowPx > 0`).
**Do this instead:** Use `@container` queries (`@tile-md/surface:grid-cols-2 @bento/surface:grid-cols-4`) driven by `SurfaceScrollHost` and `BentoTile`.

### Anti-Pattern 5: Violating Rust Crate Layering (`layers.toml`), Clavis, or LLM Runtime Boundaries

**What people do:** Read `std::env::var("OPENAI_API_KEY")`, add `reqwest` vendor LLM calls inside `vox-gui`, or open `VoxDb` directly in a GUI command.
**Why it's wrong:** Violates `AGENTS.md` kill-safety triad, `vox ci crate-edges` ([`layers.toml`](file:///Users/brbrainerd/dev/vox/docs/src/architecture/layers.toml)), and causes Turso DB lock errors.
**Do this instead:** Resolve/store all credentials via `vox_secrets` (Clavis, L1) returning redacted `SecretStatusDto` ([`commands/secrets.rs:14-27`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/secrets.rs#L14-L27)), route all LLM calls through `vox_actor_runtime::llm` (L3) via the daemon, and keep `vox-gui` (L5) strictly on `BackendCalls`.

---

## Integration Points

### External Services & Local Subsystems

| Boundary | Integration Pattern | Notes |
|----------|---------------------|-------|
| **Clavis Secret Vault (`vox-secrets`, L1)** | `commands::secrets::{list_secret_statuses, store_secret_value, clear_secret_value}` | **Never** send plaintext secret values from Rust to React; UI receives `is_present` + `redacted_preview` only. |
| **Orchestrator Daemon (`vox-orchestrator-d`, L3)** | `BackendCalls::call_tool` / `api_v2_get` over local TCP JSON-RPC or remote `GatewayClient` WebSocket/HTTPS | Every `daemon`-class command module must stay registered in `contracts/gui/command-backends.v1.yaml` and `remote/manifest.rs`. |
| **LLM Inference (`vox_actor_runtime::llm`, L3)** | Invoked exclusively inside daemon/MCP handlers (`vox_chat_message`, `vox_consensus_run`) | Zero hardcoded `http://127.0.0.1:*` or `ws://192.168.*` origins in UI bundles; discover endpoints via `axis_backend_status` and `VoxConfig`. |
| **CV Vision & Concept Art (`image-generator` + Playwright)** | Reference plates in `ui/public/assets/romanesque/v74-concept-*.jpg` scored by `e2e/review/cv-flywheel.spec.ts` | Must satisfy `romanFontCoverageRatio >= 0.85` & `flutedPillarCount >= 2` in `travertine` AND clean `Inter` high-contrast clarity in `slate`. |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| **`components/surfaces/*` ↔ `transport.ts`** | Typed async methods + `safeListen` event unlisteners | Guarded by `ipcBoundaries.test.ts` + `transportIpcGuard.test.ts`. |
| **`transport/t3Wave3Rpc.ts` ↔ `t3_wave3_tools.rs`** | JSON-serialized MCP `call_tool` payloads via `commands::mcp::invoke_mcp_tool` | Enforce 1:1 Rust↔TS schema parity tests so field renames (`thread_id` vs. `session_id`, `token` vs. `invite_id`) never silently fail. |
| **`components/ui/*` ↔ `index.css` + `tokens/*.json`** | `data-slot` + `data-state` + Tailwind v4 `@theme inline` CSS variables | Portal content (`Dialog`, `Select`, `Popover`, `DropdownMenu`, `Tooltip`) inherits from `html[data-theme="..."]` root variables. |

---

## Dependency-Ordered Phase Build Sequence (Milestone v7.4)

Build order is strictly sequenced so foundation primitives and IPC honesty contracts land **before** surface rewrites and CV Flywheel visual scoring:

```
Phase 1: Dual-Theme Tokens, shadcn/Radix UI Layer & Zero-Emoji SVG Icons
   │
   ▼
Phase 2: Rust/MCP Seam Closure & Wire DTO Parity (t3_wave3_tools + mercatus + calls.rs)
   │
   ▼
Phase 3: voxTransport Consolidation & Honest Offline Contract (0 Bypass, 0 Paper Code)
   │
   ▼
Phase 4: End-to-End Surface Control Wiring & Two-Tier Layout Persistence
   │
   ▼
Phase 5: @container Bento & SplitWorkbench Layout Rollout (33 Surfaces + 14 Docks)
   │
   ▼
Phase 6: image-generator Concept-Art Plates & Dual-Theme CV Flywheel Convergence
```

| Phase | Scope | Why This Order / Gate Unlocked |
|-------|-------|--------------------------------|
| **1. Dual-Theme shadcn/Radix & Icon Foundation** | Add Radix packages + `cva`; map `@theme inline` aliases and scope `:not([data-slot])` in `index.css`; build `Select`, `Tabs`, `Switch`, `Popover` (Portal), `DropdownMenu`, `Input`, `Textarea`, `Badge`, `Card`, `Sheet`, `Separator`, upgraded `Button`/`Dialog`/`Panel`; replace remaining Unicode symbols (`⋮⋮`, `●`/`○`) in `Icons.tsx`. | Unblocks every surface control refactor in Phases 4–5 without `!important` regressions, portal clipping, or raw emoji/glyph audit failures (`rawEmojiCount == 0`). |
| **2. Rust/MCP Seam Closure & DTO Parity** | Add `vox_guest_share_list` and align `guest_share_mint`, `session_list_external`, and `muse_pool_adopt` DTOs in `t3_wave3_tools.rs` (removing synthetic moderation fabrication); add `fs::create_dir_all` + change event to `commands/mercatus.rs`; add `vox_consensus_run` extended deadline in `remote/calls.rs`. | Ensures backend endpoints return exact, truthful payloads before stripping frontend fallbacks in Phase 3. Verified with `vox ci crate-edges` + `vox ci gui-catalog-parity`. |
| **3. `voxTransport` Consolidation & Honest Offline Contract** | Migrate all 38 `ALLOW_DIRECT_INVOKE` files and 8 direct `listen` files into `voxTransport` (fixing dropped `permissionMode` on `invoke_mcp_tool`); delete all fake `_demo`/Git OID/diff fallbacks and hardcoded `ws://127.0.0.1` URLs in `t3Wave3Rpc.ts`; lock `ipcBoundaries.test.ts` to `0` exemptions and expand `honestyScan.ts` + `noPaperFallbackGuard.test.ts`. | Establishes 100% honest IPC plumbing across the entire app (`BackendUnavailableError` offline; real errors surfaced online) before wiring new UI controls. |
| **4. End-to-End Live Surface Control Wiring** | Wire `SessionInteropModal` (live guest list, moderation, native file-picker import), `Mercatus` (`Switch`/`Input`/`Dialog` → `mercatusSaveConfig`), `ChatExecutionRail` (`axisBackendStatus` routes), `DashboardGrid` & `DockWorkspaceShell` (`getGuiPreference`/`setGuiPreference` sync), and live Settings/Repository/Skills/Approvals controls using Phase 1 primitives. | Eliminates dead controls and localStorage-only layout silos across all primary workbenches. |
| **5. `@container` Bento & SplitWorkbench Rollout** | Introduce `BentoGrid`/`BentoTile` and `SplitWorkbench` over `SurfaceScrollHost` (`@container/surface`); replace viewport `sm:`/`md:`/`lg:` grids across all 33 `SURFACE_REGISTRY` views and 14 Chat companion docks (`900×600`..`1920×1080`). | Eliminates horizontal overflow (`outerOverflowPx == 0`, `scrollHostOverflowPx == 0`), control overlap, and sparse whitespace (`chartPanelFillRatio >= 0.84`) in both full windows and narrow docks. |
| **6. Concept-Art (`image-generator`) & Dual-Theme CV Flywheel** | Generate dual-theme (`travertine` Roman White/Gold + `slate` Minimalist Grey-Blue) Bento concept-art plates via `image-generator`; register in `vlmVisionFlywheel.ts`; execute `CV_STAGE=before` → polish → `CV_STAGE=after` loops in `cv-flywheel.spec.ts` until `axeViolations == 0`, `clippedInteractiveCount == 0`, and `visualAfterScore >= 0.90`. | Final objective visual, accessibility, and architectural sign-off across themes and resolutions. |

---

## Sources

- Live Vox codebase inspection:
  - [`crates/vox-gui/src/main.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/main.rs#L1-L483), [`remote/gate.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/gate.rs#L1-L146), [`remote/manifest.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/manifest.rs#L1-L390), [`remote/calls.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/remote/calls.rs#L1-L140), [`contracts/gui/command-backends.v1.yaml`](file:///Users/brbrainerd/dev/vox/contracts/gui/command-backends.v1.yaml#L1-L80)
  - [`crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs#L580-L915)
  - [`crates/vox-gui/ui/src/transport.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport.ts#L1-L700), [`transport/t3Wave3Rpc.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts#L1-L1109), [`guards/ipcBoundaries.test.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts#L1-L105)
  - [`crates/vox-gui/ui/src/index.css`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css#L1-L1184), [`tokens/semantic.json`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/tokens/semantic.json), [`tokens/semantic.slate.json`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/tokens/semantic.slate.json)
  - [`crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts#L1-L260), [`cv-flywheel.spec.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts#L700-L760)
  - [`docs/src/architecture/layers.toml`](file:///Users/brbrainerd/dev/vox/docs/src/architecture/layers.toml#L1-L628) & [`AGENTS.md`](file:///Users/brbrainerd/dev/vox/AGENTS.md#L1-L250)
- Ecosystem references verified (2026):
  - shadcn/ui Tailwind v4 (`@theme inline`) + React 19 (`data-slot`, ref-as-prop) architecture documentation (`ui.shadcn.com/docs/tailwind-v4`)
  - Tailwind CSS v4 native `@container` queries, named containers (`@container/surface`), and `--container-*` theme variables (`tailwindcss.com/docs/responsive-design#container-queries`)
  - Tauri v2 IPC command handler & event system (`v2.tauri.app/develop/calling-rust/`)

---
*Architecture research for: Vox Axis Milestone v7.4 (`axis-live-functional-v74`)*
*Researched: 2026-11-13*
