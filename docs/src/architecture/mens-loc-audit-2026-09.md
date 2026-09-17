---
title: "MENS Subsystem LoC Audit (2026-09)"
description: "Lines-of-code accounting for the MENS training subsystem: size, test ratio, largest files against the God Object thresholds, and undocumented candle-cuda/candle-metal duplication."
category: "Architecture SSOTs"
status: "research"
---

## Why this exists

A deep-research mandate covering MENS (in-house Qwen fine-tuning) training
infrastructure — memory sizing, quantization, Ollama support, CUDA/Metal SSOT,
cloud training — explicitly asked for a "LoC audit against the codebase." None
of the three execution plans that followed the mandate produced one. This is
that audit, scoped the way the original ask implies: not a raw `wc -l` dump,
but an accounting a MENS maintainer would actually use to plan further work.

**Tooling note:** this repo has no `tokei`/`cloc`/`scc` installed and no
existing LoC-counting subcommand in `vox-code-audit` or `vox-arch-check` — the
closest existing tooling is the `arch/god_object` detector (file-size
thresholds only, not a corpus-wide counter; see below). Counts here were
produced with `wc -l` / `grep -c` over the exact directories this document
lists, run from a clean worktree at commit `3ebc4d42c`. `where-things-live.md`
confirms `[guards] loc_budget = "off"` workspace-wide (see
[Structural Limits](../../agents/governance.md#god-object-limit-multi-tier)) —
there is no per-crate LoC gate to compare against, only the file-level
God Object thresholds used below.

## Workspace scale, for context

| Language | Scope | Lines |
| --- | --- | --- |
| Rust | `crates/**/*.rs` | ~890,600 |
| TypeScript | `crates/vox-gui/ui/src/**/*.{ts,tsx}` | ~59,500 |
| Vox | `examples/**/*.vox`, `scripts/**/*.vox` | ~19,000 |

## The MENS surface

Directories counted (per the mandate's scope):

- `crates/vox-populi/src/mens/`
- `crates/vox-plugin-mens-candle-cuda/`
- `crates/vox-plugin-mens-candle-metal/`
- `crates/vox-quantize/`
- `crates/vox-ml-cli/src/commands/mens/`
- `crates/vox-ml-cli/src/commands/schola/` (native LoRA training worker for
  `vox mens train`; entirely MENS-related, no partial-inclusion judgment call
  needed)
- `crates/vox-gui/ui/src/components/surfaces/Models/`

| Crate / path | Rust files | Lines |
| --- | --- | --- |
| `vox-populi/src/mens/` | 71 | 17,708 |
| `vox-plugin-mens-candle-cuda/` | 46 | 9,492 |
| `vox-plugin-mens-candle-metal/` | 46 | 8,852 |
| `vox-quantize/` | 10 | 2,530 |
| `vox-ml-cli/src/commands/mens/` | 37 | 10,872 |
| `vox-ml-cli/src/commands/schola/` | 7 | 2,327 |
| **Rust subtotal** | **217** | **51,781** |
| `vox-gui/ui/.../surfaces/Models/` (TypeScript) | — | 464 |
| **MENS surface total** | | **~52,245** |

That's roughly **5.8% of workspace Rust** concentrated in six crates/dirs —
a meaningful, but not runaway, share for a subsystem this actively worked on
this quarter.

## Test-to-production ratio

The repo's Test-First Policy (AGENTS.md) requires a test alongside every new
`pub fn`. Measuring this by line count (not test *count*, which the policy
already gates at commit time) answers a different question: is coverage
*thin* even where a test technically exists?

Method: for each of the 217 Rust files, everything from the first
`#[cfg(test)]` or `mod tests` marker to end-of-file is counted as test code;
everything before it is production code. This undercounts files that
interleave `#[test]` functions outside a trailing `mod tests` block, so treat
it as a lower bound on test share.

| | Lines | Share |
| --- | --- | --- |
| Production (approx.) | 38,325 | 74.0% |
| Test (approx., lower bound) | 13,456 | 26.0% |

A 26% test-line share across 217 files is a reasonable floor, not a red flag
— but it's an aggregate. Two files that are dedicated test suites
(`eval_gate/tests.rs` at 921 non-blank lines, `hardware/tests.rs`) do the
heavy lifting; a maintainer adding new MENS `pub fn`s should not assume the
aggregate ratio describes their file specifically.

## Largest files vs. the God Object thresholds

The `arch/god_object` detector (`crates/vox-code-audit/src/detectors/god_object.rs`)
is this repo's actual, enforced size gate — no custom threshold invented for
this audit. Its tiers, on non-blank lines, per file:

- **Info** > 300 lines
- **Warning** > 400 lines
- **Error** > 500 lines (`Severity::Error`, blocking under the default
  `legacy` CI mode)

Running that measure over the 217 MENS files:

| Tier | Files |
| --- | --- |
| Error (>500) | 26 |
| Warning (400–500) | 6 |
| Info (300–400) | 21 |

**23 of those 26 Error-tier files are not in
`contracts/toestub/suppressions.v1.json`** — only
`vox-ml-cli/src/commands/mens/populi/dispatch.rs`,
`vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/mod.rs`, and
`vox-plugin-mens-candle-cuda/src/candle_qlora_train/training_loop/mod.rs` carry
a suppression entry. The other 23 are live, unsuppressed `arch/god_object`
Error findings today. This is worth a follow-up: either the god_object
detector isn't wired into the gate MENS code actually runs through, or these
findings are landing as Warning-surfaced-but-not-blocking noise nobody has
triaged, or CI's default `legacy` mode genuinely fails on them and it's being
tolerated/missed. Whichever it is, it's inconsistent with the "Error blocks"
contract stated in AGENTS.md's Local CI Gate Tiers section.

Ten largest MENS files by non-blank line count:

| File | Non-blank lines |
| --- | --- |
| `vox-populi/src/mens/tensor/preset_schema.rs` | 1,368 |
| `vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs` | 1,195 |
| `vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs` | 1,159 |
| `vox-plugin-mens-candle-metal/src/model.rs` | 1,096 |
| `vox-plugin-mens-candle-cuda/src/model.rs` | 1,092 |
| `vox-populi/src/mens/tensor/memory_model.rs` | 1,046 |
| `vox-ml-cli/src/commands/schola/train/gpu.rs` | 1,045 |
| `vox-populi/src/mens/cloud/pipeline_dispatch.rs` | 1,042 |
| `vox-populi/src/mens/tensor/domain_router.rs` | 925 |
| `vox-ml-cli/src/commands/mens/eval_gate/tests.rs` | 921 (test file) |

`preset_schema.rs`, `memory_model.rs`, `domain_router.rs`, `pipeline_dispatch.rs`,
and `gpu.rs` are all production code well past the 500-line Error line and are
plausible single-responsibility-violation candidates: `memory_model.rs` and
`domain_router.rs` in particular sit in `tensor/`, a directory already split
into 30+ narrow files — their size looks like a natural place decomposition
stopped rather than an intentional design choice.

## candle-cuda / candle-metal duplication

The defactor policy (AGENTS.md, Dependency Discipline §3) allows duplicating
a helper under ~50 lines with a `// vox:defactored-from <crate> <date>`
comment instead of taking a crate edge; anything larger is supposed to be
split into a shared `-types`/`-core` crate.

Comparing the 44 filenames that exist in both `vox-plugin-mens-candle-cuda/src/`
and `vox-plugin-mens-candle-metal/src/`:

- **24 of the 44 are byte-for-byte identical** between the two crates,
  totaling **3,191 lines** (one side; ~6,380 lines counting both copies).
- **Only 1 of those 24** (`candle_qlora_train/oom.rs`, 149 lines) carries a
  `vox:defactored-from` marker.
- The other 23 unmarked identical files range from 3 lines
  (`hf_layout.rs`) up to **720 lines** (`qlora_preflight.rs`), with several
  more in the hundreds: `hf_keymap.rs` (399), `checkpoint_state.rs` (236),
  `manifest.rs` (367, 13-line diff — near-identical), `merge.rs` (335).

None of the large ones — `qlora_preflight.rs`, `hf_keymap.rs`,
`checkpoint_state.rs`, `merge.rs` — fit the policy's "~50 lines" defactor
carve-out; at that size the policy calls for a shared `-types`/`-core` crate,
not duplication. This reads as undocumented, unintentional duplication that
has grown into a real maintenance-risk: a bug fixed in one backend's
`qlora_preflight.rs` has a 720-line surface to silently miss in the other
unless someone remembers to diff both crates on every change.

Files that differ meaningfully between the two backends (`model.rs`,
`inference.rs`, `candle_qlora_train/mod.rs`, device-selection code) are
expected to diverge — that's the actual CUDA/Metal split the two crates exist
for. The concern here is narrowly the 23 unmarked identical files.

## Summary

| Metric | Value |
| --- | --- |
| MENS surface total (Rust + TS) | ~52,245 lines |
| Share of workspace Rust | ~5.8% |
| Test-line share (Rust, lower bound) | ~26% |
| Largest single file | `tensor/preset_schema.rs`, 1,368 non-blank lines |
| Files over the God Object Error threshold (500 lines) | 26 (23 unsuppressed) |
| Byte-identical undocumented duplicate files, cuda↔metal | 23 files, ~3,000 lines |

**Recommendations for whoever picks up further MENS work:**

1. Triage the 23 unsuppressed Error-tier god_object findings — either they're
   real decomposition debt (most look like it) or CI isn't actually running
   this detector against these paths, which is its own bug.
2. Either mark the 23 unmarked identical candle-cuda/candle-metal files with
   `vox:defactored-from` (if they're deliberately meant to diverge later and
   staying small enough is a real plan) or extract the large ones
   (`qlora_preflight.rs`, `hf_keymap.rs`, `checkpoint_state.rs`, `manifest.rs`,
   `merge.rs`) into a shared crate per the defactor policy's own escalation
   rule.
3. `tensor/memory_model.rs` and `tensor/domain_router.rs` are good first
   decomposition targets given `tensor/` already demonstrates the target
   granularity elsewhere in the same directory.
