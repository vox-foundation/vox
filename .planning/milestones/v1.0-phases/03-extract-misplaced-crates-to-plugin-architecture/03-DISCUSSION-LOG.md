# Phase 3: Extract Misplaced Crates to Plugin Architecture - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-25
**Phase:** 03-extract-misplaced-crates-to-plugin-architecture
**Areas discussed:** Grammar-export plugin shape, Done-already criteria (SC2/SC3), Candle-in-CORE interpretation, stt-candle retirement

---

## Grammar-export plugin shape

| Option | Description | Selected |
|--------|-------------|----------|
| Split core + plugin | Small CORE core for compiler/constrained-gen; emitters to plugin via host | ✓ |
| Full plugin, host dispatch | Everything moves; CORE callers degrade without plugin | |
| Research decides | Planner picks split line after measurement | |

| Plugin absent at runtime | Selected |
|--------|----------|
| Clear error + install hint | ✓ |
| Bundled fallback | |

## Done-already criteria

| Webhook (SC2) | Selected |
|--------|----------|
| Wire host dispatch | ✓ |
| Close with evidence | |

| Webhook activation | Selected |
|--------|----------|
| Opt-in via config | ✓ |
| Always when installed | |

| SSG (SC3) | Selected |
|--------|----------|
| Accept vox-cli module | ✓ |
| Extract vox-plugin-ssg | |

## Candle-in-CORE interpretation

| Option | Selected |
|--------|----------|
| Default build + relayer vox-quantize | ✓ (then narrowed by the edge answer below) |
| Strict manifest | |

| vox-populi -> vox-quantize upward-edge exception | Selected |
|--------|----------|
| Authorize that one edge | |
| Ask me when it's real | |
| Avoid the exception (feature-gate candle inside vox-quantize, keep its layer) | ✓ |

## stt-candle retirement

| Option | Selected |
|--------|----------|
| Repoint GUI to vox-plugin-speech + drop feature | ✓ |
| Defer | |

## Claude's Discretion
- Plan split/ordering; exact `GrammarExportPlugin` method set.

## Deferred Ideas
- Relayer vox-quantize out of CORE; move vox-populi inference/qlora out of populi.
