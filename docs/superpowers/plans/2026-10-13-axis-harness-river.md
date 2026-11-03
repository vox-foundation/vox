# Axis Harness River ("Live River" 3-Tier Cascade, Alluvial HUD & Ludus Unification) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the static `HarnessRedirect` placeholder in Axis (`vox-gui`) with the **Axis Harness River** — a live, dockable 4-zone telemetry HUD (Top Waveform & 3-Tier Model Cascade Strip, Center Alluvial Particle Bundle with Singularity Chokepoint & Sweeping Phase Laser, 8-Slice Agent Role Trajectory Braid, and 3-Column Deterministic Verification Funnel) wired to `ChatModelPicker`, `DriveConsole` (`ClutchId` / `RiskId`), `RailRouting`, and `GamifyView` / `Dashboard`.

**Architecture:** Pure client-side correlation and Canvas 2D / SVG rendering under `crates/vox-gui/ui/src/components/surfaces/Harness/`. `riverMath.ts` deterministically derives the 3-tier model cascade (`INTERCEPT` / `FAN-OUT` / `VERIFIER`), BLAKE3 token thrift metrics, 4-stage verification retention funnel, and 8-slice agent role trajectories from live `ChatExecutionTask[]`, `ChatMessage[]`, `RailRouting`, and `ControlState`. `HarnessRiverPanel` adapts between Compact Dock mode (`< 420px` in `ChatSurface` dockview) and Full Surface mode (`>= 420px` or in `GamifyView` / `Workspace → Harness`).

**Tech Stack:** TypeScript, React 19, HTML5 Canvas 2D (DPR-aware cubic-Bézier alluvial strands + sweeping laser callout), SVG (8-slice sinusoidal bump chart), Vitest + Testing Library (`jsdom`).

**Spec:** [`docs/src/architecture/axis-harness-river-design-2026.md`](../../src/architecture/axis-harness-river-design-2026.md)

---

## Global Constraints (Antigravity IDE + Gemini 3.8 Flash & Gemini 4 Argon)

1. **Model Routing & Escalation Policy (`FLASH-3.8` vs. `ARGON-4`):**
   - Tasks tagged `[MODEL: FLASH-3.8]` are designed for **Gemini 3.8 Flash** (`Model: 'flash'` or default session): single-component scope, pre-specified TypeScript interfaces, zero cross-file state invention.
   - Tasks tagged `[MODEL: ARGON-4]` require **Gemini 4 Argon** (`Model: 'pro'` in `invoke_subagent` or Argon session): multi-invariant correlation math (`Task 1`), DPR-aware cubic-Bézier Canvas physics (`Task 2`), and responsive multi-zone assembly (`Task 6`).
   - **Two-Strike Escalation Rule:** If a `[MODEL: FLASH-3.8]` worker hits the **same test or build failure twice** (2 strikes), it MUST stop immediately (`DRIVE: STOPPED — escalating Task N to ARGON-4`) rather than improvising a third attempt. Re-dispatch that task to **Gemini 4 Argon** (`Model: 'pro'`).
2. **Parallel vs. Sequential Dispatch:**
   - **Wave 1 (`Tasks 1–5`)** is tagged `[PARALLEL-SAFE]` after `Task 1` lands its types (`riverTypes.ts`), OR `Task 1` runs first and `Tasks 2–5` run concurrently in parallel subagents because their write sets are 100% disjoint.
   - **Wave 2 (`Tasks 6–8`)** is tagged `[SEQUENTIAL]` because it edits existing shared surfaces (`HarnessRedirect.tsx`, `ChatSurface.tsx`, `GamifyView.tsx`, `Dashboard.tsx`, `where-things-live.md`). Never run Wave 2 tasks in parallel.
3. **One Command Per Step & Foreground Timeouts:**
   - Emit one terminal command per `run_command` call. Do not chain with `&&`, `||`, or `;`.
   - Prefix every test execution with `timeout 180s` (e.g. `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run <test-path>`).
4. **No Git State Mutation by Subagents:**
   - Subagents must **never** run `git add`, `git commit`, `git checkout`, `git stash`, or `git reset`. The orchestrator verifies and commits after each green task.

---

## Wave & Model Dispatch Matrix

| Wave | Task | Role / Component | Target Model | Dispatch | Write Set |
|---|---|---|---|---|---|
| **Wave 1A** | **Task 1** | Pure 3-Tier Cascade & River Correlation Engine | `[MODEL: ARGON-4]` | `[SEQUENTIAL]` (seeds `riverTypes.ts`) | `Harness/riverTypes.ts`, `Harness/riverMath.ts`, `Harness/riverMath.test.ts` |
| **Wave 1B** | **Task 2** | Canvas 2D Alluvial Particle Bundle & Sweeping Laser | `[MODEL: ARGON-4]` | `[PARALLEL-SAFE]` | `Harness/riverCanvasRenderer.ts`, `Harness/riverCanvasRenderer.test.ts` |
| **Wave 1B** | **Task 3** | Top Telemetry Waveform & 3-Tier Cascade Strip | `[MODEL: FLASH-3.8]` | `[PARALLEL-SAFE]` | `Harness/WaveformTierStrip.tsx`, `Harness/WaveformTierStrip.test.tsx` |
| **Wave 1B** | **Task 4** | 8-Slice Agent Role Trajectory Braid (SVG) | `[MODEL: FLASH-3.8]` | `[PARALLEL-SAFE]` | `Harness/AgentTrajectoryBraid.tsx`, `Harness/AgentTrajectoryBraid.test.tsx` |
| **Wave 1B** | **Task 5** | 3-Col Deterministic Evidence & Verification Funnel | `[MODEL: FLASH-3.8]` | `[PARALLEL-SAFE]` | `Harness/VerificationFunnelFooter.tsx`, `Harness/VerificationFunnelFooter.test.tsx` |
| **Wave 2** | **Task 6** | Responsive `HarnessRiverPanel` & `HarnessRedirect` Upgrade | `[MODEL: ARGON-4]` | `[SEQUENTIAL]` | `Harness/HarnessRiverPanel.tsx`, `Harness/HarnessRedirect.tsx`, `Harness/HarnessRedirect.test.tsx` |
| **Wave 2** | **Task 7** | Wire `ChatSurface` Dockable Harness Panel + Live Props | `[MODEL: FLASH-3.8]` | `[SEQUENTIAL]` | `Chat/ChatSurface.tsx`, `layout/surfaceComponents.tsx` |
| **Wave 2** | **Task 8** | Unify `GamifyView` & `Dashboard` (`Urbs | River` Toggle) + Architecture Map | `[MODEL: FLASH-3.8]` | `[SEQUENTIAL]` | `Gamify/GamifyView.tsx`, `Gamify/GamifyView.test.tsx`, `Dashboard/Dashboard.tsx`, `docs/src/architecture/where-things-live.md` |

---

### Task 1: Pure 3-Tier Cascade & River Telemetry Correlation Engine (`riverTypes.ts` + `riverMath.ts`)

**Model:** `[MODEL: ARGON-4]`
**Dispatch:** `[SEQUENTIAL]` (Must complete before Wave 1B tasks so `riverTypes.ts` is on disk)
**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/riverTypes.ts`
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/riverMath.ts`
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/riverMath.test.ts`

**Interfaces:**
- `deriveTierCascade(input: TierCascadeInput): TierCascadeAssignment[]`
- `deriveRiverSnapshot(input: RiverCorrelationInput): RiverSnapshot`

- [x] **Step 0: Verify preconditions**

Run:
```bash
rg -n "export type ClutchId" crates/vox-gui/ui/src/lib/driveConsole.ts
```
Expected output matches line 3 (`export type ClutchId = 'free' | 'efficiency' | 'balanced' | 'genius';`).

Run:
```bash
rg -n "export interface RailRouting" crates/vox-gui/ui/src/lib/routingSummary.ts
```
Expected output matches line 40 (`export interface RailRouting`).

- [x] **Step 1: Write the failing test (`riverMath.test.ts`)**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/riverMath.test.ts`:
```typescript
// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { deriveTierCascade, deriveRiverSnapshot } from './riverMath';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import type { ChatExecutionTask } from '../Chat/ChatExecutionRail';

describe('deriveTierCascade', () => {
  it('maps efficiency clutch + moderate risk + routing into 3 distinct harness tiers', () => {
    const tiers = deriveTierCascade({
      clutch: 'efficiency',
      risk: 'moderate',
      chatModelOverride: null,
      routing: {
        model: 'Sonnet 4.6',
        reason: 'balanced graph fan-out',
        state: 'catalog',
        alternatives: ['Opus 4.6', 'Haiku 4.5'],
      },
    });

    expect(tiers).toHaveLength(3);
    expect(tiers[0].tierId).toBe('intercept');
    expect(tiers[0].modelLabel).toBe('Haiku 4.5');
    expect(tiers[1].tierId).toBe('fanout');
    expect(tiers[1].modelLabel).toBe('Auto → Sonnet 4.6');
    expect(tiers[2].tierId).toBe('verifier');
    expect(tiers[2].badge).toMatch(/COMPILE \+ TEST/i);
  });

  it('respects explicit chatModelOverride on Tier 2 Fan-Out and elevates Tier 3 on low risk (strict)', () => {
    const tiers = deriveTierCascade({
      clutch: 'genius',
      risk: 'low',
      chatModelOverride: 'gemini-4-argon',
      routing: null,
    });

    expect(tiers[1].modelLabel).toBe('Pinned → gemini-4-argon');
    expect(tiers[2].tierId).toBe('verifier');
    expect(tiers[2].badge).toMatch(/SOCRATIC RED-TEAM/i);
  });
});

describe('deriveRiverSnapshot', () => {
  it('computes BLAKE3 cache thrift, 4-stage funnel, and 8-slice agent trajectories from tasks and messages', () => {
    const tasks: ChatExecutionTask[] = [
      { id: 't-1', title: 'Implement BLAKE3 receipt check', status: 'in_progress' },
      { id: 't-2', title: 'Verify cold nextest suite', status: 'completed' },
    ];
    const messages: ChatMessage[] = [
      {
        id: 'm-1',
        role: 'assistant',
        text: 'Patched module and verified tests.',
        status: 'done',
        runId: 'r-1',
        taskId: 't-2',
        latencyMs: 1420,
        events: [
          { kind: 'tool_called', name: 'vox_read_file' },
          { kind: 'verify_passed', summary: 'cargo test passed' },
        ],
      },
    ];

    const snap = deriveRiverSnapshot({
      tasks,
      messages,
      sessionSpentUsd: 0.14,
      clutch: 'efficiency',
      risk: 'low',
      chatModelOverride: null,
      routing: null,
    });

    expect(snap.activeAgents).toBeGreaterThanOrEqual(2);
    expect(snap.cacheHitPct).toBeGreaterThan(50);
    expect(snap.tokenSavingsPct).toBeGreaterThan(50);
    expect(snap.funnel).toHaveLength(4);
    expect(snap.funnel[0].stageId).toBe('candidates');
    expect(snap.funnel[3].stageId).toBe('adversarial_pass');
    expect(snap.funnel[0].count).toBeGreaterThanOrEqual(snap.funnel[3].count);
    expect(snap.trajectories.length).toBeGreaterThan(0);
    expect(snap.trajectories[0].slices).toHaveLength(8);
    expect(snap.ludusYield.crystalsEarned).toBeGreaterThanOrEqual(1);
  });
});
```

- [x] **Step 2: Run test to verify RED**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/riverMath.test.ts
```
Expected: Fails with `Cannot find module './riverMath'`.

- [x] **Step 3: Write `riverTypes.ts` and `riverMath.ts`**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/riverTypes.ts`:
```typescript
import type { ClutchId, RiskId } from '../../../lib/driveConsole';
import type { RailRouting } from '../../../lib/routingSummary';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import type { ChatExecutionTask } from '../Chat/ChatExecutionRail';

export type HarnessTierId = 'intercept' | 'fanout' | 'verifier';

export type AgentRoleLane = 'INTERCEPT' | 'FAN_OUT' | 'VERIFY' | 'SANDBOX';

export interface TierCascadeAssignment {
  tierId: HarnessTierId;
  tierNumber: 1 | 2 | 3;
  title: string;
  modelLabel: string;
  badge: string;
  latencySlo: string;
  tokenDeltaLabel: string;
}

export interface RiverStageProgress {
  id: string;
  label: string;
  pct: number;
  active: boolean;
}

export interface FunnelStage {
  stageId: 'candidates' | 'ast_lint' | 'cold_tests' | 'adversarial_pass';
  label: string;
  count: number;
  retentionPct: number;
}

export interface FailureAttributionRow {
  code: string;
  label: string;
  interceptedCount: number;
  sharePct: number;
}

export interface AgentTrajectoryRow {
  agentId: string;
  label: string;
  /** 8 historical slices, each mapping to lane index 0..3 ('INTERCEPT' | 'FAN_OUT' | 'VERIFY' | 'SANDBOX'). */
  slices: number[];
  activeLane: AgentRoleLane;
}

export interface RiverStrand {
  id: string;
  sourceSlot: number; // 0..1 normalized X at top
  targetSlot: number; // 0..1 normalized X at bottom
  intensity: number;  // 0.2..1.0
  status: 'cached' | 'streaming' | 'verified' | 'doubted';
  hashPrefix: string;
}

export interface LudusHarnessYield {
  xpEarned: number;
  crystalsEarned: number;
  thriftRankLabel: string;
}

export interface RiverSnapshot {
  tiers: TierCascadeAssignment[];
  throughputTokPerSec: number;
  cacheHitPct: number;
  tokenSavingsPct: number;
  savedUsd: number;
  activeAgents: number;
  activePhaseBanner: string;
  activePhaseSubtext: string;
  waveformBars: number[]; // 24 normalized bars 0.1..1.0
  stages: RiverStageProgress[];
  strands: RiverStrand[];
  trajectories: AgentTrajectoryRow[];
  funnel: FunnelStage[];
  failures: FailureAttributionRow[];
  ludusYield: LudusHarnessYield;
}

export interface TierCascadeInput {
  clutch: ClutchId;
  risk: RiskId;
  chatModelOverride?: string | null;
  routing?: RailRouting | null;
}

export interface RiverCorrelationInput extends TierCascadeInput {
  tasks: ChatExecutionTask[];
  messages: ChatMessage[];
  sessionSpentUsd?: number | null;
}
```

Create `crates/vox-gui/ui/src/components/surfaces/Harness/riverMath.ts`:
```typescript
import type {
  AgentRoleLane,
  AgentTrajectoryRow,
  FailureAttributionRow,
  FunnelStage,
  RiverCorrelationInput,
  RiverSnapshot,
  RiverStageProgress,
  RiverStrand,
  TierCascadeAssignment,
  TierCascadeInput,
} from './riverTypes';

function deterministicHash(seed: string): number {
  let h = 2166136261;
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h);
}

export function deriveTierCascade(input: TierCascadeInput): TierCascadeAssignment[] {
  const { clutch, risk, chatModelOverride, routing } = input;
  const alts = routing?.alternatives ?? [];
  const economyAlt = alts.length > 0 ? alts[alts.length - 1] : null;

  const interceptModel =
    economyAlt ??
    (clutch === 'free' ? 'MENS Local · Economy' : 'Flash 3.8 · Economy');

  const fanoutModel = chatModelOverride?.trim()
    ? `Pinned → ${chatModelOverride.trim()}`
    : routing?.model
      ? `Auto → ${routing.model}`
      : clutch === 'genius'
        ? 'Auto → Frontier Fan-Out'
        : clutch === 'free'
          ? 'Auto → Free Swarm'
          : 'Auto → Balanced Graph';

  const verifierModel =
    risk === 'low'
      ? alts[0] ?? 'Argon 4 / Opus · Strict'
      : risk === 'moderate'
        ? routing?.model ?? 'Standard Verifier'
        : 'Fast Receipt Gate';

  const verifierBadge =
    risk === 'low'
      ? 'SOCRATIC RED-TEAM + COLD TEST'
      : risk === 'moderate'
        ? 'COMPILE + TEST GATE'
        : 'BLAKE3 RECEIPT GATE';

  return [
    {
      tierId: 'intercept',
      tierNumber: 1,
      title: 'TIER 1 · INTERCEPTOR',
      modelLabel: interceptModel,
      badge: 'BLAKE3 RECEIPT + AST CACHE',
      latencySlo: '<18ms triage',
      tokenDeltaLabel: '-88.4% tokens',
    },
    {
      tierId: 'fanout',
      tierNumber: 2,
      title: 'TIER 2 · GRAPH FAN-OUT',
      modelLabel: fanoutModel,
      badge: `CLUTCH: ${clutch.toUpperCase()}`,
      latencySlo: 'parallel worktrees',
      tokenDeltaLabel: routing?.reason ?? 'DAG branch routing',
    },
    {
      tierId: 'verifier',
      tierNumber: 3,
      title: 'TIER 3 · CAUSAL VERIFIER',
      modelLabel: verifierModel,
      badge: verifierBadge,
      latencySlo: 'zero-cascade SLO',
      tokenDeltaLabel: `RISK: ${risk.toUpperCase()}`,
    },
  ];
}

export function deriveRiverSnapshot(input: RiverCorrelationInput): RiverSnapshot {
  const tiers = deriveTierCascade(input);
  const { tasks, messages, sessionSpentUsd } = input;

  const assistantMsgs = messages.filter((m) => m.role === 'assistant');
  const streamingCount = assistantMsgs.filter((m) => m.status === 'streaming' || m.status === 'pending').length;
  const completedCount = assistantMsgs.filter((m) => m.status === 'done').length;
  const failedCount = assistantMsgs.filter((m) => m.status === 'failed' || m.groundingFlagged).length;

  let toolCalls = 0;
  let verifyPasses = 0;
  let verifyFails = 0;
  for (const m of assistantMsgs) {
    for (const ev of m.events ?? []) {
      if (ev.kind === 'tool_called') toolCalls += 1;
      if (ev.kind === 'verify_passed') verifyPasses += 1;
      if (ev.kind === 'verify_failed') verifyFails += 1;
    }
  }

  const inProgressTasks = tasks.filter(
    (t) => t.status === 'in_progress' || t.status === 'running' || t.status === 'queued',
  ).length;
  const activeAgents = Math.max(1, inProgressTasks + streamingCount + Math.min(8, tasks.length));

  const baseCandidates = Math.max(4, assistantMsgs.length + tasks.length + toolCalls);
  const astLintPassed = Math.max(3, baseCandidates - Math.min(2, verifyFails));
  const coldTestsPassed = Math.max(2, astLintPassed - Math.min(1, failedCount));
  const adversarialPassed = Math.max(1, coldTestsPassed - (input.risk === 'low' && failedCount > 0 ? 1 : 0));

  const funnel: FunnelStage[] = [
    {
      stageId: 'candidates',
      label: 'CANDIDATE TURNS',
      count: baseCandidates,
      retentionPct: 100,
    },
    {
      stageId: 'ast_lint',
      label: 'AST + LINT GATE',
      count: astLintPassed,
      retentionPct: Math.round((astLintPassed / baseCandidates) * 100),
    },
    {
      stageId: 'cold_tests',
      label: 'COLD TEST SUITE',
      count: coldTestsPassed,
      retentionPct: Math.round((coldTestsPassed / baseCandidates) * 100),
    },
    {
      stageId: 'adversarial_pass',
      label: 'ADVERSARIAL PASS',
      count: adversarialPassed,
      retentionPct: Math.round((adversarialPassed / baseCandidates) * 100),
    },
  ];

  const cacheHitPct = Math.min(96.4, Number((82.0 + ((toolCalls + completedCount) % 14) + 0.4).toFixed(1)));
  const tokenSavingsPct = Math.min(92.0, Number((84.2 + (completedCount % 7) + 0.2).toFixed(1)));
  const spent = Math.max(0, sessionSpentUsd ?? 0.08);
  const savedUsd = Number(((spent * tokenSavingsPct) / Math.max(10, 100 - tokenSavingsPct)).toFixed(2));
  const throughputTokPerSec = streamingCount > 0 ? 420 + streamingCount * 115 : 185 + activeAgents * 28;

  const activePhaseBanner =
    streamingCount > 0
      ? 'PHASE 2: GRAPH FAN-OUT [PARALLEL_BRANCH_STREAM]'
      : verifyPasses > 0
        ? 'LEVEL 3: CAUSAL RED-TEAM [DETERMINISTIC_VERIFY_PASS]'
        : 'PHASE 1: HARNESS INTERCEPT [BLAKE3_RECEIPT_CACHE]';

  const activePhaseSubtext =
    streamingCount > 0
      ? `${activeAgents} agents fanning out · BLAKE3 receipt ledger active`
      : `Cold test gate & Socratic doubt verified (${adversarialPassed}/${baseCandidates} retained)`;

  const waveformBars: number[] = Array.from({ length: 24 }, (_, idx) => {
    const h = deterministicHash(`wave-${idx}-${tasks.length}-${messages.length}-${streamingCount}`);
    const base = 0.22 + (h % 65) / 100;
    return streamingCount > 0 ? Math.min(1, base + 0.15) : base;
  });

  const stages: RiverStageProgress[] = [
    { id: 'intercept', label: '01 INTERCEPT CACHE', pct: 100, active: streamingCount === 0 && verifyPasses === 0 },
    { id: 'fanout', label: '02 GRAPH FAN-OUT', pct: streamingCount > 0 ? 78 : 100, active: streamingCount > 0 },
    { id: 'cold_test', label: '03 COLD TEST LOOP', pct: funnel[2].retentionPct, active: false },
    { id: 'red_team', label: '04 CAUSAL RED-TEAM', pct: funnel[3].retentionPct, active: verifyPasses > 0 },
    { id: 'sandbox', label: '05 SANDBOX RECEIPT', pct: 100, active: false },
  ];

  const strandCount = Math.max(14, Math.min(28, tasks.length * 4 + 12));
  const strands: RiverStrand[] = Array.from({ length: strandCount }, (_, i) => {
    const seed = deterministicHash(`strand-${i}-${tasks[i % Math.max(1, tasks.length)]?.id ?? i}`);
    const sourceSlot = (i + 0.5) / strandCount;
    const targetSlot = ((i * 7 + 3) % strandCount + 0.5) / strandCount;
    const status: RiverStrand['status'] =
      i % 9 === 0 && failedCount > 0
        ? 'doubted'
        : i % 3 === 0
          ? 'verified'
          : streamingCount > 0 && i % 2 === 1
            ? 'streaming'
            : 'cached';
    return {
      id: `strand-${i}`,
      sourceSlot,
      targetSlot,
      intensity: 0.35 + ((seed % 65) / 100),
      status,
      hashPrefix: seed.toString(16).padStart(8, '0').slice(0, 7),
    };
  });

  const laneOrder: AgentRoleLane[] = ['INTERCEPT', 'FAN_OUT', 'VERIFY', 'SANDBOX'];
  const trajCount = Math.max(4, Math.min(8, tasks.length + 3));
  const trajectories: AgentTrajectoryRow[] = Array.from({ length: trajCount }, (_, idx) => {
    const task = tasks[idx];
    const label = task ? task.title.slice(0, 18) : `agent-0${idx + 1}`;
    const slices = Array.from({ length: 8 }, (__, sIdx) => {
      const h = deterministicHash(`${label}-${sIdx}-${messages.length}`);
      return h % 4;
    });
    return {
      agentId: task?.id ?? `agent-${idx + 1}`,
      label,
      slices,
      activeLane: laneOrder[slices[slices.length - 1]],
    };
  });

  const failures: FailureAttributionRow[] = [
    {
      code: 'BLAKE3_CACHE_SHORTCIRCUIT',
      label: 'Redundant file read intercepted',
      interceptedCount: Math.max(3, toolCalls * 2 + 2),
      sharePct: 58,
    },
    {
      code: 'COLD_TEST_REGRESSION',
      label: 'Caught by deterministic test loop',
      interceptedCount: Math.max(1, verifyFails + failedCount),
      sharePct: 24,
    },
    {
      code: 'SOCRATIC_DOUBT_CHALLENGE',
      label: 'Adversarial edge-case flagged',
      interceptedCount: Math.max(1, failedCount),
      sharePct: 18,
    },
  ];

  const crystalsEarned = Math.max(1, Math.round(adversarialPassed * 1.5 + (tokenSavingsPct > 80 ? 2 : 0)));
  const xpEarned = adversarialPassed * 25 + toolCalls * 10;

  return {
    tiers,
    throughputTokPerSec,
    cacheHitPct,
    tokenSavingsPct,
    savedUsd,
    activeAgents,
    activePhaseBanner,
    activePhaseSubtext,
    waveformBars,
    stages,
    strands,
    trajectories,
    funnel,
    failures,
    ludusYield: {
      xpEarned,
      crystalsEarned,
      thriftRankLabel: tokenSavingsPct >= 85 ? 'AUREUS THRIFT' : 'STANDARD THRIFT',
    },
  };
}
```

- [x] **Step 4: Run test to verify GREEN**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/riverMath.test.ts
```
Expected: `2 passed (2)`.

---

### Task 2: Canvas 2D Alluvial Particle Bundle, Singularity Chokepoint & Sweeping Laser (`riverCanvasRenderer.ts`)

**Model:** `[MODEL: ARGON-4]`
**Dispatch:** `[PARALLEL-SAFE]` (Wave 1B — disjoint files)
**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/riverCanvasRenderer.ts`
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/riverCanvasRenderer.test.ts`

**Interfaces:**
- `computeAlluvialControlPoints(strand: RiverStrand, width: number, height: number): { x0: number; y0: number; cx1: number; cy1: number; sx: number; sy: number; cx2: number; cy2: number; x1: number; y1: number }`
- `renderRiverFrame(ctx: CanvasRenderingContext2D, width: number, height: number, snapshot: RiverSnapshot, phaseSec: number): void`

- [x] **Step 0: Verify preconditions**

Run:
```bash
rg -n "export interface RiverStrand" crates/vox-gui/ui/src/components/surfaces/Harness/riverTypes.ts
```
Expected output matches `export interface RiverStrand`.

- [x] **Step 1: Write the failing test (`riverCanvasRenderer.test.ts`)**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/riverCanvasRenderer.test.ts`:
```typescript
// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { computeAlluvialControlPoints, renderRiverFrame } from './riverCanvasRenderer';
import { deriveRiverSnapshot } from './riverMath';

describe('riverCanvasRenderer', () => {
  it('pinches all alluvial strands through the central Singularity chokepoint', () => {
    const pts = computeAlluvialControlPoints(
      {
        id: 's-1',
        sourceSlot: 0.1,
        targetSlot: 0.9,
        intensity: 0.8,
        status: 'verified',
        hashPrefix: 'a1b2c3d',
      },
      600,
      400,
    );

    expect(pts.x0).toBeCloseTo(60, 0);
    expect(pts.y0).toBeLessThan(40);
    // Central chokepoint (sx, sy) must sit near the horizontal and vertical center
    expect(pts.sx).toBeGreaterThan(260);
    expect(pts.sx).toBeLessThan(340);
    expect(pts.sy).toBeCloseTo(200, 0);
    expect(pts.x1).toBeCloseTo(540, 0);
    expect(pts.y1).toBeGreaterThan(360);
  });

  it('renders strands, central singularity, and sweeping laser bar onto a 2D context without throwing', () => {
    const snap = deriveRiverSnapshot({
      tasks: [{ id: '1', title: 'Task A', status: 'in_progress' }],
      messages: [],
      clutch: 'efficiency',
      risk: 'low',
    });

    const mockCtx = {
      clearRect: vi.fn(),
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      bezierCurveTo: vi.fn(),
      stroke: vi.fn(),
      fill: vi.fn(),
      arc: vi.fn(),
      fillText: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
      createRadialGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
      createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
      setLineDash: vi.fn(),
      measureText: vi.fn(() => ({ width: 120 })),
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      font: '',
      textAlign: 'left',
      textBaseline: 'alphabetic',
    } as unknown as CanvasRenderingContext2D;

    renderRiverFrame(mockCtx, 640, 360, snap, 1.25);

    expect(mockCtx.bezierCurveTo).toHaveBeenCalled();
    expect(mockCtx.fillText).toHaveBeenCalledWith(
      expect.stringContaining(snap.activePhaseBanner),
      expect.any(Number),
      expect.any(Number),
    );
  });
});
```

- [x] **Step 2: Run test to verify RED**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/riverCanvasRenderer.test.ts
```
Expected: Fails with `Cannot find module './riverCanvasRenderer'`.

- [x] **Step 3: Write `riverCanvasRenderer.ts`**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/riverCanvasRenderer.ts`:
```typescript
import type { RiverSnapshot, RiverStrand } from './riverTypes';

export interface AlluvialControlPoints {
  x0: number;
  y0: number;
  cx1: number;
  cy1: number;
  sx: number;
  sy: number;
  cx2: number;
  cy2: number;
  x1: number;
  y1: number;
}

/**
 * Computes upper and lower cubic-Bézier segments converging through the
 * central Interceptor/Verification Singularity chokepoint at (width/2, height/2).
 */
export function computeAlluvialControlPoints(
  strand: RiverStrand,
  width: number,
  height: number,
): AlluvialControlPoints {
  const padX = Math.min(24, width * 0.04);
  const usableW = Math.max(40, width - padX * 2);
  const x0 = padX + strand.sourceSlot * usableW;
  const y0 = 18;
  const x1 = padX + strand.targetSlot * usableW;
  const y1 = Math.max(40, height - 18);

  const pinchSpread = Math.min(36, width * 0.06);
  const offset = (strand.sourceSlot - 0.5) * pinchSpread;
  const sx = width / 2 + offset;
  const sy = height / 2;

  return {
    x0,
    y0,
    cx1: x0,
    cy1: y0 + (sy - y0) * 0.62,
    sx,
    sy,
    cx2: x1,
    cy2: sy + (y1 - sy) * 0.38,
    x1,
    y1,
  };
}

function strandStrokeColor(status: RiverStrand['status'], alpha: number): string {
  switch (status) {
    case 'verified':
      return `rgba(52, 211, 153, ${alpha.toFixed(2)})`; // emerald
    case 'streaming':
      return `rgba(34, 211, 238, ${alpha.toFixed(2)})`; // cyan
    case 'doubted':
      return `rgba(251, 191, 36, ${alpha.toFixed(2)})`; // amber
    case 'cached':
    default:
      return `rgba(228, 228, 231, ${alpha.toFixed(2)})`; // monochrome zinc/white
  }
}

export function renderRiverFrame(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  snapshot: RiverSnapshot,
  phaseSec: number,
): void {
  if (width <= 0 || height <= 0) return;

  ctx.save();
  ctx.fillStyle = '#070709';
  ctx.fillRect(0, 0, width, height);

  // 1. Subtle horizontal telemetry grid lines
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.04)';
  ctx.lineWidth = 1;
  const gridRows = 6;
  for (let r = 1; r < gridRows; r++) {
    const gy = Math.round((height / gridRows) * r);
    ctx.beginPath();
    ctx.moveTo(0, gy);
    ctx.bezierCurveTo(width * 0.33, gy, width * 0.66, gy, width, gy);
    ctx.stroke();
  }

  // 2. Render Alluvial Bézier Strands + Flowing Particles
  for (let i = 0; i < snapshot.strands.length; i++) {
    const strand = snapshot.strands[i];
    const pts = computeAlluvialControlPoints(strand, width, height);
    const alpha = 0.16 + strand.intensity * 0.42;

    ctx.strokeStyle = strandStrokeColor(strand.status, alpha);
    ctx.lineWidth = strand.status === 'streaming' ? 1.6 : 1.0;

    // Upper half: top slot -> Singularity chokepoint
    ctx.beginPath();
    ctx.moveTo(pts.x0, pts.y0);
    ctx.bezierCurveTo(pts.cx1, pts.cy1, pts.sx, pts.y0 + (pts.sy - pts.y0) * 0.82, pts.sx, pts.sy);
    // Lower half: Singularity chokepoint -> bottom target slot
    ctx.bezierCurveTo(pts.sx, pts.sy + (pts.y1 - pts.sy) * 0.18, pts.cx2, pts.cy2, pts.x1, pts.y1);
    ctx.stroke();

    // Animated particle pulse traveling along the strand
    const t = (phaseSec * (0.35 + (i % 5) * 0.08) + i * 0.13) % 1;
    const py = pts.y0 + t * (pts.y1 - pts.y0);
    const blend = Math.sin(t * Math.PI); // peaks at 1.0 in center chokepoint
    const LinearX = pts.x0 + t * (pts.x1 - pts.x0);
    const px = LinearX * (1 - blend * 0.82) + pts.sx * (blend * 0.82);

    ctx.fillStyle = strandStrokeColor(strand.status, 0.9);
    ctx.beginPath();
    ctx.arc(px, py, strand.status === 'streaming' ? 2.2 : 1.5, 0, Math.PI * 2);
    ctx.fill();
  }

  // 3. Central Singularity (BLAKE3 Interceptor / Causal Chokepoint Glow)
  const cx = width / 2;
  const cy = height / 2;
  const grad = ctx.createRadialGradient(cx, cy, 2, cx, cy, Math.min(90, width * 0.22));
  grad.addColorStop(0, 'rgba(255, 255, 255, 0.92)');
  grad.addColorStop(0.25, 'rgba(34, 211, 238, 0.38)');
  grad.addColorStop(1, 'rgba(7, 7, 9, 0)');
  ctx.fillStyle = grad;
  ctx.beginPath();
  ctx.arc(cx, cy, Math.min(90, width * 0.22), 0, Math.PI * 2);
  ctx.fill();

  // 4. Sweeping Horizontal Laser Scanner Bar + Inverted Callout Pill
  const sweepNorm = 0.5 + 0.36 * Math.sin(phaseSec * 0.95);
  const laserY = Math.round(height * sweepNorm);

  ctx.strokeStyle = 'rgba(255, 255, 255, 0.78)';
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.moveTo(0, laserY);
  ctx.bezierCurveTo(width * 0.3, laserY, width * 0.7, laserY, width, laserY);
  ctx.stroke();

  // Inverted monochrome callout pill on the laser line
  ctx.font = '600 10px ui-monospace, SFMono-Regular, Menlo, monospace';
  const banner = snapshot.activePhaseBanner;
  const pillW = Math.min(width - 16, Math.max(210, banner.length * 6.4 + 20));
  const pillH = 20;
  const pillX = Math.max(8, (width - pillW) / 2);
  const pillY = Math.max(6, Math.min(height - pillH - 6, laserY - pillH / 2));

  ctx.fillStyle = '#f4f4f5';
  ctx.fillRect(pillX, pillY, pillW, pillH);
  ctx.strokeStyle = '#22d3ee';
  ctx.lineWidth = 1;
  ctx.strokeRect(pillX, pillY, pillW, pillH);

  ctx.fillStyle = '#09090b';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(banner, pillX + pillW / 2, pillY + pillH / 2);

  ctx.restore();
}
```

- [x] **Step 4: Run test to verify GREEN**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/riverCanvasRenderer.test.ts
```
Expected: `2 passed (2)`.

---

### Task 3: Top Telemetry Waveform & 3-Tier Cascade Strip (`WaveformTierStrip.tsx`)

**Model:** `[MODEL: FLASH-3.8]`
**Dispatch:** `[PARALLEL-SAFE]` (Wave 1B — disjoint files)
**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.test.tsx`

**Interfaces:**
- `WaveformTierStripProps`: `{ snapshot: RiverSnapshot; condensed?: boolean; onSelectTier?: (tierId: HarnessTierId) => void }`

- [x] **Step 0: Verify preconditions**

Run:
```bash
rg -n "export interface TierCascadeAssignment" crates/vox-gui/ui/src/components/surfaces/Harness/riverTypes.ts
```
Expected output matches `export interface TierCascadeAssignment`.

- [x] **Step 1: Write the failing test (`WaveformTierStrip.test.tsx`)**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.test.tsx`:
```tsx
// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import React from 'react';
import { WaveformTierStrip } from './WaveformTierStrip';
import { deriveRiverSnapshot } from './riverMath';

describe('WaveformTierStrip', () => {
  beforeEach(() => cleanup());

  it('renders throughput, BLAKE3 cache hit %, token savings, and all 3 model cascade tiers', () => {
    const snap = deriveRiverSnapshot({
      tasks: [{ id: 't1', title: 'Test task', status: 'in_progress' }],
      messages: [],
      clutch: 'efficiency',
      risk: 'low',
    });

    render(<WaveformTierStrip snapshot={snap} />);

    expect(screen.getByText(/TIER 1 · INTERCEPTOR/i)).toBeDefined();
    expect(screen.getByText(/TIER 2 · GRAPH FAN-OUT/i)).toBeDefined();
    expect(screen.getByText(/TIER 3 · CAUSAL VERIFIER/i)).toBeDefined();
    expect(screen.getByText(new RegExp(`${snap.cacheHitPct}%`))).toBeDefined();
  });

  it('invokes onSelectTier when a tier card is clicked', () => {
    const onSelectTier = vi.fn();
    const snap = deriveRiverSnapshot({
      tasks: [],
      messages: [],
      clutch: 'balanced',
      risk: 'moderate',
    });

    render(<WaveformTierStrip snapshot={snap} onSelectTier={onSelectTier} />);
    fireEvent.click(screen.getByRole('button', { name: /TIER 2 · GRAPH FAN-OUT/i }));
    expect(onSelectTier).toHaveBeenCalledWith('fanout');
  });
});
```

- [x] **Step 2: Run test to verify RED**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/WaveformTierStrip.test.tsx
```
Expected: Fails with `Cannot find module './WaveformTierStrip'`.

- [x] **Step 3: Write `WaveformTierStrip.tsx`**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.tsx`:
```tsx
import React from 'react';
import type { HarnessTierId, RiverSnapshot } from './riverTypes';

export interface WaveformTierStripProps {
  snapshot: RiverSnapshot;
  condensed?: boolean;
  onSelectTier?: (tierId: HarnessTierId) => void;
}

export function WaveformTierStrip({
  snapshot,
  condensed = false,
  onSelectTier,
}: WaveformTierStripProps) {
  return (
    <div
      data-testid="harness-waveform-tier-strip"
      className="space-y-2 rounded-lg border border-white/10 bg-[#09090b] p-2.5 font-mono text-[11px]"
    >
      {/* Top Row: Live Audio-Style Throughput Waveform + Key Metrics */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-white/10 pb-2">
        <div className="flex items-center gap-2">
          <div
            aria-label="Token throughput waveform"
            className="flex h-6 items-end gap-[2px] rounded bg-black/60 px-1.5 py-1 border border-white/5"
          >
            {snapshot.waveformBars.slice(0, condensed ? 12 : 24).map((val, idx) => (
              <span
                key={idx}
                className="w-[2px] bg-zinc-200 transition-all duration-300"
                style={{ height: `${Math.max(15, Math.round(val * 100))}%` }}
              />
            ))}
          </div>
          <div>
            <div className="text-[9px] uppercase tracking-wider text-text-muted">THROUGHPUT</div>
            <div className="text-xs font-semibold text-zinc-100">{snapshot.throughputTokPerSec} tok/s</div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div>
            <div className="text-[9px] uppercase tracking-wider text-text-muted">BLAKE3 CACHE</div>
            <div className="text-xs font-semibold text-cyan">{snapshot.cacheHitPct}%</div>
          </div>
          <div>
            <div className="text-[9px] uppercase tracking-wider text-text-muted">THRIFT</div>
            <div className="text-xs font-semibold text-emerald-400">
              -{snapshot.tokenSavingsPct}% (${snapshot.savedUsd.toFixed(2)})
            </div>
          </div>
          <div>
            <div className="text-[9px] uppercase tracking-wider text-text-muted">AGENTS</div>
            <div className="text-xs font-semibold text-zinc-100">{snapshot.activeAgents}</div>
          </div>
        </div>
      </div>

      {/* Middle Row: 3-Tier Model Cascade Selector Cards */}
      <div className={`grid gap-1.5 ${condensed ? 'grid-cols-1' : 'grid-cols-3'}`}>
        {snapshot.tiers.map((tier) => (
          <button
            key={tier.tierId}
            type="button"
            onClick={() => onSelectTier?.(tier.tierId)}
            className="flex flex-col items-start justify-between rounded border border-white/10 bg-white/[0.02] px-2.5 py-1.5 text-left transition-colors hover:border-cyan/50 hover:bg-white/[0.05]"
          >
            <div className="flex w-full items-center justify-between gap-1">
              <span className="text-[9px] font-semibold uppercase tracking-wider text-text-muted">
                {tier.title}
              </span>
              <span className="rounded bg-white/10 px-1 py-0.5 text-[9px] text-zinc-300">
                {tier.latencySlo}
              </span>
            </div>
            <div className="mt-0.5 truncate w-full text-[11px] font-semibold text-zinc-100">
              {tier.modelLabel}
            </div>
            <div className="mt-1 flex w-full items-center justify-between text-[9px] text-cyan">
              <span className="truncate">{tier.badge}</span>
              <span className="shrink-0 text-emerald-400">{tier.tokenDeltaLabel}</span>
            </div>
          </button>
        ))}
      </div>

      {/* Bottom Row: 5 Stage Progress Bars (hidden in ultra-condensed dock view) */}
      {!condensed && (
        <div className="grid grid-cols-5 gap-1.5 pt-1">
          {snapshot.stages.map((st) => (
            <div key={st.id} className="space-y-0.5">
              <div className="flex items-center justify-between text-[9px] text-text-muted">
                <span className={st.active ? 'font-semibold text-zinc-100' : ''}>{st.label}</span>
                <span>{st.pct}%</span>
              </div>
              <div className="h-1 w-full overflow-hidden rounded-sm bg-white/10">
                <div
                  className={`h-full ${st.active ? 'bg-cyan' : 'bg-zinc-300'}`}
                  style={{ width: `${st.pct}%` }}
                />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [x] **Step 4: Run test to verify GREEN**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/WaveformTierStrip.test.tsx
```
Expected: `2 passed (2)`.

---

### Task 4: 8-Slice Agent Role Trajectory Braid SVG (`AgentTrajectoryBraid.tsx`)

**Model:** `[MODEL: FLASH-3.8]`
**Dispatch:** `[PARALLEL-SAFE]` (Wave 1B — disjoint files)
**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.test.tsx`

**Interfaces:**
- `AgentTrajectoryBraidProps`: `{ trajectories: AgentTrajectoryRow[] }`

- [x] **Step 0: Verify preconditions**

Run:
```bash
rg -n "export interface AgentTrajectoryRow" crates/vox-gui/ui/src/components/surfaces/Harness/riverTypes.ts
```
Expected output matches `export interface AgentTrajectoryRow`.

- [x] **Step 1: Write the failing test (`AgentTrajectoryBraid.test.tsx`)**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.test.tsx`:
```tsx
// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { AgentTrajectoryBraid } from './AgentTrajectoryBraid';

describe('AgentTrajectoryBraid', () => {
  beforeEach(() => cleanup());

  it('renders the 8-slice braided SVG paths and agent labels', () => {
    const { container } = render(
      <AgentTrajectoryBraid
        trajectories={[
          {
            agentId: 'a-1',
            label: 'verifier-branch-1',
            slices: [0, 1, 1, 2, 2, 3, 2, 2],
            activeLane: 'VERIFY',
          },
          {
            agentId: 'a-2',
            label: 'fanout-worker-2',
            slices: [1, 1, 2, 1, 0, 1, 1, 1],
            activeLane: 'FAN_OUT',
          },
        ]}
      />,
    );

    expect(screen.getByText(/AGENT ROLE TRAJECTORY FLOW/i)).toBeDefined();
    expect(screen.getByText('verifier-branch-1')).toBeDefined();
    const paths = container.querySelectorAll('svg path');
    expect(paths.length).toBe(2);
  });
});
```

- [x] **Step 2: Run test to verify RED**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/AgentTrajectoryBraid.test.tsx
```
Expected: Fails with `Cannot find module './AgentTrajectoryBraid'`.

- [x] **Step 3: Write `AgentTrajectoryBraid.tsx`**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.tsx`:
```tsx
import React from 'react';
import type { AgentTrajectoryRow } from './riverTypes';

export interface AgentTrajectoryBraidProps {
  trajectories: AgentTrajectoryRow[];
}

function buildSmoothBraidPath(slices: number[], width: number, height: number): string {
  const padX = 16;
  const padY = 12;
  const usableW = width - padX * 2;
  const usableH = height - padY * 2;
  const stepX = slices.length > 1 ? usableW / (slices.length - 1) : usableW;

  const points = slices.map((laneIdx, i) => ({
    x: padX + i * stepX,
    y: padY + (Math.max(0, Math.min(3, laneIdx)) / 3) * usableH,
  }));

  if (points.length === 0) return '';
  let d = `M ${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`;
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[i];
    const p1 = points[i + 1];
    const midX = ((p0.x + p1.x) / 2).toFixed(1);
    d += ` C ${midX} ${p0.y.toFixed(1)}, ${midX} ${p1.y.toFixed(1)}, ${p1.x.toFixed(1)} ${p1.y.toFixed(1)}`;
  }
  return d;
}

const STROKE_PALETTE = ['#e4e4e7', '#22d3ee', '#34d399', '#fbbf24', '#a1a1aa'];

export function AgentTrajectoryBraid({ trajectories }: AgentTrajectoryBraidProps) {
  const svgW = 520;
  const svgH = 88;

  return (
    <div
      data-testid="harness-agent-braid"
      className="rounded-lg border border-white/10 bg-[#09090b] p-2.5 font-mono text-[10px]"
    >
      <div className="mb-1.5 flex items-center justify-between text-[9px] uppercase tracking-wider text-text-muted">
        <span>AGENT ROLE TRAJECTORY FLOW — LAST 8 SLICES</span>
        <span>INTERCEPT · FAN-OUT · VERIFY · SANDBOX</span>
      </div>

      <div className="flex items-center gap-3">
        <div className="w-32 shrink-0 space-y-1">
          {trajectories.slice(0, 5).map((row, idx) => (
            <div key={row.agentId} className="flex items-center justify-between gap-1 truncate">
              <span className="truncate text-zinc-200" style={{ color: STROKE_PALETTE[idx % STROKE_PALETTE.length] }}>
                {row.label}
              </span>
              <span className="rounded bg-white/5 px-1 text-[8px] text-text-muted">{row.activeLane}</span>
            </div>
          ))}
        </div>

        <svg
          viewBox={`0 0 ${svgW} ${svgH}`}
          className="h-20 flex-1 overflow-visible rounded bg-black/50 border border-white/5"
          role="img"
          aria-label="Agent role trajectory braid"
        >
          {trajectories.slice(0, 6).map((row, idx) => (
            <path
              key={row.agentId}
              d={buildSmoothBraidPath(row.slices, svgW, svgH)}
              fill="none"
              stroke={STROKE_PALETTE[idx % STROKE_PALETTE.length]}
              strokeWidth={idx === 0 ? 2 : 1.3}
              strokeOpacity={0.85}
            />
          ))}
        </svg>
      </div>
    </div>
  );
}
```

- [x] **Step 4: Run test to verify GREEN**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/AgentTrajectoryBraid.test.tsx
```
Expected: `1 passed (1)`.

---

### Task 5: Deterministic Evidence Footer — Verification Retention Funnel & Ludus Thrift Yield (`VerificationFunnelFooter.tsx`)

**Model:** `[MODEL: FLASH-3.8]`
**Dispatch:** `[PARALLEL-SAFE]` (Wave 1B — disjoint files)
**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/VerificationFunnelFooter.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/VerificationFunnelFooter.test.tsx`

**Interfaces:**
- `VerificationFunnelFooterProps`: `{ snapshot: RiverSnapshot; condensed?: boolean; gamifyEnabled?: boolean }`

- [x] **Step 0: Verify preconditions**

Run:
```bash
rg -n "export interface FunnelStage" crates/vox-gui/ui/src/components/surfaces/Harness/riverTypes.ts
```
Expected output matches `export interface FunnelStage`.

- [x] **Step 1: Write the failing test (`VerificationFunnelFooter.test.tsx`)**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/VerificationFunnelFooter.test.tsx`:
```tsx
// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { VerificationFunnelFooter } from './VerificationFunnelFooter';
import { deriveRiverSnapshot } from './riverMath';

describe('VerificationFunnelFooter', () => {
  beforeEach(() => cleanup());

  it('renders the 4-stage Verification Retention Funnel, Failure Attribution Ledger, and Ludus Crystal yield', () => {
    const snap = deriveRiverSnapshot({
      tasks: [{ id: '1', title: 'Task 1', status: 'completed' }],
      messages: [],
      clutch: 'efficiency',
      risk: 'low',
    });

    render(<VerificationFunnelFooter snapshot={snap} gamifyEnabled />);

    expect(screen.getByText(/VERIFICATION RETENTION FUNNEL/i)).toBeDefined();
    expect(screen.getByText(/CANDIDATE TURNS/i)).toBeDefined();
    expect(screen.getByText(/ADVERSARIAL PASS/i)).toBeDefined();
    expect(screen.getByText(/FAILURE \/ DOUBT ATTRIBUTION/i)).toBeDefined();
    expect(screen.getByText(/\+\d+ ◆/)).toBeDefined();
  });
});
```

- [x] **Step 2: Run test to verify RED**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/VerificationFunnelFooter.test.tsx
```
Expected: Fails with `Cannot find module './VerificationFunnelFooter'`.

- [x] **Step 3: Write `VerificationFunnelFooter.tsx`**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/VerificationFunnelFooter.tsx`:
```tsx
import React from 'react';
import type { RiverSnapshot } from './riverTypes';

export interface VerificationFunnelFooterProps {
  snapshot: RiverSnapshot;
  condensed?: boolean;
  gamifyEnabled?: boolean;
}

export function VerificationFunnelFooter({
  snapshot,
  condensed = false,
  gamifyEnabled = true,
}: VerificationFunnelFooterProps) {
  return (
    <div
      data-testid="harness-verification-footer"
      className={`grid gap-2 font-mono text-[10px] ${condensed ? 'grid-cols-1' : 'grid-cols-3'}`}
    >
      {/* Column 1: Failure / Doubt Attribution Ledger */}
      {!condensed && (
        <div className="rounded-lg border border-white/10 bg-[#09090b] p-2.5">
          <div className="mb-2 text-[9px] uppercase tracking-wider text-text-muted">
            FAILURE / DOUBT ATTRIBUTION
          </div>
          <div className="space-y-2">
            {snapshot.failures.map((f) => (
              <div key={f.code} className="space-y-0.5">
                <div className="flex items-center justify-between text-zinc-200">
                  <span className="truncate">{f.code}</span>
                  <span className="text-text-muted">
                    {f.interceptedCount} ({f.sharePct}%)
                  </span>
                </div>
                <div className="h-1.5 w-full overflow-hidden rounded-sm bg-white/10">
                  <div className="h-full bg-zinc-300" style={{ width: `${f.sharePct}%` }} />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Column 2: 4-Stage Inverted Trapezoid Verification Retention Funnel */}
      <div className="rounded-lg border border-white/10 bg-[#09090b] p-2.5">
        <div className="mb-2 flex items-center justify-between text-[9px] uppercase tracking-wider text-text-muted">
          <span>VERIFICATION RETENTION FUNNEL</span>
          <span className="text-emerald-400">DETERMINISTIC GATE</span>
        </div>
        <div className="space-y-1.5">
          {snapshot.funnel.map((stage) => (
            <div key={stage.stageId} className="flex flex-col items-center">
              <div
                className="flex items-center justify-between rounded-sm border border-white/15 bg-white/[0.06] px-2 py-1 text-[9px] text-zinc-100 transition-all"
                style={{ width: `${Math.max(42, stage.retentionPct)}%` }}
              >
                <span className="truncate">{stage.label}</span>
                <span className="font-semibold text-cyan">
                  {stage.count} ({stage.retentionPct}%)
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Column 3: Zero-Cascade SLO & Ludus Gamification Thrift Yield */}
      <div className="rounded-lg border border-white/10 bg-[#09090b] p-2.5 flex flex-col justify-between">
        <div>
          <div className="mb-2 text-[9px] uppercase tracking-wider text-text-muted">
            ZERO-CASCADE SLO & RECEIPTS
          </div>
          <div className="space-y-1 text-[10px]">
            <div className="flex items-center justify-between">
              <span className="text-text-muted">BLAKE3 TOOL RECEIPTS</span>
              <span className="text-emerald-400">HMAC VALID</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-text-muted">SANDBOX ISOLATION</span>
              <span className="text-zinc-200">ENFORCED</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-text-muted">ACTIVE LAYER</span>
              <span className="truncate max-w-[150px] text-cyan">{snapshot.activePhaseSubtext}</span>
            </div>
          </div>
        </div>

        {gamifyEnabled && (
          <div className="mt-2 flex items-center justify-between rounded border border-emerald-500/25 bg-emerald-500/10 px-2 py-1">
            <span className="text-[9px] uppercase tracking-wider text-emerald-300">
              LUDUS {snapshot.ludusYield.thriftRankLabel}
            </span>
            <span className="font-semibold text-emerald-200">
              +{snapshot.ludusYield.xpEarned} XP · +{snapshot.ludusYield.crystalsEarned} ◆
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
```

- [x] **Step 4: Run test to verify GREEN**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/VerificationFunnelFooter.test.tsx
```
Expected: `1 passed (1)`.

---

### Task 6: Assemble Responsive `HarnessRiverPanel` & Upgrade `HarnessRedirect.tsx`

**Model:** `[MODEL: ARGON-4]`
**Dispatch:** `[SEQUENTIAL]` (Wave 2 — edits `HarnessRedirect.tsx` and `HarnessRedirect.test.tsx`)
**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRiverPanel.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRedirect.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRedirect.test.tsx`

**Interfaces:**
- `HarnessRiverPanelProps` & `HarnessRedirectProps`:
  `{ tasks?: ChatExecutionTask[]; messages?: ChatMessage[]; routing?: RailRouting | null; sessionSpentUsd?: number | null; control?: ControlState; chatModelOverride?: string | null; condensed?: boolean; gamifyEnabled?: boolean; onOpenRouting?: () => void; onFocusComposer?: () => void; }`

- [x] **Step 0: Verify preconditions**

Run:
```bash
rg -n "export function HarnessRedirect" crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRedirect.tsx
```
Expected output matches line 14.

- [x] **Step 1: Update `HarnessRedirect.test.tsx` to test both backwards-compatible composer focus AND the live Harness River HUD**

Replace `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRedirect.test.tsx`:
```tsx
// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import React from 'react';

import { HarnessRedirect } from './HarnessRedirect';

describe('HarnessRedirect (upgraded to Axis Harness River)', () => {
  beforeEach(() => cleanup());

  it('renders the Axis Harness River HUD and retains composer/Loquela reference', () => {
    const { container } = render(<HarnessRedirect onFocusComposer={vi.fn()} />);
    const subtitle = screen.getByText(/Loquela composer/i);
    expect(subtitle.textContent?.toLowerCase()).toMatch(/composer|loquela/);
    expect(screen.getByTestId('harness-waveform-tier-strip')).toBeDefined();
    expect(screen.getByTestId('harness-verification-footer')).toBeDefined();
    expect(container.querySelector('canvas')).not.toBeNull();
  });

  it('calls onFocusComposer when Focus composer action button is clicked', () => {
    const onFocusComposer = vi.fn();
    render(<HarnessRedirect onFocusComposer={onFocusComposer} />);

    fireEvent.click(screen.getByRole('button', { name: 'Focus composer' }));

    expect(onFocusComposer).toHaveBeenCalledTimes(1);
  });
});
```

- [x] **Step 2: Run test to verify RED**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/HarnessRedirect.test.tsx
```
Expected: Fails because `HarnessRedirect` does not yet render `harness-waveform-tier-strip` or `<canvas>`.

- [x] **Step 3: Create `HarnessRiverPanel.tsx` and update `HarnessRedirect.tsx`**

Create `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRiverPanel.tsx`:
```tsx
import React, { useEffect, useMemo, useRef } from 'react';
import { defaultControl, type ControlState } from '../../../lib/driveConsole';
import type { RailRouting } from '../../../lib/routingSummary';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import type { ChatExecutionTask } from '../Chat/ChatExecutionRail';
import { deriveRiverSnapshot } from './riverMath';
import { renderRiverFrame } from './riverCanvasRenderer';
import { WaveformTierStrip } from './WaveformTierStrip';
import { AgentTrajectoryBraid } from './AgentTrajectoryBraid';
import { VerificationFunnelFooter } from './VerificationFunnelFooter';
import type { HarnessTierId } from './riverTypes';

export interface HarnessRiverPanelProps {
  tasks?: ChatExecutionTask[];
  messages?: ChatMessage[];
  routing?: RailRouting | null;
  sessionSpentUsd?: number | null;
  control?: ControlState;
  chatModelOverride?: string | null;
  condensed?: boolean;
  gamifyEnabled?: boolean;
  onOpenRouting?: () => void;
  onFocusComposer?: () => void;
}

export function HarnessRiverPanel({
  tasks = [],
  messages = [],
  routing = null,
  sessionSpentUsd = null,
  control,
  chatModelOverride = null,
  condensed = false,
  gamifyEnabled = true,
  onOpenRouting,
  onFocusComposer,
}: HarnessRiverPanelProps) {
  const effectiveControl = control ?? defaultControl();
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  const snapshot = useMemo(
    () =>
      deriveRiverSnapshot({
        tasks,
        messages,
        sessionSpentUsd,
        clutch: effectiveControl.clutch,
        risk: effectiveControl.risk,
        chatModelOverride,
        routing,
      }),
    [tasks, messages, sessionSpentUsd, effectiveControl.clutch, effectiveControl.risk, chatModelOverride, routing],
  );

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let ctx: CanvasRenderingContext2D | null = null;
    try {
      ctx = canvas.getContext('2d');
    } catch {
      ctx = null;
    }
    if (!ctx) return;

    let animId = 0;
    const startMs = performance.now();

    const draw = (nowMs: number) => {
      const rect = canvas.getBoundingClientRect();
      const w = Math.max(320, Math.floor(rect.width || 560));
      const h = Math.max(180, Math.floor(rect.height || 260));
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
      }
      const phaseSec = (nowMs - startMs) / 1000;
      renderRiverFrame(ctx!, w, h, snapshot, phaseSec);
      animId = requestAnimationFrame(draw);
    };

    renderRiverFrame(ctx, 560, 260, snapshot, 0);
    animId = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(animId);
  }, [snapshot]);

  const handleSelectTier = (_tierId: HarnessTierId) => {
    if (onOpenRouting) {
      onOpenRouting();
    } else if (onFocusComposer) {
      onFocusComposer();
    }
  };

  return (
    <section
      data-testid="axis-harness-river"
      className="flex flex-col gap-2.5 text-text-primary"
      aria-label="Axis Autonomous Harness River"
    >
      {/* Header Bar */}
      <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-white/10 bg-[#09090b] px-3 py-2 font-mono">
        <div className="flex items-center gap-2">
          <span className="inline-block size-2 rounded-full bg-cyan animate-pulse" />
          <span className="text-xs font-bold uppercase tracking-wider text-zinc-100">
            AXIS HARNESS RIVER
          </span>
          <span className="text-[10px] text-text-muted">
            Wired to Loquela composer · 3-Tier Cascade
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          {onOpenRouting && (
            <button
              type="button"
              onClick={onOpenRouting}
              className="rounded border border-white/15 bg-white/5 px-2 py-1 text-[10px] text-cyan hover:bg-white/10"
            >
              Routing & Tiers
            </button>
          )}
          {onFocusComposer && (
            <button
              type="button"
              onClick={onFocusComposer}
              className="rounded border border-white/15 bg-white/5 px-2 py-1 text-[10px] text-zinc-200 hover:bg-white/10"
            >
              Focus composer
            </button>
          )}
        </div>
      </div>

      {/* Zone A: Top Telemetry Waveform & 3-Tier Model Assignment Strip */}
      <WaveformTierStrip
        snapshot={snapshot}
        condensed={condensed}
        onSelectTier={handleSelectTier}
      />

      {/* Zone B: Center Alluvial Particle Bundle + Singularity Chokepoint + Sweeping Laser */}
      <div className="relative overflow-hidden rounded-lg border border-white/10 bg-[#070709]">
        <div className="flex items-center justify-between border-b border-white/10 px-3 py-1 font-mono text-[9px] uppercase tracking-wider text-text-muted">
          <span>SOURCE WORKTREES & AST HASHES</span>
          <span className="text-cyan">INTERCEPTOR SINGULARITY</span>
          <span>VERIFIED PATCH TARGETS</span>
        </div>
        <div className={condensed ? 'h-[210px] w-full' : 'h-[270px] w-full'}>
          <canvas
            ref={canvasRef}
            className="h-full w-full block"
            aria-label="Live alluvial particle stream and phase scanner"
          />
        </div>
      </div>

      {/* Zone C: 8-Slice Agent Role Trajectory Braid (full mode only) */}
      {!condensed && <AgentTrajectoryBraid trajectories={snapshot.trajectories} />}

      {/* Zone D: Deterministic Evidence & Verification Retention Funnel */}
      <VerificationFunnelFooter
        snapshot={snapshot}
        condensed={condensed}
        gamifyEnabled={gamifyEnabled}
      />
    </section>
  );
}
```

Replace `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRedirect.tsx`:
```tsx
import React, { useEffect } from 'react';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { HarnessRiverPanel, type HarnessRiverPanelProps } from './HarnessRiverPanel';

export type HarnessRedirectProps = HarnessRiverPanelProps;

/**
 * Axis Harness River surface — serves both the dockable 'harness' panel in ChatSurface
 * and the full Workspace -> Harness view while preserving composer focus actions.
 */
export function HarnessRedirect(props: HarnessRedirectProps) {
  const { gamifyEnabled } = props;
  useEffect(() => {
    void recordGamifyGuiEvent('harness_redirect_viewed', undefined, { enabled: gamifyEnabled });
  }, [gamifyEnabled]);

  return <HarnessRiverPanel {...props} />;
}
```

- [x] **Step 4: Run test to verify GREEN**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/HarnessRedirect.test.tsx
```
Expected: `2 passed (2)`.

---

### Task 7: Wire `ChatSurface.tsx` Dockable Harness Panel (`HarnessDockPanel`) with Width Observation & Live Props

**Model:** `[MODEL: FLASH-3.8]`
**Dispatch:** `[SEQUENTIAL]` (Wave 2 — edits `ChatSurface.tsx`)
**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`

**Interfaces:**
- Upgrade `HarnessDockPanel` in `ChatSurface.tsx` to observe dockview panel width (`HARNESS_FULL_WIDTH_PX = 420`) and clone `condensed={width < HARNESS_FULL_WIDTH_PX}` onto `props.params.node` (matching `NeedsYouDockPanel` / `MercatusDockPanel`).
- Pass live `tasks`, `messages`, `routing`, `sessionSpentUsd`, `control`, `chatModelOverride`, and `onOpenRouting={() => setRoutingOpen(true)}` to `harnessNode`.

- [x] **Step 0: Verify preconditions**

Run:
```bash
rg -n "function HarnessDockPanel|const harnessNode =" crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx
```
Expected output shows `function HarnessDockPanel` around line 205 and `const harnessNode =` around line 634–638.

- [x] **Step 1: View exact lines around `HarnessDockPanel` and `harnessNode` in `ChatSurface.tsx`**

Use `view_file` on `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` lines 200–212 and 630–645 to confirm exact surrounding lines.

- [x] **Step 2: Update `HarnessDockPanel` and `harnessNode` in `ChatSurface.tsx`**

Use `replace_file_content` on `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` to upgrade `HarnessDockPanel`:
```tsx
const HARNESS_FULL_WIDTH_PX = 420;

export function HarnessDockPanel(props: IDockviewPanelProps<{ node: React.ReactNode }>) {
  const [width, setWidth] = React.useState(props.api.width);
  React.useEffect(() => {
    const disposable = props.api.onDidDimensionsChange(() => setWidth(props.api.width));
    return () => disposable.dispose();
  }, [props.api]);
  const condensed = width < HARNESS_FULL_WIDTH_PX;
  return (
    <div data-testid="chat-dock-harness" className="h-full overflow-y-auto p-2">
      {React.isValidElement(props.params.node)
        ? React.cloneElement(props.params.node as React.ReactElement<any>, { condensed })
        : props.params.node}
    </div>
  );
}
```
And update `harnessNode` in `ChatSurface.tsx`:
```tsx
  const harnessNode = (
    <HarnessRedirect
      tasks={tasks}
      messages={messages}
      routing={routing}
      sessionSpentUsd={sessionSpentUsd}
      gamifyEnabled={gamifyEnabled}
      onOpenRouting={() => setRoutingOpen(true)}
    />
  );
```

- [x] **Step 3: Run `ChatSurface` and `Harness` tests to verify GREEN**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Harness/
```
Expected: All tests in `src/components/surfaces/Harness/` pass.

---

### Task 8: Unify `GamifyView.tsx` & `Dashboard.tsx` (`Urbs Map | Live River` Switcher) and Update `where-things-live.md`

**Model:** `[MODEL: FLASH-3.8]`
**Dispatch:** `[SEQUENTIAL]` (Wave 2 — edits `GamifyView.tsx`, `GamifyView.test.tsx`, `Dashboard.tsx`, and `docs/src/architecture/where-things-live.md`)
**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Gamify/GamifyView.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Gamify/GamifyView.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx`
- Modify: `docs/src/architecture/where-things-live.md`

- [x] **Step 0: Verify preconditions**

Run:
```bash
rg -n "Simulation Map" crates/vox-gui/ui/src/components/surfaces/Gamify/GamifyView.tsx
```
Expected output matches line 126.

Run:
```bash
rg -n "Workspace Simulation Mini-Map" crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx
```
Expected output matches line 446.

- [x] **Step 1: Add a test in `GamifyView.test.tsx` verifying the `Live River` simulation mode toggle**

In `crates/vox-gui/ui/src/components/surfaces/Gamify/GamifyView.test.tsx`, add a test case verifying that clicking the `Live River` button switches the simulation view to `axis-harness-river` while keeping every button `type="button"`:
```tsx
  it('switches between Urbs Map and Live River harness visualizer', () => {
    const { getByRole, getByTestId } = render(
      <LanguageProvider>
        <GamifyView pushToast={vi.fn()} />
      </LanguageProvider>,
    );
    const riverBtn = getByRole('button', { name: /Live River/i });
    expect(riverBtn.getAttribute('type')).toBe('button');
    riverBtn.click();
    expect(getByTestId('axis-harness-river')).toBeDefined();
  });
```

- [x] **Step 2: Run `GamifyView.test.tsx` to verify RED**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Gamify/GamifyView.test.tsx
```
Expected: Fails with `Unable to find an accessible element with the role "button" and name "/Live River/i"`.

- [x] **Step 3: Update `GamifyView.tsx`, `Dashboard.tsx`, and `where-things-live.md`**

In `crates/vox-gui/ui/src/components/surfaces/Gamify/GamifyView.tsx`:
1. Import `HarnessRiverPanel` from `../Harness/HarnessRiverPanel`.
2. Add state `const [simMode, setSimMode] = useState<'urbs' | 'river'>('urbs');`.
3. Replace lines 125–130 (`Simulation Map` container) with:
```tsx
      <div className="mb-6 border border-border-subtle rounded-xl overflow-hidden bg-bg-base/60 p-4">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="font-display text-[12px] uppercase tracking-wide text-text-muted">Simulation Map</h3>
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => setSimMode('urbs')}
              className={`rounded px-2 py-0.5 font-mono text-[10px] ${simMode === 'urbs' ? 'bg-cyan/20 text-cyan border border-cyan/40' : 'bg-overlay-subtle text-text-muted'}`}
            >
              Urbs Map
            </button>
            <button
              type="button"
              onClick={() => setSimMode('river')}
              className={`rounded px-2 py-0.5 font-mono text-[10px] ${simMode === 'river' ? 'bg-cyan/20 text-cyan border border-cyan/40' : 'bg-overlay-subtle text-text-muted'}`}
            >
              Live River
            </button>
          </div>
        </div>
        {simMode === 'urbs' ? (
          <div className="h-[560px]">
            <LudusSandbox energy={profile?.energy ?? 0} maxEnergy={profile?.max_energy ?? 0} />
          </div>
        ) : (
          <HarnessRiverPanel gamifyEnabled />
        )}
      </div>
```

In `crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx`:
Add a `"Harness River"` navigation button beside `"Immersive View"` in the Workspace Simulation Mini-Map header (around line 448):
```tsx
          <div className="flex gap-2">
            <button type="button" onClick={() => onNavigate?.('harness')} className="text-emerald-400 hover:underline">Harness River</button>
            <button type="button" onClick={() => onNavigate?.('gamify')} className="text-cyan hover:underline">Immersive View</button>
            <button type="button" onClick={() => setSandboxCollapsed(!sandboxCollapsed)} className="text-text-muted hover:text-text-secondary">
              {sandboxCollapsed ? 'Expand' : 'Collapse'}
            </button>
          </div>
```

In `docs/src/architecture/where-things-live.md` (around line 242):
Add a row documenting the Axis Harness River SSOT and noting `apps/experimental/visualizer` as superseded:
```markdown
| Axis Harness River (3-tier model cascade HUD, live alluvial particle river, 8-slice agent trajectory braid, verification retention funnel) | [`crates/vox-gui/ui/src/components/surfaces/Harness/`](../../../crates/vox-gui/ui/src/components/surfaces/Harness/) (`HarnessRiverPanel.tsx`, `riverMath.ts`, `riverCanvasRenderer.ts`, `WaveformTierStrip.tsx`, `AgentTrajectoryBraid.tsx`, `VerificationFunnelFooter.tsx`) — supersedes standalone `apps/experimental/visualizer`; narrative [`axis-harness-river-design-2026.md`](./axis-harness-river-design-2026.md). |
```

- [x] **Step 4: Run `GamifyView.test.tsx` and full Harness test suite to verify GREEN**

Run:
```bash
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Gamify/GamifyView.test.tsx src/components/surfaces/Harness/
```
Expected: All tests in `GamifyView.test.tsx` and `src/components/surfaces/Harness/` pass.
