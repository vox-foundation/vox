---
phase: 03-extract-misplaced-crates-to-plugin-architecture
plan: 06
subsystem: vox-speech / vox-gui / vox-ml-cli (Oratio STT)
status: complete
tags: [oratio, whisper, candle, D-08, D-12, SC-4, mutation-proof, lockfile]
requires:
  - "03-05 complete (4f791903a, 0724cda26, 66c29256c, bd60ce91f)"
provides:
  - "vox-speech feature `audio-decode` (symphonia + rubato); `stt-sherpa` includes it"
  - "vox-speech has no stt-candle/cuda feature and no candle-*/tokenizers/rand 0.9/byteorder dependency"
affects:
  - "Phase 3 SC#4 (both halves closed with evidence)"
tech-stack:
  added: []
  removed: ["candle-core, candle-nn, candle-transformers, tokenizers, rand 0.9, byteorder (from vox-speech only)"]
  patterns: ["lockfile-only commit staged from the HEAD blob with one package block replaced", "manifest check proven load-bearing by mutation"]
key-files:
  created: []
  deleted:
    - crates/vox-speech/src/backends/candle_engine.rs
    - crates/vox-speech/src/backends/candle_whisper.rs
    - crates/vox-speech/src/backends/logit_processors.rs
    - crates/vox-speech/src/backends/multilingual.rs
    - crates/vox-speech/src/backends/melfilters.bytes
    - crates/vox-speech/src/backends/melfilters128.bytes
  modified:
    - crates/vox-speech/Cargo.toml
    - crates/vox-speech/src/lib.rs
    - crates/vox-speech/src/backend_dispatch.rs
    - crates/vox-speech/src/backends/mod.rs
    - crates/vox-speech/src/backends/audio_io.rs
    - crates/vox-speech/src/traits.rs
    - crates/vox-speech/src/subtitle/srt.rs
    - crates/vox-gui/Cargo.toml
    - crates/vox-ml-cli/Cargo.toml
    - Cargo.lock
    - crates/vox-gui/src/commands/mic.rs
    - docs/src/architecture/where-things-live.md
    - docs/src/reference/oratio-speech.md
decisions:
  - "Audio decoding moves from stt-candle to a named `audio-decode` feature; stt-sherpa includes it and vox-ml-cli `oratio` enables it, so dictation, session, subtitle and Sherpa paths keep decoding."
  - "whisper_backend() has no in-process fallback: a registered oratio-plugin transcriber, or an actionable `vox plugin install oratio` error."
  - "candle_backend_status_json keeps its signature and always returns {\"stt_candle\": false}."
metrics:
  started: 2026-09-27T04:28:00Z
  completed: 2026-09-27T04:45:00Z
  duration: "~17 min"
estimate:
  tokens: 90000
  tasks: 2
actuals:
  tokens: 26947
  tasks: 2
  commits: 3
plan_head_before: fb2b166c47b716909396b4b433af6ceadf5b6708
---

# Phase 3 Plan 06: Delete vox-speech's in-process Candle Whisper Summary

vox-speech no longer has the `stt-candle` or `cuda` feature, the four Candle Whisper modules, the melfilter assets, or any candle-*/tokenizers/rand 0.9/byteorder dependency. Candle Whisper now lives only in vox-plugin-speech, and vox-speech reaches it through the transcriber the host registers (03-05). Audio decoding moved to a new `audio-decode` feature. `stt-sherpa` includes it and vox-ml-cli `oratio` enables it, so no decoding path was lost.

## Commits

| # | SHA | Subject | Paths |
|---|-----|---------|-------|
| A | `8984d8263` | refactor(03-06): remove vox-speech's in-process Candle Whisper code and feature | the 15 planned paths (6 deletions), no Cargo.lock |
| B | `7febcbb15` | refactor(03-06): drop vox-speech's Candle-only dependencies | `Cargo.lock`, `crates/vox-speech/Cargo.toml` |
| D | `707994086` | docs(03-06): Candle Whisper now lives only in vox-plugin-speech | mic.rs, where-things-live.md, oratio-speech.md |

The start SHA was `fb2b166c4`, recorded in `target/phase03-06-start-sha.txt`. `git rev-list --count fb2b166c4..HEAD` = 3.

## Verification evidence

All cargo output was captured with `--color never`, and the clippy logs have 0 ESC bytes.

**T1 RED** (`target/phase03-06-red.txt`): `false`, `rc=1`, `candle_deps=3 stt_candle=true cuda=true`.

**T1 verify 1** passes after commit B with `verify1=0`. vox-speech has `candle_deps=0`, and its features are `[audio-decode, cloud, compiler-rerank, default, serve, stt-sherpa]`.

**T1 verify 2** (build matrix): `lib=0 decode=0 serve=0 sherpa=0 mlcli=0 mcp=0 gui=0`.
- m1: `unregistered_whisper_is_an_actionable_error ... ok`. That test now runs unconditionally; 74 passed.
- m2: `resample_pcm_scales_length ... ok` and `format_then_parse_roundtrip ... ok`; 77 passed.

**Step 5a:** before commit B, the working-tree vox-speech lock block equals HEAD's (`block-equal`).

**Step 6b:** `bcheck=0 bmlcli=0`. The working-tree lock changed by exactly six `-` lines in the vox-speech block: `byteorder`, `candle-core`, `candle-nn`, `candle-transformers`, `rand 0.9.4`, `tokenizers 0.21.4`.

**T1 verify 3** (`verify3=0`):
- The staged lock is the HEAD blob with only the vox-speech block replaced. It adds 0 lines and deletes only the six whitelisted lines.
- The staged block equals the block cargo wrote, and the package-name lists are identical.
- `affected-crates --check` exited 0, and `contracts/ci` is unchanged, so no regen was needed.
- Post-commit numstats for Cargo.lock: A→B is `0 6`, and fb2b→A is empty.

**Lockfile integrity (6e):** `cargo metadata --offline` exits 0. `git diff --stat -- Cargo.lock` still shows the other sessions' `+19` lines, uncommitted and untouched.

**hakari (6f, advisory):** before is `rc=0` with no diff output, and after is the same. `diff before after` is empty, so there is no new drift to follow up.

**SC#4 CORE scan** (`target/phase03-06-core-scan.txt`, 03-02's command verbatim) lists only L4 crates:
```
vox-plugin-mens-candle-core L4
vox-plugin-mens-candle-cuda L4
vox-plugin-mens-candle-metal L4
```
It has no L0–L3 or `Lunassigned` line, and vox-speech's candle-* count is 0.

**Mutation proof:**
- The vox-speech Cargo.toml was clean against HEAD before the mutation (`clean-before`).
- The mutant added `candle-core = { workspace = true, optional = true }`. Presence check: `mutant-present=1`.
- The mutated manifest parsed: `cargo metadata` succeeded, and `jq -e` printed `false` (`target/phase03-06-mutant.txt`, rc=1). The check therefore watches the dependency list.
- After removing the line, `git diff --quiet HEAD` passed (`restored-identical=0`), the re-check prints `true`, and the working-tree lock block is unchanged.

**T2 verify 1:** `t2verify1=0`. The staged where-things-live row contains ``Candle Whisper lives in `vox-plugin-speech` ``.

**T2 verify 2 (clippy):** `c1=0 c2=0 c3=0 c4=0 c5=0 c6=101`, and `t2verify2=0` (0 diagnostics in this plan's files).
- `c6=101` is the known pre-existing dependency failure in `crates/vox-search/src/crawler.rs:77:5` (`sort_by_key`) and `:111:17` (collapsible `if`). That file is clean in the tree, so the error comes from HEAD. Because the dependency fails, vox-gui itself was never linted under `-D`.
- As 03-05 did, I also ran `cargo clippy -p vox-gui --all-targets` without `-D`. It passed with rc=0 and `plan-files=0`. All of its warnings are in another session's dirty `commands/search_probe.rs`/`tests/search_probe_test.rs` and in vox-search's crawler.rs.
- I also ran the four vox-speech clippy runs before commit A. All were rc=0, so no follow-up lint commit was needed.

**T2 verify 3 (commit audit):** `t2verify3=0`. The commits hold exactly 15, 2 and 3 paths as planned. Every changed line of mic.rs in D is a `//` or `///` comment line (the grep for non-comment lines matched nothing).

**Overall verification 2:** `git status --porcelain -- crates/vox-speech` is empty, so no mutant was left behind.

**Overall verification 3 (crate-edges):** exit 1 with the same three pre-existing violations 03-04 and 03-05 recorded. They come from HEAD manifests, not from this plan, which only removed external dependencies:
```
NEW EDGE not in baseline: vox-gui -> vox-db-types
NEW EDGE not in baseline: vox-gui -> vox-research-shim
NEW EDGE not in baseline: vox-research-shim -> vox-compiler
Error: crate-edges: 3 violation(s)
```
No line names vox-speech or vox-ml-cli, and there is no UPWARD line. Nothing was edited: no exceptions entry, no `edges` change, no `--tighten`, no fan-in snapshot change.

**Hooks:** fmt-fix and tdd-guard passed on A and D; B skipped tdd-guard because it has no `.rs`. fmt-fix printed "formatted 53 file(s)". That is its known side effect on other sessions' dirty `.rs` files, and none of those files were staged or reverted. My paths were clean after each commit.

## Deviations from Plan

1. **Doc lint ran per file.** `vox-doc-pipeline --lint-only --paths a b` only linted the first path: the log shows a single "linting docs/src/architecture/where-things-live.md" line. I ran `--paths reference/oratio-speech.md` separately (`doclint-oratio=0`, no hard errors). Verify block 1 still ran as written and passed.
2. **where-things-live staging.** The file was clean at execution time; the plan expected it to be dirty. The HEAD-blob method (`hash-object` + `update-index`) therefore staged HEAD plus the one row. Numstat is `1 1`, and `git diff HEAD` showed only that row before staging.
3. **srt.rs stub gate.** As the plan says, the stub is now gated `not(feature = "audio-decode")`. Because `lib.rs` gates `pub mod subtitle` on `audio-decode`, the stub is never compiled. It was dead under the old gates too. I kept it as the plan specified.
4. **Extra checks not in the plan:**
   - Clippy of the four vox-speech feature sets before commit A.
   - `cargo clippy -p vox-gui` without `-D`.
   - A grep for external users of the removed re-exports (`ENV_*`, `LanguageEnvOverride`, `transcribe_audio_file*`, `backends::candle*|logit*|multilingual|audio_io`) outside vox-speech. It found none.
5. `VOX_SKIP_FRESHNESS_CHECK` was not needed. `LEAN_CTX_EXTRA_ROOTS=/Users/brbrainerd/dev/vox` was set after lean-ctx refused `target/` reads, because another session had switched its active root.

## Behaviour notes

- **Sherpa-only builds decode audio now.** `stt-sherpa` implies `audio-decode`. Before, a Sherpa-only build bailed on audio files; vox-gui never hit this because it also had stt-candle.
- **`transcript_status()` has two variants**, keyed on `stt-sherpa`. Both say Whisper runs through the `oratio` plugin when the host registered it.
- **Unregistered Whisper always returns the `vox plugin install oratio` error.** Earlier stt-candle builds fell back to in-process Candle instead.

## Follow-ups (recorded, not fixed)

- The pre-existing clippy `-D` failure in `crates/vox-search/src/crawler.rs` still blocks `-D` linting of vox-gui.
- The vox-gui sidecar autobuild package bug (deferred-items.md, 03-05) is unchanged. The sidecars in `target/release` were present, so autobuild did not run.
- `.github/workflows/ci.yml`'s CUDA exclusion list still names vox-speech. The plan calls this harmless and left it unedited.

## Known Stubs

None.

## Threat Flags

None. No new endpoint or trust boundary; the symphonia decoder code is unchanged and sits behind a new feature gate (T-03-23).

## Self-Check: PASSED

- `crates/vox-speech/src/backends/candle_whisper.rs` and `melfilters.bytes` do not exist.
- Commits `8984d8263`, `7febcbb15` and `707994086` are present in `git log`.
