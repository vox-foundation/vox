---
phase: 446-roman-scriptorium-theme-and-cv-flywheel
status: verified
threats_open: 0
reviewed_at: 2026-11-16T03:58:00Z
---

# Phase 446 Security Assessment (`TERM-ROMA-CV-01..04`)

**Status:** Verified (`0` open threats)

## 1. Threat Model & Mitigations Verified

| ID | Category | Surface / Asset | Threat | Mitigation & Evidence | Status |
|---|---|---|---|---|---|
| `T-446-01` | Secret & Env Hygiene | `e2e/lib/terminalRichFixtures.ts`, `e2e/review/states.ts`, `446-UI-SPEC.md` | Accidental leakage of live API keys, bearer tokens, internal hostnames, or workspace secrets inside E2E synthetic fixtures or CV ledgers. | All terminal commands, outputs, paths, and model identifiers in `terminalRichFixtures.ts`, `states.ts`, `AgentTurnContent.test.tsx`, and `SlashMetaContent.test.tsx` use strictly synthetic, non-secret fixtures (`openrouter/claude-sonnet`, `ollama/qwen3-coder-14b`, workspace-local paths); runtime Vitest guard `src/__tests__/noVersionedModelIds.test.ts` passes (`0` violations). | Closed |
| `T-446-02` | XSS / HTML Injection | `sgrToSpans.ts`, `BlockCard.tsx`, `TerminalTab.tsx`, `JsonLedgerTable.tsx` | Unescaped ANSI/OSC terminal output or clickable path/URL detection injecting raw DOM markup (`dangerouslySetInnerHTML` / `innerHTML`). | Zero `dangerouslySetInnerHTML` or `innerHTML` sinks; SGR parser `sgrToSpans.ts` tokenizes into React text/element spans and sanitizes clickable URLs/file links before mounting. | Closed |
| `T-446-03` | DOM/Focus Hijack & UI Spoofing | `src/index.css`, `xtermAddons.ts`, `TerminalApprovalCard.tsx`, `HistorySearchPopover.tsx` | Hidden `.xterm-helper-textarea` receiving stray focus rings/chrome bleed or overlapping/clipped controls obscuring security/approval CTAs at narrow viewports (`900x600`). | `.xterm-helper-textarea` locked to an invisible `0x0` box isolated from light/dark input normalizers; `TerminalApprovalCard` (`max-h-44 min-[1100px]:max-h-none`) and `HistorySearchPopover` (`max-h-[min(340px,calc(100vh-140px))]`, `tabIndex={0}`) prevent control clipping or overlap across `900x600..1920x1080` (`clippedInteractiveCount === 0`, `overlappingControlPairs === 0` across all `112` matrix cells). | Closed |
| `T-446-04` | Browser Serialization Sandbox & Audit Fail-Closed Integrity | `e2e/review/terminalAudits.ts`, `e2e/review/audits.ts`, `e2e/review/vlmVisionFlywheel.ts`, `e2e/review/states.ts` | Closure capture causing silent `ReferenceError`, missing 2D canvas context silently returning `0` pixel defects, or `AxeBuilder` errors silently reporting `0` violations. | `terminalAudits.ts`, `audits.ts`, and `terminalRichFixtures.ts` are verified closure-free via standalone `new Function('return (' + fn.toString() + ')')()` Vitest assertions; `decodePngRgbaInPage` throws explicitly on null 2D canvas context; `AxeBuilder` errors fail closed as `axeViolations = 1`. | Closed |

## 2. Static & Runtime Security Gates

- `toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src` -> exit `0` (`No issues found`)
- `toestub --rules vox/llm-provider-call crates` -> verified `0` direct vendor calls in Phase 446 UI/test edits (re-verified in Step 5 workspace sweep)
- `vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md` -> exit `0` (`no hard errors`)
