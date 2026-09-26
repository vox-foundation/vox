# Phase 3: Extract Misplaced Crates to Plugin Architecture - Pattern Map

**Mapped:** 2026-09-25
**Files analyzed:** 34 new/modified files (grouped below by criterion)
**Analogs found:** 33 / 34 (the webhook event-drain extension has no poll-style precedent)

All analog paths below are git-tracked source (checked with `git ls-files`). None are `.vox/` or install mirrors. Line numbers are HEAD as of 2026-09-25.

---

## READ FIRST: findings that change the plan

These came up while mapping analogs. RESEARCH.md does not cover them, and each one blocks or reshapes a task.

1. **`vox-plugin-grammar-export` and a `GrammarExport` extension already existed. They were deleted on purpose.**
   Commit `0a8d1518c` (2026-05-24) removed `crates/vox-plugin-grammar-export/` along with `crates/vox-plugin-api/src/extensions/grammar_export.rs`, its `as_grammar_export` accessor and its catalog row. The deletion is recorded as decision **D-4 / D-18** in `docs/src/architecture/crate-audit-and-plan-2026.md:140,266,426,440`, with the reason "61 LoC pass-through; library stays". Phase 3 D-01 recreates both.
   - The old code is the **closest analog** for this work. Recover it with:
     - `git show 0a8d1518c^:crates/vox-plugin-grammar-export/src/lib.rs`
     - `git show 0a8d1518c^:crates/vox-plugin-grammar-export/Cargo.toml`
     - `git show 0a8d1518c^:crates/vox-plugin-grammar-export/Plugin.toml`
     - `git show 0a8d1518c^:crates/vox-plugin-api/src/extensions/grammar_export.rs`
   - The planner should add a doc task that marks crate-audit D-4/D-18 as superseded by Phase 3 D-01. Without it, the two architecture docs contradict each other. The difference this time is that the emitters really move; last time the plugin was only a pass-through.

2. **The `crate-edges` gate fails on every new edge, not only on upward edges.** `crates/vox-cli-ci/src/crate_edges.rs:100-111` reports `Violation::NewEdge` for any live edge that is missing from the baseline `edges` and has no `exceptions` entry. This phase adds these edges:
   | Edge | In baseline today? | Notes |
   |------|--------------------|-------|
   | `vox-plugin-grammar-export -> vox-grammar-export / vox-plugin-api / vox-plugin-sdk` (new crate) | no | Precedent: `a7aaf48d4` (new `vox-plugin-mens-candle-core`) added the new crate's edges to the baseline `edges` in the same commit. The carried-forward rule says STOP and ask with the exact gate output. Plan a `checkpoint:human-verify`. |
   | `vox-orchestrator -> vox-plugin-host` | **no** (`vox-orchestrator` has no plugin-host edge; see `crates/vox-orchestrator/Cargo.toml`) | Needed only if the webhook poller lives in `vox-orchestrator`. See item 3. |
   | `vox-orchestrator-mcp -> vox-plugin-host` | yes (`crate-edges.allow.v1.json:1426`) | |
   | `vox-cli -> vox-plugin-host` | yes (`:266`) | |
   | `vox-cli-ci -> vox-plugin-host` | yes (`:410`) | |
   | `vox-gui -> vox-plugin-host` | yes (`:958`) | Currently `optional` behind the `oratio` feature. See item 5. |

3. **Recommendation: put the webhook poller in `vox-orchestrator-mcp` so no new edge is needed.** `vox-orchestrator-mcp` (L4) already has the `vox-plugin-host` edge and already starts config-gated background pollers (`crates/vox-orchestrator-mcp/src/server_state.rs:415-444`). It already reaches `HopperIntake` through `vox_orchestrator::hopper` (`crates/vox-orchestrator-mcp/src/http_gateway/mod.rs:124-147`).
   - The config struct can still live in `vox-orchestrator/src/config/`. It is plain data and needs no plugin edge.
   - Placing the poller in `vox-orchestrator` itself requires a user-authorized exception for `vox-orchestrator -> vox-plugin-host`.

4. **An ABI bump touches every `Plugin.toml`.** `crates/vox-cli-ci/src/plugin_surface.rs:14-17,165-205` requires each code/composite `Plugin.toml` `abi-version` to equal `VOX_PLUGIN_ABI_VERSION`. It also requires each `extensions/<module>.rs` to have an accessor named exactly `as_<module>` in `abi.rs` (`:103,124-158`).
   - `contracts/plugin/extension-points.v1.yaml` is generated (`vox ci plugin-surface-sync --write`).
   - Precedent: `39ed9bf60` bumped the ABI from 11 to 12 and synced 13 manifests in one commit.
   - These 11 tracked `Plugin.toml` files use `abi-version = 12`: `crates/vox-plugin-{browser,mens-candle-cuda,mens-candle-metal,nvml-probe,populi-mesh,publication,runtime-container,runtime-wasm,speech,webhook}/Plugin.toml` and `crates/vox-plugin-host/tests/fixtures/noop-code/Plugin.toml`.
   - **Do both new extensions (grammar_export + webhook drain) in a single 12 -> 13 bump.**

5. **Deleting `stt-candle` breaks things outside vox-gui.**
   - `crates/vox-speech/Cargo.toml:46` has `serve = [..., "stt-candle"]`, and `crates/vox-speech/src/serve.rs:7,75` calls `backends::candle_whisper::transcribe_pcm_internal` directly.
   - `crates/vox-ml-cli/Cargo.toml:16` has `oratio = [..., "vox-speech/serve", ...]`, and `crates/vox-ml-cli/src/commands/oratio_cmd.rs:773` calls `vox_speech::serve::run_serve_worker`.
   - `crates/vox-speech/Cargo.toml:39` has `cuda = ["candle-core/cuda", ...]`, which names deps that are about to be removed.
   - In `vox-gui`, `vox-plugin-host` is `optional` behind `oratio` (`crates/vox-gui/Cargo.toml:15,43`). `mic.rs` is always compiled, so pointing it at the plugin means making that dep non-optional. The edge already exists, so this is fine.

6. **Loading `vox-plugin-webhook` starts a listener before any configuration.** Its hand-written `init` (`crates/vox-plugin-webhook/src/lib.rs:67-98`) calls `tokio::spawn`s `serve()` on `VOX_WEBHOOK_ADDR` (default `0.0.0.0:9080`). With no ingress token it runs in "degraded (no-auth) mode" (`:79-83`).
   - D-04 (no config means no listener) and fail-closed (Phase 2 D-02) therefore also require removing the auto-spawn from `init`. `start_listening` should be the only entry point, and it should refuse to start without an ingress token.
   - Otherwise `start_listening` binds a second server on top of the one `init` started.

7. **`automaton.rs` has no consumers.** `grep` finds nothing outside `crates/vox-grammar-export/src/automaton.rs`, and `json_schema.rs` does not import it (RESEARCH.md says it does). Deleting it is simpler than moving it.

8. **The plugin loader requires matching versions.** `load_code_plugin` refuses to load when `entry.version != env!("CARGO_PKG_VERSION")` (`crates/vox-plugin-host/src/lib.rs:167-174`). Workspace version is `0.6.0`; the webhook and speech `Plugin.toml` files say `0.1.0`. Installation handles this (`crates/vox-cli/src/commands/plugin/install.rs:420,510`), but any test or harness that records a `PluginEntry` by hand must use `env!("CARGO_PKG_VERSION")`. `lib.rs:610-618` shows how.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| **SC#1 grammar-export** | | | | |
| `crates/vox-plugin-api/src/extensions/grammar_export.rs` (new) | ABI extension trait | request-response (FFI) | `crates/vox-plugin-api/src/extensions/speech_to_text.rs`; deleted `git show 0a8d1518c^:crates/vox-plugin-api/src/extensions/grammar_export.rs` | exact |
| `crates/vox-plugin-api/src/extensions/mod.rs` | module registry | — | itself (lines 4-15) | exact |
| `crates/vox-plugin-api/src/abi.rs` | ABI root accessor | — | `as_speech_to_text` (lines 102-106), default test (lines 195-199) | exact |
| `crates/vox-plugin-api/src/lib.rs` | ABI version const | — | lines 11-27 (bump policy) | exact |
| `contracts/plugin/extension-points.v1.yaml` | generated contract | — | `vox ci plugin-surface-sync --write` | generated |
| 11x `crates/vox-plugin-*/Plugin.toml` (`abi-version`) | manifest | — | commit `39ed9bf60` | exact |
| `crates/vox-plugin-grammar-export/Cargo.toml` (new) | config | — | `crates/vox-plugin-speech/Cargo.toml` + deleted crate's Cargo.toml | exact |
| `crates/vox-plugin-grammar-export/Plugin.toml` (new) | manifest | — | `crates/vox-plugin-speech/Plugin.toml`; deleted Plugin.toml | exact |
| `crates/vox-plugin-grammar-export/src/lib.rs` (new) | plugin entry | request-response (FFI) | `crates/vox-plugin-speech/src/lib.rs` + `src/audio.rs:1-31,192-198` | exact |
| `crates/vox-plugin-grammar-export/src/{json_schema,lark,ssot_markdown,x_grammar_2,gbnf}.rs` (moved) | utility (emitters) | transform | the current files in `crates/vox-grammar-export/src/` (moved with `git mv`) | exact |
| `crates/vox-plugin-grammar-export/tests/export_test.rs` (new, split) | test | transform | `crates/vox-grammar-export/tests/export_test.rs` | exact |
| `crates/vox-grammar-export/src/lib.rs` + `Cargo.toml` (shrink) | library | transform | itself | exact |
| `crates/vox-cli/src/commands/grammar.rs` | CLI controller | request-response | `crates/vox-gui/src/commands/oratio.rs:45-59`; `crates/vox-orchestrator-mcp/src/oratio_tools.rs:39-53` | role-match |
| `crates/vox-cli/src/commands/ci/run_body_helpers/grammar.rs` | CI gate | request-response | same as above | role-match |
| `crates/vox-cli-ci/src/grammar_ssot_parity.rs` | CI gate | request-response | same as above | role-match |
| `crates/vox-populi/Cargo.toml` (drop `vox-grammar-export`, line 107) | config | — | — (single-line deletion; zero usages) | n/a |
| `crates/vox-plugin-catalog/catalog.toml` | config | — | `webhook` entry lines 162-169 | exact |
| `contracts/ci/crate-layers.v1.json`, `docs/src/architecture/layers.toml`, `docs/src/architecture/where-things-live.md` | contracts/docs | — | `vox-plugin-webhook` rows (`crate-layers.v1.json:88`, `layers.toml:267`, `where-things-live.md:340`) | exact |
| **SC#2 webhook** | | | | |
| `crates/vox-plugin-api/src/extensions/webhook_inbox.rs` (new; name is Claude's discretion) | ABI extension trait | batch drain (poll) | `crates/vox-plugin-api/src/extensions/http_listener.rs`; `audio_capture.rs` (`read_chunk -> RVec<u8>`) | role-match |
| `crates/vox-plugin-webhook/src/lib.rs` | plugin entry | event-driven -> poll buffer | itself (lines 104-152) | exact |
| `crates/vox-plugin-webhook/Plugin.toml`, `catalog.toml` webhook row | manifest | — | themselves | exact |
| `crates/vox-orchestrator/src/config/webhook_intake.rs` (new) | config | — | `crates/vox-orchestrator/src/config/scientia_research_mesh.rs` | exact |
| `crates/vox-orchestrator/src/config/{mod.rs,orchestrator_fields.rs,impl_default.rs}` | config wiring | — | `scientia_research_mesh` / `budget_gate_config` field sites | exact |
| `crates/vox-orchestrator-mcp/src/webhook_intake.rs` (new, recommended home) | service / background poller | poll -> CRUD submit | `crates/vox-orchestrator/src/clarification_db_inbox_poll.rs` + `server_state.rs:415-444` | role-match |
| `crates/vox-orchestrator-mcp/src/server_state.rs` (spawn call) | wiring | — | `spawn_scientia_research_mesh_background_jobs` (lines 339, 408, 415-444) | exact |
| **SC#3 SSG (evidence only)** | | | | |
| `docs/src/architecture/where-things-live.md:427` | doc | — | `vox-dashboard` retired row (line 430) | exact |
| `.planning/ROADMAP.md:81` | doc | — | — | n/a |
| **SC#4 Candle** | | | | |
| `crates/vox-quantize/Cargo.toml` + `src/lib.rs` | config + library root | — | `crates/vox-plugin-speech/Cargo.toml:12-25,43-54` (optional deps behind a feature); `crates/vox-populi/src/lib.rs:428-431` | exact |
| `crates/vox-populi/Cargo.toml:167`, `crates/vox-ml-cli/Cargo.toml:70` | config | — | `crates/vox-gui/Cargo.toml:42` (`features = [...]` on an optional dep) | exact |
| `crates/vox-gui/src/commands/mic.rs` (`transcribe_audio_file`, lines 32-60) | Tauri command helper | request-response | `crates/vox-gui/src/commands/oratio.rs:31-76` | exact |
| `crates/vox-gui/Cargo.toml` (lines 15, 43, 44) | config | — | itself | exact |
| `crates/vox-speech/Cargo.toml` + `src/{lib.rs,backend_dispatch.rs,backends/mod.rs,traits.rs,subtitle/srt.rs,serve.rs}` | library | — | existing `#[cfg(not(feature = "stt-candle"))]` fallback arms | exact |

---

## Pattern Assignments

### `crates/vox-plugin-api/src/extensions/grammar_export.rs` (new ABI extension trait)

**Analog:** `crates/vox-plugin-api/src/extensions/speech_to_text.rs` (whole file, 49 lines).

Required shape. `plugin_surface.rs` parses this file with regexes, so each of these is mandatory:
- `^pub trait (\w+)`: the PascalCase name used in the `Plugin.toml` `extension-points`.
- `^pub const \w+_REVISION: u32 = N;`
- The file stem must match the accessor. For `grammar_export.rs` the accessor must be `as_grammar_export`.

```rust
//! SpeechToText extension point — Whisper / similar STT engines.

use abi_stable::{sabi_trait, std_types::*};

pub const SPEECH_TO_TEXT_REVISION: u32 = 2;

#[sabi_trait]
pub trait SpeechToText: Send + Sync {
    fn revision(&self) -> u32 {
        SPEECH_TO_TEXT_REVISION
    }
    // ... JSON-in / JSON-out, RResult<RString, RBoxError>:
    fn transcribe_path(
        &self,
        path: RStr<'_>,
        config_json: RStr<'_>,
    ) -> RResult<RString, RBoxError> { ... }
}
```

Deleted precedent (`git show 0a8d1518c^:crates/vox-plugin-api/src/extensions/grammar_export.rs`). This is the exact method set to restore or refine. CONTEXT names the trait `GrammarExportPlugin`, and the trait name is not constrained, so either name works:
```rust
pub const GRAMMAR_EXPORT_REVISION: u32 = 1;

#[sabi_trait]
pub trait GrammarExport: Send + Sync {
    fn revision(&self) -> u32 { GRAMMAR_EXPORT_REVISION }
    fn export(&self, config_json: RStr<'_>) -> RResult<RString, RBoxError>;
    fn grammar_version_matches_compiler(&self) -> bool;
}
```
Suggested boundary: `config_json` is `serde_json::to_string(&GrammarExportConfig)`, and the return value is `serde_json::to_string(&GrammarExportResult)`. Both types derive `Serialize`/`Deserialize` (`crates/vox-grammar-export/src/lib.rs:42-78`) and stay in CORE, so each side (de)serializes the same type.

---

### `crates/vox-plugin-api/src/abi.rs` + `extensions/mod.rs` + `lib.rs` (accessor + ABI bump)

**Import** (copy `abi.rs:19`):
```rust
use crate::extensions::speech_to_text::SpeechToText_TO;
```
**Accessor** (copy `abi.rs:102-106`; add it inside `trait VoxPlugin`):
```rust
    /// Optional accessor: if this plugin provides a SpeechToText implementation,
    /// return Some(trait object). Default impl returns None.
    fn as_speech_to_text(&self) -> ROption<SpeechToText_TO<'static, RBox<()>>> {
        ROption::RNone
    }
```
**Default-RNone test** (copy `abi.rs:195-199` into `mod semcov_wave5_tests`):
```rust
    #[test]
    fn default_as_speech_to_text_returns_rnone() {
        let p = MinimalPlugin;
        assert!(p.as_speech_to_text().is_rnone());
    }
```
**mod.rs:** add `pub mod grammar_export;` (and `pub mod webhook_inbox;`) in alphabetical order (lines 4-15).
**lib.rs:13:** `VOX_PLUGIN_ABI_VERSION: u32 = 12` becomes `13`. Leave `VOX_PLUGIN_ABI_MIN_SUPPORTED` (line 27) alone; lines 20-26 say additive changes bump only VERSION.
**Then:** `vox ci plugin-surface-sync --write` regenerates `contracts/plugin/extension-points.v1.yaml`. Update all 11 `abi-version = 12` manifests (READ FIRST #4).

---

### `crates/vox-plugin-grammar-export/` (new plugin crate)

**Analog:** `crates/vox-plugin-speech/` (current SDK-macro style), plus the deleted crate for content.

**Cargo.toml.** Copy the shape of `crates/vox-plugin-speech/Cargo.toml:1-11,29-32,55-58`:
```toml
[package]
name = "vox-plugin-speech"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false
description = "..."

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
vox-plugin-sdk = { workspace = true }
vox-plugin-api = { workspace = true }
abi_stable = { workspace = true }   # REQUIRED even with the SDK — see vox-plugin-sdk/src/lib.rs:105-108
...
workspace-hack = { version = "0.6", path = "../workspace-hack" }

[lints]
workspace = true
```
Also add `vox-grammar-export = { workspace = true }` (L4 -> L1, downward), `serde_json`, `anyhow`, and `vox-language-surface` (needed by the moved `ssot_markdown.rs:7`). The workspace glob `members = ["crates/*"]` (root `Cargo.toml:3`) picks the crate up automatically. Plugin crates are not listed in `[workspace.dependencies]`.

**Plugin.toml.** Copy `crates/vox-plugin-speech/Plugin.toml` exactly and change only id, name, description, extension-points and artifact names:
```toml
[plugin]
id = "grammar-export"          # host resolves crates/vox-plugin-<id>/ for the install hint (vox-plugin-host/src/lib.rs:112)
name = "Grammar Export"
version = "0.1.0"
description = "..."
license = "Apache-2.0"

[plugin.host]
min-vox-version = "0.5.0"

[plugin.payload]
kind = "code"
abi-version = 13               # must equal VOX_PLUGIN_ABI_VERSION after the bump

[plugin.payload.provides]
extension-points = ["GrammarExportPlugin"]   # must equal the `pub trait` name exactly

[plugin.payload.artifacts]
"windows-x86_64" = "vox_plugin_grammar_export.dll"
"linux-x86_64"   = "libvox_plugin_grammar_export.so"
"macos-aarch64"  = "libvox_plugin_grammar_export.dylib"
```

**src/lib.rs, export glue.** Use the SDK macro (`crates/vox-plugin-speech/src/lib.rs:14-16`), not the hand-written `#[export_root_module]` block the deleted crate and webhook still use:
```rust
vox_plugin_sdk::declare_plugin! {
    init: |host| audio::make_plugin(host),
}
```
**Plugin struct and accessor.** Copy `crates/vox-plugin-speech/src/audio.rs:6-31`:
```rust
use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::abi::{VoxPlugin, VoxPlugin_TO, VoxPluginRef};
use vox_plugin_api::extensions::speech_to_text::{SpeechToText, SpeechToText_TO};

#[derive(Clone)]
pub(crate) struct OratioPlugin;

impl VoxPlugin for OratioPlugin {
    fn id(&self) -> RString { RString::from("oratio") }
    fn shutdown(&self) -> RResult<(), RBoxError> { RResult::ROk(()) }
    fn as_speech_to_text(&self) -> ROption<SpeechToText_TO<'static, RBox<()>>> {
        ROption::RSome(SpeechToText_TO::from_value(self.clone(), TD_Opaque))
    }
}
```
**Error mapping across the FFI boundary.** Copy `audio.rs:155`: `Err(e) => RResult::RErr(RBoxError::new(std::io::Error::other(e.to_string())))`.

**The `export()` dispatcher to move.** Take it verbatim from `crates/vox-grammar-export/src/lib.rs:91-122`, including the Gbnf CVE-2026-2069 `Err` arm and the `rule_count` computation. It calls `vox_grammar_export::ebnf::emit_ebnf()` and `vox_grammar_export::versioning::compute_ebnf_hash()` across the crate boundary. `x_grammar_2.rs:6,10` uses `crate::grammar_ir::Grammar` and `crate::ebnf::emit_ebnf()`. After the move these become `vox_grammar_export::grammar_ir::Grammar` and `vox_grammar_export::ebnf::emit_ebnf()`.

**Tests.** Split `crates/vox-grammar-export/tests/export_test.rs` (433 lines). The gbnf, lark, json_schema and `export()` tests (roughly lines 90-248) move to `crates/vox-plugin-grammar-export/tests/export_test.rs` with their imports repointed. The EBNF, compact-prompt and versioning tests stay. Import block to adapt (lines 1-6):
```rust
use vox_grammar_export::ebnf::emit_ebnf;
use vox_grammar_export::gbnf::emit_gbnf;
use vox_grammar_export::json_schema::emit_json_schema;
use vox_grammar_export::lark::emit_lark;
use vox_grammar_export::versioning::{get_compiler_version, get_version, verify_grammar_alignment};
use vox_grammar_export::{GrammarExportConfig, GrammarFormat, export};
```
**Test through the trait object.** Copy `crates/vox-plugin-sdk/src/lib.rs:408-415`: `vox_plugin_sdk::wrap(Plugin)`, then call `as_grammar_export().into_option().unwrap().export(...)`. That exercises the real FFI-erased path without dlopen.

**TOESTUB for moved files.** In `a7aaf48d4`, tdd-guard / `skeleton/untested-pub-api` treated moved files as new code. The fix there was per-function entries in `contracts/toestub/suppressions.v1.json`, each checked for "zero tests in original location". This mostly does not apply here, because the emitters keep their tests through the split test file.

---

### `crates/vox-grammar-export/src/lib.rs` + `Cargo.toml` (shrink to the CORE core)

Keep: `GrammarFormat`, `GrammarExportConfig`, `GrammarExportResult`, `grammar_version_matches_compiler` (lines 1-78, 124-131), plus `pub mod compact_prompt; pub mod ebnf; pub mod grammar_ir; pub mod versioning;`.
Remove: `pub mod automaton; gbnf; json_schema; lark; ssot_markdown; x_grammar_2;` and `pub fn export` (lines 91-122). Delete `automaton.rs` outright (READ FIRST #7).
Drop `vox-language-surface` from `Cargo.toml:15` only if nothing left in the crate uses it. `ssot_markdown.rs` was the only user found.
The CORE consumers must still compile with no plugin present: `vox-compiler/src/llm_prompt.rs:9`, `vox-constrained-gen/src/{earley.rs:13,196,268; pda.rs:14,242}`, `vox-orchestrator-mcp/src/grammar_tools.rs:11` (EBNF only, which stays CORE, so no change) and `vox-cli/.../run_body_helpers/grammar.rs:12,98`.

---

### `crates/vox-cli/src/commands/grammar.rs`, `run_body_helpers/grammar.rs:60-114`, `crates/vox-cli-ci/src/grammar_ssot_parity.rs` (host-dispatch consumers)

**Analog:** `crates/vox-orchestrator-mcp/src/oratio_tools.rs:39-56` (anyhow flavor, which suits vox-cli and vox-cli-ci since both use `anyhow::Result`):
```rust
    let plugin = vox_plugin_host::cached_code_plugin("oratio")
        .map_err(|e| anyhow::anyhow!("oratio plugin load: {e}"))?;
    let stt = plugin
        .plugin
        .as_speech_to_text()
        .into_option()
        .ok_or_else(|| anyhow::anyhow!("oratio plugin missing SpeechToText accessor"))?;

    let config_json = serde_json::json!({ "language": language_hint }).to_string();

    let transcription_json = stt
        .transcribe_path(path_str.as_str().into(), config_json.as_str().into())
        .into_result()
        .map_err(|e| anyhow::anyhow!("transcribe_path plugin: {e}"))?;

    let v: serde_json::Value = serde_json::from_str(transcription_json.as_str())
        .map_err(|e| anyhow::anyhow!("plugin returned invalid JSON: {e}"))?;
```
A `String`-error variant, used by Tauri, is at `crates/vox-gui/src/commands/oratio.rs:45-66`.

**D-02 actionable error comes for free.** When the plugin is missing, `load_code_plugin` returns `LoadError::InitFailed` with this text (`crates/vox-plugin-host/src/lib.rs:154-159`):
```rust
        errors::LoadError::InitFailed(format!(
            "plugin '{plugin_id}' is not installed.\n\nTo install it, run:\n\n{}",
            format_install_hint(plugin_id, None)
        ))
```
`format_install_hint` (lines 66-81) adds `vox plugin install --path crates/vox-plugin-grammar-export --yes` when run from a checkout. Do not write a new formatter. The `{e}` in `.map_err(|e| ... "{e}")` forwards the whole message. Keep `grammar.rs`'s existing `eprintln! + std::process::exit(1)` style (lines 38-44).

**D-02 test.** Copy `crates/vox-plugin-host/src/lib.rs:368-391`: build an empty `Registry::new()` and call `load_code_plugin(&registry, "grammar-export")`. Assert `Err(LoadError::InitFailed(msg))` and that `msg.contains("vox plugin install grammar-export")`. This needs no dlopen and no network.

**`run_grammar_export_check` split.** `ebnf::emit_ebnf()` (line 12) and `versioning::verify_grammar_alignment()` (line 98) stay direct CORE calls. Only the loop at lines 63-93 goes through the plugin. The existing `export(&config)?` on `GrammarFormat::Gbnf` (line 77) propagates the CVE `Err`. Run `cargo run -q -p vox-cli -- ci grammar-export-check` on HEAD first to record whether it is already red (RESEARCH Open Q3).

**`grammar_ssot_parity.rs` note.** This is a CI gate in `vox-cli-ci` (L3). Moving `ssot_markdown` behind the plugin means the gate needs the plugin built and installed on CI runners, or the gate will fail with the D-02 error. The planner must add a CI install step or accept that dependency.

---

### `crates/vox-plugin-api/src/extensions/webhook_inbox.rs` (new poll extension, D-10)

**Analog:** `crates/vox-plugin-api/src/extensions/http_listener.rs` (whole file):
```rust
//! HttpListener extension point — HTTP listener plugins.
//! Used by Webhook and similar plugins.

use abi_stable::{sabi_trait, std_types::*};

pub const HTTP_LISTENER_REVISION: u32 = 1;

#[sabi_trait]
pub trait HttpListener: Send + Sync {
    fn revision(&self) -> u32 {
        HTTP_LISTENER_REVISION
    }
    fn start_listening(&self, config_json: RStr<'_>) -> RResult<(), RBoxError>;
    fn stop_listening(&self) -> RResult<(), RBoxError>;
}
```
A `RVec` return type has precedent in `audio_capture.rs` (`read_chunk -> RResult<RVec<u8>, RBoxError>`). Shape for D-10:
`fn poll_events(&self, max: u32) -> RResult<RVec<RString>, RBoxError>;`. Each `RString` is `serde_json::to_string(&WebhookEvent)`. The fields `id, source, event_type, payload, received_at` come from `crates/vox-plugin-webhook/src/webhook/handler.rs:23-34`, which already derives `Serialize`/`Deserialize`.
Alternative with no new module: add a defaulted `poll_events` to `HttpListener` and bump `HTTP_LISTENER_REVISION` to 2. `speech_to_text.rs:39-48` shows a defaulted optional method. Either option needs the ABI bump; share it with grammar_export.

---

### `crates/vox-plugin-webhook/src/lib.rs` (implement drain; remove auto-listen)

**Analog:** itself. Accessor pattern (lines 117-119):
```rust
    fn as_http_listener(&self) -> ROption<HttpListener_TO<'static, RBox<()>>> {
        ROption::RSome(HttpListener_TO::from_value(WebhookHttpListener, TD_Opaque))
    }
```
**Event source to drain.** `WebhookState.event_sink: Arc<tokio::sync::broadcast::Sender<WebhookEvent>>` (`webhook/router.rs:34,43-47`), which gets `state.event_sink.send(event)` at `router.rs:224`. Inside `start_listening` (lines 129-147), call `state.event_sink.subscribe()` before `serve()` moves the state. Store the `broadcast::Receiver` in plugin-owned state, for example a `std::sync::Mutex<Option<Receiver>>` in a `static OnceLock` or a field on the listener struct, which then can no longer be a unit struct. `poll_events` loops `try_recv()` up to `max`. Treat `Lagged(n)` the way `webhook/bridge.rs:127-134` does (warn and continue).
**Remove.** Drop the `init()` auto-spawn (lines 69-93) and the degraded no-auth path (lines 79-83 and 137-139). Missing ingress token means `RErr`, which fails closed.
**Tests.** Follow the existing `#[tokio::test]` pattern at lines 190-234 (`start_listening` on `127.0.0.1:0`). Add `poll_events` tests that push through `state.event_sink.send(...)` without HTTP.
**Also update** `Plugin.toml:16` and `catalog.toml:167` `extension-points` to add the new trait name.
The lint override at line 26, `#![allow(dead_code, unused_imports)]`, can shrink once the bridge types are no longer dead.

---

### `crates/vox-orchestrator/src/config/webhook_intake.rs` (new, D-04 opt-in)

**Analog:** `crates/vox-orchestrator/src/config/scientia_research_mesh.rs` (whole file, 60 lines). It already combines an opt-in flag, a poll interval and a unit test:
```rust
use serde::{Deserialize, Serialize};

use super::defaults::default_false;
use super::orchestrator_fields::OrchestratorConfig;

fn default_intake_consumer_poll_interval_ms() -> u64 {
    30_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScientiaResearchMeshConfig {
    #[serde(default = "default_false")]
    pub intake_writer_enabled: bool,
    #[serde(default = "default_false")]
    pub intake_consumer_poll_enabled: bool,
    #[serde(default = "default_intake_consumer_poll_interval_ms")]
    pub intake_consumer_poll_interval_ms: u64,
}
...
#[cfg(test)]
mod tests {
    use crate::config::OrchestratorConfig;

    #[test]
    fn intake_writer_active_follows_flags() {
        let mut c = OrchestratorConfig::default();
        assert!(!c.research_mesh_intake_writer_active());
        ...
    }
}
```
**Presence-gates-load field.** For "no section means nothing loads", the model is `budget_gate_config` (`orchestrator_fields.rs:569-571`) rather than a bool flag:
```rust
    /// Optional configuration for the orchestrator-policy budget gate (D7).
    #[serde(default)]
    pub budget_gate_config: Option<crate::budget_gate::BudgetGateConfig>,
```
Add a `pub webhook: Option<WebhookIntakeConfig>` field near `orchestrator_fields.rs:507-509`, add `webhook: None,` to the struct literal in `impl_default.rs` (next to line 161), and add `mod webhook_intake;` plus `pub use webhook_intake::WebhookIntakeConfig;` in `config/mod.rs:16,24`.
**Env overlay:** skip it. `impl_env.rs:926-943` shows every env override goes through `vox_secrets::SecretId`, so adding one means registering SecretIds (Secret Management policy). D-04 only needs the TOML section. If the ingress token is a secret, resolve it with `vox_secrets::resolve_secret(...)` and never read it with `std::env::var` (webhook `lib.rs:74,135` does the latter today).
**D-04 test.** `OrchestratorConfig::default().webhook.is_none()`. The poller spawn function returns early on `None`, and the test asserts that no plugin load was attempted (see the next section).

---

### `crates/vox-orchestrator-mcp/src/webhook_intake.rs` (new poller) + `server_state.rs` spawn

**Spawn-site analog:** `crates/vox-orchestrator-mcp/src/server_state.rs:427-443`, a config-gated background job with an interval clamp:
```rust
        if self
            .orchestrator_config
            .scientia_research_mesh
            .intake_consumer_poll_enabled
        {
            let root = self.repository.root.clone();
            let ms = self
                .orchestrator_config
                .scientia_research_mesh
                .intake_consumer_poll_interval_ms
                .max(1_000);
            vox_publisher::research_mesh::spawn_research_mesh_intake_consumer(
                root,
                std::time::Duration::from_millis(ms),
            );
        }
```
Called from `server_state.rs:339` and `:408`. Add a sibling `spawn_webhook_intake_poller()` call at both sites, gated on `if let Some(cfg) = &self.orchestrator_config.webhook`. The hopper handle is `self.orchestrator.hopper()`, which returns `Arc<dyn HopperIntake>` (`crates/vox-orchestrator/src/orchestrator/accessors.rs:561`; `ServerState.orchestrator` at `server_state.rs:62`).

**Loop analog:** `crates/vox-orchestrator/src/clarification_db_inbox_poll.rs:20-25`:
```rust
    let handle = tokio::spawn(async move {
        let mut tick = tokio::time::interval(vox_config::timeouts::D_5S);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            ...
```
**Plugin load:** `vox_plugin_host::cached_code_plugin("webhook")`. It takes `&'static str`, is blocking discovery plus dlopen, and should be wrapped in `tokio::task::spawn_blocking` (see the doc comment at `vox-plugin-host/src/lib.rs:135`). Then call `.plugin.as_webhook_inbox().into_option()` and `start_listening(config_json)` once, and poll on every tick.
**Submit analog:** `crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs:704-713`:
```rust
    let item = gs
        .hopper
        .submit(
            body.intent,
            body.affinity_hints,
            body.priority_hint,
            body.source,
            body.session_id,
        )
        .await;
```
Use `IntakeSource::Webhook` and `PriorityHint::Unspecified` (`dashboard_api.rs:683-689`). The trait signature is at `crates/vox-orchestrator/src/hopper/store.rs:54-66`.
**Test without network.** Put the pure mapping (JSON string to `(intent, affinity_hints)`) in its own function and unit-test it with synthetic payloads. For the submit leg, copy `crates/vox-orchestrator/src/hopper/store.rs:638-650` (`InMemoryHopper::headless()`, then `.submit(..., IntakeSource::Webhook, None)`, then assert `inbox().len()`).

**Routing to re-derive (D-10 defactor).** Copy the logic, not the types, from `crates/vox-plugin-webhook/src/webhook/bridge.rs:62-77`:
```rust
        let kind = match (event.source.as_ref(), event.event_type.as_ref()) {
            ("github" | "gitlab", "push" | "tag_push") => InboxItemKind::GitPush,
            ("github" | "gitlab", "pull_request" | "merge_request") => InboxItemKind::PullRequest,
            ("discord" | "slack", _) => InboxItemKind::ChannelMessage,
            _ => InboxItemKind::ExternalEvent,
        };
```
Also keep the GitLab deprecation `tracing::warn!` (lines 63-69).

---

### `docs/src/architecture/where-things-live.md:427` + `.planning/ROADMAP.md:81` (SSG evidence)

**Analog row:** `where-things-live.md:430`:
```
| `vox-dashboard` | **Retired** (deleted 2026-05-12, ADR-037 / ADR-045). Use `vox-gui` (Tauri 2). |
```
Replace line 427 (`| \`vox-ssg\` | Static site generator for Vox docs surface. |`) with the same wording pattern: "**Folded** into `vox-cli/src/utils/ssg/` (L4) by `9d385a60b`, 2026-05-12; not a CORE crate". Amend ROADMAP line 81 to "vox-ssg is not a CORE crate" (D-05).

---

### `crates/vox-quantize/Cargo.toml` + `src/lib.rs` (D-07 optional Candle)

**Manifest analog:** `crates/vox-plugin-speech/Cargo.toml:12-27,43-46`, which puts heavy deps behind a named feature with `dep:`, and GPU sub-features that refer to them:
```toml
[features]
default = ["stt-candle"]
stt-candle = [
    "dep:candle-core",
    ...
]
cuda = ["candle-core/cuda", "candle-nn/cuda", "candle-transformers/cuda"]

[dependencies]
candle-core         = { workspace = true, optional = true }
```
For vox-quantize, `default` must stay `[]`. D-06 requires that no unconditional Candle dependency remains in L0-L3.
Change `candle-core = { workspace = true }` (line 9) to `optional = true`. Add a feature, for example `engine = ["dep:candle-core"]`. Change `cuda`/`metal` (lines 25-26) to `["engine", "candle-core/cuda"]` or use `candle-core?/cuda`.

**Module-gate analog:** `crates/vox-populi/src/lib.rs:428-431`:
```rust
/// MENS Mn-T2 inference backends (Candle CPU/CUDA/Metal, llama.cpp RPC, Ollama).
#[cfg(feature = "mens-candle-qlora")]
#[allow(missing_docs)]
pub mod inference;
```
Also `crates/vox-ml-cli/src/commands/mod.rs:29-30` (`#[cfg(feature = "quantize")] pub mod quantize;`).
Every module in vox-quantize reaches Candle, including `error.rs:13-14` (`#[from] candle_core::Error`) and `device.rs:4`. Gate all 8 `pub mod` lines, the `pub use` block (`lib.rs:15-19`) and `semcov_wave23_tests` behind the feature. With the feature off, the crate is empty, which is fine.

**Consumer analog:** `crates/vox-gui/Cargo.toml:42`: `vox-ml-cli = { workspace = true, optional = true, features = ["oratio-mic"] }`. Apply it to:
- `crates/vox-populi/Cargo.toml:167`, which becomes `vox-quantize = { workspace = true, optional = true, features = ["engine"] }`
- `crates/vox-ml-cli/Cargo.toml:70`, with the same change

The comment style to keep is at `vox-populi/Cargo.toml:162-166`.
Verify with `cargo check -p vox-quantize` (default, no Candle), `-p vox-populi --features mens-candle-qlora` and `-p vox-ml-cli --features quantize`. Because the dep is tagged `optional` plus `features`, Cargo only enables it when `dep:vox-quantize` activates.

---

### `crates/vox-gui/src/commands/mic.rs` (D-08 repoint to plugin) + `oratio.rs` shared helper

**Analog:** `crates/vox-gui/src/commands/oratio.rs:31-76` (`imp::transcribe_path_to_dto`): `.txt`/`.md` passthrough through `vox_speech::transcribe_path_detailed`, anything else through `cached_code_plugin("oratio")`, then `as_speech_to_text()`, then `transcribe_path`, then `vox_speech::refine_raw_text(&raw_text, &ctx)`.

**Behavior to preserve from `mic.rs:32-60`:**
- A fresh context on every call, not the cached one:
  ```rust
  let ctx = CorrectionContext::from_runtime(
      &vox_speech::OratioRuntimeConfig::resolve(),
      Default::default(),
      false,
  );
  ```
- The full error chain in the message: `.map_err(|e| format!("transcription failed: {e:#}"))`. The plugin returns `RBoxError`, so format its `Display` fully.
- The return type is `Result<String, String>` holding `detail.refined_text`.

**Recommended:** one shared helper, for example `pub(crate) fn transcribe_via_plugin(path, &CorrectionContext) -> Result<TranscribeDetail, String>`, called by both `oratio.rs::imp::transcribe_path_to_dto` and `mic.rs::transcribe_audio_file`. `oratio.rs` currently uses `CorrectionContext::default()` (line 38). Keep each caller's own context.
**Existing tests that must stay green:** `mic.rs:305-383`. `transcribe_audio_file_honors_domain_mode_env_var` and `transcribe_audio_file_refines_text_passthrough` use `.txt` passthrough and need no plugin. `synthetic_wav_is_well_formed_and_routed` (line 384 onward) currently expects `stt-candle` in-process; after the change it must accept the D-02 "plugin not installed" error string as the "clear error, never panic" outcome.
**Cargo:** in `crates/vox-gui/Cargo.toml`, make line 43 `vox-plugin-host` non-optional, remove `"dep:vox-plugin-host"` from `oratio` (line 15), and drop `"stt-candle"` from line 44.

---

### `crates/vox-speech/**` (D-08 delete `stt-candle`)

**Analog:** the existing `#[cfg(not(feature = "stt-candle"))]` arms, which become the only code path. Sites: `backends/mod.rs:9-17`, `lib.rs:24,55,70,76,85,89`, `backend_dispatch.rs:7,50-73,152,165`, `traits.rs:140-161`, `subtitle/srt.rs:1-8,119,257` (these become `feature = "stt-sherpa"` only).
**Manifest:** in `crates/vox-speech/Cargo.toml`, delete lines 21-36 (`stt-candle` and its TODO), the candle-only deps (lines 62-64, 66-76, 78-79), and `cuda` (line 39). Keep `hf-hub` and `rubato`, which `stt-sherpa` needs (line 44).
**Blocker:** `serve` (line 46) must drop `"stt-candle"`. `serve.rs:7,75` must dispatch through the plugin (`as_speech_to_text().transcribe(pcm, cfg)` takes raw PCM, per `speech_to_text.rs:16-20`) or be retired together with vox-ml-cli's `oratio` feature (`vox-ml-cli/Cargo.toml:16`, `oratio_cmd.rs:773`). That adds a `vox-speech -> vox-plugin-host` edge, which is not in the baseline, so check it. The planner has to choose; this is outside D-08's stated file set.

---

## Shared Patterns

### Plugin host dispatch (grammar CLI/CI, webhook poller, GUI mic)
**Source:** `crates/vox-plugin-host/src/lib.rs:318-340` (`cached_code_plugin`) and `:136-159` (`load_code_plugin_by_id`, `InitFailed` install hint).
**Apply to:** every consumer that moves off a direct library call.
Follow the order from `oratio_tools.rs:39-56`: load, then call the accessor with `.into_option().ok_or_else(...)`, then call the method with `.into_result().map_err(...)`, then `serde_json::from_str`. Do not add a fallback to an in-process copy (D-02).

### JSON-over-FFI boundary
**Source:** `speech_to_text.rs` (`RStr` config_json in, `RResult<RString, RBoxError>` JSON out); `audio.rs:140-155`.
**Apply to:** `GrammarExportPlugin::export` and `WebhookInbox::poll_events`. Serialize existing serde types (`GrammarExportConfig`/`GrammarExportResult`, `WebhookEvent`). Do not invent new `#[repr(C)]` structs.

### Optional heavy dependency behind a feature
**Source:** `crates/vox-plugin-speech/Cargo.toml:12-54`; `crates/vox-populi/Cargo.toml:56-66,162-167`; `crates/vox-populi/src/lib.rs:428-431`.
**Apply to:** vox-quantize (D-07) and the vox-speech cleanup (D-08).

### Defactor comment (Dependency Discipline rule 3)
**Source:** `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs:20-37`. The explanatory comment goes above the marker, and the marker sits directly on the copied item:
```rust
// ... why this is mirrored, and what pins it against drift ...
// vox:defactored-from vox-plugin-catalog 2026-09-05
pub(crate) const ML_BACKEND_CANDIDATES: &[vox_plugin_host::ExtensionCandidate] = &[
```
Other examples: `crates/vox-cli/src/commands/plugin/install.rs:431` (`// vox:defactored-from voxup 2026-08-24 (voxup::download::parse_checksums, ~13 lines)`) and `crates/vox-gui/src/drive/listener.rs:9`.
**Apply to:** the webhook routing match, as `// vox:defactored-from vox-plugin-webhook 2026-09-25 (webhook::bridge::OrchestratorInboxItem::from_webhook, ~15 lines)`. Keep the copy under 50 lines.

### New crate registration (the same commit as the crate)
**Source:** how `vox-plugin-webhook` is registered.
- `contracts/ci/crate-layers.v1.json:88`: `"vox-plugin-webhook": 4`
- `docs/src/architecture/layers.toml:267`: `vox-plugin-webhook = { layer = 4, kind = "plugin" }`
- `docs/src/architecture/where-things-live.md:335-340`: a plugin table row
- `crates/vox-plugin-catalog/catalog.toml:162-169`:
  ```toml
  [[plugin]]
  id = "webhook"
  payload-kind = "code"
  description = "..."
  status = "stable"
  extension-points = ["HttpListener"]
  default-source = "local:crates/vox-plugin-webhook"
  bundled-in = ["vox-server", "vox-dev"]
  ```
  Then regenerate with `cargo run -p vox-cli -- ci generate-plugin-catalog-docs`, which updates `docs/src/reference/plugin-catalog.generated.md` and `distribution-bundles.generated.md`.
- `contracts/ci/crate-edges.allow.v1.json`: STOP and ask (READ FIRST #2).
- Regenerate `crate-graph.v1.json` / `crate-build-map.v1.json` in the same commit (CONTEXT carried-forward rule).

### Fail-closed network surface
**Source:** Phase 2 D-02; `crates/vox-plugin-webhook/src/lib.rs:79-83` is the counter-example to remove.
**Apply to:** the webhook config and `start_listening`. No config means no load. No ingress token means an error, never degraded mode.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `WebhookInbox::poll_events` drain inside `vox-plugin-webhook` | plugin-internal buffer | broadcast to poll | No plugin buffers events for the host to pull. The closest shapes are `AudioCapture::read_chunk` (a pull-style `RVec`) and `bridge.rs:116-141` (broadcast `recv` loop and `Lagged` handling). Use RESEARCH.md §Webhook option 1. |

---

## Metadata

**Analog search scope:** `crates/vox-plugin-{api,host,sdk,speech,webhook}`, `crates/vox-grammar-export`, `crates/vox-{cli,cli-ci,orchestrator,orchestrator-mcp,gui,speech,quantize,populi,ml-cli}`, `contracts/ci`, `contracts/plugin`, `docs/src/architecture/{where-things-live.md,layers.toml,crate-audit-and-plan-2026.md}`, and git history (`0a8d1518c`, `39ed9bf60`, `a7aaf48d4`).
**Files scanned:** about 45
**Pattern extraction date:** 2026-09-25
