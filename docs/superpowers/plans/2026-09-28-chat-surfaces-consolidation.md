# Chat Surfaces Consolidation and Data Honesty (Plan 3a) — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task docs/superpowers/plans/2026-09-28-chat-surfaces-consolidation.md <N>`), per
> [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, reviews the diff and commits. Steps use checkbox
> (`- [ ]`) syntax. A step labelled **Commit (Claude Code)** is not the agent's.
>
> **Supersedes** tasks 19–23 of the index plan
> [`2026-09-28-chat-surface-trace-and-latest-models.md`](2026-09-28-chat-surface-trace-and-latest-models.md).
>
> **Revision 2 (2026-09-28)** after review (T1/T2 FAIL, T3/T7/T8 WARN). Amended sections carry
> `<!-- AMENDED: T<n> — reason -->` markers. Summary: the trace plan
> ([`2026-09-28-chat-turn-trace.md`](2026-09-28-chat-turn-trace.md)) owns `routingModelLabel` and `MODE_NAMES` in
> `lib/turnEvents.ts`, so this plan no longer creates `routingLabel.ts`/`resolvedModelLabel`; the rail task is split in
> two with anchored micro-edits (Phase 5 plan 05-05 edits the same hook body); the rail's routing comes from the one
> App-level query; the guard is backslash-aware; Playwright overrides live in one shared helper. Now 9 tasks.

**Goal:** Each global engine fact has one home (status bar cards), the chat rail only shows this session, the composer
says each thing once in plain words, the research popover shows only measured health, and no GUI file, test or mock
carries a versioned cloud model id.

**Architecture:** Display rules live in small tested helpers: the trace plan's `lib/turnEvents.ts`
(`routingModelLabel` — "a version only when `resolved_from === 'catalog'`"; `MODE_NAMES`), and this plan's
`lib/modelFamily.ts` (family keys mirroring the Rust `family_key`), `lib/routingSummary.ts` (adapts the global
`RoutingSummary` to `routingModelLabel`) and `config/budget.ts#formatSpend` ("never `/ $0`"). The status bar renders five
cards from those helpers; App fetches the routing summary once and hands it to both the status bar and the rail; the
rail and composer shed duplicated blocks; the research popover renders only `getResearchEngineStatus` fields; a vitest
guard scans `src/` and `e2e/` for versioned cloud ids and the mocks are checked against the bootstrap catalog's family
keys.

**Tech Stack:** TypeScript, React 19, Vitest + Testing Library (jsdom), Playwright (chromium), `@tanstack/react-query`
(already installed). GUI only; no Rust.

**Spec:** [`docs/src/architecture/chat-surface-design-critique-2026-09-28.md`](../../src/architecture/chat-surface-design-critique-2026-09-28.md)
(one-home-per-fact rule; matrix rows 1–15 and 27–35; canonical vocabulary).

**Prerequisites (checked in each task's Step 0):** Phase 5 (`.planning/phases/05-*`, including 05-05's waiting-lock
rail rows) is landed, and the trace plan has landed (a) the optional `family`, `resolved_from`
(`'catalog' | 'bootstrap' | 'local'`) and `reason` fields on `RoutingSummary` (`crates/vox-gui/ui/src/types/tauri.ts`)
and (b) `export function routingModelLabel(e: TurnEventDto): string` and `export const MODE_NAMES` (a frozen `Record<"free"|"efficiency"|"balanced"|"genius", string>`) and `export function modeLabel(wire: string): string` (wire
value → Free / Efficient / Balanced / Genius) in `crates/vox-gui/ui/src/lib/turnEvents.ts`.

## Global Constraints

- GUI only: every file is under `crates/vox-gui/ui/` except `contracts/gui/hud-tiles.v1.yaml` (Task 2, a 60-line file).
  No Rust, no new dependency, no new crate edge, no Cargo.lock change.
- **No versioned cloud model id literals** in any file you write or edit (rule 9 of the brief). Test fixtures use family
  keys (`deepseek/deepseek-flash`, `anthropic/claude-sonnet`) or the fictional vendor `acme/…` when a version-shaped id is
  needed to prove it is *hidden*. Never write real vendor release ids.
- Wire values are unchanged: clutch ids `free | efficiency | balanced | genius`, HUD tile ids, execution modes
  `chat | task | plan`, Tauri command names.
- Do not create a second routing-label or mode-name formatter: use `routingModelLabel` / `MODE_NAMES` from
  `lib/turnEvents.ts`. <!-- AMENDED: T1 — single owner is the trace plan -->
- Test-first: write the tests, run them, save the failing output to the named `target/3a-*-red.txt` file, and only then
  edit implementation code. A RED log that says `passed` for a test the step says must fail is a broken step: STOP.
- Existing tests are **not** edited except the ones a step names by title. Any other existing test that breaks is a
  STOP: report its file, title and failing assertion.
- `tsc` (`pnpm typecheck`) excludes `*.test.ts(x)` and `e2e/`; tests may pass props a component no longer declares.
- Files already over the 500-line governance limit — `App.tsx` (~2090), `ChatSurface.tsx` (~1130), `Loquela.tsx`
  (~1010) — get only the edits shown (they shrink or stay flat). Never add a component to them.
- **Micro-edits only in shared-history files.** `useChatExecutionData.ts` and `ChatExecutionRail.tsx` were edited by
  Phase 5 (05-03, 05-05 adds synthetic "Waiting for resource lock" rows and `LockWaiting` handling inside the hook's
  `refresh` body). Edit only the quoted lines; never replace a function body or file tail. <!-- AMENDED: T3 — de-clobber -->
- Commands are foreground and `timeout`-prefixed: vitest/playwright/typecheck `timeout 300s` (full vitest `timeout
  600s`). Exit 124 means it hung: STOP. Playwright reuses the dev server on :1420 (`reuseExistingServer`); never `curl`.
- Edits are anchored on the quoted existing text, never on line numbers; if a quoted anchor is not found exactly once,
  STOP.
- The agent never runs `git add` / `git commit` / `git stash` / `git checkout`.

## File Structure

| File (under `crates/vox-gui/ui/` unless noted) | Status | Responsibility | Tasks |
|---|---|---|---|
| `src/lib/turnEvents.ts` | consume (trace plan) | `routingModelLabel`, `MODE_NAMES` | 1, 5 |
| `src/lib/modelFamily.ts` | create | `familyKey` (mirror of Rust `family_key`), `VERSIONED_CLOUD_MODEL_ID` | 1 |
| `src/lib/modelFamily.test.ts` | create | key rules + bootstrap-catalog seam | 1 |
| `src/lib/routingSummary.ts` | create | `routingCardValue`, `RailRouting`, `railRoutingFromSummary`, `modelStateHint` (adapters over `routingModelLabel`) | 1 |
| `src/lib/routingSummary.test.ts` | create | adapter tests | 1 |
| `src/config/budget.ts` | modify | add `formatSpend` (T1); delete `formatBudgetCap` (T2) | 1, 2 |
| `src/config/budget.test.ts` | create | `formatSpend` | 1 |
| `src/hooks/useHudTiles.ts` + `.test.ts` | modify | five cards, card-name labels, retired ids dropped on load | 2 |
| `src/components/surfaces/Settings/HudTilesEditor.test.tsx` | modify | stop using the retired `queue_depth` tile | 2 |
| `contracts/gui/hud-tiles.v1.yaml` (repo root) | modify | same five kinds + `retired_tile_kinds` | 2 |
| `src/components/layout/BottomStatusBar.tsx` + `.test.tsx` | modify | Engine / Spend (popover) / Mesh / Routing / Needs you cards, truncation, freshness tooltip | 2 |
| `src/components/layout/AppShell.tsx` | modify | forward `routingSummary`, `sessionSpentUsd`, `needsYouCount` | 2 |
| `src/App.tsx` | modify | routing-summary query (T2); rail plumbing + `chatRouting` (T4); `riskSlot` (T5) | 2, 4, 5 |
| `src/components/surfaces/Chat/ChatExecutionRail.tsx` + `.test.tsx` | modify | Routing section (next turn), no roster/resources, per-turn context refresh | 3 |
| `src/hooks/useChatExecutionData.ts` + `.test.ts` | modify | tasks only (no routing fetch, no mesh) | 3 |
| `src/components/surfaces/Chat/ChatSurface.tsx` | modify | pass `routing`, `turnsCompleted`; drop dead rail props | 4 |
| `src/components/layout/surfaceComponents.tsx` | modify | same plumbing | 4 |
| `src/lib/driveConsole.ts` | modify | mode labels from `MODE_NAMES` | 5 |
| `src/components/surfaces/Loquela/DriveConsole.tsx` + `.test.tsx` | modify | Mode hint, Spend label, `Risk: …`, `riskExtra` | 5 |
| `src/components/surfaces/Loquela/RiskPopover.tsx` + `.test.tsx` | modify | renders `children` (Check replies); copy says "check replies" | 5 |
| `src/components/surfaces/Chat/GroundingCheckToggle.tsx` + `.test.tsx` | modify | label "Check replies" | 5 |
| `src/components/surfaces/Loquela/Loquela.tsx` + `.test.tsx` | modify | `riskSlot`, daemon-only cap (T5); no "session $", Run hint, Plan mode (T6); ids (T8) | 5, 6, 8 |
| `src/App.test.tsx` | modify | grounding test opens the Risk popover | 5 |
| `src/lib/slashRouter.ts` + `.test.ts` | modify | delete `formatSessionBudget` | 6 |
| `src/components/common/StatusBarCluster.tsx` + `.test.tsx` | modify | honest research popover | 7 |
| `src/__tests__/noVersionedModelIds.test.ts` | create | the guard (backslash-aware) | 8 |
| `e2e/lib/tauriMock.families.test.ts` | create | mock ↔ bootstrap-catalog family seam | 8 |
| `e2e/lib/tauriMock.ts`, `e2e/lib/tauriMockRich.ts`, `e2e/chat-trust-chips.spec.ts` and 7 unit-test files | modify | family keys instead of versions | 8 |
| `e2e/lib/invokeOverrides.ts` | create | one data-driven invoke-override helper for specs | 9 |
| `e2e/status-bar-surfaces.spec.ts` | modify | cards spec + `status-bar-cards.png` | 9 |
| `e2e/chat-surfaces-consolidation.spec.ts` | create | rail / composer / research popover specs + screenshots | 9 |

---

### Task 1: Display-rule helpers — family keys, the routing-summary adapter, spend

<!-- AMENDED: T1 — `routingLabel.ts`/`resolvedModelLabel` dropped (the trace plan owns `routingModelLabel`);
`routingCardValue` and the rail selector now require a decision (family without a decision used to render
"Auto → family (offline)"); `MODEL_STATE_HINTS` is a Map of the four real `ModelConfidence` values; the unused
`ResolvedFrom` export and the speculative explore/exploit hints are gone. -->

**Files:**
- Create: `crates/vox-gui/ui/src/lib/modelFamily.ts`, `crates/vox-gui/ui/src/lib/modelFamily.test.ts`
- Create: `crates/vox-gui/ui/src/lib/routingSummary.ts`, `crates/vox-gui/ui/src/lib/routingSummary.test.ts`
- Modify: `crates/vox-gui/ui/src/config/budget.ts` (append `formatSpend`)
- Create: `crates/vox-gui/ui/src/config/budget.test.ts`

**Interfaces:**
- Consumes: `RoutingSummary` (`src/types/tauri.ts`) with the trace plan's optional `family`, `resolved_from`, `reason`;
  `TurnEventDto` (`src/types/dashboard.ts`: `{ kind: string; [key: string]: unknown }`); the trace plan's
  `routingModelLabel(e: TurnEventDto): string` (catalog ⇒ `resolved_id`; local ⇒ `<resolved_id> (local)`; otherwise
  `<family> (offline)`); `contracts/orchestration/model-catalog.bootstrap.v1.json` (a JSON array of objects with `id`).
- Produces:
  - `export function familyKey(slug: string): string`
  - `export const VERSIONED_CLOUD_MODEL_ID: RegExp` (case-insensitive, no `g` flag)
  - `export function routingCardValue(summary: RoutingSummary | null | undefined): string`
  - `export interface RailRouting { model: string; reason: string | null; state: string | null; alternatives: string[] }`
  - `export function railRoutingFromSummary(summary: RoutingSummary | null | undefined): RailRouting | null`
  - `export function modelStateHint(state: string | null | undefined): string | null`
  - `export function formatSpend(spent: number, cap: number | null): string`

- [ ] **Step 0: Preconditions.** Run from `/Users/brbrainerd/dev/vox`:
  1. `rg -n -A16 "export interface RoutingSummary" crates/vox-gui/ui/src/types/tauri.ts` — the block must contain a line
     with `family`, a line with `resolved_from` and a line with `reason`.
  2. `rg -n "export function routingModelLabel|export const MODE_NAMES = Object.freeze" crates/vox-gui/ui/src/lib/turnEvents.ts`
     — exactly 2 hits.
  3. `ls crates/vox-gui/ui/src/lib/modelFamily.ts crates/vox-gui/ui/src/lib/routingSummary.ts crates/vox-gui/ui/src/config/budget.test.ts`
     — all three "No such file".
  On any mismatch: `DRIVE: STOPPED step 0: trace plan outputs (RoutingSummary fields / routingModelLabel / MODE_NAMES) not landed or files already exist`.

- [ ] **Step 1: Write the failing tests.** Create `crates/vox-gui/ui/src/lib/modelFamily.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { familyKey, VERSIONED_CLOUD_MODEL_ID } from './modelFamily';

// src/lib -> src -> ui -> vox-gui -> crates -> repo root
const here = dirname(fileURLToPath(import.meta.url));
const BOOTSTRAP = resolve(here, '../../../../../contracts/orchestration/model-catalog.bootstrap.v1.json');
const catalogIds = (): string[] =>
  (JSON.parse(readFileSync(BOOTSTRAP, 'utf8')) as Array<{ id: string }>).map((e) => e.id);

describe('familyKey (mirror of the Rust models::family::family_key)', () => {
  it('drops version numbers and v-markers across naming shapes', () => {
    expect(familyKey('acme/widget-pro-4.8')).toBe('acme/widget-pro');
    expect(familyKey('acme/widget-v4.1-flash')).toBe('acme/widget-flash');
    expect(familyKey('acme/widget-v4-flash-0731')).toBe('acme/widget-flash');
    expect(familyKey('acme/gizmo-6-luna')).toBe('acme/gizmo-luna');
    expect(familyKey('Acme/Gizmo-Ultra-5.5')).toBe('acme/gizmo-ultra');
  });

  it('drops release-stage qualifiers and keeps the other words', () => {
    expect(familyKey('acme/gizmo-3.1-flash-lite-preview')).toBe('acme/gizmo-flash-lite');
    expect(familyKey('acme/gizmo-latest')).toBe('acme/gizmo');
  });

  it('keeps parameter sizes and :free as their own families; drops other variants', () => {
    expect(familyKey('acme/zeta3.8-27b:free')).toBe('acme/zeta-27b:free');
    expect(familyKey('acme/zeta3.8-27b')).toBe('acme/zeta-27b');
    expect(familyKey('acme/zeta3-235b-a22b')).toBe('acme/zeta-235b-a22b');
    expect(familyKey('acme/zeta-3.1-8b')).not.toBe(familyKey('acme/zeta-3.1-70b'));
    expect(familyKey('acme/zeta-2:nitro')).toBe('acme/zeta');
  });

  it('mixed letter+digit tokens keep only their letters; an org-less slug still keys', () => {
    expect(familyKey('acme/k2.6-thinking')).toBe('acme/k-thinking');
    expect(familyKey('widget-pro-2')).toBe('widget-pro');
  });

  it('is idempotent on a family key', () => {
    expect(familyKey('deepseek/deepseek-flash')).toBe('deepseek/deepseek-flash');
    expect(familyKey('anthropic/claude-sonnet')).toBe('anthropic/claude-sonnet');
  });
});

describe('familyKey over the bootstrap catalog (contract seam)', () => {
  it('no catalog family key keeps a version or trips the versioned-id pattern', () => {
    const ids = catalogIds();
    expect(ids.length).toBeGreaterThan(10);
    for (const id of ids) {
      const key = familyKey(id);
      expect(key, id).not.toMatch(/\d+\.\d+|(^|[-/])v\d/);
      expect(VERSIONED_CLOUD_MODEL_ID.test(key), `${id} -> ${key}`).toBe(false);
    }
  });

  it('the versioned-id pattern does see the catalog versions that keying removes', () => {
    expect(catalogIds().filter((id) => VERSIONED_CLOUD_MODEL_ID.test(id)).length).toBeGreaterThan(5);
  });

  it('two catalog members of one line share one key (the sonnet ids)', () => {
    const sonnet = catalogIds().filter((id) => id.startsWith('anthropic/') && id.includes('sonnet'));
    expect(sonnet.length).toBeGreaterThanOrEqual(2);
    expect(new Set(sonnet.map(familyKey)).size).toBe(1);
  });
});
```

Create `crates/vox-gui/ui/src/lib/routingSummary.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { modelStateHint, railRoutingFromSummary, routingCardValue } from './routingSummary';

const summary = (over: Record<string, unknown> = {}, preview: Record<string, unknown> = {}) =>
  ({
    active_model: null,
    exploration_spent_usd: 0,
    exploration_budget_usd: 1,
    routing_priority: { efficiency: 50, precision: 50, latency: 50, availability: 50, balance: 50, mobile: 50 },
    arm_count: 1,
    model_count: 1,
    decision_preview: {
      selected_model: 'acme/widget-flash-4.8',
      discovery_state: 'confirmed',
      alternatives: [],
      rejection_reasons: [],
      intelligence_score: 0.5,
      efficiency_score: 0.5,
      latency_score: 0.5,
      ...preview,
    },
    family: 'acme/widget-flash',
    reason: 'lowest cost that fits the mode',
    ...over,
  }) as never;

describe('routingCardValue (global pick, via routingModelLabel)', () => {
  it('catalog pick shows the concrete id', () => {
    expect(routingCardValue(summary({ resolved_from: 'catalog' }))).toBe('Auto → acme/widget-flash-4.8');
  });

  it('bootstrap pick shows the family plus "(offline)", never the version', () => {
    const value = routingCardValue(summary({ resolved_from: 'bootstrap' }));
    expect(value).toBe('Auto → acme/widget-flash (offline)');
    expect(value).not.toContain('4.8');
  });

  it('a daemon without family/resolved_from is treated as offline and never shows a version', () => {
    expect(routingCardValue(summary({ family: undefined, resolved_from: undefined }))).toBe(
      'Auto → acme/widget-flash (offline)',
    );
  });

  it('a locally served pick shows its local id with "(local)"', () => {
    expect(
      routingCardValue(summary({ resolved_from: 'local', family: 'mens/finetune' }, { selected_model: 'mens/finetune-run' })),
    ).toBe('Auto → mens/finetune-run (local)');
  });

  it('reads plain "Auto" with no summary, no decision, or a blank pick — even when a family is present', () => {
    expect(routingCardValue(null)).toBe('Auto');
    expect(routingCardValue(summary({ decision_preview: null, resolved_from: 'bootstrap' }))).toBe('Auto');
    expect(routingCardValue(summary({ resolved_from: 'bootstrap' }, { selected_model: '  ' }))).toBe('Auto');
  });
});

describe('railRoutingFromSummary', () => {
  it('returns null without a summary or a decision, even when a family is present', () => {
    expect(railRoutingFromSummary(null)).toBeNull();
    expect(railRoutingFromSummary(summary({ decision_preview: null, resolved_from: 'bootstrap' }))).toBeNull();
  });

  it('off-catalog: alternatives become family keys, deduped, chosen family excluded, at most 3', () => {
    const r = railRoutingFromSummary(
      summary(
        { resolved_from: 'bootstrap' },
        {
          alternatives: [
            'acme/widget-flash-4.7',
            'acme/gizmo-pro-2',
            'acme/gizmo-pro-3',
            'acme/zeta-1',
            'acme/omega-9',
          ],
        },
      ),
    );
    expect(r).toEqual({
      model: 'acme/widget-flash (offline)',
      reason: 'lowest cost that fits the mode',
      state: 'confirmed',
      alternatives: ['acme/gizmo-pro', 'acme/zeta', 'acme/omega'],
    });
  });

  it('catalog: alternatives stay concrete ids', () => {
    const r = railRoutingFromSummary(
      summary({ resolved_from: 'catalog' }, { alternatives: ['acme/gizmo-pro-2', 'acme/zeta-1'] }),
    );
    expect(r?.model).toBe('acme/widget-flash-4.8');
    expect(r?.alternatives).toEqual(['acme/gizmo-pro-2', 'acme/zeta-1']);
  });

  it('a blank reason or state becomes null', () => {
    const r = railRoutingFromSummary(summary({ resolved_from: 'bootstrap', reason: '  ' }, { discovery_state: '' }));
    expect(r?.reason).toBeNull();
    expect(r?.state).toBeNull();
  });
});

describe('modelStateHint', () => {
  it('explains the discovery states the server sends (ModelConfidence)', () => {
    expect(modelStateHint('confirmed')).toMatch(/eligible for routing/);
    expect(modelStateHint('Provisional')).toMatch(/not yet measured/);
    expect(modelStateHint('shadowed')).toMatch(/not routed yet/);
    expect(modelStateHint('deprecated')).toMatch(/retired/);
  });

  it('returns null for anything else, including words the server never sends and inherited keys', () => {
    expect(modelStateHint('exploit')).toBeNull();
    expect(modelStateHint('constructor')).toBeNull();
    expect(modelStateHint(null)).toBeNull();
  });
});
```

Create `crates/vox-gui/ui/src/config/budget.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { formatSpend } from './budget';

describe('formatSpend', () => {
  it('shows a positive cap', () => {
    expect(formatSpend(12.34, 50)).toBe('$12.34 / $50.00');
  });

  it('never renders a zero, negative or missing cap', () => {
    expect(formatSpend(12.34, 0)).toBe('$12.34');
    expect(formatSpend(12.34, -1)).toBe('$12.34');
    expect(formatSpend(12.34, null)).toBe('$12.34');
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/modelFamily.test.ts src/lib/routingSummary.test.ts src/config/budget.test.ts > target/3a-t1-red.txt 2>&1; tail -30 target/3a-t1-red.txt`
Expected: FAIL — `modelFamily.test.ts` and `routingSummary.test.ts` cannot resolve `./modelFamily` / `./routingSummary`;
`budget.test.ts` fails with `formatSpend is not a function` (or an import error). If any of the three files reports
`passed`, STOP.

- [ ] **Step 3: Implement.** Create `crates/vox-gui/ui/src/lib/modelFamily.ts`:

```ts
/**
 * Version-free model family keys: a line-by-line mirror of the Rust
 * `vox_orchestrator::models::family::family_key` (model-routing plan, Task 2).
 * The GUI shows a family instead of a version whenever a model id was not
 * read from the live OpenRouter catalog.
 */

/** Release-stage words that do not start a new family. */
const QUALIFIERS = new Set(['preview', 'beta', 'exp', 'experimental', 'latest']);

/** `8b`, `70b`, `a22b`: a parameter size is a different cost point, so it stays in the key. */
function isSizeToken(tok: string): boolean {
  if (!tok.endsWith('b')) return false;
  const body = tok.slice(0, -1).replace(/^a+/, '');
  return /^[0-9.]+$/.test(body) && /[0-9]/.test(body);
}

/** `org/name-words[:free]`, lowercased, with version numbers, `v4` markers and stage words dropped. */
export function familyKey(slug: string): string {
  const lower = slug.toLowerCase();
  const colon = lower.indexOf(':');
  const base = colon >= 0 ? lower.slice(0, colon) : lower;
  const variant = colon >= 0 ? lower.slice(colon + 1) : '';
  const slash = base.indexOf('/');
  const org = slash >= 0 ? base.slice(0, slash) : '';
  const name = slash >= 0 ? base.slice(slash + 1) : base;
  const words: string[] = [];
  for (const tok of name.split('-')) {
    if (tok === '' || QUALIFIERS.has(tok)) continue;
    if (!/[0-9]/.test(tok) || isSizeToken(tok)) {
      words.push(tok);
      continue;
    }
    if (/^[0-9.]+$/.test(tok) || /^v[0-9.]+$/.test(tok)) continue;
    const letters = tok.replace(/[^a-z]/g, '');
    if (letters) words.push(letters);
  }
  let key = org ? `${org}/${words.join('-')}` : words.join('-');
  if (variant === 'free') key += ':free';
  return key;
}

/**
 * A versioned cloud model id (Claude, GPT, Gemini, o-series, DeepSeek, Llama, Qwen, Kimi, GLM, Grok release
 * numbers). The GUI never hardcodes one; `src/__tests__/noVersionedModelIds.test.ts` enforces it.
 */
export const VERSIONED_CLOUD_MODEL_ID =
  /(opus|sonnet|haiku|fable)-\d|gpt-\d|gemini-\d|\bo\d-mini|deepseek-v\d|llama-?\d|qwen\d|kimi-k\d|glm-\d|grok-\d/i;
```

Create `crates/vox-gui/ui/src/lib/routingSummary.ts`:

```ts
import type { RoutingSummary } from '../types/tauri';
import type { TurnEventDto } from '../types/dashboard';
import { familyKey } from './modelFamily';
import { routingModelLabel } from './turnEvents';

/**
 * The global routing pick as a label, through the trace plan's `routingModelLabel` (one rule for the turn trace,
 * the status bar and the rail). Null without a decision. A daemon that predates `family` / `resolved_from` is
 * treated as bootstrap, so a version it sends is never shown.
 */
function summaryModelLabel(summary: RoutingSummary | null | undefined): string | null {
  const selected = summary?.decision_preview?.selected_model?.trim();
  if (!summary || !selected) return null;
  const event: TurnEventDto = {
    kind: 'routing_decision',
    family: summary.family?.trim() || familyKey(selected),
    resolved_id: selected,
    resolved_from: summary.resolved_from ?? 'bootstrap',
  };
  return routingModelLabel(event);
}

/** Status-bar Routing card value: `Auto → <label>`, or `Auto` when the engine reported no decision. */
export function routingCardValue(summary: RoutingSummary | null | undefined): string {
  const label = summaryModelLabel(summary);
  return label ? `Auto → ${label}` : 'Auto';
}

/** The chat rail's Routing section (the engine's pick for the next turn). */
export interface RailRouting {
  /** `routingModelLabel` of the pick. */
  model: string;
  reason: string | null;
  /** `decision_preview.discovery_state` exactly as the server sent it. */
  state: string | null;
  /** At most 3: catalog ids when catalog-resolved, else family keys (deduped, the chosen family excluded). */
  alternatives: string[];
}

export function railRoutingFromSummary(summary: RoutingSummary | null | undefined): RailRouting | null {
  const model = summaryModelLabel(summary);
  const preview = summary?.decision_preview;
  if (!summary || !preview || !model) return null;
  const fromCatalog = summary.resolved_from === 'catalog';
  const chosen = fromCatalog ? preview.selected_model : summary.family?.trim() || familyKey(preview.selected_model);
  const alternatives: string[] = [];
  for (const alt of preview.alternatives ?? []) {
    const shown = fromCatalog ? alt : familyKey(alt);
    if (shown && shown !== chosen && !alternatives.includes(shown)) alternatives.push(shown);
    if (alternatives.length === 3) break;
  }
  return {
    model,
    reason: summary.reason?.trim() || null,
    state: preview.discovery_state?.trim() || null,
    alternatives,
  };
}

/** Tooltip text for `decision_preview.discovery_state` (the server sends `ModelConfidence` values). */
const MODEL_STATE_HINTS = new Map([
  ['confirmed', 'Confirmed: measured and eligible for routing.'],
  ['provisional', 'Provisional: newly discovered, not yet measured.'],
  ['shadowed', 'Shadowed: evaluated in the background, not routed yet.'],
  ['deprecated', 'Deprecated: being retired from routing.'],
]);

export function modelStateHint(state: string | null | undefined): string | null {
  return state ? MODEL_STATE_HINTS.get(state.toLowerCase()) ?? null : null;
}
```

Append to `crates/vox-gui/ui/src/config/budget.ts` (after `formatBudgetCap`):

```ts

/** `$12.34 / $50.00` when a positive cap is known, else `$12.34` — never `/ $0`. */
export function formatSpend(spent: number, cap: number | null): string {
  const s = `$${spent.toFixed(2)}`;
  return cap != null && cap > 0 ? `${s} / $${cap.toFixed(2)}` : s;
}
```


- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/modelFamily.test.ts src/lib/routingSummary.test.ts src/config/budget.test.ts 2>&1 | tail -15 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -15`
Expected: all tests pass; typecheck exits 0.

- [ ] **Step 5: Mutation proofs** (restore after each; `git diff --stat` must show only this task's files).
  1. In `routingSummary.ts` replace `resolved_from: summary.resolved_from ?? 'bootstrap',` with
     `resolved_from: summary.resolved_from ?? 'catalog',`. Run the Step 2 command into `target/3a-t1-mutant-a.txt`:
     `a daemon without family/resolved_from is treated as offline…` must FAIL. Restore.
  2. In `routingSummary.ts` replace `if (!summary || !selected) return null;` with `if (!summary) return null;`. Run into
     `target/3a-t1-mutant-b.txt`: `reads plain "Auto" with no summary, no decision, or a blank pick…` must FAIL (a
     family alone must not produce a label). Restore.
  3. In `modelFamily.ts` replace `if (!/[0-9]/.test(tok) || isSizeToken(tok)) {` with `if (!/[0-9]/.test(tok)) {`. Run
     into `target/3a-t1-mutant-c.txt`: `keeps parameter sizes and :free…` must FAIL. Restore.
  4. In `budget.ts` replace `cap > 0` with `cap >= 0`. Run into `target/3a-t1-mutant-d.txt`: `never renders a zero,
     negative or missing cap` must FAIL. Restore.

- [ ] **Step 6: Commit (Claude Code)**

```bash
cd /Users/brbrainerd/dev/vox
F="crates/vox-gui/ui/src/lib/modelFamily.ts crates/vox-gui/ui/src/lib/modelFamily.test.ts crates/vox-gui/ui/src/lib/routingSummary.ts crates/vox-gui/ui/src/lib/routingSummary.test.ts crates/vox-gui/ui/src/config/budget.ts crates/vox-gui/ui/src/config/budget.test.ts"
git add -- ${=F}
git commit -m "feat(gui): family keys, routing-summary adapter and spend format" -- ${=F}
```

---

### Task 2: Status bar cards — Engine, Spend, Mesh, Routing, Needs you

<!-- AMENDED: T2 — `HudTilesEditor.test.tsx` uses the retired `queue_depth` tile: named and edited here, and the Step 9
command covers `src/components/surfaces/Settings`; card values are truncated with the full text in `title` (catalog ids
and reasons are long); `routingCardValue` now comes from `lib/routingSummary.ts`. -->

**Files:**
- Modify: `crates/vox-gui/ui/src/hooks/useHudTiles.ts`, `crates/vox-gui/ui/src/hooks/useHudTiles.test.ts`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Settings/HudTilesEditor.test.tsx` (retired tile id only)
- Modify: `contracts/gui/hud-tiles.v1.yaml`
- Modify: `crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx`, `crates/vox-gui/ui/src/components/layout/BottomStatusBar.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/layout/AppShell.tsx`, `crates/vox-gui/ui/src/App.tsx`
- Modify: `crates/vox-gui/ui/src/config/budget.ts` (delete `formatBudgetCap`)

**Interfaces:**
- Consumes (Task 1): `formatSpend`, `routingCardValue`; `RoutingSummary`; `voxTransport.getRoutingSummaryLive()`;
  `attention.totalCount` (already passed to AppShell as `needsYouCount`); App's `sessionSpentUsd`.
- Produces:
  - `HUD_TILE_KINDS = ['active_agents','budget_burn','mesh_peers','active_model','pending_approvals']` (ids unchanged; two retired)
  - `HUD_TILE_LABELS` = Engine / Spend / Mesh / Routing / Needs you; `export const RETIRED_HUD_TILE_IDS: ReadonlySet<string>`
  - `BottomStatusBarProps`: removes `activeModel`, `pendingApprovals`; adds `routingSummary?: RoutingSummary | null`,
    `sessionSpentUsd?: number | null`, `needsYouCount?: number | null`
  - test ids: `bottom-status-bar-engine`, `-spend`, `-spend-popover`, `-mesh`, `-routing`, `-needs-you`, `-freshness`;
    dialog `Spend detail`; card label spans carry `data-card-label`; card value spans carry `data-card-value`, are
    truncated (`truncate`, max 28ch) and hold the full value in `title`.
  - `AppShellProps`: removes `activeModel`, `pendingApprovals`; adds `routingSummary?`, `sessionSpentUsd?`.

- [ ] **Step 0: Preconditions.** `cd /Users/brbrainerd/dev/vox && rg -n "export function formatSpend|export function routingCardValue" crates/vox-gui/ui/src/config/budget.ts crates/vox-gui/ui/src/lib/routingSummary.ts`
  (2 hits); `rg -n "queue_depth" crates/vox-gui/ui/src/components/surfaces/Settings/HudTilesEditor.test.tsx` (2 hits:
  `HUD_TILE_LABELS.queue_depth` and `id: 'queue_depth'`). (Other `queue_depth` hits in `Dashboard*`, `dashboardLayout.ts`
  and `Mesh*` are unrelated dashboard-widget / mesh fields — leave them.) `rg -n "formatBudgetCap" crates/vox-gui/ui/src` (only `config/budget.ts` and `BottomStatusBar.tsx`);
  `rg -n "pendingApprovals=\{attention.approvals.length\}|activeModel=\{activeModel\}|const meshNodes = useMeshNodes\(20_000\);" crates/vox-gui/ui/src/App.tsx`
  (3 hits, one each). Any mismatch: STOP.

- [ ] **Step 1: Write the failing tests.**

In `crates/vox-gui/ui/src/hooks/useHudTiles.test.ts` make exactly these edits to existing tests, then append the new
describe:
  - In `is part of the HUD tile SSOT with a label`: `expect(HUD_TILE_LABELS.pending_approvals).toBe('Pending approvals');` → `expect(HUD_TILE_LABELS.pending_approvals).toBe('Needs you');`
  - Replace the whole test `defaultHudTiles() returns all 7 kinds in order` with:

```ts
  it('defaultHudTiles() returns the 5 card kinds in order', () => {
    const config = defaultHudTiles();
    expect(config.tiles.map((t) => t.kind)).toEqual([
      'active_agents',
      'budget_burn',
      'mesh_peers',
      'active_model',
      'pending_approvals',
    ]);
  });
```

  - In `toggleHudTile updates enabled flag for matching id`: both `'queue_depth'` literals → `'mesh_peers'`.
  - In `reorderHudTile moves tile from fromIndex to toIndex`: replace the expected array with
    `['budget_burn', 'mesh_peers', 'active_agents', 'active_model', 'pending_approvals']`.

Append:

```ts
describe('retired HUD tiles (merged into the Engine and Spend cards)', () => {
  it('drops queue_depth and openrouter_spend from a stored config and keeps the other choices', () => {
    const cfg = validateHudTilesConfig({
      version: 1,
      tiles: [
        { id: 'active_agents', kind: 'active_agents', enabled: false },
        { id: 'queue_depth', kind: 'queue_depth', enabled: true },
        { id: 'openrouter_spend', kind: 'openrouter_spend', enabled: false },
        { id: 'mesh_peers', kind: 'mesh_peers', enabled: true },
      ],
    });
    expect(cfg.tiles).toEqual([
      { id: 'active_agents', kind: 'active_agents', enabled: false },
      { id: 'mesh_peers', kind: 'mesh_peers', enabled: true },
    ]);
  });

  it('still rejects an id that was never a tile', () => {
    expect(() =>
      validateHudTilesConfig({ version: 1, tiles: [{ id: 'queue_depthx', kind: 'active_agents', enabled: true }] }),
    ).toThrow(/unknown tile id/i);
  });

  it('labels are the card names', () => {
    expect(HUD_TILE_LABELS).toEqual({
      active_agents: 'Engine',
      budget_burn: 'Spend',
      mesh_peers: 'Mesh',
      active_model: 'Routing',
      pending_approvals: 'Needs you',
    });
  });
});
```

In `crates/vox-gui/ui/src/components/surfaces/Settings/HudTilesEditor.test.tsx` make exactly these edits (the tile it
toggles is retired; `mesh_peers` stays):
  - title `lists all 7 tile kinds from HUD_TILE_KINDS` → `lists every tile kind from HUD_TILE_KINDS`;
  - `    const checkbox = screen.getByLabelText(HUD_TILE_LABELS.queue_depth) as HTMLInputElement;` →
    `    const checkbox = screen.getByLabelText(HUD_TILE_LABELS.mesh_peers) as HTMLInputElement;`;
  - `          expect.objectContaining({ id: 'queue_depth', enabled: false }),` →
    `          expect.objectContaining({ id: 'mesh_peers', enabled: false }),`.

In `crates/vox-gui/ui/src/components/layout/BottomStatusBar.test.tsx` make exactly these edits to existing tests:
  - In `renders every enabled tile as a compact one-line segment`: `expect(screen.getByText('Agents')).toBeInTheDocument();` → `expect(screen.getByText('Engine')).toBeInTheDocument();`
  - In `clicking the agents segment navigates to the agents view`: `fireEvent.click(screen.getByText('Agents').closest('button')!);` → `fireEvent.click(screen.getByText('Engine').closest('button')!);`
  - In `the configure trigger opens a live-apply checkbox menu that stays open across toggles`: `{ name: /mesh peers/i }` → `{ name: /^mesh$/i }` and `{ name: /budget burn/i }` → `{ name: /^spend$/i }`.
  - Replace the whole test `mesh segment falls back to a bare peer count, still worded "online", when meshNodes is not supplied` with:

```tsx
  it('mesh card shows "—" until the mesh node list arrives (one source: useMeshNodes)', () => {
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={defaultHudTiles()}
        onHudTilesChange={vi.fn()}
        onNavigate={vi.fn()}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
      />,
    );
    expect(screen.getByTestId('bottom-status-bar-mesh').textContent).toBe('Mesh—');
  });
```

Add these two import lines directly below the file's existing imports:

```tsx
import { within } from '@testing-library/react';
import type { ComponentProps } from 'react';
```

Append at the end of the file:

```tsx
const cardKpis = (budget: { value: number; cap: number; source: 'daemon' | 'fallback' }) => ({
  ...INITIAL_KPIS,
  activeAgents: { ...INITIAL_KPIS.activeAgents, value: 9 },
  queueDepth: { ...INITIAL_KPIS.queueDepth, value: 44 },
  budgetBurn: { ...INITIAL_KPIS.budgetBurn, ...budget },
});

const summary = (over: Record<string, unknown> = {}, selected = 'acme/widget-flash-4.8') => ({
  active_model: null,
  exploration_spent_usd: 0,
  exploration_budget_usd: 1,
  routing_priority: { efficiency: 50, precision: 50, latency: 50, availability: 50, balance: 50, mobile: 50 },
  arm_count: 1,
  model_count: 1,
  decision_preview: {
    selected_model: selected,
    discovery_state: 'confirmed',
    alternatives: [],
    rejection_reasons: [],
    intelligence_score: 0.5,
    efficiency_score: 0.5,
    latency_score: 0.5,
  },
  family: 'acme/widget-flash',
  reason: 'lowest cost that fits the mode',
  ...over,
});

function renderBar(over: Partial<ComponentProps<typeof BottomStatusBar>> = {}) {
  return render(
    <BottomStatusBar
      kpis={cardKpis({ value: 12.34, cap: 50, source: 'daemon' })}
      hudTilesConfig={defaultHudTiles()}
      onHudTilesChange={vi.fn()}
      onNavigate={vi.fn()}
      lastOrchEventAt={null}
      orchUsesPolling={false}
      liveFreshMs={10_000}
      {...over}
    />,
  );
}

describe('BottomStatusBar cards (chat-surfaces plan 3a)', () => {
  it('Engine reads "9 agents · 44 queued" and opens Agents', () => {
    const onNavigate = vi.fn();
    renderBar({ onNavigate });
    const card = screen.getByTestId('bottom-status-bar-engine');
    expect(card).toHaveTextContent('Engine9 agents · 44 queued');
    fireEvent.click(card);
    expect(onNavigate).toHaveBeenCalledWith('agents');
  });

  it('Engine says "1 agent", not "1 agents"', () => {
    renderBar({
      kpis: { ...cardKpis({ value: 0, cap: 50, source: 'daemon' }), activeAgents: { ...INITIAL_KPIS.activeAgents, value: 1 } },
    });
    expect(screen.getByTestId('bottom-status-bar-engine')).toHaveTextContent('1 agent · 44 queued');
  });

  it('Spend shows a daemon cap as "$12.34 / $50.00"', () => {
    renderBar();
    expect(screen.getByTestId('bottom-status-bar-spend')).toHaveTextContent('Spend$12.34 / $50.00');
  });

  it('Spend never renders a zero cap or the fallback placeholder cap', () => {
    const { unmount } = renderBar({ kpis: cardKpis({ value: 12.34, cap: 0, source: 'daemon' }) });
    expect(screen.getByTestId('bottom-status-bar-spend').textContent).toBe('Spend$12.34');
    unmount();
    renderBar({ kpis: cardKpis({ value: 12.34, cap: 50, source: 'fallback' }) });
    expect(screen.getByTestId('bottom-status-bar-spend').textContent).toBe('Spend$12.34');
  });

  it('the Spend popover splits engine total, OpenRouter, this session and local models', () => {
    renderBar({ openrouterSpendUsd: 1.5, sessionSpentUsd: 0.25 });
    fireEvent.click(screen.getByTestId('bottom-status-bar-spend'));
    const dialog = screen.getByRole('dialog', { name: 'Spend detail' });
    const row = (label: string) => within(dialog).getByText(label).nextElementSibling?.textContent;
    expect(row('Engine total')).toBe('$12.34 / $50.00');
    expect(row('OpenRouter')).toBe('$1.50');
    expect(row('This session')).toBe('$0.25');
    expect(row('Local models')).toBe('not metered');
    expect(screen.getByTestId('bottom-status-bar-spend')).toHaveAttribute('aria-expanded', 'true');
  });

  it('an unknown OpenRouter or session figure reads "unknown", never $0.00', () => {
    renderBar({ openrouterSpendUsd: null, sessionSpentUsd: null });
    fireEvent.click(screen.getByTestId('bottom-status-bar-spend'));
    const dialog = screen.getByRole('dialog', { name: 'Spend detail' });
    expect(within(dialog).getByText('OpenRouter').nextElementSibling?.textContent).toBe('unknown');
    expect(within(dialog).getByText('This session').nextElementSibling?.textContent).toBe('unknown');
  });

  it('Escape closes the Spend popover and returns focus to the Spend card', () => {
    renderBar();
    const card = screen.getByTestId('bottom-status-bar-spend');
    fireEvent.click(card);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Spend detail' })).toBeNull();
    expect(document.activeElement).toBe(card);
  });

  it('Routing shows the family and "(offline)" when the pick was not read from the catalog', () => {
    renderBar({ routingSummary: summary({ resolved_from: 'bootstrap' }) });
    const card = screen.getByTestId('bottom-status-bar-routing');
    expect(card).toHaveTextContent('RoutingAuto → acme/widget-flash (offline)');
    expect(card.textContent).not.toContain('4.8');
  });

  it('Routing shows a concrete version only when resolved_from is "catalog"', () => {
    renderBar({ routingSummary: summary({ resolved_from: 'catalog' }) });
    expect(screen.getByTestId('bottom-status-bar-routing')).toHaveTextContent('Auto → acme/widget-flash-4.8');
  });

  it('a long routing value is truncated in the bar and kept whole in its title', () => {
    const longId = `acme/${'widget-'.repeat(12)}flash-20260901`;
    renderBar({ routingSummary: summary({ resolved_from: 'catalog' }, longId) });
    const value = screen.getByTestId('bottom-status-bar-routing').querySelector('[data-card-value]')!;
    expect(value.className).toContain('truncate');
    expect(value).toHaveAttribute('title', `Auto → ${longId}`);
  });

  it('Routing reads "Auto" before any summary arrives', () => {
    renderBar();
    expect(screen.getByTestId('bottom-status-bar-routing').textContent).toBe('RoutingAuto');
  });

  it('Needs you counts approvals plus questions and opens Needs you', () => {
    const onNavigate = vi.fn();
    renderBar({ needsYouCount: 3, onNavigate });
    const card = screen.getByTestId('bottom-status-bar-needs-you');
    expect(card).toHaveTextContent('Needs you3');
    fireEvent.click(card);
    expect(onNavigate).toHaveBeenCalledWith('needs-you');
  });

  it('no retired segment renders', () => {
    renderBar({ openrouterSpendUsd: 1.5 });
    for (const id of ['agents', 'queue', 'budget', 'model', 'openrouter', 'approvals']) {
      expect(screen.queryByTestId(`bottom-status-bar-${id}`)).toBeNull();
    }
  });

  it('every Configure checkbox label equals the label on the card it toggles', () => {
    renderBar();
    const cardLabels = Array.from(document.querySelectorAll('[data-card-label]')).map((el) => el.textContent);
    fireEvent.click(screen.getByRole('button', { name: /configure status bar/i }));
    const menuLabels = screen.getAllByRole('checkbox').map((cb) => cb.closest('label')?.textContent?.trim());
    expect(cardLabels).toEqual(['Engine', 'Spend', 'Mesh', 'Routing', 'Needs you']);
    expect(menuLabels).toEqual(cardLabels);
  });

  it('the freshness pill explains Polling in its tooltip', () => {
    renderBar({ orchUsesPolling: true });
    expect(screen.getByTestId('bottom-status-bar-freshness')).toHaveAttribute(
      'title',
      expect.stringMatching(/^Polling/),
    );
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/hooks/useHudTiles.test.ts src/components/layout/BottomStatusBar.test.tsx > target/3a-t2-red.txt 2>&1; tail -40 target/3a-t2-red.txt`
Expected: FAIL — `drops queue_depth and openrouter_spend…` (4 tiles returned), `labels are the card names`, every
test in `BottomStatusBar cards (chat-surfaces plan 3a)` (no such test ids / labels today; the `agents` segment still
exists), and the edited `renders every enabled tile…` (`Engine` not found). `still rejects an id that was never a
tile` passes before and after (regression). If any other new test passes, STOP.

- [ ] **Step 3: Implement `useHudTiles.ts`.** Replace everything from the line `export const HUD_TILE_KINDS = [`
through the closing `};` of `HUD_TILE_LABELS` (this span includes the existing `export type HudTileKind = …` line) with:

```ts
export const HUD_TILE_KINDS = [
  'active_agents',
  'budget_burn',
  'mesh_peers',
  'active_model',
  'pending_approvals',
] as const;

export type HudTileKind = (typeof HUD_TILE_KINDS)[number];

/** Card names — the Configure menu shows exactly these, so menu and cards cannot drift. */
export const HUD_TILE_LABELS: Record<HudTileKind, string> = {
  active_agents: 'Engine',
  budget_burn: 'Spend',
  mesh_peers: 'Mesh',
  active_model: 'Routing',
  pending_approvals: 'Needs you',
};

/**
 * Tile ids retired 2026-09-28: queue depth merged into the Engine card, OpenRouter spend into the Spend
 * card's popover. A stored config that still lists them loads with those entries dropped (other choices kept).
 */
export const RETIRED_HUD_TILE_IDS: ReadonlySet<string> = new Set(['queue_depth', 'openrouter_spend']);
```

(`rg -c "export type HudTileKind" src/hooks/useHudTiles.ts` must print `1` afterwards.) In `validateHudTilesConfig`, replace

```ts
  const tiles: HudTileEntry[] = raw.tiles.map((entry, index) => {
```

with

```ts
  const tiles: HudTileEntry[] = raw.tiles
    .filter((entry) => !(isRecord(entry) && typeof entry.id === 'string' && RETIRED_HUD_TILE_IDS.has(entry.id)))
    .map((entry, index) => {
```

- [ ] **Step 4: Edit `contracts/gui/hud-tiles.v1.yaml`** (targeted edits; the file stays otherwise byte-identical):
  - under `tile_kinds:` delete the lines `  - queue_depth` and `  - openrouter_spend`;
  - under `data_bindings:` delete the 3-line `queue_depth:` block and the 3-line `openrouter_spend:` block;
  - under `default_profile:` → `tiles:` delete the 3-line `- id: queue_depth` entry and the 3-line `- id: openrouter_spend` entry;
  - insert directly after the (now 5-item) `tile_kinds:` list, before the blank line and `data_bindings:`:

```yaml

# Retired 2026-09-28 (chat-surfaces consolidation): queue_depth merged into the
# Engine card (active_agents), openrouter_spend into the Spend card popover
# (budget_burn). Stored configs that list them load with those entries dropped.
retired_tile_kinds:
  - queue_depth
  - openrouter_spend
```

- [ ] **Step 5: Implement `BottomStatusBar.tsx`** with these anchored replacements.

(a) Replace the import block (from `import React, { useEffect, useRef, useState } from 'react';` through
`import { StatusBarCluster } from '../common/StatusBarCluster';`) with:

```tsx
import React, { useEffect, useRef, useState } from 'react';
import { Glass } from '../ui/Glass';
import { Icon } from '../ui/Icons';
import { formatSpend } from '../../config/budget';
import { useFreshness } from '../../hooks/useFreshness';
import {
  resolveVisibleHudTiles,
  toggleHudTile,
  HUD_TILE_LABELS,
  type HudTilesConfig,
  type HudTileKind,
} from '../../hooks/useHudTiles';
import { INITIAL_KPIS } from '../../data/initialState';
import { WORKBENCH_TABBAR_TRAILING_SLOT_ID } from '../../lib/domIds';
import { routingCardValue } from '../../lib/routingSummary';
import type { RoutingSummary } from '../../types/tauri';
import type { MeshNode } from '../surfaces/Mesh/MeshView';
import { StatusBarCluster } from '../common/StatusBarCluster';
```

(b) Replace the whole `export interface BottomStatusBarProps { … }` with:

```tsx
export interface BottomStatusBarProps {
  kpis: KpiState;
  hudTilesConfig: HudTilesConfig;
  onHudTilesChange: (config: HudTilesConfig) => void;
  onNavigate: (view: string) => void;
  lastOrchEventAt: number | null;
  orchUsesPolling: boolean;
  liveFreshMs: number;
  /** Global routing pick (get_routing_summary_live); a version is shown only when catalog-resolved. */
  routingSummary?: RoutingSummary | null;
  openrouterSpendUsd?: number | null;
  /** This chat session's spend (get_llm_spend sessionUsd), shown in the Spend popover. */
  sessionSpentUsd?: number | null;
  /** Approvals plus open questions (attention inbox `totalCount`). */
  needsYouCount?: number | null;
  meshNodes?: MeshNode[];
  gamifyEnabled?: boolean;
  onOpenAchievements?: () => void;
  onOpenResearchDrawer?: () => void;
}
```

(c) Replace the whole `function freshnessClasses(…) { … }` with:

```tsx
function freshnessClasses(tone: 'live' | 'poll' | 'stale') {
  if (tone === 'live') {
    return {
      pill: 'border-emerald-400/20 bg-emerald-400/4 text-emerald-300',
      dot: 'bg-emerald-400',
      label: 'Live',
      title: 'Live: receiving engine events',
    };
  }
  if (tone === 'poll') {
    return {
      pill: 'border-amber-400/20 bg-amber-400/4 text-amber-300',
      dot: 'bg-amber-400',
      label: 'Poll',
      title: 'Polling: no event stream, refreshing on a timer',
    };
  }
  return {
    pill: 'border-border-subtle bg-overlay-subtle text-text-muted',
    dot: 'bg-text-muted',
    label: 'Offline',
    title: 'Offline: no engine data recently',
  };
}
```

(d) Replace the whole `function Segment({ … }) { … }` with:

```tsx
function Segment({
  testId,
  label,
  value,
  onClick,
  expanded,
  buttonRef,
}: {
  testId: string;
  label: string;
  value: string;
  onClick: () => void;
  /** Set only on a card that opens a popover. */
  expanded?: boolean;
  buttonRef?: React.Ref<HTMLButtonElement>;
}) {
  return (
    <button
      ref={buttonRef}
      type="button"
      data-testid={testId}
      onClick={onClick}
      aria-haspopup={expanded === undefined ? undefined : 'dialog'}
      aria-expanded={expanded}
      className="inline-flex items-center gap-1.5 rounded-sm px-2 py-0.5 text-[10px] text-text-muted hover:bg-overlay-subtle hover:text-text-secondary transition"
    >
      <span data-card-label className="uppercase tracking-[0.14em] text-text-muted">{label}</span>
      <span
        data-card-value
        title={value}
        className="inline-block max-w-[28ch] truncate align-bottom font-mono tabular-nums text-text-secondary"
      >
        {value}
      </span>
    </button>
  );
}
```

(e) Replace everything from the line `export function BottomStatusBar({` up to (not including) the line
`      {/* Fixed home for surface-level chrome that needs to sit inline with` with:

```tsx
export function BottomStatusBar({
  kpis,
  hudTilesConfig,
  onHudTilesChange,
  onNavigate,
  lastOrchEventAt,
  orchUsesPolling,
  liveFreshMs,
  routingSummary = null,
  openrouterSpendUsd = null,
  sessionSpentUsd = null,
  needsYouCount = null,
  meshNodes,
  gamifyEnabled = false,
  onOpenAchievements,
  onOpenResearchDrawer,
}: BottomStatusBarProps) {
  const tone = useFreshness(lastOrchEventAt, {
    freshMs: liveFreshMs,
    usesPolling: orchUsesPolling,
  });
  const fresh = freshnessClasses(tone);
  const visible = resolveVisibleHudTiles(hudTilesConfig);

  // One popover at a time: the Configure menu or the Spend card's detail.
  const [openPanel, setOpenPanel] = useState<'configure' | 'spend' | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const spendTriggerRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (openPanel === null) return;
    const activeTrigger = openPanel === 'spend' ? spendTriggerRef : triggerRef;
    const onOutside = (e: MouseEvent) => {
      const target = e.target as Node;
      if (panelRef.current?.contains(target)) return;
      if (activeTrigger.current?.contains(target)) return;
      setOpenPanel(null);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpenPanel(null);
        activeTrigger.current?.focus();
      }
    };
    document.addEventListener('mousedown', onOutside);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onOutside);
      document.removeEventListener('keydown', onKey);
    };
  }, [openPanel]);

  const budget = kpis.budgetBurn;
  // A cap only when the daemon reported a positive one: never `/ $0`, never the fallback placeholder.
  const spendValue = formatSpend(budget.value, budget.source === 'daemon' ? budget.cap : null);
  const usd = (v: number | null) => (v == null || Number.isNaN(v) ? 'unknown' : `$${v.toFixed(2)}`);
  const agentsN = kpis.activeAgents.value;
  const engineValue = `${agentsN} ${agentsN === 1 ? 'agent' : 'agents'} · ${kpis.queueDepth.value} queued`;
  // Mesh has one source (vox_mesh_nodes via useMeshNodes); until it answers, show a dash, not a second count.
  const meshValue =
    meshNodes == null
      ? '—'
      : `${meshNodes.filter((n) => n.status === 'online').length}/${meshNodes.length} online`;

  const renderSegment = (kind: HudTileKind): React.ReactNode => {
    const label = HUD_TILE_LABELS[kind];
    switch (kind) {
      case 'active_agents':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-engine"
            label={label}
            value={engineValue}
            onClick={() => onNavigate('agents')}
          />
        );
      case 'budget_burn':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-spend"
            label={label}
            value={spendValue}
            expanded={openPanel === 'spend'}
            buttonRef={spendTriggerRef}
            onClick={() => setOpenPanel((p) => (p === 'spend' ? null : 'spend'))}
          />
        );
      case 'mesh_peers':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-mesh"
            label={label}
            value={meshValue}
            onClick={() => onNavigate('mesh')}
          />
        );
      case 'active_model':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-routing"
            label={label}
            value={routingCardValue(routingSummary)}
            onClick={() => onNavigate('models')}
          />
        );
      case 'pending_approvals':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-needs-you"
            label={label}
            value={String(needsYouCount ?? 0)}
            onClick={() => onNavigate('needs-you')}
          />
        );
      default:
        return null;
    }
  };

  return (
    <Glass
      data-testid="bottom-status-bar"
      role="status"
      aria-label="Operator status"
      className="flex h-7 w-full items-center gap-1 p-0 px-3 rounded-none border-x-0 border-b-0 shadow-none text-[10px] text-text-muted"
    >
      <div className="relative flex min-w-0 flex-1">
        <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
          {visible.map((kind) => renderSegment(kind))}
        </div>
        {openPanel === 'spend' ? (
          <div
            ref={panelRef}
            role="dialog"
            aria-label="Spend detail"
            data-testid="bottom-status-bar-spend-popover"
            className="absolute bottom-full left-0 z-50 mb-1 w-64 rounded-lg border border-border-subtle bg-bg-base p-3 shadow-2xl"
          >
            <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-[11px]">
              <dt className="text-text-muted">Engine total</dt>
              <dd className="font-mono tabular-nums text-text-secondary">{spendValue}</dd>
              <dt className="text-text-muted">OpenRouter</dt>
              <dd className="font-mono tabular-nums text-text-secondary">{usd(openrouterSpendUsd)}</dd>
              <dt className="text-text-muted">This session</dt>
              <dd className="font-mono tabular-nums text-text-secondary">{usd(sessionSpentUsd)}</dd>
              <dt className="text-text-muted">Local models</dt>
              <dd className="font-mono tabular-nums text-text-secondary">not metered</dd>
            </dl>
            <button
              type="button"
              onClick={() => {
                setOpenPanel(null);
                onNavigate('settings');
              }}
              className="mt-2 w-full rounded-sm border border-border-subtle px-2 py-1 text-[11px] text-text-secondary hover:bg-overlay-subtle"
            >
              Budget settings
            </button>
          </div>
        ) : null}
      </div>
      {gamifyEnabled && onOpenAchievements && (
        <button
          type="button"
          data-testid="achievements-trigger"
          aria-label="Open achievements"
          onClick={onOpenAchievements}
          className="inline-flex shrink-0 items-center justify-center rounded-sm px-1.5 py-0.5 text-amber-300/80 hover:bg-overlay-subtle hover:text-amber-200 transition"
        >
          <Icon.trophy className="size-3.5" aria-hidden="true" />
        </button>
      )}
      <StatusBarCluster onOpenDrawer={onOpenResearchDrawer} />
      <div className="relative shrink-0">
        <button
          ref={triggerRef}
          type="button"
          onClick={() => setOpenPanel((p) => (p === 'configure' ? null : 'configure'))}
          aria-expanded={openPanel === 'configure'}
          aria-label="Configure status bar"
          className="rounded-sm px-1.5 py-0.5 text-[10px] text-text-muted hover:bg-overlay-subtle hover:text-text-secondary transition"
        >
          Configure ▾
        </button>
        {openPanel === 'configure' ? (
          <div
            ref={panelRef}
            className="absolute bottom-full right-0 z-50 mb-1 w-56 rounded-lg border border-border-subtle bg-bg-base p-2 shadow-2xl"
          >
            {hudTilesConfig.tiles.map((tile) => (
              <label
                key={tile.id}
                className="flex items-center gap-2 rounded-sm px-2 py-1 text-[11px] text-text-secondary hover:bg-overlay-subtle"
              >
                <input
                  type="checkbox"
                  checked={tile.enabled}
                  onChange={(e) =>
                    onHudTilesChange(toggleHudTile(hudTilesConfig, tile.id, e.target.checked))
                  }
                  className="rounded-sm border-border-subtle bg-bg-base text-brass focus:ring-brass/40 focus:ring-offset-bg-base size-3.5"
                />
                {HUD_TILE_LABELS[tile.kind]}
              </label>
            ))}
          </div>
        ) : null}
      </div>
      <div
        data-testid="bottom-status-bar-freshness"
        title={fresh.title}
        className={`ml-auto inline-flex shrink-0 items-center gap-1.5 rounded-sm border px-2 py-0.5 ${fresh.pill}`}
      >
        <span className={`size-1.5 rounded-full ${fresh.dot}`} />
        <span className="uppercase tracking-[0.14em]">{fresh.label}</span>
      </div>

```

- [ ] **Step 6: Delete `formatBudgetCap`** from `crates/vox-gui/ui/src/config/budget.ts` (the doc comment line
  `/** Format cap for display; shows em-dash when unknown. */` and the whole function). `rg -n formatBudgetCap
  crates/vox-gui/ui/src` must then print nothing.

- [ ] **Step 7: Wire AppShell.** In `crates/vox-gui/ui/src/components/layout/AppShell.tsx`:
  - `import type { Toast } from '../../types/tauri';` → `import type { RoutingSummary, Toast } from '../../types/tauri';`
  - in `AppShellProps`, delete the lines `  pendingApprovals: number;` and `  activeModel?: string | null;`, and add directly
    after `  openrouterSpendUsd?: number | null;`:

```tsx
  /** Global routing pick for the status bar's Routing card. */
  routingSummary?: RoutingSummary | null;
  /** This chat session's spend, for the Spend popover. */
  sessionSpentUsd?: number | null;
```

  - in the component's destructuring, delete `  pendingApprovals,` and `  activeModel,`, and add `  routingSummary,` and
    `  sessionSpentUsd,` directly after `  openrouterSpendUsd,`;
  - in the `<BottomStatusBar` JSX: `        activeModel={activeModel}` → `        routingSummary={routingSummary}`;
    `        pendingApprovals={pendingApprovals}` → `        needsYouCount={needsYouCount}`; and add
    `        sessionSpentUsd={sessionSpentUsd}` directly after `        openrouterSpendUsd={openrouterSpendUsd}`.

- [ ] **Step 8: Wire App.** In `crates/vox-gui/ui/src/App.tsx`:
  - directly after the line `import { voxTransport, listenAgentEvents, chatTurn as sendChatTurnRaw, type AgentEventFrame } from './transport';` add
    `import { useQuery } from '@tanstack/react-query';`
  - directly after `  const meshNodes = useMeshNodes(20_000);` add:

```tsx
  // Global routing pick for the status bar's Routing card (same 20 s cadence as the mesh card).
  const routingSummaryQuery = useQuery({
    queryKey: ['routing-summary-live'],
    queryFn: () => voxTransport.getRoutingSummaryLive(),
    refetchInterval: 20_000,
  });
```

  - in the `<AppShell` JSX: delete the line `        pendingApprovals={attention.approvals.length}`; replace
    `        activeModel={activeModel}` with `        routingSummary={routingSummaryQuery.data ?? null}`; add
    `        sessionSpentUsd={sessionSpentUsd}` directly after `        openrouterSpendUsd={openrouterSpendUsd}`.
    (`needsYouCount={attention.totalCount}` is already passed.)

- [ ] **Step 9: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/hooks/useHudTiles.test.ts src/components/layout src/components/surfaces/Settings src/config src/App.test.tsx 2>&1 | tail -25 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -15`
Expected: all pass; typecheck exits 0. `git diff --numstat -- contracts/` shows one file with under 20 changed lines.

- [ ] **Step 10: Mutation proofs** (restore after each; `git diff` must show only this task's edits).
  1. In `useHudTiles.ts` delete the `.filter((entry) => …RETIRED_HUD_TILE_IDS…)` line. Run the Step 2 command into
     `target/3a-t2-mutant-a.txt`: `drops queue_depth and openrouter_spend…` must FAIL (it throws `unknown tile id`). Restore.
  2. In `BottomStatusBar.tsx` replace `budget.source === 'daemon' ? budget.cap : null` with `budget.cap`. Run into
     `target/3a-t2-mutant-b.txt`: `Spend never renders a zero cap or the fallback placeholder cap` must FAIL. Restore.

- [ ] **Step 11: Commit (Claude Code)**

```bash
cd /Users/brbrainerd/dev/vox
F="crates/vox-gui/ui/src/hooks/useHudTiles.ts crates/vox-gui/ui/src/hooks/useHudTiles.test.ts crates/vox-gui/ui/src/components/surfaces/Settings/HudTilesEditor.test.tsx contracts/gui/hud-tiles.v1.yaml crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx crates/vox-gui/ui/src/components/layout/BottomStatusBar.test.tsx crates/vox-gui/ui/src/components/layout/AppShell.tsx crates/vox-gui/ui/src/App.tsx crates/vox-gui/ui/src/config/budget.ts"
git add -- ${=F}
git commit -m "feat(gui): status bar cards Engine, Spend, Mesh, Routing, Needs you" -- ${=F}
```

---

### Task 3: The chat rail and its hook — Routing (next turn), session scope only, live context meter

<!-- AMENDED: T3 — split into Task 3 (rail component + hook, vitest only) and Task 4 (plumbing, typecheck). Anchored
micro-edits only: Phase 5 plan 05-05 adds synthetic "Waiting for resource lock" rows and `LockWaiting` handling inside
the hook's `refresh` body, so no function body or file tail is replaced. The hook no longer fetches routing at all (App
owns the one `get_routing_summary_live` query, Task 2/4). The section carries a "next turn" qualifier because it shows
the engine's global preview, not a turn's decision (the trace plan shows those). New rail tests pass `kpis` so RED fails
on behaviour, not on a crash of the old component. -->

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx`, `…/Chat/ChatExecutionRail.test.tsx`
- Modify: `crates/vox-gui/ui/src/hooks/useChatExecutionData.ts`, `crates/vox-gui/ui/src/hooks/useChatExecutionData.test.ts`

**Interfaces:**
- Consumes (Task 1): `RailRouting`, `modelStateHint` from `lib/routingSummary.ts`; `ChatMessage` (`role`, `status`).
- Produces:
  - `ChatExecutionRailProps = { tasks; routing?: RailRouting | null; sessionSpentUsd?; sessionId?; onOpenRouting?; turnsCompleted?: number }`
    (removed: `kpis`, `intents`, `activeModel`, `openrouterSpendUsd`, `onNavigate`, `agents`, `selectedAgentId`, `onOpenAgent`;
    type `ChatExecutionRailKpis` deleted)
  - `export function countCompletedTurns(messages: ChatMessage[]): number`
  - rail DOM: region `Routing` with a `next turn` qualifier (`execution-rail-routing-scope`), button
    `execution-rail-routing` reading `Routes to MODEL` plus ` — REASON` when a reason exists (truncated, full text in
    `title`), a closed `<details>` "Why this model" holding `execution-rail-routing-state` (title = `modelStateHint`) and
    `Alternatives: …`; region `Session spend`.
  - `ChatExecutionData = { tasks: ChatExecutionTask[] }` — `intentsFromRoutingSummary`, `intents` and `meshPeers` deleted;
    the hook calls neither `getRoutingSummaryLive` nor `getOrchestratorStatusBin`.
- Typecheck is expected to FAIL after this task until Task 4 lands (App/ChatSurface still pass removed props); Claude
  commits Tasks 3 and 4 together.

- [ ] **Step 0: Preconditions.** From `crates/vox-gui/ui`:
  - `rg -n "execution-rail-lock-chip" src/components/surfaces/Chat/ChatExecutionRail.tsx` (Phase 5 chip, 1+ hit);
  - `rg -n "Waiting for resource lock" src/hooks/useChatExecutionData.ts` (05-05 landed, 1 hit);
  - `rg -n "export function modelStateHint|export interface RailRouting" src/lib/routingSummary.ts` (2 hits);
  - each of these must print exactly one hit in `src/hooks/useChatExecutionData.ts`:
    `rg -n -F "const [rows, summary, statusBin, activityRows] = await Promise.all(["`,
    `rg -n -F "voxTransport.getRoutingSummaryLive(),"`,
    `rg -n -F "voxTransport.getOrchestratorStatusBin().catch(() => null),"`,
    `rg -n -F "setIntents(intentsFromRoutingSummary(summary));"`,
    `rg -n -F "setMeshPeers(meshPeersFromStatusBin(statusBin));"`,
    `rg -n -F "return { tasks, intents, meshPeers };"`.
  Any mismatch: `DRIVE: STOPPED step 0: <which anchor>`.

- [ ] **Step 1: Write the failing tests.**

In `ChatExecutionRail.test.tsx` delete these whole existing tests (by title; they assert removed UI):
`shows resource strip with agents, queue depth, and mesh peers from props`; `shows OpenRouter cost segment when
openrouterSpendUsd is provided`; `hides OpenRouter segment when openrouterSpendUsd is omitted`; `shows current model
label when activeModel is provided`; `navigates when resource segments are clicked`; `renders intent map section with
up to three intent lines and opens the Routing drawer`; `omits intent map when intents prop is empty`; `lists live
agents and opens topology from the roster`; `renders Agents and Queue as compact segments, not KPI cards with metric
rules`. Keep every other test unchanged. Add below the existing imports:

```tsx
import { fireEvent, within } from '@testing-library/react';
import { countCompletedTurns } from './ChatExecutionRail';
```

Append at the end of the file (every render passes the old `kpis`/`onNavigate` props, so the RED run exercises the old
component's behaviour instead of crashing on a missing prop):

```tsx
describe('ChatExecutionRail — this session only (plan 3a)', () => {
  const routing = {
    model: 'deepseek/deepseek-flash (offline)',
    reason: 'lowest cost that fits the mode',
    state: 'confirmed',
    alternatives: ['anthropic/claude-haiku', 'google/gemini-flash'],
  };
  const old = { kpis: sampleKpis, onNavigate: vi.fn() };

  it('shows a Routing section for the next turn and never the word Intents', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={routing} {...old} />
      </LanguageProvider>,
    );
    const region = screen.getByRole('region', { name: 'Routing' });
    expect(within(region).getByTestId('execution-rail-routing-scope')).toHaveTextContent('next turn');
    expect(within(region).getByTestId('execution-rail-routing').textContent).toBe(
      'Routes to deepseek/deepseek-flash (offline) — lowest cost that fits the mode',
    );
    expect(screen.queryByText(/intents/i)).toBeNull();
  });

  it('truncates the routing line and keeps the full text in its title', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={routing} {...old} />
      </LanguageProvider>,
    );
    const line = screen.getByTestId('execution-rail-routing');
    expect(line.className).toContain('truncate');
    expect(line).toHaveAttribute('title', line.textContent);
  });

  it('omits the dash when the server sent no reason', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={{ ...routing, reason: null }} {...old} />
      </LanguageProvider>,
    );
    expect(screen.getByTestId('execution-rail-routing').textContent).toBe('Routes to deepseek/deepseek-flash (offline)');
  });

  it('keeps alternatives and the model state behind a closed disclosure, with an explaining tooltip', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={routing} {...old} />
      </LanguageProvider>,
    );
    const details = screen.getByText('Why this model').closest('details')!;
    expect(details).not.toHaveAttribute('open');
    expect(details).toHaveTextContent('Alternatives: anthropic/claude-haiku, google/gemini-flash');
    const state = screen.getByTestId('execution-rail-routing-state');
    expect(details.contains(state)).toBe(true);
    expect(state).toHaveAttribute('title', expect.stringContaining('eligible for routing'));
  });

  it('clicking the routing line opens the Routing panel', () => {
    const onOpenRouting = vi.fn();
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} routing={routing} onOpenRouting={onOpenRouting} {...old} />
      </LanguageProvider>,
    );
    fireEvent.click(screen.getByTestId('execution-rail-routing'));
    expect(onOpenRouting).toHaveBeenCalledTimes(1);
  });

  it('renders no Agents roster and no global Resources block even when handed their old inputs', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[]}
          agents={[{ id: 'a1', codename: 'Aquila', task: 'Refactor', phase: 'Idle' }]}
          activeModel="deepseek/deepseek-flash"
          openrouterSpendUsd={3.2}
          {...old}
        />
      </LanguageProvider>,
    );
    expect(screen.queryByRole('region', { name: /agent shards/i })).toBeNull();
    expect(screen.queryByLabelText('Resource strip')).toBeNull();
    for (const id of ['agents', 'queue', 'mesh', 'model', 'openrouter']) {
      expect(screen.queryByTestId(`execution-rail-${id}`)).toBeNull();
    }
  });

  it('keeps the one session-scoped number: Session spend', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} sessionId="sess-a" sessionSpentUsd={0.25} {...old} />
      </LanguageProvider>,
    );
    expect(screen.getByRole('region', { name: 'Session spend' })).toHaveTextContent('$0.25');
  });

  it('refreshes the context meter once per completed turn, not once per session', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation((cmd: string) =>
      cmd === 'get_context_budget' ? Promise.resolve(mockBudget) : Promise.resolve(null),
    );
    vi.mocked(invoke).mockClear();
    const budgetCalls = () => vi.mocked(invoke).mock.calls.filter(([c]) => c === 'get_context_budget').length;
    const ui = (turns: number) => (
      <LanguageProvider>
        <ChatExecutionRail tasks={[]} sessionId="sess-a" turnsCompleted={turns} {...old} />
      </LanguageProvider>
    );
    const { rerender } = render(ui(0));
    await waitFor(() => expect(budgetCalls()).toBe(1));
    rerender(ui(1));
    await waitFor(() => expect(budgetCalls()).toBe(2));
    rerender(ui(1));
    rerender(ui(2));
    await waitFor(() => expect(budgetCalls()).toBe(3));
    expect(screen.getByRole('meter')).toBeInTheDocument();
  });

  it('Phase 5 lock chips still render under their task beside the Routing section (regression)', () => {
    render(
      <LanguageProvider>
        <ChatExecutionRail
          tasks={[
            {
              id: 't1',
              title: 'Migrate orders table',
              status: 'running',
              lock: { resourceId: 'db://orders/42', state: 'holding' },
            },
          ]}
          routing={routing}
          {...old}
        />
      </LanguageProvider>,
    );
    const tasksRegion = screen.getByRole('region', { name: /active tasks/i });
    expect(within(tasksRegion).getByTestId('execution-rail-lock-chip')).toHaveTextContent('holding db://orders/42');
    expect(screen.getByRole('region', { name: 'Routing' })).toBeInTheDocument();
  });
});

describe('countCompletedTurns', () => {
  const m = (role: 'user' | 'assistant' | 'system', status: 'pending' | 'streaming' | 'done' | 'failed', i: number) => ({
    id: `m${i}`,
    role,
    text: '',
    status,
    runId: 'r',
  });

  it('counts finished assistant replies only (done or failed)', () => {
    expect(
      countCompletedTurns([
        m('user', 'done', 1),
        m('assistant', 'done', 2),
        m('assistant', 'failed', 3),
        m('assistant', 'streaming', 4),
        m('assistant', 'pending', 5),
        m('system', 'done', 6),
      ]),
    ).toBe(2);
    expect(countCompletedTurns([])).toBe(0);
  });
});
```

In `useChatExecutionData.test.ts` (edit only these lines):
  - in the import list delete the line `  intentsFromRoutingSummary,`;
  - in the `routingSummary` helper: `  active_model: 'claude-sonnet-4',` → `  active_model: 'deepseek/deepseek-flash',`;
  - delete the whole `describe('intentsFromRoutingSummary', () => { … });` block;
  - in the test `loads tasks and intents for the active session`: change its title to
    `loads tasks for the active session; routing and mesh are not this hook's job`; replace the two lines
    `    expect(result.current.intents).toEqual(['mens-v1 · explore']);` and `    expect(result.current.meshPeers).toBe(2);`
    with `    expect(Object.keys(result.current)).toEqual(['tasks']);`; replace
    `    expect(mockGetRoutingSummaryLive).toHaveBeenCalled();` with
    `    expect(mockGetRoutingSummaryLive).not.toHaveBeenCalled();` and add directly after it
    `    expect(mockGetOrchestratorStatusBin).not.toHaveBeenCalled();`;
  - in the test whose title starts `polls every` delete the line `    expect(mockGetRoutingSummaryLive).toHaveBeenCalledTimes(2);`.

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/ChatExecutionRail.test.tsx src/hooks/useChatExecutionData.test.ts > target/3a-t3-red.txt 2>&1; tail -40 target/3a-t3-red.txt`
Expected FAIL, each for its behaviour: `countCompletedTurns` is not a function (the `countCompletedTurns` describe);
no `Routing` region / no `execution-rail-routing` (the four routing tests and the lock-chip regression test, which fails
only on its last assertion); the roster and Resources render (`renders no Agents roster…`); no `Session spend` region;
one context fetch only (`refreshes the context meter…`, stuck at 1); and the edited hook test (keys are
`tasks,intents,meshPeers`; routing was fetched). If one of these passes, STOP.

- [ ] **Step 3: Implement `ChatExecutionRail.tsx`** (anchored micro-edits; the tasks section and `SessionSpendTrack`
stay as they are).

(a) Delete the lines `import { Pill } from '../../ui/Pill';` and `import type { Agent } from '../../../types/dashboard';`.
Directly after `import { getContextBudget, type ContextBudgetPayload } from '../../../transport';` add:

```tsx
import type { ChatMessage } from '../../../lib/chatCorrelation';
import { modelStateHint, type RailRouting } from '../../../lib/routingSummary';
```

(b) Replace the whole `export interface ChatExecutionRailKpis { … }` and the whole
`export interface ChatExecutionRailProps { … }` with:

```tsx
export interface ChatExecutionRailProps {
  tasks: ChatExecutionTask[];
  /** The engine's routing pick for the next turn (App's one routing query); the section is hidden when null. */
  routing?: RailRouting | null;
  /** This session's spend — the one money figure the rail owns (global spend lives in the status bar). */
  sessionSpentUsd?: number | null;
  /** Active chat session id — passed to get_context_budget so the meter shows real token usage. */
  sessionId?: string | null;
  /** Opens the inline Routing panel (folded Matrix surface — gui-ia-blueprint: matrix → chat rail). */
  onOpenRouting?: () => void;
  /** Completed assistant turns in this session (`countCompletedTurns`); each change re-reads the context budget. */
  turnsCompleted?: number;
}

/** Finished assistant replies (done or failed): the rail re-reads the context budget when this grows. */
export function countCompletedTurns(messages: ChatMessage[]): number {
  return messages.filter((m) => m.role === 'assistant' && (m.status === 'done' || m.status === 'failed')).length;
}
```

(c) Replace the destructuring from the line `export function ChatExecutionRail({` through the line
`}: ChatExecutionRailProps) {` with:

```tsx
export function ChatExecutionRail({
  tasks,
  routing = null,
  sessionSpentUsd,
  sessionId,
  onOpenRouting,
  turnsCompleted = 0,
}: ChatExecutionRailProps) {
```

(d) Replace this exact effect (directly below `const [budget, setBudget] = useState<ContextBudgetPayload | null>(null);`)

```tsx
  useEffect(() => {
    let cancelled = false;
    setBudget(null);
    getContextBudget(sessionId)
      .then((next) => {
        if (!cancelled) setBudget(next);
      })
      .catch(() => {
        if (!cancelled) setBudget(null);
      });
    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  const peerLabel = kpis.mesh.peers === 1 ? '1 peer' : `${kpis.mesh.peers} peers`;
```

with

```tsx
  const budgetSessionRef = useRef(sessionId);

  // Re-read the context budget on every session change and every completed turn (read once per session, it went stale).
  useEffect(() => {
    let cancelled = false;
    if (budgetSessionRef.current !== sessionId) {
      budgetSessionRef.current = sessionId;
      setBudget(null); // a new session never shows the previous session's reading
    }
    getContextBudget(sessionId)
      .then((next) => {
        if (!cancelled) setBudget(next);
      })
      .catch(() => {
        if (!cancelled) setBudget(null);
      });
    return () => {
      cancelled = true;
    };
  }, [sessionId, turnsCompleted]);

  const stateHint = modelStateHint(routing?.state);
  const routingLine = routing
    ? `Routes to ${routing.model}${routing.reason ? ` — ${routing.reason}` : ''}`
    : '';
```

(e) Replace the whole Intent-map block — from the line `        {intents != null && intents.length > 0 && (` through
its closing `        )}` (the block contains `aria-label="Intent map"`) — with:

```tsx
        {routing && (
          <section role="region" aria-label="Routing" className="flex flex-col gap-1 pt-3">
            <div className="mb-1.5 flex items-baseline justify-between gap-2 border-b border-border-subtle pb-1">
              <span className="font-display text-[9px] uppercase tracking-[0.28em] text-text-muted">Routing</span>
              <span
                data-testid="execution-rail-routing-scope"
                title="The engine's current pick for the next turn; each reply's own routing is in its trace"
                className="text-[9px] text-text-muted"
              >
                next turn
              </span>
            </div>
            <button
              type="button"
              data-testid="execution-rail-routing"
              title={routingLine}
              onClick={() => onOpenRouting?.()}
              className="truncate rounded-sm px-2 py-1 text-left text-[11px] text-text-secondary transition hover:bg-overlay-subtle hover:text-brass"
            >
              {routingLine}
            </button>
            {(routing.state || routing.alternatives.length > 0) && (
              <details className="px-2 text-[10px] text-text-muted">
                <summary className="cursor-pointer select-none">Why this model</summary>
                {routing.state && (
                  <p className="mt-1">
                    State:{' '}
                    <span
                      data-testid="execution-rail-routing-state"
                      title={stateHint ?? undefined}
                      className="underline decoration-dotted"
                    >
                      {routing.state}
                    </span>
                  </p>
                )}
                {routing.alternatives.length > 0 && (
                  <p className="mt-0.5">Alternatives: {routing.alternatives.join(', ')}</p>
                )}
              </details>
            )}
          </section>
        )}
```

(f) Delete the whole Agents-roster block — from the line `        {agents.length > 0 && (` through its closing
`        )}` (the block contains `aria-label="Agent shards"`).

(g) Replace the whole Resources block — from the line `        <section aria-label="Resource strip" className="flex flex-col gap-1 pt-3">`
through its closing `        </section>` — with:

```tsx
        {sessionSpentUsd != null && !Number.isNaN(sessionSpentUsd) && (
          <section aria-label="Session spend" className="flex flex-col gap-1 pt-3">
            <SessionSpendTrack
              key={sessionId ?? 'none'}
              sessionId={sessionId}
              sessionSpentUsd={sessionSpentUsd}
            />
          </section>
        )}
```

Afterwards `rg -n "kpis|intents|agents|peerLabel|Pill|activeModel|openrouterSpendUsd" src/components/surfaces/Chat/ChatExecutionRail.tsx`
must print nothing (run from `crates/vox-gui/ui`).

- [ ] **Step 4: Implement `useChatExecutionData.ts`** (anchored micro-edits only; leave every other line — including
05-05's waiting-lock code — untouched):
  1. delete the line `import { decode } from '@msgpack/msgpack';`;
  2. delete the line `import type { OrchestratorStatus, RoutingSummary } from '../types/tauri';`;
  3. delete the whole function `export function intentsFromRoutingSummary(summary: RoutingSummary | null): string[] { … }`
     and the whole function `function meshPeersFromStatusBin(statusBin: Uint8Array | null): number { … }`;
  4. in `export interface ChatExecutionData`, delete the lines `  intents: string[];` and `  meshPeers: number;`;
  5. delete the lines `  const [intents, setIntents] = useState<string[]>([]);` and `  const [meshPeers, setMeshPeers] = useState(0);`;
  6. delete every line whose trimmed text is exactly `setIntents([]);` or `setMeshPeers(0);` (4 lines: two in the
     no-session branch, two in the `catch`);
  7. replace `        const [rows, summary, statusBin, activityRows] = await Promise.all([` with
     `        const [rows, activityRows] = await Promise.all([`, and delete the lines
     `          voxTransport.getRoutingSummaryLive(),` and `          voxTransport.getOrchestratorStatusBin().catch(() => null),`;
  8. delete the lines `        setIntents(intentsFromRoutingSummary(summary));` and `        setMeshPeers(meshPeersFromStatusBin(statusBin));`;
  9. replace `  return { tasks, intents, meshPeers };` with `  return { tasks };`.
  Afterwards `rg -n "intents|meshPeers|statusBin|getRoutingSummaryLive|decode\(" src/hooks/useChatExecutionData.ts` prints
  nothing, and `git diff --numstat -- src/hooks/useChatExecutionData.ts` shows only deletions plus 2 changed lines.

- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/ChatExecutionRail.test.tsx src/hooks/useChatExecutionData.test.ts 2>&1 | tail -25`
Expected: all pass, including the kept context-meter, spend-spark, lock-chip and 05-05 waiting-lock tests. (Typecheck
is not run here: App/ChatSurface still pass the removed props until Task 4.)

- [ ] **Step 6: Mutation proof.** In `ChatExecutionRail.tsx` change `}, [sessionId, turnsCompleted]);` to
  `}, [sessionId]);`. Run the Step 2 command into `target/3a-t3-mutant.txt`: `refreshes the context meter once per
  completed turn…` must FAIL. Restore; `git diff --stat` lists only this task's four files.

- [ ] **Step 7: Commit (Claude Code)** — together with Task 4 (typecheck is red in between).

---

### Task 4: Rail plumbing — App computes `chatRouting` from the one routing query

<!-- AMENDED: T3 — new task (plumbing half of the old Task 3); routing comes from App's `routingSummaryQuery`. -->

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`, `…/Chat/ChatSurface.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/layout/surfaceComponents.tsx`
- Modify: `crates/vox-gui/ui/src/App.tsx`

**Interfaces:**
- Consumes: Task 3's rail props and `countCompletedTurns`; Task 2's `routingSummaryQuery` in App; Task 1's
  `railRoutingFromSummary`.
- Produces: `ChatSurface` prop `routing?: RailRouting | null` replaces `intents`; `SurfaceProps.chatRouting` replaces
  `chatIntents`; App `const chatRouting = useMemo(() => railRoutingFromSummary(routingSummaryQuery.data ?? null), …)`.

- [ ] **Step 0: Preconditions.** From `crates/vox-gui/ui`: `rg -n "export function countCompletedTurns" src/components/surfaces/Chat/ChatExecutionRail.tsx` (1);
  `rg -n "routingSummaryQuery" src/App.tsx` (Task 2, 2+ hits); `rg -n "intents: chatIntents|chatExecutionKpis|chatActiveModel: activeModel|onOpenAgentInFlow: \(agentId" src/App.tsx` (4+ hits);
  `rg -n -F "it('has exactly one accessible h1 for the surface root (axe page-has-heading-one)'" src/components/surfaces/Chat/ChatSurface.test.tsx` (1). Any miss: STOP.

- [ ] **Step 1: Write the failing test.** In `ChatSurface.test.tsx`, insert directly above the line
  `  it('has exactly one accessible h1 for the surface root (axe page-has-heading-one)', async () => {` (inside
  `describe('ChatSurface', …)`, so its `beforeEach` mock applies):

```tsx
  it('hands the rail its routing and re-reads the context budget when a turn completes (plan 3a)', async () => {
    const base = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((cmd: string, ...rest: unknown[]) =>
      cmd === 'get_context_budget'
        ? Promise.resolve({
            max_context_tokens: 1000,
            reserved_tokens: 0,
            threshold_tokens: 800,
            usable_tokens: 1000,
            strategy: 'balanced',
            used_tokens: 10,
          })
        : base(cmd, ...rest),
    );
    const reply = (status: 'streaming' | 'done'): ChatMessage => ({
      id: 'a1',
      role: 'assistant',
      text: 'x',
      status,
      runId: 'r1',
    });
    const routing = { model: 'deepseek/deepseek-flash (offline)', reason: null, state: null, alternatives: [] };
    const ui = (messages: ChatMessage[]) => (
      <LanguageProvider>
        <ChatSurface
          pushToast={noopToast}
          onNavigate={vi.fn()}
          messages={messages}
          composer={<div>composer</div>}
          activeSessionId="sess-a"
          routing={routing}
        />
      </LanguageProvider>
    );
    const budgetCalls = () => invokeMock.mock.calls.filter(([c]) => c === 'get_context_budget').length;
    const { rerender } = render(ui([reply('streaming')]));
    expect(await screen.findByTestId('execution-rail-routing')).toHaveTextContent(
      'Routes to deepseek/deepseek-flash (offline)',
    );
    await waitFor(() => expect(budgetCalls()).toBeGreaterThan(0));
    const before = budgetCalls();
    rerender(ui([reply('done')]));
    await waitFor(() => expect(budgetCalls()).toBe(before + 1));
  });
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/ChatSurface.test.tsx > target/3a-t4-red.txt 2>&1; tail -25 target/3a-t4-red.txt`
Expected: the new test FAILS (ChatSurface does not pass `routing` to the rail, so `execution-rail-routing` never
appears). Every other ChatSurface test passes. If the new test passes, STOP.

- [ ] **Step 3: Plumb ChatSurface.** In `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`:
  - replace the import `import {\n  ChatExecutionRail,\n  type ChatExecutionRailKpis,\n  type ChatExecutionTask,\n} from './ChatExecutionRail';`
    with `import { ChatExecutionRail, countCompletedTurns, type ChatExecutionTask } from './ChatExecutionRail';` and add
    `import type { RailRouting } from '../../../lib/routingSummary';` directly after it;
  - delete the line `import type { Agent } from '../../../types/dashboard';` (its only use is the `flowAgents` prop);
  - in `interface ChatSurfaceProps`: `  intents?: string[];` → `  routing?: RailRouting | null;`; delete the lines
    `  executionKpis?: ChatExecutionRailKpis;`, `  activeModel?: string | null;`, `  openrouterSpendUsd?: number | null;`,
    `  onOpenAgentInFlow?: (agentId: string) => void;`, the 5-line doc comment whose middle line starts
    `   * Same agent-graph data that feeds Agents → Flow` (with its `/**` and `*/` lines), `  flowAgents?: Agent[];`,
    `  flowSelectedAgentId?: string;`, `  onFlowSelectAgent?: (id: string) => void;`;
  - in the `export function ChatSurface({` destructuring: `  intents,` → `  routing,`; delete `  executionKpis,`,
    `  activeModel,`, `  openrouterSpendUsd,`, `  onOpenAgentInFlow,`, `  flowAgents = [],`, `  flowSelectedAgentId,`,
    `  onFlowSelectAgent,`;
  - replace the whole `const railKpis = executionKpis ?? { … };` statement and the whole
    `const executionRailNode = onNavigate ? ( … ) : null;` statement with:

```tsx
  const executionRailNode = onNavigate ? (
    <ChatExecutionRail
      tasks={tasks}
      routing={routing}
      sessionSpentUsd={sessionSpentUsd}
      sessionId={activeSessionId}
      onOpenRouting={() => setRoutingOpen(true)}
      turnsCompleted={countCompletedTurns(messages)}
    />
  ) : null;
```

- [ ] **Step 4: Plumb surfaceComponents.** In `crates/vox-gui/ui/src/components/layout/surfaceComponents.tsx`:
  - in `import type {\n  ChatExecutionRailKpis,\n  ChatExecutionTask,\n} from '../surfaces/Chat/ChatExecutionRail';` delete
    the `  ChatExecutionRailKpis,` line, and add `import type { RailRouting } from '../../lib/routingSummary';` after that import;
  - in `SurfaceProps`: `  chatIntents?: string[];` → `  chatRouting?: RailRouting | null;`; delete
    `  chatExecutionKpis?: ChatExecutionRailKpis;`, `  chatActiveModel?: string | null;`,
    `  chatOpenrouterSpendUsd?: number | null;`, `  onOpenAgentInFlow?: (agentId: string) => void;`;
  - in the `<ChatSurface` JSX: `          intents={props.chatIntents}` → `          routing={props.chatRouting}`; delete the
    lines `executionKpis={props.chatExecutionKpis}`, `activeModel={props.chatActiveModel}`,
    `openrouterSpendUsd={props.chatOpenrouterSpendUsd}`, `onOpenAgentInFlow={props.onOpenAgentInFlow}`,
    `flowAgents={props.data.agents}`, `flowSelectedAgentId={props.selectedAgentId}`,
    `onFlowSelectAgent={props.setSelectedAgentId}`.

- [ ] **Step 5: Plumb App.** In `crates/vox-gui/ui/src/App.tsx`:
  - replace

```tsx
  const {
    tasks: chatTasks,
    intents: chatIntents,
    meshPeers: chatMeshPeers,
  } = useChatExecutionData(activeSessionId);
```

    with

```tsx
  const { tasks: chatTasks } = useChatExecutionData(activeSessionId);
```

  - directly after the `const routingSummaryQuery = useQuery({ … });` statement (Task 2) add:

```tsx
  // The rail's Routing section reads the same one query as the status bar's Routing card.
  const chatRouting = useMemo(
    () => railRoutingFromSummary(routingSummaryQuery.data ?? null),
    [routingSummaryQuery.data],
  );
```

    and add `import { railRoutingFromSummary } from './lib/routingSummary';` directly after the line
    `import { useQuery } from '@tanstack/react-query';`;
  - delete the whole `const chatExecutionKpis = useMemo( … );` statement (it ends `[kpis, chatMeshPeers],\n  );`);
  - delete the whole `const activeModel = useMemo(() => { … }, [orchQuery.data]);` statement (Task 2 removed its other use);
  - in the `surfaceProps` object: `    chatIntents,` → `    chatRouting,`; delete `    chatExecutionKpis,`,
    `    chatActiveModel: activeModel,`, `    chatOpenrouterSpendUsd: openrouterSpendUsd,` and the 4-line entry
    starting `    onOpenAgentInFlow: (agentId: string) => {`.
  - `rg -n "chatIntents|chatExecutionKpis|chatMeshPeers|chatActiveModel|activeModel\b" src/App.tsx` must print nothing.

- [ ] **Step 6: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat src/hooks src/components/layout src/App.test.tsx 2>&1 | tail -25 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -15`
Expected: all pass; typecheck exits 0.

- [ ] **Step 7: Mutation proof.** In `ChatSurface.tsx` replace `      turnsCompleted={countCompletedTurns(messages)}` with
  `      turnsCompleted={0}`. Run the Step 2 command into `target/3a-t4-mutant.txt`: the new test must FAIL (no refetch
  on turn completion). Restore; `git diff --stat` lists only Tasks 3–4 files.

- [ ] **Step 8: Commit (Claude Code)** — Tasks 3 and 4 in one commit.

```bash
cd /Users/brbrainerd/dev/vox
F="crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.test.tsx crates/vox-gui/ui/src/hooks/useChatExecutionData.ts crates/vox-gui/ui/src/hooks/useChatExecutionData.test.ts crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx crates/vox-gui/ui/src/components/layout/surfaceComponents.tsx crates/vox-gui/ui/src/App.tsx"
git add -- ${=F}
git commit -m "feat(gui): chat rail shows this session and the next-turn routing pick" -- ${=F}
```

---

### Task 5: Composer — full mode names, Spend label, `Risk: …`, Check replies inside Risk

<!-- AMENDED: T4→T5 — renumbered; mode labels derive from the trace plan's `MODE_NAMES` (one source); `RiskPopover`
copy says "check replies" instead of the retired "grounding"; the clutch buttons' native `title` is dropped on purpose
(the visible, focus-reachable hint replaces it). -->

**Files:**
- Modify: `crates/vox-gui/ui/src/lib/driveConsole.ts`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Loquela/DriveConsole.tsx`, `…/Loquela/DriveConsole.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Loquela/RiskPopover.tsx`, `…/Loquela/RiskPopover.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/GroundingCheckToggle.tsx`, `…/Chat/GroundingCheckToggle.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`, `…/Loquela/Loquela.test.tsx`
- Modify: `crates/vox-gui/ui/src/App.tsx`, `crates/vox-gui/ui/src/App.test.tsx`

**Interfaces:**
- Consumes (Task 1): `formatSpend`; (trace plan) `MODE_NAMES: Readonly<Record<"free" | "efficiency" | "balanced" | "genius", string>>` and `modeLabel(wire: string): string` in `src/lib/turnEvents.ts`.
- Produces: `CLUTCH_DETENTS` labels taken from `MODE_NAMES` — `Free | Efficient | Balanced | Genius` (ids unchanged); `DriveConsole` prop
  `riskExtra?: React.ReactNode`, test ids `drive-mode-hint`, `drive-console-spend`; `RiskPopover` prop
  `children?: React.ReactNode`; `Loquela` prop `riskSlot?: React.ReactNode`; `GroundingCheckToggle` accessible name
  `Check replies on|off`, text `Check replies: on|off`.

- [ ] **Step 0: Preconditions.** From `crates/vox-gui/ui`: `rg -n "label: 'Effic.'|label: 'Bal.'" src/lib/driveConsole.ts` (2);
  `rg -n "trailingSlot=\{" src/App.tsx` (1, followed by `<GroundingCheckToggle`); `rg -n "budgetUsd=\{sessionBudget\?\.cap \?\? 0\}" src/components/surfaces/Loquela/Loquela.tsx` (1);
  `rg -n "export function formatSpend" src/config/budget.ts` (1);
  `rg -n "export const MODE_NAMES = Object.freeze" src/lib/turnEvents.ts` (1);
  `rg -n "enforce grounding|grounding, raise approval" src/components/surfaces/Loquela/RiskPopover.tsx` (2). Any miss: STOP.

- [ ] **Step 1: Write the failing tests.**

Append to `DriveConsole.test.tsx`, and add these lines below its existing imports:

```tsx
import { within } from '@testing-library/react';
import { CLUTCH_DETENTS } from '../../../lib/driveConsole';
import { modeLabel } from '../../../lib/turnEvents';
```

Tests to append:

```tsx
describe('DriveConsole vocabulary (plan 3a)', () => {
  const base = {
    control: defaultControl(),
    onControlChange: vi.fn(),
    spentUsd: 0.42,
    budgetUsd: 1.0,
  };

  it('takes every mode name from MODE_NAMES (one source with the turn trace)', () => {
    expect(CLUTCH_DETENTS.map((d) => d.label)).toEqual(CLUTCH_DETENTS.map((d) => modeLabel(d.id)));
  });

  it('names every mode in full inside a "Mode" radiogroup', () => {
    render(<DriveConsole {...base} />);
    expect(screen.getAllByRole('radio').map((r) => r.textContent)).toEqual(['Free', 'Efficient', 'Balanced', 'Genius']);
    expect(screen.getByRole('radiogroup', { name: /^Mode/ })).toBeTruthy();
  });

  it("shows the hovered or focused mode's one-line hint, and hides it after", () => {
    render(<DriveConsole {...base} />);
    expect(screen.queryByTestId('drive-mode-hint')).toBeNull();
    const genius = screen.getByRole('radio', { name: 'Genius' });
    fireEvent.mouseEnter(genius);
    expect(screen.getByTestId('drive-mode-hint')).toHaveTextContent('Most intelligent solutions');
    fireEvent.mouseLeave(genius);
    expect(screen.queryByTestId('drive-mode-hint')).toBeNull();
    const efficient = screen.getByRole('radio', { name: 'Efficient' });
    fireEvent.focus(efficient);
    const hint = screen.getByTestId('drive-mode-hint');
    expect(hint).toHaveTextContent('Most out of the tokens you spend');
    expect(efficient).toHaveAttribute('aria-describedby', hint.id);
    fireEvent.blur(efficient);
    expect(screen.queryByTestId('drive-mode-hint')).toBeNull();
  });

  it('labels the risk trigger "Risk: LEVEL"', () => {
    render(<DriveConsole {...base} />);
    expect(screen.getByRole('button', { name: /risk: moderate/i })).toHaveTextContent('Risk: Moderate');
  });

  it('puts extra risk controls (Check replies) inside the Risk popover, not in the strip', () => {
    render(<DriveConsole {...base} riskExtra={<button type="button">Check replies: off</button>} />);
    expect(screen.queryByText('Check replies: off')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /risk: moderate/i }));
    const dialog = screen.getByRole('dialog', { name: /acceptable risk/i });
    expect(within(dialog).getByText('Check replies: off')).toBeTruthy();
  });

  it('Spend shows the cap only when it is positive', () => {
    const { rerender } = render(<DriveConsole {...base} spentUsd={12.34} budgetUsd={50} />);
    expect(screen.getByTestId('drive-console-spend')).toHaveTextContent('Spend$12.34 / $50.00');
    rerender(<DriveConsole {...base} spentUsd={12.34} budgetUsd={0} />);
    expect(screen.getByTestId('drive-console-spend').textContent).toBe('Spend$12.34');
  });
});
```

Append to `RiskPopover.test.tsx` (add `import { within } from '@testing-library/react';` below its imports):

```tsx
describe('RiskPopover extra controls', () => {
  it('renders children below the postures', () => {
    render(
      <RiskPopover risk="moderate" onChange={() => {}} open onClose={() => {}}>
        <button type="button">Check replies: off</button>
      </RiskPopover>,
    );
    expect(within(screen.getByRole('dialog')).getByRole('button', { name: 'Check replies: off' })).toBeTruthy();
  });

  it('uses the canonical "check replies", never the retired "grounding"', () => {
    for (const risk of ['high', 'moderate', 'low'] as const) {
      const { unmount } = render(<RiskPopover risk={risk} onChange={() => {}} open onClose={() => {}} />);
      expect(screen.getByRole('dialog').textContent).not.toMatch(/grounding/i);
      unmount();
    }
    render(<RiskPopover risk="moderate" onChange={() => {}} open onClose={() => {}} />);
    expect(screen.getByRole('dialog')).toHaveTextContent(/check replies/i);
  });
});
```

In `GroundingCheckToggle.test.tsx` replace, everywhere in the file: `/grounding check off/i` → `/check replies off/i`;
`/grounding check on/i` → `/check replies on/i`; `'grounding: off'` → `'Check replies: off'`; `'grounding: on'` →
`'Check replies: on'`. Nothing else in that file changes.

Append inside `describe('Loquela', …)` in `Loquela.test.tsx` (before its final `});`):

```tsx
  it('shows full mode names and still sends the wire value "efficiency"', () => {
    const onSubmit = vi.fn();
    renderLoquela({ onSubmit });
    fireEvent.click(screen.getByRole('radio', { name: 'Balanced' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Efficient' }));
    const ta = screen.getByLabelText('Task composer');
    fireEvent.change(ta, { target: { value: 'ship it' } });
    fireEvent.keyDown(ta, { key: 'Enter' });
    expect(onSubmit.mock.calls[0][0].clutch).toBe('efficiency');
  });

  it('never shows a spend cap the daemon did not report (fallback source)', () => {
    renderLoquela({ sessionBudget: { spent: 1, cap: 50, source: 'fallback' } });
    expect(screen.getByTestId('drive-console-spend').textContent).toBe('Spend$1.00');
  });

  it('renders riskSlot inside the Risk popover', () => {
    renderLoquela({ riskSlot: <button type="button">Check replies: off</button> });
    expect(screen.queryByText('Check replies: off')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /^risk: /i }));
    expect(screen.getByText('Check replies: off')).toBeTruthy();
  });
```

In `App.test.tsx`, in the test titled `enabling the grounding check toggle forwards grounding_check_enabled=true to
chat_turn`: change the title to `enabling Check replies in the Risk popover forwards grounding_check_enabled=true to
chat_turn`, and replace these four lines

```tsx
    // Accessible name comes from the button's `aria-label` ("Grounding check
    // on/off"), not its visible text ("grounding: on/off") — RTL's `name`
    // matcher matches the accessible name, so the pattern must include "check".
    const groundingToggle = await screen.findByRole('button', { name: /grounding check (on|off)/i });
```

with

```tsx
    // The toggle lives inside the Risk popover as "Check replies" (chat-surfaces plan 3a).
    await user.click(await screen.findByRole('button', { name: /^risk: /i }));
    const groundingToggle = await screen.findByRole('button', { name: /check replies (on|off)/i });
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela src/components/surfaces/Chat/GroundingCheckToggle.test.tsx src/App.test.tsx > target/3a-t5-red.txt 2>&1; tail -40 target/3a-t5-red.txt`
Expected: FAIL — the six `DriveConsole vocabulary` tests, both new `RiskPopover extra controls` tests, the four
edited GroundingCheckToggle tests, the three new Loquela tests and the edited App test. If any of those passes, STOP.

- [ ] **Step 3: Implement.**

`src/lib/driveConsole.ts`: directly after its first line (the `// Mirror of contracts/gui/drive-console.v1.yaml …`
comment) add `import { modeLabel } from './turnEvents';`, and replace the whole `export const CLUTCH_DETENTS … = [ … ];`
statement with:

```ts
/** Canonical mode name for a clutch id — one source: `MODE_NAMES` in `lib/turnEvents.ts` (the turn trace uses it too). */
const modeName = (id: ClutchId): string => modeLabel(id);

export const CLUTCH_DETENTS: { id: ClutchId; label: string; hint: string }[] = [
  { id: 'free',       label: modeName('free'),       hint: 'Free models only' },
  { id: 'efficiency', label: modeName('efficiency'), hint: 'Most out of the tokens you spend; delegates to free agents on simple tasks' },
  { id: 'balanced',   label: modeName('balanced'),   hint: 'Balanced cost/quality' },
  { id: 'genius',     label: modeName('genius'),     hint: 'Most intelligent solutions; budget relaxed' },
];
```

`RiskPopover.tsx`: replace the two `COPY` lines

```ts
  moderate: 'Confirm + enforce grounding. Balanced safety.',
  low: 'Enforce verification + grounding, raise approval, spend safety tokens, lean model up.',
```

with

```ts
  moderate: 'Confirm, and check replies. Balanced safety.',
  low: 'Enforce verification, check replies, raise approval, spend safety tokens, lean model up.',
```

`DriveConsole.tsx`:
(a) Replace the first three import lines with:

```tsx
import React, { useEffect, useId, useRef, useState } from 'react';
import { CLUTCH_DETENTS, RISK_POSTURES, type ClutchId, type ControlState } from '../../../lib/driveConsole';
import { formatSpend } from '../../../config/budget';
import { RiskPopover } from './RiskPopover';
```

(b) Replace everything from `interface DriveConsoleProps {` through the line
`  const pct = budgetUsd > 0 ? Math.min(100, (spentUsd / budgetUsd) * 100) : 0;` with:

```tsx
interface DriveConsoleProps {
  control: ControlState;
  onControlChange: (next: Partial<ControlState>) => void;
  spentUsd: number;
  /** Positive only when the daemon reported a cap; 0 hides the cap and the bar. */
  budgetUsd: number;
  burnPerMin?: number;
  /** Extra controls shown inside the Risk popover (App passes Check replies). */
  riskExtra?: React.ReactNode;
}

export function DriveConsole({
  control,
  onControlChange,
  spentUsd,
  budgetUsd,
  burnPerMin,
  riskExtra,
}: DriveConsoleProps) {
  const [riskOpen, setRiskOpen] = useState(false);
  const [hintFor, setHintFor] = useState<ClutchId | null>(null);
  const hintId = useId();
  const riskAnchorRef = useRef<HTMLSpanElement>(null);
  const risk = RISK_POSTURES.find(r => r.id === control.risk)!;
  const pct = budgetUsd > 0 ? Math.min(100, (spentUsd / budgetUsd) * 100) : 0;
  const hint = hintFor ? CLUTCH_DETENTS.find(d => d.id === hintFor)?.hint ?? null : null;
```

(c) (The clutch buttons' native `title={d.hint}` is dropped on purpose: the visible hint below — shown on hover and on
keyboard focus, linked by `aria-describedby` — replaces it; keeping both would show two tooltips.)
Replace everything from the line `      {/* ① Clutch */}` up to (not including) the line
`      {/* ③ Risk — trigger + upward-anchored popover share a relative anchor so` with:

```tsx
      {/* ① Mode */}
      <div className="relative flex items-center gap-1 border-r border-white/[0.07] px-2.5 py-1.5">
        <span className="text-zinc-500" aria-hidden>⚙</span>
        <div role="radiogroup" aria-label="Mode — how much to spend" className="flex gap-0.5">
          {CLUTCH_DETENTS.map(d => (
            <button
              key={d.id}
              type="button"
              role="radio"
              aria-checked={control.clutch === d.id}
              aria-describedby={hintFor === d.id ? hintId : undefined}
              onClick={() => onControlChange({ clutch: d.id })}
              onMouseEnter={() => setHintFor(d.id)}
              onMouseLeave={() => setHintFor(null)}
              onFocus={() => setHintFor(d.id)}
              onBlur={() => setHintFor(null)}
              className={`min-h-[24px] rounded px-1.5 font-medium ${
                control.clutch === d.id
                  ? 'bg-brass/16 text-brass'
                  : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              {d.label}
            </button>
          ))}
        </div>
        {hint && (
          <span
            id={hintId}
            role="tooltip"
            data-testid="drive-mode-hint"
            className="pointer-events-none absolute bottom-full left-0 z-40 mb-1 whitespace-nowrap rounded border border-white/10 bg-bg-base px-2 py-0.5 text-[10px] text-text-secondary"
          >
            {hint}
          </span>
        )}
      </div>

      {/* ② Spend — engine-wide; the status bar's Spend card has the breakdown */}
      <div
        data-testid="drive-console-spend"
        className="flex items-center gap-2 border-r border-white/[0.07] px-2.5 py-1.5"
        title="Engine spend across all sessions"
      >
        <span className="text-zinc-500">Spend</span>
        <span className="font-mono text-brass">{formatSpend(spentUsd, budgetUsd > 0 ? budgetUsd : null)}</span>
        {budgetUsd > 0 && (
          <span className="relative h-[3px] w-12 rounded-sm bg-white/8">
            <span
              className="absolute inset-y-0 left-0 rounded-sm bg-linear-to-r from-emerald-400 to-brass"
              style={{ width: `${pct}%` }}
            />
          </span>
        )}
        {burnPerMin != null && (
          <span className="text-zinc-500">↑${burnPerMin.toFixed(2)}/m</span>
        )}
      </div>

```

(d) Replace `          <span>{risk.label}</span>` with `          <span>Risk: {risk.label}</span>`.

(e) Replace the self-closing RiskPopover element

```tsx
        <RiskPopover
          open={riskOpen}
          risk={control.risk}
          onChange={(n) => { onControlChange(n); setRiskOpen(false); }}
          onClose={() => setRiskOpen(false)}
        />
```

with

```tsx
        <RiskPopover
          open={riskOpen}
          risk={control.risk}
          onChange={(n) => { onControlChange(n); setRiskOpen(false); }}
          onClose={() => setRiskOpen(false)}
        >
          {riskExtra}
        </RiskPopover>
```

`RiskPopover.tsx`: add `  children?: React.ReactNode;` as the last field of `interface RiskPopoverProps`; change
`export function RiskPopover({ risk, open, onChange, onClose }: RiskPopoverProps) {` to
`export function RiskPopover({ risk, open, onChange, onClose, children }: RiskPopoverProps) {`; and insert directly
after the `{RISK_POSTURES.map(p => ( … ))}` block (before the dialog's closing `</div>`):

```tsx
      {children ? (
        <div className="mt-2 border-t border-white/10 pt-2">
          <div className="mb-1 text-[10px] uppercase tracking-widest text-zinc-500">After each reply</div>
          {children}
        </div>
      ) : null}
```

`GroundingCheckToggle.tsx`: replace
`      aria-label={`Grounding check ${enabled ? 'on' : 'off'}`}` with
`      aria-label={`Check replies ${enabled ? 'on' : 'off'}`}`;
replace `      title="When on, replies get a non-blocking post-reply confidence check"` with
`      title="When on, each reply gets a background check for unsupported claims; it never blocks the reply"`;
replace `      grounding: {enabled ? 'on' : 'off'}` with `      Check replies: {enabled ? 'on' : 'off'}`.

`Loquela.tsx`:
  - directly after `  trailingSlot?: React.ReactNode;` (in `interface LoquelaProps`) add

```tsx
  /** Rendered inside the Risk popover (App passes the Check replies toggle). */
  riskSlot?: React.ReactNode;
```

  - in the destructuring, directly after `  trailingSlot,` add `  riskSlot,`;
  - replace `            budgetUsd={sessionBudget?.cap ?? 0}` with

```tsx
            budgetUsd={sessionBudget?.source === 'daemon' ? sessionBudget.cap : 0}
            riskExtra={riskSlot}
```

`App.tsx`: in the `const loquelaComposer = (` element replace `      trailingSlot={` (the line directly above
`        <GroundingCheckToggle`) with `      riskSlot={`.

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela src/components/surfaces/Chat src/App.test.tsx 2>&1 | tail -25 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -15`
Expected: all pass, including the untouched `renders all four clutch detents…` and `GroundingCheckToggle toolbar
placement wiring`; typecheck exits 0.

- [ ] **Step 5: Mutation proofs** (restore after each).
  1. In `Loquela.tsx` replace `budgetUsd={sessionBudget?.source === 'daemon' ? sessionBudget.cap : 0}` with
     `budgetUsd={sessionBudget?.cap ?? 0}`. Run the Step 2 command into `target/3a-t5-mutant-a.txt`: `never shows a spend
     cap the daemon did not report` must FAIL. Restore.
  2. In `DriveConsole.tsx` delete the line `          {riskExtra}`. Run into `target/3a-t5-mutant-b.txt`: `puts extra risk
     controls (Check replies) inside the Risk popover…` and `renders riskSlot inside the Risk popover` must FAIL. Restore.
  3. In `driveConsole.ts` replace `modeLabel(id)` with `id`. Run into `target/3a-t5-mutant-c.txt`: `takes
     every mode name from MODE_NAMES…` and `names every mode in full…` must FAIL. Restore.

- [ ] **Step 6: Commit (Claude Code)**

```bash
cd /Users/brbrainerd/dev/vox
F="crates/vox-gui/ui/src/lib/driveConsole.ts crates/vox-gui/ui/src/components/surfaces/Loquela/DriveConsole.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/DriveConsole.test.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/RiskPopover.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/RiskPopover.test.tsx crates/vox-gui/ui/src/components/surfaces/Chat/GroundingCheckToggle.tsx crates/vox-gui/ui/src/components/surfaces/Chat/GroundingCheckToggle.test.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.test.tsx crates/vox-gui/ui/src/App.tsx crates/vox-gui/ui/src/App.test.tsx"
git add -- ${=F}
git commit -m "feat(gui): full mode names, Risk label, Check replies inside Risk" -- ${=F}
```

---

### Task 6: Composer — no duplicate spend, honest Run hint, reachable Plan mode

<!-- AMENDED: T5→T6 — renumbered; the Plan hint no longer names the tool (`vox_plan`); the Run test is noted as only partly RED. -->

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`, `…/Loquela/Loquela.test.tsx`
- Modify: `crates/vox-gui/ui/src/lib/slashRouter.ts`, `crates/vox-gui/ui/src/lib/slashRouter.test.ts`

**Interfaces:**
- Consumes: `executionMode: 'chat' | 'task' | 'plan'` (existing state; `buildChatTurn` maps `plan` → `execution: 'plan'`).
- Produces: module const `SEND_MODES` in `Loquela.tsx`; menu button `Set send mode: Plan`; Run button accessible name
  `Run (Enter)` with `<kbd>↵</kbd>`; `formatSessionBudget` deleted.

- [ ] **Step 0: Preconditions.** From `crates/vox-gui/ui`: `rg -n "formatSessionBudget" src` (hits only in
  `lib/slashRouter.ts`, `lib/slashRouter.test.ts`, `components/surfaces/Loquela/Loquela.tsx`);
  `rg -n "⌘↵</kbd>" src/components/surfaces/Loquela/Loquela.tsx` (1); `rg -n "aria-label=\"Set send mode: Quick chat\"" src/components/surfaces/Loquela/Loquela.tsx` (1);
  `rg -n "riskSlot" src/components/surfaces/Loquela/Loquela.tsx` (Task 5 landed). Any miss: STOP.

- [ ] **Step 1: Write the failing tests.** Append inside `describe('Loquela', …)` in `Loquela.test.tsx`:

```tsx
  it('does not repeat global spend as "session $x / $y" in the toolbar', () => {
    renderLoquela({ sessionBudget: { spent: 1.23, cap: 50, source: 'daemon' } });
    expect(screen.queryByText(/session \$/i)).toBeNull();
  });

  it('the Run button aria text and its visible shortcut hint agree', () => {
    renderLoquela();
    const run = screen.getByRole('button', { name: 'Run (Enter)' });
    expect(run.querySelector('kbd')?.textContent).toBe('↵');
  });

  it('offers Plan as a send mode, labels the trigger with it, and submits execution_mode "plan"', () => {
    const onSubmit = vi.fn();
    renderLoquela({ onSubmit });
    fireEvent.click(screen.getByRole('button', { name: /choose send mode/i }));
    fireEvent.click(screen.getByRole('button', { name: 'Set send mode: Plan' }));
    expect(screen.getByRole('button', { name: /choose send mode/i })).toHaveTextContent('Plan');
    expect(screen.queryByLabelText('Interaction mode')).toBeNull();
    const ta = screen.getByLabelText('Task composer');
    fireEvent.change(ta, { target: { value: 'draft the migration' } });
    fireEvent.keyDown(ta, { key: 'Enter' });
    expect(onSubmit.mock.calls[0][0].execution_mode).toBe('plan');
  });
```

In the existing test `the Run button carries its own keyboard-shortcut hint, with no other disconnected shortcut hint
elsewhere`, replace only `    expect(runButton).toHaveTextContent('⌘↵');` with `    expect(runButton).toHaveTextContent('↵');`.

In `slashRouter.test.ts` delete the line `  formatSessionBudget,` from the import list and delete the whole test
`formats session budget for display`.

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela/Loquela.test.tsx src/lib/slashRouter.test.ts > target/3a-t6-red.txt 2>&1; tail -30 target/3a-t6-red.txt`
Expected: FAIL — the three new Loquela tests (`session $1.23 / $50.00` is rendered; no Plan entry; and in the Run test only the `kbd` assertion fails — its `Run (Enter)` aria query already matches today, so that test is only partly RED by design).
`slashRouter.test.ts` passes. If a new Loquela test passes, STOP.

- [ ] **Step 3: Implement** in `Loquela.tsx`:
  - in the `from '../../../lib/slashRouter'` import delete the line `  formatSessionBudget,`;
  - insert directly above the line `export function Loquela({`:

```tsx
/** Send modes. `plan` is a real execution (buildChatTurn → `execution: 'plan'`), so it gets a menu entry. */
const SEND_MODES: { id: 'chat' | 'task' | 'plan'; label: string; hint: string }[] = [
  { id: 'chat', label: 'Quick chat', hint: 'Synchronous reply, no background task' },
  { id: 'task', label: 'Background task', hint: 'Dispatch as an autonomous task, not blocking' },
  { id: 'plan', label: 'Plan', hint: 'Draft a plan first' },
];

```

  - replace `              <kbd className="rounded-sm border border-current px-1 text-[9px] opacity-75">⌘↵</kbd>` with
    `              <kbd className="rounded-sm border border-current px-1 text-[9px] opacity-75">↵</kbd>`;
  - replace `<span>{executionMode === 'chat' ? 'Quick chat' : 'Background task'}</span>` with
    `<span>{SEND_MODES.find(m => m.id === executionMode)?.label}</span>`;
  - replace the two menu buttons — from the line starting `              <button type="button" aria-label="Set send mode: Quick chat"`
    through the `</button>` that closes the `Set send mode: Background task` button — with:

```tsx
              {SEND_MODES.map(m => (
                <button key={m.id} type="button" aria-label={`Set send mode: ${m.label}`} onClick={() => { setExecutionMode(m.id); setModeOpen(false); }} className={`flex w-full items-start gap-2 rounded-sm px-2 py-1.5 text-left hover:bg-overlay-subtle ${executionMode === m.id ? "bg-overlay-subtle" : ""}`}>
                  <div className="flex-1">
                    <div className="text-[11px] text-text-primary">{m.label}</div>
                    <div className="font-mono text-[9px] text-text-muted">{m.hint}</div>
                  </div>
                </button>
              ))}
```

  - replace the whole block from `          {(estCost != null || sessionBudget || trailingSlot != null) && (` through its
    matching closing `          )}` (the last block before `        </div>` / `      </Glass>`) with:

```tsx
          {(estCost != null || trailingSlot != null) && (
            <div className="ml-auto flex items-center gap-2">
              {estCost != null && (
                <span className="font-mono text-[9px] text-text-muted tabular-nums">
                  ~{tokens} tok · ~${estCost.toFixed(3)}
                </span>
              )}
              {trailingSlot}
            </div>
          )}
```

In `slashRouter.ts` delete the doc comment `/** Display string for session budget next to token estimate. */` and the
whole `export function formatSessionBudget(…) { … }`.

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela src/lib/slashRouter.test.ts src/App.test.tsx 2>&1 | tail -20 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -15`
Expected: all pass (including `hides Effort on Quick chat and shows it on Background task` and `renders trailingSlot
at the far right…`); `rg -n formatSessionBudget crates/vox-gui/ui/src` prints nothing; typecheck exits 0.

- [ ] **Step 5: No guard in this task** (no mutation step). `git diff --stat` must list only the four files above.

- [ ] **Step 6: Commit (Claude Code)**

```bash
cd /Users/brbrainerd/dev/vox
F="crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.test.tsx crates/vox-gui/ui/src/lib/slashRouter.ts crates/vox-gui/ui/src/lib/slashRouter.test.ts"
git add -- ${=F}
git commit -m "fix(gui): composer drops the mislabelled session spend and adds Plan" -- ${=F}
```

---

### Task 7: Honest research popover

<!-- AMENDED: T6→T7 — renumbered only. -->

**Files:**
- Modify: `crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx`, `crates/vox-gui/ui/src/components/common/StatusBarCluster.test.tsx`

**Interfaces:**
- Consumes: `getResearchEngineStatus(): Promise<ResearchEngineStatusDto>` (`active_lane`, `fast_timeout_ms`,
  `deep_timeout_ms`, `providers[{ id, name, is_keyless, is_enabled, has_key, quota_usage? }]`).
- Produces: test ids `status-bar-cluster-provider-{provider id}`, `status-bar-cluster-unknown`; dialog name `Research engine status`.

- [ ] **Step 0: Preconditions.** `rg -n "Wikipedia \(Live\)|0 Plaintext Keys|getResearchEngineStatus\(\).then\(setStatus\).catch\(\(\) => \{\}\);|const activeLane = status\?\.active_lane \?\? 'fast';|\{onOpenDrawer && \(" crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx`
  must show 5 hits. Otherwise STOP.

- [ ] **Step 1: Write the failing tests.** In `StatusBarCluster.test.tsx`, in the test `toggles 4-quadrant popover on
  trigger click`, change the title to `toggles the research popover on trigger click` and replace its four lines
  `expect(screen.getByText(/Keyless Engines/i))…`, `…/Search Quotas/i…`, `…/Lane Routing/i…`, `…/Clavis Vault/i…` with:

```tsx
    expect(await screen.findByText('Providers')).toBeInTheDocument();
    expect(screen.getByText('Lane')).toBeInTheDocument();
```

Append at the end of the file:

```tsx
describe('StatusBarCluster honesty (plan 3a)', () => {
  const provider = (id: string, name: string, over: Record<string, unknown> = {}) => ({
    id,
    name,
    is_keyless: true,
    is_enabled: true,
    has_key: false,
    quota_usage: null,
    ...over,
  });
  const status = (over: Record<string, unknown> = {}) => ({
    active_lane: 'fast',
    fast_timeout_ms: 4000,
    deep_timeout_ms: 15000,
    providers: [],
    free_key_offers: [],
    ...over,
  });

  it('lists each provider exactly as the engine reports it, with no hardcoded health', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation(async () =>
      status({
        providers: [
          provider('wikipedia', 'Wikipedia'),
          provider('arxiv', 'arXiv', { is_enabled: false }),
          provider('tavily', 'Tavily Search', { is_keyless: false, has_key: false }),
        ],
      }),
    );
    render(<StatusBarCluster />);
    fireEvent.click(screen.getByTestId('status-bar-cluster-trigger'));
    expect(await screen.findByTestId('status-bar-cluster-provider-wikipedia')).toHaveTextContent('Wikipediaon');
    expect(screen.getByTestId('status-bar-cluster-provider-arxiv')).toHaveTextContent('arXivoff');
    expect(screen.getByTestId('status-bar-cluster-provider-tavily')).toHaveTextContent('Tavily Searchon · no key');
    expect(screen.queryByTestId('status-bar-cluster-provider-openalex')).toBeNull();
    const popover = screen.getByTestId('status-bar-cluster-popover');
    expect(popover).not.toHaveTextContent('✓');
    expect(popover).not.toHaveTextContent(/online/i);
    expect(popover).not.toHaveTextContent(/plaintext/i);
    expect(popover).toHaveTextContent('Fast (≤4s)');
  });

  it('a failed status fetch reads "unknown" and drops an earlier reading (no stale green)', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation(async () => status({ providers: [provider('wikipedia', 'Wikipedia')] }));
    render(<StatusBarCluster />);
    const trigger = screen.getByTestId('status-bar-cluster-trigger');
    fireEvent.click(trigger);
    expect(await screen.findByTestId('status-bar-cluster-provider-wikipedia')).toBeInTheDocument();
    fireEvent.click(trigger);
    vi.mocked(invoke).mockImplementation(async () => {
      throw new Error('daemon down');
    });
    fireEvent.click(trigger);
    expect(await screen.findByTestId('status-bar-cluster-unknown')).toBeInTheDocument();
    expect(screen.queryByTestId('status-bar-cluster-provider-wikipedia')).toBeNull();
    expect(screen.getByTestId('status-bar-cluster-popover')).not.toHaveTextContent('✓');
    expect(trigger).toHaveTextContent('unknown');
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/common/StatusBarCluster.test.tsx > target/3a-t7-red.txt 2>&1; tail -30 target/3a-t7-red.txt`
Expected: FAIL — the edited toggle test (`Providers` not found) and both honesty tests. If either honesty test passes, STOP.

- [ ] **Step 3: Implement** in `StatusBarCluster.tsx`.
(a) Replace

```tsx
  useEffect(() => {
    getResearchEngineStatus().then(setStatus).catch(() => {});
  }, [isOpen]);
```

with

```tsx
  // A failed fetch clears the reading: the popover never shows health it did not just receive.
  useEffect(() => {
    let cancelled = false;
    getResearchEngineStatus()
      .then((next) => {
        if (!cancelled) setStatus(next);
      })
      .catch(() => {
        if (!cancelled) setStatus(null);
      });
    return () => {
      cancelled = true;
    };
  }, [isOpen]);
```

(b) Replace everything from the line `  const activeLane = status?.active_lane ?? 'fast';` up to (not including) the line
`          {onOpenDrawer && (` with:

```tsx
  const activeLane = status?.active_lane ?? null;
  const tavily = status?.providers.find((p) => p.id === 'tavily');
  const tavilyQuota = tavily?.quota_usage ?? null;
  const tavilyRemaining = tavilyQuota
    ? Math.max(0, tavilyQuota.units_limit - tavilyQuota.units_spent)
    : null;
  // Seconds from the engine's own timeouts, never a hardcoded figure.
  const seconds = (ms: number) => `${Number((ms / 1000).toFixed(1))}s`;

  return (
    <div className={`relative inline-flex items-center ${className}`}>
      <button
        ref={triggerRef}
        type="button"
        data-testid="status-bar-cluster-trigger"
        onClick={() => setIsOpen((o) => !o)}
        aria-expanded={isOpen}
        aria-label="Research & Engine Status"
        className="inline-flex items-center gap-1.5 rounded-sm px-2 py-0.5 text-[10px] text-text-muted hover:bg-overlay-subtle hover:text-text-secondary transition"
      >
        <span className="uppercase tracking-[0.14em] text-text-muted">Research</span>
        <span className="font-mono tabular-nums text-text-secondary">
          {activeLane === 'deep' ? '🔬 Deep' : activeLane === 'fast' ? '⚡ Fast' : 'unknown'}
          {tavilyQuota && tavilyRemaining !== null ? ` · ${tavilyRemaining}/${tavilyQuota.units_limit}` : ''}
        </span>
      </button>

      {isOpen && (
        <div
          ref={popoverRef}
          data-testid="status-bar-cluster-popover"
          role="dialog"
          aria-label="Research engine status"
          className="absolute bottom-full right-0 z-50 mb-1 w-80 rounded-xl border border-border-subtle bg-bg-base p-4 shadow-2xl space-y-3 text-xs"
        >
          <div className="flex items-center justify-between border-b border-border-subtle pb-2">
            <span className="font-display font-semibold text-text-primary tracking-wide">Research engine</span>
          </div>

          {status === null ? (
            <p data-testid="status-bar-cluster-unknown" className="text-[11px] text-text-muted">
              Research status unknown: the engine did not report.
            </p>
          ) : (
            <div className="space-y-2 text-[11px]">
              <div>
                <div className="font-medium text-text-muted uppercase text-[9px] tracking-wider">Providers</div>
                <ul className="mt-1 space-y-0.5 font-mono text-[10px]">
                  {status.providers.map((p) => (
                    <li
                      key={p.id}
                      data-testid={`status-bar-cluster-provider-${p.id}`}
                      className="flex justify-between gap-2"
                    >
                      <span className="text-text-secondary">{p.name}</span>
                      <span className="text-text-muted">
                        {p.is_enabled ? 'on' : 'off'}
                        {!p.is_keyless && !p.has_key ? ' · no key' : ''}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
              {tavilyQuota && (
                <div className="font-mono text-[10px] text-text-secondary">
                  Tavily quota: {tavilyRemaining}/{tavilyQuota.units_limit} left
                </div>
              )}
              <div>
                <div className="font-medium text-text-muted uppercase text-[9px] tracking-wider">Lane</div>
                <div className="font-mono text-[10px] space-y-0.5">
                  <div className={activeLane === 'fast' ? 'text-brass font-semibold' : 'text-text-muted'}>
                    ⚡ Fast (≤{seconds(status.fast_timeout_ms)})
                  </div>
                  <div className={activeLane === 'deep' ? 'text-brass font-semibold' : 'text-text-muted'}>
                    🔬 Deep (≤{seconds(status.deep_timeout_ms)})
                  </div>
                </div>
              </div>
            </div>
          )}

```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/common src/components/layout 2>&1 | tail -15 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -10`
Expected: all pass (including `closes popover on Escape…` and `invokes onOpenDrawer…`); typecheck exits 0;
`rg -n "Wikipedia \(Live\)|Plaintext|Online" crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx` prints nothing.

- [ ] **Step 5: Mutation proof.** Replace `if (!cancelled) setStatus(null);` with `/* keep last */`. Run the Step 2
  command into `target/3a-t7-mutant.txt`: `a failed status fetch reads "unknown"…` must FAIL. Restore; `git diff` shows
  only this task's edits.

- [ ] **Step 6: Commit (Claude Code)**

```bash
cd /Users/brbrainerd/dev/vox
F="crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx crates/vox-gui/ui/src/components/common/StatusBarCluster.test.tsx"
git add -- ${=F}
git commit -m "fix(gui): research popover shows only what the engine reported" -- ${=F}
```

---

### Task 8: Mocks from contract families, and a no-versioned-model-id guard

<!-- AMENDED: T7→T8 — renumbered; the guard is backslash-aware (escaped regex literals such as `/ollama\/llama3/i` in
ChatModelPicker.test.tsx were judged as `/llama3/i`), with a self-test and a mutation for it. -->

**Files:**
- Create: `crates/vox-gui/ui/src/__tests__/noVersionedModelIds.test.ts`, `crates/vox-gui/ui/e2e/lib/tauriMock.families.test.ts`
- Modify: `crates/vox-gui/ui/e2e/lib/tauriMock.ts`, `crates/vox-gui/ui/e2e/lib/tauriMockRich.ts`, `crates/vox-gui/ui/e2e/chat-trust-chips.spec.ts`
- Modify: `crates/vox-gui/ui/src/lib/axisDrive.test.ts`, `src/lib/sessionChatStore.test.ts`, `src/lib/modelPicker.test.ts`,
  `src/lib/buildChatTurn.test.ts`, `src/lib/chatCorrelation.test.ts`, `src/components/surfaces/Chat/ChatModelPicker.test.tsx`,
  `src/components/surfaces/Loquela/Loquela.test.tsx` (all under `crates/vox-gui/ui/`)
- Temporary (mutation only, deleted in Step 7): `crates/vox-gui/ui/src/__tests__/mutantVersionedId.fixture.ts`

**Interfaces:**
- Consumes (Task 1): `familyKey`, `VERSIONED_CLOUD_MODEL_ID`; `mockInitScript` (`e2e/lib/tauriMockShared.ts`);
  `installTauriMock`; the bootstrap catalog JSON.
- Produces: `tauriMock` answers `list_model_cards` / `get_active_model` / `get_routing_summary_live` /
  `get_selection_policy` / `explain_model_selection` / `suggest_model_for_task` with family keys; the routing summary
  carries `family: 'anthropic/claude-sonnet'`, `resolved_from: 'bootstrap'`, `reason: 'lowest cost that fits the mode'`,
  `decision_preview.discovery_state: 'confirmed'`, `alternatives: ['anthropic/claude-haiku', 'deepseek/deepseek-flash']`
  (Task 9 asserts these exact strings).

- [ ] **Step 0: Preconditions.** `rg -n "export const VERSIONED_CLOUD_MODEL_ID|export function familyKey" crates/vox-gui/ui/src/lib/modelFamily.ts` (2);
  `rg -n "export function mockInitScript" crates/vox-gui/ui/e2e/lib/tauriMockShared.ts` (1);
  `ls crates/vox-gui/ui/src/__tests__/noVersionedModelIds.test.ts` (must not exist). Any mismatch: STOP.

- [ ] **Step 1: Write the guard.** Create `crates/vox-gui/ui/src/__tests__/noVersionedModelIds.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, extname, join, relative, resolve } from 'node:path';
import { VERSIONED_CLOUD_MODEL_ID } from '../lib/modelFamily';

// src/__tests__ -> src -> ui
const SELF = fileURLToPath(import.meta.url);
const UI_ROOT = resolve(dirname(SELF), '../..');
const SCAN_DIRS = ['src', 'e2e'];
const EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.mjs', '.json']);
const SKIP_DIRS = new Set(['node_modules', 'screens']);
/**
 * Local model names may be literal (AGENTS.md: local MENS revisions are exempt): Ollama, MENS runs,
 * mesh- and locally-served models, and the MENS fine-tuning base.
 */
const LOCAL_PREFIXES = ['ollama/', 'mens/', 'local/', 'mesh/', 'qwen/qwen3-8b'];
const TOKEN_CHAR = /[A-Za-z0-9_./:@\\-]/;

/**
 * The whole id-like token around a regex hit (so a match inside `ollama/…` is judged by its prefix). Test regex
 * literals escape slashes (`/ollama\/llama3/i`), so backslashes and a leading `/` delimiter are dropped first.
 */
function tokenAt(line: string, index: number): string {
  let start = index;
  let end = index;
  while (start > 0 && TOKEN_CHAR.test(line[start - 1])) start -= 1;
  while (end < line.length && TOKEN_CHAR.test(line[end])) end += 1;
  return line.slice(start, end).replace(/\\/g, '').replace(/^\/+/, '');
}

function findVersionedIds(text: string): string[] {
  const re = new RegExp(VERSIONED_CLOUD_MODEL_ID.source, 'gi');
  const hits: string[] = [];
  for (const line of text.split('\n')) {
    for (const m of line.matchAll(re)) {
      const token = tokenAt(line, m.index ?? 0);
      if (LOCAL_PREFIXES.some((p) => token.toLowerCase().startsWith(p))) continue;
      hits.push(token);
    }
  }
  return hits;
}

function walk(dir: string, out: string[]): void {
  for (const name of readdirSync(dir)) {
    if (SKIP_DIRS.has(name)) continue;
    const full = join(dir, name);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (EXTENSIONS.has(extname(name))) out.push(full);
  }
}

describe('findVersionedIds (guard self-test)', () => {
  it('flags versioned cloud ids, including bare mock shortnames, and exempts local model names', () => {
    const sample = [
      "id: 'anthropic/claude-opus-4.7',",
      "active: 'opus-4-8',",
      "family: 'anthropic/claude-opus',",
      "local: 'ollama/llama3', base: 'Qwen/Qwen3-8B', run: 'mens/runs/qwen3_27b_metal_check/quant_q6_k',",
      "other: 'openai/gpt-5.2-mini', deep: 'deepseek/deepseek-v4-flash', o: 'openai/o3-mini',",
    ].join('\n');
    expect(findVersionedIds(sample)).toEqual([
      'anthropic/claude-opus-4.7',
      'opus-4-8',
      'openai/gpt-5.2-mini',
      'deepseek/deepseek-v4-flash',
      'openai/o3-mini',
    ]);
  });

  it('reads ids inside escaped regex literals: exempts a local one, flags a cloud one', () => {
    const sample = [
      String.raw`expect(screen.queryByRole('option', { name: /ollama\/llama3/i })).toBeNull();`,
      String.raw`expect(screen.queryByRole('option', { name: /openai\/gpt-5\.2-mini/i })).toBeNull();`,
    ].join('\n');
    expect(findVersionedIds(sample)).toEqual(['openai/gpt-5.2-mini/i']);
  });
});

describe('no versioned cloud model ids in GUI source, tests or mocks', () => {
  it('src/ and e2e/ hold only family keys or exempt local names', () => {
    const files: string[] = [];
    for (const d of SCAN_DIRS) walk(join(UI_ROOT, d), files);
    const offenders = files
      .filter((f) => resolve(f) !== resolve(SELF))
      .flatMap((f) => findVersionedIds(readFileSync(f, 'utf8')).map((id) => `${relative(UI_ROOT, f)}: ${id}`));
    expect(offenders).toEqual([]);
  });
});
```

- [ ] **Step 2: Write the mock seam test.** Create `crates/vox-gui/ui/e2e/lib/tauriMock.families.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { mockInitScript } from './tauriMockShared';
import { installTauriMock } from './tauriMock';
import { familyKey } from '../../src/lib/modelFamily';

// e2e/lib -> e2e -> ui -> vox-gui -> crates -> repo root
const here = dirname(fileURLToPath(import.meta.url));
const BOOTSTRAP = resolve(here, '../../../../../contracts/orchestration/model-catalog.bootstrap.v1.json');
const CONTRACT_FAMILIES = new Set(
  (JSON.parse(readFileSync(BOOTSTRAP, 'utf8')) as Array<{ id: string }>).map((e) => familyKey(e.id)),
);
const LOCAL_PROVIDERS = new Set(['mens', 'local', 'ollama', 'populi_local']);

function makeFakeWindow(): any {
  const storage: Record<string, string> = {};
  return {
    localStorage: {
      setItem: (k: string, v: string) => {
        storage[k] = v;
      },
      getItem: (k: string) => storage[k] ?? null,
    },
  };
}

async function withMock<T>(fn: (invoke: (cmd: string) => Promise<any>) => Promise<T>): Promise<T> {
  const prev = (global as any).window;
  const win = makeFakeWindow();
  (global as any).window = win;
  try {
    // eslint-disable-next-line no-new-func -- exercising the exact addInitScript path
    new Function(mockInitScript(installTauriMock, 'chat'))();
    return await fn((cmd) => win.__TAURI_INTERNALS__.invoke(cmd));
  } finally {
    (global as any).window = prev;
  }
}

describe('tauriMock model data comes from contract families', () => {
  it('every cloud model card id is a family key of the bootstrap catalog', async () => {
    await withMock(async (invoke) => {
      const cards = (await invoke('list_model_cards')) as Array<{ id: string; provider: string }>;
      const cloud = cards.filter((c) => !LOCAL_PROVIDERS.has(c.provider));
      expect(cloud.length).toBeGreaterThan(0);
      for (const c of cloud) expect(CONTRACT_FAMILIES.has(c.id), c.id).toBe(true);
    });
  });

  it('the routing summary names a contract family, says where it came from, and carries no version', async () => {
    await withMock(async (invoke) => {
      const s = await invoke('get_routing_summary_live');
      expect(['catalog', 'bootstrap', 'local']).toContain(s.resolved_from);
      expect(CONTRACT_FAMILIES.has(s.family), s.family).toBe(true);
      expect(typeof s.reason).toBe('string');
      for (const id of [s.active_model, s.decision_preview.selected_model, ...s.decision_preview.alternatives]) {
        expect(CONTRACT_FAMILIES.has(id), id).toBe(true);
      }
    });
  });

  it('selection policy, explanation, suggestion and active model use contract families', async () => {
    await withMock(async (invoke) => {
      for (const id of (await invoke('get_selection_policy')).chain) expect(CONTRACT_FAMILIES.has(id), id).toBe(true);
      expect(CONTRACT_FAMILIES.has((await invoke('explain_model_selection')).chosen)).toBe(true);
      expect(CONTRACT_FAMILIES.has(await invoke('suggest_model_for_task'))).toBe(true);
      expect(CONTRACT_FAMILIES.has(await invoke('get_active_model'))).toBe(true);
    });
  });
});
```

- [ ] **Step 3: Run to verify failure; save the output; check the offender list**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/noVersionedModelIds.test.ts e2e/lib/tauriMock.families.test.ts > target/3a-t8-red.txt 2>&1; tail -80 target/3a-t8-red.txt`
Expected: both self-tests PASS; `src/ and e2e/ hold only family keys…` FAILS with an offender list; all three
`tauriMock model data…` tests FAIL. Collect the distinct file paths in the offender list. They must be a subset of:
`e2e/chat-trust-chips.spec.ts`, `e2e/lib/tauriMock.ts`, `e2e/lib/tauriMockRich.ts`, `src/lib/axisDrive.test.ts`,
`src/lib/sessionChatStore.test.ts`, `src/lib/modelPicker.test.ts`, `src/lib/buildChatTurn.test.ts`,
`src/lib/chatCorrelation.test.ts`, `src/components/surfaces/Chat/ChatModelPicker.test.tsx`,
`src/components/surfaces/Loquela/Loquela.test.tsx`. If ANY other file is named, or more than 12 files are named,
end with `DRIVE: STOPPED step 3: guard names files outside the plan: <comma-separated list>`. If a self-test fails,
STOP (the pattern or token logic differs from the plan).

- [ ] **Step 4: Fix the mocks** (exact string replacements).

`e2e/lib/tauriMock.ts`:
  - `  const modelIds = ['mens-8b', 'opus-4-8', 'sonnet-4-6', 'haiku-4-5', 'qwen-coder-7b', 'local-llama'];` →
    `  const modelIds = ['mens-8b', 'anthropic/claude-opus', 'anthropic/claude-sonnet', 'anthropic/claude-haiku', 'qwen-coder-7b', 'local-llama'];`
  - `  const modelNames = ['Mens 8B', 'Opus 4.8', 'Sonnet 4.6', 'Haiku 4.5', 'Qwen Coder 7B', 'Local Llama'];` →
    `  const modelNames = ['Mens 8B', 'Claude Opus', 'Claude Sonnet', 'Claude Haiku', 'Qwen Coder 7B', 'Local Llama'];`
  - `        case 'get_active_model': return 'opus-4-8';` → `        case 'get_active_model': return 'anthropic/claude-sonnet';`
  - replace the whole `        case 'get_routing_summary_live':` case (its `return { … };`, 7 lines) with:

```ts
        case 'get_routing_summary_live':
          // Family keys from the bootstrap catalog (see e2e/lib/tauriMock.families.test.ts); no live catalog in e2e.
          return {
            active_model: 'anthropic/claude-sonnet', exploration_spent_usd: 2.4, exploration_budget_usd: 50,
            arm_count: 6, model_count: 7,
            family: 'anthropic/claude-sonnet', resolved_from: 'bootstrap', reason: 'lowest cost that fits the mode',
            decision_preview: { selected_model: 'anthropic/claude-sonnet', discovery_state: 'confirmed',
              alternatives: ['anthropic/claude-haiku', 'deepseek/deepseek-flash'], rejection_reasons: ['budget cap'],
              intelligence_score: 0.92, efficiency_score: 0.7, latency_score: 0.6 },
          };
```

  - `        case 'get_selection_policy': return { chain: ['opus-4-8', 'sonnet-4-6', 'haiku-4-5'], free_tier: true };` →
    `        case 'get_selection_policy': return { chain: ['anthropic/claude-opus', 'anthropic/claude-sonnet', 'anthropic/claude-haiku'], free_tier: true };`
  - `        case 'explain_model_selection': return { chosen: 'opus-4-8', reason: 'highest quality within budget' };` →
    `        case 'explain_model_selection': return { chosen: 'anthropic/claude-sonnet', reason: 'lowest cost that fits the mode' };`
  - `        case 'suggest_model_for_task': return 'sonnet-4-6';` → `        case 'suggest_model_for_task': return 'anthropic/claude-sonnet';`
  - in the `inference_provider_status` line: `local_models: ['llama3.2']` → `local_models: ['local-llama-small']`

`e2e/lib/tauriMockRich.ts`: `    local_models: i >= 3 ? ['llama3.2', 'qwen-coder-7b', 'mens-8b-instruct-longname'] : [],` →
`    local_models: i >= 3 ? ['local-llama-small', 'qwen-coder-7b', 'mens-8b-instruct-longname'] : [],`

`e2e/chat-trust-chips.spec.ts`: both `        model_id: 'opus-4-8',` → `        model_id: 'anthropic/claude-opus',`

Unit tests (replace every occurrence in the named file; nothing else changes):
  - `src/lib/axisDrive.test.ts`: `'openai/gpt-4o'` → `'openai/gpt-mini'`
  - `src/lib/sessionChatStore.test.ts`: `'anthropic/claude-opus-4.7'` → `'anthropic/claude-opus'`
  - `src/lib/chatCorrelation.test.ts`: `'anthropic/claude-opus-4.7'` → `'anthropic/claude-opus'`
  - `src/lib/buildChatTurn.test.ts`: `'openrouter/anthropic/claude-opus-5'` → `'openrouter/anthropic/claude-opus'`
  - `src/lib/modelPicker.test.ts`: `claude-sonnet-4'` → `claude-sonnet'` (covers `'anthropic/claude-sonnet-4'` and
    `'openrouter/anthropic/claude-sonnet-4'`); `'openai/gpt-5.2-mini'` → `'openai/gpt-mini'`
  - `src/components/surfaces/Chat/ChatModelPicker.test.tsx`: `openai/gpt-5.2-mini` → `openai/gpt-mini`;
    `openai\/gpt-5\.2-mini` → `openai\/gpt-mini`; `anthropic/claude-opus-4.7` → `anthropic/claude-opus`;
    `anthropic/claude-sonnet-4` → `anthropic/claude-sonnet`
  - `src/components/surfaces/Loquela/Loquela.test.tsx`: `openrouter/anthropic/claude-sonnet-4` → `openrouter/anthropic/claude-sonnet`

If after these replacements the guard still names a line in one of the listed files, apply the same kind of
replacement to that literal (versioned cloud id → its `familyKey`) and list each extra replacement in your report.

- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 600s pnpm --dir crates/vox-gui/ui exec vitest run 2>&1 | tail -25 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -10`
Expected: the whole vitest suite passes (guard, seam test, and every test whose literals changed); typecheck exits 0.

- [ ] **Step 6: Mutation proof — the guard catches a new versioned id.** Create
  `crates/vox-gui/ui/src/__tests__/mutantVersionedId.fixture.ts` containing exactly
  `export const MUTANT = 'anthropic/claude-opus-4.7';`. Run
  `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/noVersionedModelIds.test.ts > target/3a-t8-mutant-a.txt 2>&1; tail -15 target/3a-t8-mutant-a.txt`:
  must FAIL naming `src/__tests__/mutantVersionedId.fixture.ts: anthropic/claude-opus-4.7`.

- [ ] **Step 7: Delete exactly `crates/vox-gui/ui/src/__tests__/mutantVersionedId.fixture.ts`**, then mutation-prove
  the exemption is load-bearing but narrow: in the guard change `const LOCAL_PREFIXES = ['ollama/', 'mens/', 'local/', 'mesh/', 'qwen/qwen3-8b'];`
  to `const LOCAL_PREFIXES = ['ollama/', 'local/', 'mesh/', 'qwen/qwen3-8b'];`, run the Step 6 command into
  `target/3a-t8-mutant-b.txt`: must FAIL naming `e2e/lib/tauriMock.ts: mens/runs/qwen3_27b_metal_check/quant_q6_k`.
  Restore the line. Re-run the Step 6 command: passes. `git status --short` shows no fixture file.
  Then mutation-prove the backslash handling: in `tokenAt` replace `.replace(/\\/g, '')` with nothing (keep
  `.replace(/^\/+/, '')`), run the Step 6 command into `target/3a-t8-mutant-c.txt`: `reads ids inside escaped regex
  literals…` must FAIL. Restore; re-run: passes.

- [ ] **Step 8: Commit (Claude Code)**

```bash
cd /Users/brbrainerd/dev/vox/crates/vox-gui/ui
F="src/__tests__/noVersionedModelIds.test.ts e2e/lib/tauriMock.families.test.ts e2e/lib/tauriMock.ts e2e/lib/tauriMockRich.ts e2e/chat-trust-chips.spec.ts src/lib/axisDrive.test.ts src/lib/sessionChatStore.test.ts src/lib/modelPicker.test.ts src/lib/buildChatTurn.test.ts src/lib/chatCorrelation.test.ts src/components/surfaces/Chat/ChatModelPicker.test.tsx src/components/surfaces/Loquela/Loquela.test.tsx"
git add -- ${=F}
git commit -m "test(gui): mocks use contract families; guard against versioned model ids" -- ${=F}
```

---

### Task 9: Playwright — status bar cards, rail, composer, research popover

<!-- AMENDED: T8→T9 — renumbered; the `curl` dev-server probe is gone (the agent guard denies network; Playwright's
`reuseExistingServer` handles the server); one shared helper `e2e/lib/invokeOverrides.ts` replaces the per-spec
`overrideInvoke` / `installChatMock` copies (a spec cannot import from another spec); rail text is "Routes to … (next
turn)". -->

**Files:**
- Create: `crates/vox-gui/ui/e2e/lib/invokeOverrides.ts`
- Modify: `crates/vox-gui/ui/e2e/status-bar-surfaces.spec.ts`
- Create: `crates/vox-gui/ui/e2e/chat-surfaces-consolidation.spec.ts`
- Outputs (gitignored, viewed by Claude): `crates/vox-gui/ui/review-bundle/latest/{status-bar-cards,chat-rail-routing,composer-modes,composer-risk-check-replies,research-popover-honest}.png`
- Temporary (mutation only; must end unchanged): `crates/vox-gui/ui/e2e/lib/tauriMock.ts`

**Interfaces:**
- Consumes: `addRichMockInitScript` (`e2e/lib/tauriMockRich.ts`; status with 9 agents, 44 queued, `$12.34` of `$50`),
  `addMockInitScript` + `installTauriMock` (Task 8's family-key routing summary), every DOM contract from Tasks 2–7.
- Produces: `export async function addInvokeOverrides(page: Page, overrides: { responses?: Record<string, unknown>; reject?: string[] }): Promise<void>`;
  two specs; five screenshots.

- [ ] **Step 0: Preconditions.** From `crates/vox-gui/ui`: `rg -n "export async function addRichMockInitScript" e2e/lib/tauriMockRich.ts`
  (1); `rg -n "resolved_from: 'bootstrap', reason: 'lowest cost that fits the mode'" e2e/lib/tauriMock.ts` (1);
  `rg -n "bottom-status-bar-engine|execution-rail-routing-scope|drive-mode-hint|status-bar-cluster-unknown" src` (4+ hits);
  `rg -n "reuseExistingServer" playwright.config.ts` (1); `ls e2e/lib/invokeOverrides.ts` ("No such file"). Any miss: STOP.

- [ ] **Step 1: Create the shared helper** `crates/vox-gui/ui/e2e/lib/invokeOverrides.ts`:

```ts
import type { Page } from '@playwright/test';

export interface InvokeOverrides {
  /** Command → value answered instead of the base mock. */
  responses?: Record<string, unknown>;
  /** Commands that throw (to exercise failure paths). */
  reject?: string[];
}

/**
 * Wraps the Tauri invoke mock installed before it (add this AFTER `addMockInitScript` / `addRichMockInitScript`):
 * listed commands answer from `responses` or throw when in `reject`; everything else falls through to the base mock.
 * Self-contained: `addInitScript` serialises only the function body.
 */
export async function addInvokeOverrides(page: Page, overrides: InvokeOverrides): Promise<void> {
  await page.addInitScript(
    (arg: { responses: Record<string, unknown>; reject: string[] }) => {
      const internals = (window as any).__TAURI_INTERNALS__;
      const base = internals?.invoke;
      if (typeof base !== 'function') throw new Error('addInvokeOverrides must run after a base mock');
      internals.invoke = async (cmd: string, args?: any) => {
        if (arg.reject.includes(cmd)) throw new Error(`${cmd} unavailable (test)`);
        if (Object.prototype.hasOwnProperty.call(arg.responses, cmd)) return arg.responses[cmd];
        return base(cmd, args);
      };
    },
    { responses: overrides.responses ?? {}, reject: overrides.reject ?? [] },
  );
}
```

- [ ] **Step 2: Extend `e2e/status-bar-surfaces.spec.ts`.** Replace its first two lines
  (`import { test, expect } from '@playwright/test';` and `import { installOperatorShellMock } from './lib/operatorShellMock';`)
  with:

```ts
import { test, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installOperatorShellMock } from './lib/operatorShellMock';
import { addRichMockInitScript } from './lib/tauriMockRich';
import { addInvokeOverrides } from './lib/invokeOverrides';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

/** `acme/…` is fictional: a version-shaped id is needed to prove the bar hides it off-catalog. */
const routingSummary = (resolvedFrom: 'catalog' | 'bootstrap') => ({
  active_model: null,
  exploration_spent_usd: 0,
  exploration_budget_usd: 50,
  routing_priority: { efficiency: 50, precision: 50, latency: 50, availability: 50, balance: 50, mobile: 50 },
  arm_count: 3,
  model_count: 12,
  decision_preview: {
    selected_model: 'acme/widget-flash-20260901',
    discovery_state: 'confirmed',
    alternatives: ['acme/gizmo-pro-20260801'],
    rejection_reasons: [],
    intelligence_score: 0.7,
    efficiency_score: 0.8,
    latency_score: 0.6,
  },
  family: 'acme/widget-flash',
  resolved_from: resolvedFrom,
  reason: 'lowest cost that fits the mode',
});
```

Append at the end of the file:

```ts
test.describe('status bar cards (chat-surfaces plan 3a)', () => {
  test('Engine, Spend, Mesh, Routing and Needs you each read one source', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await addRichMockInitScript(page, 'chat');
    await addInvokeOverrides(page, {
      responses: {
        get_routing_summary_live: routingSummary('bootstrap'),
        get_llm_spend: { sessionUsd: 0.25, dayUsd: 1.5, totalUsd: 1.5, dailyBudgetUsd: 50, perSessionBudgetUsd: 10 },
      },
    });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    const bar = page.getByTestId('bottom-status-bar');
    await expect(bar.getByTestId('bottom-status-bar-engine')).toContainText('9 agents · 44 queued');
    await expect(bar.getByTestId('bottom-status-bar-spend')).toContainText('$12.34 / $50.00');
    await expect(bar.getByTestId('bottom-status-bar-mesh')).toContainText('2/2 online');
    await expect(bar.getByTestId('bottom-status-bar-routing')).toContainText('Auto → acme/widget-flash (offline)');
    await expect(bar.getByTestId('bottom-status-bar-routing')).not.toContainText('20260901');
    await expect(bar.getByTestId('bottom-status-bar-needs-you')).toContainText(/Needs you\d+/);
    for (const gone of ['agents', 'queue', 'budget', 'model', 'openrouter', 'approvals']) {
      await expect(page.getByTestId(`bottom-status-bar-${gone}`)).toHaveCount(0);
    }

    await bar.getByTestId('bottom-status-bar-spend').click();
    const popover = page.getByRole('dialog', { name: 'Spend detail' });
    await expect(popover).toContainText('Engine total');
    await expect(popover).toContainText('$1.50');
    await expect(popover).toContainText('This session');
    await expect(popover).toContainText('not metered');

    mkdirSync(OUT_DIR, { recursive: true });
    await page.screenshot({ path: join(OUT_DIR, 'status-bar-cards.png'), clip: { x: 0, y: 620, width: 1440, height: 280 } });

    await page.getByRole('button', { name: /configure status bar/i }).click();
    for (const name of ['Engine', 'Spend', 'Mesh', 'Routing', 'Needs you']) {
      await expect(page.getByRole('checkbox', { name, exact: true })).toBeVisible();
    }
  });

  test('Routing shows the concrete id only for a catalog-resolved pick', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await addRichMockInitScript(page, 'chat');
    await addInvokeOverrides(page, { responses: { get_routing_summary_live: routingSummary('catalog') } });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expect(page.getByTestId('bottom-status-bar-routing')).toContainText('Auto → acme/widget-flash-20260901');
  });
});
```

- [ ] **Step 3: Create `e2e/chat-surfaces-consolidation.spec.ts`:**

```ts
import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';
import { addInvokeOverrides, type InvokeOverrides } from './lib/invokeOverrides';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

const SESSION = [
  { session_id: 'surfaces-session', title: 'Surfaces', updated_at: 'now', message_count: 0, conversation_id: 1 },
];

async function openChat(page: Page, overrides: InvokeOverrides): Promise<void> {
  await page.setViewportSize({ width: 1440, height: 900 });
  await addMockInitScript(page, installTauriMock, 'chat');
  await addInvokeOverrides(page, {
    ...overrides,
    responses: { chat_list_sessions: SESSION, ...(overrides.responses ?? {}) },
  });
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
  const sessionTab = page.getByRole('tab', { name: /Surfaces/i });
  if (await sessionTab.isVisible()) await sessionTab.click();
}

test('the rail shows this session: next-turn Routing from the contract-family mock, lock chip, no roster, no global resources', async ({ page }) => {
  await openChat(page, {
    responses: {
      list_orchestrator_tasks: [
        {
          id: 77,
          description: 'Migrate orders table',
          priority: 'normal',
          lifecycle: 'in_progress',
          agent_id: 3,
          session_id: 'surfaces-session',
          estimated_complexity: 1,
          depends_on: [],
          write_files: [],
          remote_node: null,
          origin: 'orchestrator',
        },
      ],
      activity_query: [
        {
          id: 10,
          ts_ms: 1,
          agent_id: '3',
          session_id: 'surfaces-session',
          kind: 'LockAcquired',
          summary: 'Lock acquired',
          detail_json: JSON.stringify({
            type: 'lock_acquired',
            agent_id: 3,
            path: 'db://orders/42',
            exclusive: true,
            session_id: 'surfaces-session',
            task_id: 77,
          }),
        },
      ],
    },
  });

  const routing = page.getByRole('region', { name: 'Routing' });
  // Seam: App's one routing query answers from tauriMock (Task 8): family-keyed, resolved_from 'bootstrap'.
  await expect(routing.getByTestId('execution-rail-routing-scope')).toHaveText('next turn');
  await expect(routing.getByTestId('execution-rail-routing')).toHaveText(
    'Routes to anthropic/claude-sonnet (offline) — lowest cost that fits the mode',
  );
  await routing.getByText('Why this model').click();
  await expect(routing.getByTestId('execution-rail-routing-state')).toHaveAttribute('title', /eligible for routing/);
  await expect(routing).toContainText('Alternatives: anthropic/claude-haiku, deepseek/deepseek-flash');
  await expect(page.getByRole('region', { name: /agent shards/i })).toHaveCount(0);
  await expect(page.getByLabel('Resource strip')).toHaveCount(0);
  await expect(
    page.getByRole('region', { name: /active tasks/i }).getByTestId('execution-rail-lock-chip'),
  ).toContainText('holding db://orders/42');
  await expect(page.getByTestId('bottom-status-bar-routing')).toContainText('Auto → anthropic/claude-sonnet (offline)');

  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'chat-rail-routing.png') });
});

test('the composer names modes in full, keeps Check replies under Risk, and repeats no global spend', async ({ page }) => {
  await openChat(page, {});

  for (const name of ['Free', 'Efficient', 'Balanced', 'Genius']) {
    await expect(page.getByRole('radio', { name, exact: true })).toBeVisible();
  }
  await page.getByRole('radio', { name: 'Efficient', exact: true }).hover();
  await expect(page.getByTestId('drive-mode-hint')).toContainText('Most out of the tokens you spend');
  await expect(page.getByText(/session \$/)).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Run (Enter)' })).toContainText('↵');
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'composer-modes.png') });

  await page.getByRole('button', { name: /^Risk: / }).click();
  const risk = page.getByRole('dialog', { name: /acceptable risk/i });
  await expect(risk.getByRole('button', { name: /check replies (on|off)/i })).toBeVisible();
  await expect(risk).not.toContainText(/grounding/i);
  await expect(page.getByRole('button', { name: /grounding check/i })).toHaveCount(0);
  await page.screenshot({ path: join(OUT_DIR, 'composer-risk-check-replies.png') });
});

test('the research popover shows only reported provider state', async ({ page }) => {
  await openChat(page, {});

  await page.getByTestId('status-bar-cluster-trigger').click();
  const popover = page.getByTestId('status-bar-cluster-popover');
  await expect(popover.getByTestId('status-bar-cluster-provider-wikipedia')).toContainText('on');
  await expect(popover).not.toContainText('✓');
  await expect(popover).not.toContainText('Online');
  await expect(popover).not.toContainText('Plaintext');
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'research-popover-honest.png') });
});

test('a failed research status reads "unknown" with no green checks', async ({ page }) => {
  await openChat(page, { reject: ['get_research_engine_status'] });

  await page.getByTestId('status-bar-cluster-trigger').click();
  await expect(page.getByTestId('status-bar-cluster-unknown')).toBeVisible();
  await expect(page.getByTestId('status-bar-cluster-trigger')).toContainText('unknown');
  await expect(page.getByTestId('status-bar-cluster-popover')).not.toContainText('✓');
});
```

- [ ] **Step 4: Run**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/status-bar-surfaces.spec.ts e2e/chat-surfaces-consolidation.spec.ts e2e/chat-trust-chips.spec.ts --project=chromium --reporter=line 2>&1 | tail -30 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -10`
Expected: every test passes (the two original status-bar tests and the Phase 5 holding/waiting lock tests in
`chat-trust-chips.spec.ts` included); the five PNGs exist under `crates/vox-gui/ui/review-bundle/latest/`. A failure
that names a selector from Tasks 2–7 is a STOP (report the selector and the received text), not a reason to edit
source here. Exit 124 (hung) is a STOP.

- [ ] **Step 5: Mutation proof — the rail and the card really follow the mock's `resolved_from`.** In
  `e2e/lib/tauriMock.ts` change `resolved_from: 'bootstrap', reason:` to `resolved_from: 'catalog', reason:`. Run
  `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/chat-surfaces-consolidation.spec.ts --project=chromium --reporter=line -g "the rail shows" > target/3a-t9-mutant.txt 2>&1; tail -20 target/3a-t9-mutant.txt`:
  the rail test must FAIL (the line reads `Routes to anthropic/claude-sonnet — …` without `(offline)`). Restore;
  `git diff -- crates/vox-gui/ui/e2e/lib/tauriMock.ts` prints nothing.

- [ ] **Step 6: Commit (Claude Code)** — after viewing the five screenshots.

```bash
cd /Users/brbrainerd/dev/vox
F="crates/vox-gui/ui/e2e/lib/invokeOverrides.ts crates/vox-gui/ui/e2e/status-bar-surfaces.spec.ts crates/vox-gui/ui/e2e/chat-surfaces-consolidation.spec.ts"
git add -- ${=F}
git commit -m "test(gui): Playwright specs for status bar cards, rail, composer, research" -- ${=F}
```

---

## Decisions (resolved 2026-09-28; open decisions delegated to Claude)

<!-- AMENDED: T1/T3 — decisions 3, 4, 9 and 11 revised or added after review. -->

1. **HUD tile ids stay stable; two are retired, not renamed.** `active_agents` renders the Engine card, `budget_burn`
   Spend, `active_model` Routing, `pending_approvals` Needs you. `queue_depth` and `openrouter_spend` are retired and
   *dropped on load* rather than failing validation, so a user's other tile choices survive (Task 2).
2. **Spend's "local" line reads `not metered`.** Nothing meters local inference cost; the popover says so instead of
   deriving `total − OpenRouter` across two different accounting windows.
3. **One routing-label rule, owned by the trace plan.** `routingModelLabel` in `lib/turnEvents.ts`: catalog ⇒
   `resolved_id`; local ⇒ `<resolved_id> (local)`; bootstrap ⇒ `<family> (offline)`. This plan only adapts the global
   `RoutingSummary` to it (`lib/routingSummary.ts`), treating a daemon without `resolved_from` as bootstrap and requiring
   a decision before any label appears. Mode names come from the trace plan's `MODE_NAMES` the same way.
4. **The rail's state line shows `decision_preview.discovery_state`, which the server fills with `ModelConfidence`
   (`confirmed | provisional | shadowed | deprecated`)**, not explore/exploit (the old mock's `exploit` was invented).
   The tooltip covers exactly those four values; speculative bandit hints were dropped.
5. **`plan` send mode is added, not removed**: it is wired end to end (`buildChatTurn` → `execution: 'plan'`, the drive
   bus's `setExecution('plan')`), and the old trigger mislabelled it "Background task".
6. **Run hint is `↵`, aria `Run (Enter)`**, matching Stop. Enter sends; ⌘/Ctrl+Enter also still works.
7. **The guard pattern extends the brief's list with bare `(opus|sonnet|haiku|fable)-\d`**: the fake ids that reached
   screenshots (`opus-4-8`) had no `claude-` prefix. Local names (`ollama/`, `mens/`, `local/`, `mesh/`, the MENS base
   `Qwen/Qwen3-8B`) are exempt by token prefix, after backslashes and a leading regex `/` are stripped.
8. **Version-shaped test ids use the fictional vendor `acme/…`**: proving "the version is hidden unless catalog" needs
   a version to hide, and a real vendor's would trip the guard.
9. **One routing fetch, one home.** App runs the only `get_routing_summary_live` query; the status bar's Routing card
   and the rail (`chatRouting`, computed in App) both read it. The hook no longer fetches routing or orchestrator status.
   The card and rail read `decision_preview.selected_model`, not `active_model` (a GUI-process preference).
10. **Mesh has one source (`useMeshNodes`).** The status bar shows `—` until it answers; the rail no longer shows mesh.
11. **The rail's Routing section is labelled "next turn"** because it is the engine's global preview, not a turn's
    decision; each turn's own `routing_decision` renders in the trace plan's turn trace.
12. **Status-bar values are truncated (max 28ch) with the full text in `title`** — catalog ids and reasons are
    server data but long. The rail's routing line truncates the same way.
13. **The clutch buttons lose their native `title`** in favour of the visible hover/focus hint linked by
    `aria-describedby` (keyboard users get it too; two tooltips would duplicate it).

## Deferred

- **Engine card popover** listing busy agents (it links to Agents today).
- Tasks 24–27 of the index (labels, colour tokens, type scale/contrast/glows, orphans incl. `ChatModelPicker`) are the
  visual-language plan (`2026-09-28-chat-visual-language.md`). `ChatModelPicker.test.tsx` is fixed here only because the
  guard scans it.
- A metered spend source for local/mesh inference, and a "Responsive" mode (routing does not define one yet).

## Execution Order

<!-- AMENDED: all — 9 tasks; runs after the trace plan and Phase 5, before the visual-language plan. -->

- **Place in the program:** after Phase 5 (all of 05-0x, including 05-05's waiting-lock rows) and after the trace plan
  (`2026-09-28-chat-turn-trace.md`, which lands the `RoutingSummary` fields, `routingModelLabel` and `MODE_NAMES`);
  **before** the visual-language plan (`2026-09-28-chat-visual-language.md`), which restyles the surfaces this plan
  reshapes.
- **Sequence:** 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9. Task 7 touches no other task's files and may run any time after
  Task 1. Tasks 3 and 4 are committed together (typecheck is red between them).
- **Shared files (sequential):** `src/App.tsx` (2 → 4 → 5); `Loquela.tsx` (5 → 6); `Loquela.test.tsx` (5 → 6 → 8);
  `src/config/budget.ts` (1 → 2); `e2e/lib/tauriMock.ts` (8 → 9's temporary mutation).
- **Pre-flight (Claude, before Task 1):** the trace plan's outputs exist — `RoutingSummary.family/resolved_from/reason`
  in `types/tauri.ts`, populated by `crates/vox-gui/src/commands/models.rs#get_routing_summary` (the Tauri command
  behind `get_routing_summary_live`), and `routingModelLabel` plus `export const MODE_NAMES` (a frozen `Record<"free"|"efficiency"|"balanced"|"genius", string>`) and `export function modeLabel(wire: string): string` in
  `src/lib/turnEvents.ts` (amended trace plan, verified 2026-09-28: `export const MODE_NAMES = Object.freeze({…})` and
  `export function modeLabel(wire)`). This plan uses `modeLabel` to read names; if the trace plan lands differently,
  fix the name in Tasks 1 and 5 before driving.
- **Suggested models:** `gemini-3.8-flash-medium` for Tasks 1, 5, 6, 7; `-high` for Tasks 2, 3, 4 (multi-file anchored
  edits) and 8 (many literal replacements).
- **Plan-end gates (Claude):** `timeout 600s pnpm --dir crates/vox-gui/ui exec vitest run`, `typecheck`, the Task 9
  Playwright command, then `vox ci pre-push` (fast tier). View every screenshot.
- **SDD ledger:**
  - D1 tile ids stable, two retired and dropped on load — ruling: settled
  - D2 local spend "not metered" — ruling: settled
  - D3 routing labels and mode names owned by the trace plan (`routingModelLabel`, `MODE_NAMES`); adapter requires a decision — ruling: settled (cross-plan)
  - D4 discovery_state is ModelConfidence; four hints, no speculative ones — ruling: settled
  - D5 add Plan send mode — ruling: settled
  - D6 Run hint `↵` / aria `Run (Enter)` — ruling: settled
  - D7 guard pattern + bare shortnames + backslash-aware local-prefix exemption — ruling: settled
  - D8 fictional `acme/` ids for version-hiding tests — ruling: settled
  - D9 one App-level routing query feeds card and rail; hook fetches tasks only — ruling: settled
  - D10 mesh single source, `—` before data — ruling: settled
  - D11 rail Routing labelled "next turn" — ruling: settled
  - D12 status-bar values truncated with full `title` — ruling: settled
  - D13 native clutch `title` dropped for the visible hint — ruling: settled
  - Rail split into Task 3 (component + hook, micro-edits; 05-05 shares the hook body) and Task 4 (plumbing) — settled
  - `App.tsx` 2 → 4 → 5, `Loquela.tsx` 5 → 6, `Loquela.test.tsx` 5 → 6 → 8 — sequential — settled
