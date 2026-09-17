---
title: "Agent browser driver — Design Spec (snapshot, profiles, attach)"
description: "Normative types, launch modes, MCP tools, and safety rules for Stagehand-class semantic driving on the existing chromiumoxide stack."
category: "Architecture SSOTs"
status: "roadmap"
training_eligible: true
training_rationale: "Contract for implementing snapshot+ref, named profiles, and Chrome attach without a second browser engine."
---

# Agent browser driver — Design Spec

**Date:** 2026-09-07
**Research:** [`docs/src/architecture/agent-browser-driver-research-2026.md`](../../src/architecture/agent-browser-driver-research-2026.md)
**Plan:** [`docs/superpowers/plans/2026-09-07-agent-browser-driver.md`](../plans/2026-09-07-agent-browser-driver.md)
**Supersedes (this program only):** the 2026-08-31 GUI-axis line “never `vox_browser_snapshot`” and the deferred `vox_browser_snapshot` / `vox_browser_click_ref` bullets in [`vox-gui-browser-support-2026.md`](../../src/architecture/vox-gui-browser-support-2026.md). Those were “not in that plan,” not a product ban.

This spec is the contract. Executors do not invent types, env vars, crate edges, or tool names that are not named here.

## 0. Goal

A Vox operator can, from Loquela chat or the Browser surface: (1) get a compact accessibility snapshot with stable refs, (2) click/fill those refs without CSS guessing, (3) choose whether the session is ephemeral, a named saved profile, or attached to their real Chrome, and (4) see the live page and take human control — without adding Playwright, Stagehand, Node, or a second Chromium to the product path.

Chat is MCP (`heavy-browser` on shipped `vox-cli` / `vox-gui` / `vox-orchestrator-d`). Vox `Browser.*` builtins stay CSS-only in v1 (accepted gap).

## 1. Non-goals

- Binding `playwright-rust`, vendoring Stagehand, or shipping `npx @playwright/mcp` / `chrome-devtools-mcp` as the product driver.
- Driving arbitrary sites inside the Tauri wry/WebView2 host. Preview iframe stays localhost-only (enforce on `preview_start`; today this is convention, not code).
- OS-level computer use (screen control).
- Stealth / Cloudflare bypass as a advertised feature.
- Persistent cookies written to Turso / Tier A.
- A Vox-branded Chromium fork (Atlas lesson).
- New workspace crate. New modules land in crates listed in §4.
- New crate-to-crate dependency edges. `vox-plugin-browser` already depends on `vox-config`. Do not add `url`, `regex`, or `vox-crypto` to that crate (`url` is transitive via chromiumoxide; that does not allow `use url::`).
- Changing Playwright’s role: it remains CI / E2E / preview validation.
- Merging cross-origin iframe AX trees, SPA `MutationObserver` invalidation, NeedsYou feedback rows, or Vox `Browser.snapshot` builtins.

## 2. Already shipped (do not rebuild)

| Surface | Reality |
| --- | --- |
| CDP engine | `crates/vox-plugin-browser` chromiumoxide 0.9.1; `GetFullAxTree` via `ax_tree` (returns a **JSON array** of nodes) |
| Trait | `BrowserAutomation` revision **4** in `crates/vox-plugin-api/src/extensions/browser_automation.rs` |
| MCP | 26 `vox_browser_*` tools, `heavy-browser` gated, `browser_act` = visible-text → `llm_bridge::call_llm` → CSS/XPath |
| Control lock | In-memory `human` / `agent` in `browser_tools.rs`; identity from `VOX_MCP_CALLER_ROLE` via `trusted_caller_role()`, **not** request `actor` |
| GUI | Preview iframe + `vox://browser-frame` + `browser_open_session` → `vox_browser_open` (MCP only) |
| Launch | Single `HostInner`; `BrowserConfig` never sets `user_data_dir` (chromiumoxide still writes `$TMP/chromiumoxide-runner`); `VOX_CHROME_EXECUTABLE`, `VOX_BROWSER_NO_SANDBOX` |

## 3. Shared types

Copy these names. Later tasks must not rename them.

### 3.1 Snapshot

```rust
/// Compact AX snapshot for LLM / GUI. Refs are valid until the next
/// `snapshot` on the same `page_id`, until navigation (`goto` / `back` /
/// `forward` / `reload` / `close`), or until `DOM.getBoxModel` fails
/// for that backend node (SPA mutation without navigation).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AxSnapshot {
    pub page_id: String,
    pub url: String,
    pub title: String,
    /// YAML-ish tree, Playwright MCP shape. Interactive nodes carry `[ref=eN]`.
    pub tree: String,
    pub refs: std::collections::BTreeMap<String, AxRef>,
    pub truncated: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AxRef {
    pub ref_id: String,          // "e1"
    pub role: String,            // "button" | "textbox" | ...
    pub name: String,
    pub backend_dom_node_id: i64,
    pub sensitive: bool,         // password / payment-like
    pub box_css: Option<AxBox>,  // viewport CSS px; None if not resolvable
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AxBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    pub interactive_only: bool,
    pub max_depth: u32,
    pub max_nodes: u32,
    pub include_boxes: bool,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            interactive_only: true,
            max_depth: 12,
            max_nodes: 80,
            include_boxes: false,
        }
    }
}
```

Do **not** `#[derive(Default)]` on `SnapshotOptions`. Rust `Default` for `bool`/`u32` is `false`/`0` and would violate this contract.

`compact_ax_snapshot(nodes: &[serde_json::Value], opts: &SnapshotOptions) -> CompactAx` is a **pure** function. It does not talk to Chrome. `CompactAx { tree, refs, truncated }` is the compact core; `engine.snapshot()` wraps it into `AxSnapshot` with `page_id` / `url` / `title`. `tree` must stay in the 200–800 token band for typical docs pages when `interactive_only` is true.

**Live CDP node shape** (chromiumoxide 0.9.1 `serde_json::to_value`): camelCase keys; `role` / `name` are `AxValue` objects `{ "type": "role"|"computedString", "value": "…" }`. Parsers must read `role.value` then `chromeRole.value`, tolerate missing `type`, and treat `role` as a bare string if tests send that. Unit fixtures must include at least one realistic node with `type`.

**v1 snapshot is the root frame only.** Do not merge child-frame `GetFullAxTree(frameId=…)`. Cross-origin iframe content is absent.

**Interactive roles (eligible for a ref only when `backendDOMNodeId` is present):**
`button`, `link`, `textbox`, `searchbox`, `checkbox`, `radio`, `combobox`, `listbox`, `listitem`, `menuitem`, `tab`, `switch`, `slider`, `spinbutton`. Compare case-insensitively (`button` vs `RootWebArea`). Nodes without `backendDOMNodeId` may appear as text when `interactive_only` is false; they never get `[ref=eN]`.

**Sensitive:** `true` when role is `textbox`/`searchbox` and the accessible name matches (case-insensitive, word-boundary) `password`, `passwd`, `\bpin\b`, `cvv`, `cvc`, `card number`, or `credit card`, **or** AX `properties` include an autocomplete value containing `password`. Do not treat “username” as sensitive. Do not treat “PIN your location” as sensitive.

**Ref assignment:** sequential `e1`, `e2`, … in tree order. Refs are **not** stable across snapshots.

**Click/fill path:** chromiumoxide 0.9 has no public `Element::from_backend_node_id`. Resolve via `DOM.getBoxModel` / `DOM.getContentQuads` with JSON key `backendNodeId` (same `BackendNodeId` type; different wire name from AX `backendDOMNodeId`), then existing `click_xy` at the box center. `fill_ref` = that click then `type_text`. If box model fails, return `stale_ref`. Do not use a JS `TreeWalker`.

**Prompt wrapping:** `engine.snapshot()` calls `wrap_snapshot_tree` **once**. MCP must not wrap again.

```text
BEGIN_PAGE_SNAPSHOT nonce=<16 hex>
<tree>
END_PAGE_SNAPSHOT nonce=<16 hex>
```

The nonce is an injection delimiter, **not** a MAC. The model must be told that text inside the wrapper is untrusted page content.

### 3.2 Launch and host identity

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserLaunchMode {
    Ephemeral,
    Named,
    Attach,
}

/// Process-local Chrome identity. One Chromium per key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostKey {
    Ephemeral,
    Named(String),
    Attach(String), // normalized loopback cdp_url
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserLaunchOptions {
    pub url: String,
    #[serde(default = "default_true")]
    pub headless: bool,
    #[serde(default)]
    pub mode: BrowserLaunchMode, // default Ephemeral
    /// Required when mode == Named. Kebab `[a-z0-9][a-z0-9-]{0,62}`.
    pub profile_id: Option<String>,
    /// Required when mode == Attach. Loopback HTTP or ws debugger URL.
    pub cdp_url: Option<String>,
}

fn default_true() -> bool { true }
```

`open(url, headless)` stays **ephemeral** (revision-4 compatible). New work goes through `open_ex(options_json)`.

**Host map (required):** replace the singleton `Mutex<Option<HostInner>>` with `HashMap<HostKey, HostInner>`. Each `page_id` records its `HostKey`. `close` of the last page on a key drops **that** host only. The same `Named(profile_id)` cannot launch twice (Chromium locks `user_data_dir`) — return `host_mode_conflict`. Attach `drop` disconnects; it must **not** kill the user’s Chrome.

**Named profile path:** `vox_config::paths::browser_profiles_dir()` / `{profile_id}/`. This is **user data** under `VOX_DATA_DIR` (or `VOX_BROWSER_PROFILES_DIR`), **not** Tier D cache. Reject `profile_id` that fail kebab validation, contain `/` `\` `.` `..`, or match Windows reserved names (`con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`) case-insensitively. Implement with char checks — no `regex` crate.

**Attach:** `chromiumoxide::Browser::connect` (0.9.1 accepts `http://127.0.0.1:9222` and fetches `/json/version`). `cdp_url` host must be loopback (`127.0.0.1`, `localhost`, `[::1]`). If connect fails, remediation names `chrome --remote-debugging-port=9222` and Chrome 144+ `chrome://inspect/#remote-debugging`.

### 3.3 Cookie consent

Named mode **does not** start until the caller sets `save_profile: true` on `open_ex` **or** `{browser_profiles_dir()}/consents.json` already has `ProfileConsent { save_cookies: true }` for that `profile_id`.

```rust
pub struct ProfileConsent {
    pub profile_id: String,
    pub save_cookies: bool, // user said yes
    pub created_unix_s: u64,
}
```

**SSOT is `consents.json` only.** The GUI checkbox is per-request intent (`save_profile`); do not store a second copy in `localStorage` or `BrowserState`.

Cookie export files **are** secrets-adjacent: write them only under the profile dir, never log contents, never put values in MCP JSON. Use `std::fs` (not `vox-spool`, checksum-manifest, or `vox-secrets`). Add `crates/vox-plugin-browser/**` to `direct_fs_write_allowlist` in `contracts/db/data-storage-policy.v1.yaml`, or write only through already-allowlisted `vox-config` helpers.

Primary persist is Chromium `user_data_dir`. Portable sidecar uses **Storage** APIs: `Browser::get_cookies` / `Browser::set_cookies` (`Storage.getCookies` / `Storage.setCookies`). chromiumoxide 0.9.1 does **not** generate `Network.getAllCookies`.

Attach-mode `cookies_export` requires the same explicit consent (checkbox / `save_profile` or stored `ProfileConsent`) — exporting the user’s real Chrome cookies is credential export.

**Import path jail:** canonicalize both `browser_profiles_dir()` and the import path, then `strip_prefix`. Do **not** use raw `Path::starts_with` (`/safe/profiles` must not accept `/safe/profiles-evil/...`). Adapt the pattern in `vox_repository::resolve_local_path_under_repo_root`.

### 3.4 Host allowlist

When `VOX_BROWSER_ALLOWED_HOSTS` is unset or empty: allow all hosts for **ephemeral** `open` (current behavior). Empty allowlist with Named or Attach is a documented operator risk; GUI copy should say to set the var in production.

When set: comma-separated hostnames (no scheme). Always allowed regardless of the list: `localhost`, `127.0.0.1`, `[::1]`, `data:` URLs, and `about:blank`. `open` / `open_ex` / `goto` / `browser_act` `goto` reject other hosts with `host_not_allowed`.

Matching is exact hostname or a single leading `*.` suffix (`*.example.com` matches `app.example.com`, not `example.com`). Parse the host by hand (no `url` crate): strip scheme, take the host before `/` and `:port`.

### 3.5 Sensitive action result

If `click_ref` / `fill_ref` targets `sensitive: true` and `trusted_caller_role()` is **not** `human`:

```json
{ "ok": false, "needs_human": true, "reason": "password_field", "ref": "e3" }
```

Do **not** click or type. Request-body `actor` must not bypass this. When the trusted role **is** `human`, perform the action (`respect_sensitive: false`).

v1 surfaces this as MCP structured data plus a GUI toast / Browser `action_log`. There is **no** browser → NeedsYou feedback path today. Do **not** invent one. Do **not** return this object as a successful no-op that agents treat as “tool succeeded” without reading `data.ok`. Contract: `ToolResult` success bit may be true (so the JSON is parseable) **and** `data.ok` is `false`. Document that callers must check `data.ok` / `data.needs_human`.

## 4. File ownership

| File | Responsibility |
| --- | --- |
| Create: `crates/vox-plugin-browser/src/snapshot.rs` | Pure compact + types + `Default` + wrap |
| Create: `crates/vox-plugin-browser/src/policy.rs` | Allowlist, `parse_profile_id`, consent, path jail |
| Create: `crates/vox-plugin-browser/src/hosts.rs` | `HostKey`, launch/connect, `user_data_dir` |
| Create: `crates/vox-plugin-browser/src/ref_actions.rs` | RefMap, `snapshot`, `click_ref`, `fill_ref` |
| Split (Task 0): `engine.rs` → `host.rs` / `input.rs` / `resolve.rs` as needed | Stay under `arch/god_object` 500-line hard cap (~924 lines today, unsuppressed) |
| Modify: `crates/vox-plugin-browser/src/browser.rs` | Sync sabi wrappers |
| Modify: `crates/vox-plugin-browser/src/lib.rs` | `mod` lines |
| Modify: `crates/vox-plugin-api/src/extensions/browser_automation.rs` | Revision **5** |
| Modify: `crates/vox-plugin-api/tests/browser_automation_compile.rs` | `DummyBrowser` + `revision_constant_is_five` |
| Modify: `crates/vox-config/src/paths.rs` | `browser_profiles_dir()` |
| Modify: `crates/vox-orchestrator-mcp/src/params.rs` | New param structs (compile-safe without `heavy-browser`) |
| Modify: `crates/vox-orchestrator-mcp/src/browser_tools.rs` | New handlers; `browser_act` snapshot loop; `require_browser_revision(5)` |
| Modify: `crates/vox-orchestrator-mcp/src/dispatch.rs` | Match arms + `SKIP_DISPATCH_PROBE` |
| Modify: `crates/vox-orchestrator-mcp/src/input_schemas.rs` | Schema arms |
| Modify: `crates/vox-foundation/src/primitives/agentos_mutation.rs` | Mutating ref / open_ex / cookies_import |
| Modify: `contracts/operations/catalog.v1.yaml` | New operation rows (hand-edit SSOT) |
| Generate: `contracts/mcp/tool-registry.canonical.yaml` + `contracts/capability/capability-registry.yaml` | `vox ci operations-sync --target all --write` — **do not hand-edit** |
| Modify: `contracts/config/env-vars.v1.yaml` | New `VOX_*` |
| Modify: `contracts/db/data-storage-policy.v1.yaml` | `direct_fs_write_allowlist` if plugin writes files |
| Modify: `crates/vox-gui/src/commands/browser.rs` | `BrowserOpenInput` + `browser_snapshot` |
| Modify: `crates/vox-gui/src/commands/daemon.rs` | `VOX_MCP_CALLER_ROLE=human` on spawn |
| Modify: `crates/vox-gui/ui/src/components/surfaces/Browser/*` | Consent + overlay (split `BrowserView.tsx` — already 782 lines) |
| Modify: `docs/src/architecture/vox-gui-browser-support-2026.md` | Promote snapshot from deferred |
| Modify: `docs/src/architecture/where-things-live.md` | Snapshot / profiles / attach row |

**No new crate. No `exceptions` ledger.** MCP must not import `vox-plugin-browser` types — JSON strings only.

## 5. Trait revision 5

Add methods to `BrowserAutomation`. Existing `open` / `ax_tree` stay. New methods:

```rust
fn snapshot(&self, page_id: RStr<'_>, options_json: RStr<'_>) -> RResult<RString, RBoxError>;
fn click_ref(&self, page_id: RStr<'_>, ref_id: RStr<'_>) -> RResult<RString, RBoxError>;
fn fill_ref(&self, page_id: RStr<'_>, ref_id: RStr<'_>, value: RStr<'_>) -> RResult<RString, RBoxError>;
fn open_ex(&self, options_json: RStr<'_>) -> RResult<RString, RBoxError>;
fn cookies_export(&self, page_id: RStr<'_>) -> RResult<RString, RBoxError>;
fn cookies_import(&self, page_id: RStr<'_>, cookies_json: RStr<'_>) -> RResult<(), RBoxError>;
```

`BROWSER_AUTOMATION_REVISION = 5`.

Sabi `click_ref` / `fill_ref` take no `respect_sensitive`. The plugin wrapper hard-codes nothing about role. MCP sets engine `respect_sensitive = trusted_caller_role() != Human` (or passes `options_json` `{"respect_sensitive":true}` if the engine helper is not reachable from sabi — pick one; default omitted = `true`).

Plugin host checks **ABI version only**, never `revision()`. After this bump, run `vox ci plugin-surface-sync --write` (`contracts/plugin/extension-points.v1.yaml`). MCP must call `require_browser_revision(5)` before new methods and return a remediation to rebuild/reinstall the browser plugin. Bump `VOX_PLUGIN_ABI_VERSION` only if additive-trait policy requires it; document the choice in the Task 3 commit.

`click_ref` / `fill_ref` resolve `ref_id` from the last snapshot on that `page_id`. Missing/stale/unresolvable backend node → `stale_ref`.

## 6. MCP tools (new)

All `heavy-browser`. Mutating tools honor `ensure_control_lock`. New names are auto-gated by the `vox_browser_` prefix in `dispatchable_under_features`. Still add dispatch arms, `input_schemas`, `SKIP_DISPATCH_PROBE` entries, and `agentos_mutation` rows for mutating tools (`click_ref`, `fill_ref`, `open_ex`, `cookies_import`).

Default CI `nextest -p vox-orchestrator-mcp` does **not** enable `heavy-browser`. Put compile-safe tests in `params.rs`, or run `cargo test -p vox-orchestrator-mcp --features heavy-browser`.

| Tool | Mutating | Params |
| --- | --- | --- |
| `vox_browser_snapshot` | no | `page_id`, `interactive_only` (default true), `max_depth` (default 12), `max_nodes` (default 80), `include_boxes` (default false) |
| `vox_browser_click_ref` | yes | `page_id`, `ref` (`e1`), optional `actor` (lock hint only; ignored for role) |
| `vox_browser_fill_ref` | yes | `page_id`, `ref`, `value`, optional `actor` |
| `vox_browser_open_ex` | yes | `BrowserLaunchOptions` + `save_profile: bool` (default false) |
| `vox_browser_cookies_export` | no | `page_id` — returns cookie **count** and a profile-relative path, not cookie values |
| `vox_browser_cookies_import` | yes | `page_id`, `path` (must be under `browser_profiles_dir()` after canonicalize + `strip_prefix`) |

`vox_browser_open` remains. It is ephemeral-only.

`vox_browser_act` must change: take a snapshot (`interactive_only`), send wrapped `tree` to `llm_bridge::call_llm`, require JSON `{ "action": "click_ref"|"fill_ref"|"goto"|"wait"|"noop", "ref": "...", "value": "...", "url": "..." }`. CSS `target` is accepted only as a fallback when `ref` is absent **and** the model supplied a selector — do not advertise it in the system prompt. Extend the control-lock `matches!` to include `click_ref` / `fill_ref`.

**Registration:** edit `contracts/operations/catalog.v1.yaml` only, then `vox ci operations-sync --target all --write`. Do not hand-edit `capability-registry.yaml` or `tool-registry.canonical.yaml`.

## 7. GUI

- Browser toolbar: launch mode `Ephemeral` | `Named` | **Connect Chrome** (do not reuse the word “Attach” — `browser_attach_session` already means “select existing `page_id`”).
- Named: profile picker + checkbox “Save cookies and site data for this profile” (unchecked by default). First save writes `ProfileConsent` via MCP.
- Connect Chrome: text field for loopback `cdp_url`.
- Spawn the GUI daemon with `VOX_MCP_CALLER_ROLE=human` so human lock + sensitive fills work.
- Overlay: Tauri command `browser_snapshot` → MCP `vox_browser_snapshot` (`include_boxes: true`). UI cannot call MCP directly. Position labels with the inverse of `mapClickToViewport` (letterboxed `object-contain`). Re-snapshot when overlay is enabled. Human clicks stay on `click_xy`.
- `needs_human` → toast + `action_log`. Not NeedsYou.
- Preview iframe: reject non-loopback URLs in `preview_start`.
- Do not persist cookies in `BrowserState`. Split `BrowserView.tsx` (already 782 lines) before adding toolbar/overlay.

## 8. Env vars

Add to `contracts/config/env-vars.v1.yaml` in the same commit as first `std::env::var` (env-parity). `owner_crate` is `vox-plugin-browser` or `vox-config` — do **not** copy the retired `vox-browser` owner on existing `VOX_CHROME_*` rows. Real descriptions, not `TODO`. Not `SecretId`s.

| Name | Owner | Meaning |
| --- | --- | --- |
| `VOX_BROWSER_ALLOWED_HOSTS` | `vox-plugin-browser` | Comma-separated host allowlist; unset/empty allows all (ephemeral). localhost / `data:` / `about:blank` always allowed |
| `VOX_BROWSER_PROFILES_DIR` | `vox-config` | Override `browser_profiles_dir()`; default `<VOX_DATA_DIR>/browser-profiles` |
| `VOX_BROWSER_ATTACH_SMOKE` | `vox-plugin-browser` | If attach smoke reads this via `std::env::var`, register it; otherwise keep the smoke `#[ignore]`-only |

Existing `VOX_CHROME_EXECUTABLE` and `VOX_BROWSER_NO_SANDBOX` stay.

## 9. Safety

- Control lock unchanged (trusted env role).
- Sensitive refs: §3.5.
- Host allowlist: §3.4.
- Snapshot wrapping: §3.1 (delimiter, not a MAC).
- Login / CAPTCHA: if the snapshot tree (case-insensitive) contains `captcha`, `recaptcha`, or `hcaptcha`, `vox_browser_act` returns `needs_human` / `captcha` and does not loop. Docs pages that mention “captcha” may false-positive — acceptable.
- Purchase / create-account: out of v1 automation policy — document in GUI copy. No extra classifier in v1.
- Redact query strings in engine/MCP **error** logs. Never log cookie JSON.

## 10. Testing

- **Required (no Chrome):** `compact_ax_snapshot` with realistic AxValue `type` fixtures; `SnapshotOptions::default()` values; `profile_id` validation including `con`; host allowlist including `data:` and `*.example.com` vs `example.com`; sensitive word-boundary; wrap nonce begin==end; empty AX; `max_nodes` truncation; `open_ex` consent with a temp dir (not string-equal helpers); path jail sibling-prefix; cookie export JSON has no `"value"` key; MCP param schema in `params.rs`; `browser_act_system_prompt` contains `click_ref` and does not contain `css` / `xpath`.
- **Ignored smoke (local Chrome):** `#[ignore = "slow; requires local Chrome/Chromium binary"]` — snapshot+click_ref on `data:text/html`; named profile cookie survives close+reopen; attach skipped unless `VOX_BROWSER_ATTACH_SMOKE=1` (and that var is in the env contract if read from Rust).
- GUI: vitest for consent mapper and overlay ref list. Playwright e2e only if an existing `browser-*.spec.ts` can assert the new toolbar without a live CDP host.

## 11. Success criteria

1. Chat can `snapshot` → `click_ref` on a local HTML fixture without CSS, on an **unlocked** page or a page whose lock owner is `agent` (not a GUI-opened human-locked tab).
2. Ephemeral launch writes **nothing under `browser_profiles_dir()`** after `close` of the last ephemeral tab. (chromiumoxide may still leave `$TMP/chromiumoxide-runner`; that is out of scope.)
3. Named launch without `save_profile` and without stored consent errors `consent_required`.
4. Named launch with consent writes under `browser_profiles_dir()` and a second `open_ex` with the same `profile_id` sees the cookie set in the smoke test. Ephemeral then named does not share `user_data_dir`.
5. `VOX_BROWSER_ALLOWED_HOSTS=example.com` rejects `https://evil.test` on `open` / `goto` / act-`goto`.
6. No new crate, no Playwright on the product path, no cookie values in MCP tool results.

## 12. Phases (implementation order)

0. Split `engine.rs` under the 500-line god-object cap.
1. Pure snapshot + trait/engine ref actions + MCP snapshot/click_ref/fill_ref.
2. `open_ex` host map + named + consent + profiles dir + allowlist.
3. `browser_act` snapshot loop + captcha pause.
4. GUI mode/consent/overlay + daemon caller role.
5. Attach + cookie export/import path (no values to the model).
