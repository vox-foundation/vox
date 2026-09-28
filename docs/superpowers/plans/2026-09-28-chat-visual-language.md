# Chat Visual Language: One Vocabulary, Status Tokens, Readable Type, Zero Serious Axe — Implementation Plan (Plan 3b)

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task docs/superpowers/plans/2026-09-28-chat-visual-language.md <N>`), per
> [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, reviews the diff and commits. Tasks marked
> **Owner: Claude** are not driven. Steps use checkbox (`- [ ]`) syntax.
>
> **Revision 2 (2026-09-28).** Amended after review (no Critical; Tasks 1–6 WARN, Task 7 PASS). Changes carry
> `<!-- AMENDED: T<n> — reason -->` markers. Task numbering changed: old Task 6 is split into Task 6 (contrast and
> headings) and Task 7 (Activity labels + capture acceptance); the verification sweep is now Task 8.

**Goal:** The chat surface, composer and status bar speak one English vocabulary from one label source, show status
colour only through the `--color-status-*` tokens, never render data text under 11 px, use exactly two tracked-caps
styles, carry no glows, and the review capture reports zero `serious`/`critical` axe violations for every
`chat--*--wide` state. Dead chat modules are deleted and the stale review states capture real UI again.

**Architecture:** GUI-only (`crates/vox-gui/ui`). `lib/navigation.ts` `NAV_LABELS` becomes a projection of
`lib/lexicon.ts` `LEXICON`; the command palette reads the lexicon too. Four static guard tests under `src/__tests__/`
share one helper (`sourceScan.ts`) that lists the chat/composer/status-bar sources, strips comments and extracts
user-visible strings; each guard is mutation-proven. Mechanical class rewrites are done with one `sed` per task over
an explicit file list, after a short list of hand-made exceptions (brand accents stay brass). Acceptance for layout,
contrast and heading order is the existing review capture (`e2e/review/capture.spec.ts`) summarised by a small script
that lives only under `target/` (gitignored).

**Tech Stack:** React 19 + Tailwind v4 (`text-(--token)` arbitrary-variable classes, as already used by
`components/dashboard/WidgetErrorBoundary.tsx`), vitest 3, Playwright 1.62 + `@axe-core/playwright`, Style Dictionary
tokens (read-only here).

**Spec:** [`docs/src/architecture/chat-surface-design-critique-2026-09-28.md`](../../src/architecture/chat-surface-design-critique-2026-09-28.md)
(§Consistency, §Accessibility, §Canonical vocabulary, matrix rows 37–39). Expands tasks 24–27 of the index plan
[`2026-09-28-chat-surface-trace-and-latest-models.md`](2026-09-28-chat-surface-trace-and-latest-models.md)
(24 → Task 3, 25 → Task 4, 26 → Tasks 5–7, 27 → Tasks 1–2).

**Prerequisites:** Phase 5 (`.planning/phases/05-*`) is complete, and **Plan 3a
(`2026-09-28-chat-surfaces-consolidation.md`) is fully committed**. 3a rewrites `Loquela.tsx`, `DriveConsole.tsx`,
`RiskPopover.tsx`, `BottomStatusBar.tsx`, `StatusBarCluster.tsx`, `ChatExecutionRail.tsx`, `lib/driveConsole.ts` and
`hooks/useHudTiles.ts`, so this plan's guards and class rewrites must run over 3a's final text.

## Canonical vocabulary (from the critique; Task 3 enforces the "Stop using" column)

| Concept | Use | Stop using |
|---|---|---|
| Model selection for a turn | **Routing** | Intents, auto-route, Auto · Router, Cascade |
| Money spent | **Spend** (with scope: global / session / turn) | Budget burn, OR Spend, "session" for global |
| Attention minutes | **Attention** | budget |
| Cost/quality preset | **Mode**: Free, Efficient, Balanced, Genius (and Responsive if defined) | Clutch, Effic., Bal. |
| Acceptable risk | **Risk** | bare "Moderate" |
| Post-reply confidence check | **Check replies** | grounding |
| Items waiting on a human | **Needs you** | Approvals (as the umbrella term) |
| Model version | the catalog-resolved id, or the family (`claude-opus (latest)`) when unresolved | any hardcoded version string |

English mode never shows Latin or code names: Loquela, Oratio, Mercatus, Scientia, Axis Inspector, Secretary,
Graphify, VoxGraph. The "Secretary" toast becomes **Suggested task**. Model tiers read **Auto, Local, Mesh, Cloud**.

## Global Constraints

- GUI-only. No Rust, no `contracts/**`, no new dependency. Generated files are never edited: `src/styles/tokens.generated.*`
  and `src/generated/surfaceRegistry.generated.ts` are **read** by tests only. (If a token value ever had to change, the
  source is `crates/vox-gui/ui/tokens/*.json` and the generator is `pnpm --dir crates/vox-gui/ui tokens:build`; this plan
  changes no token.)
- Brass (`text-brass`, `bg-brass/…`, `ring-brass`) is for brand accents and the active state, never for warnings.
  Status meaning uses `--color-status-pass|fail|warn|info` through Tailwind arbitrary-variable classes:
  `text-(--color-status-warn)`, `bg-(--color-status-warn)/8`, `border-(--color-status-warn)/30`,
  `from-(--color-status-warn)/40`.
- Test-first: every guard test is written and run **before** the class rewrite, and its failing output is saved to
  the `target/visual-lang/*-red.txt` file each RED step names, before any implementation edit. A guard that already
  passes before the rewrite (because an earlier plan already removed every offender) is recorded as such in the RED
  log and is then proven by its mutation step — never skipped.
- Commands run from `/Users/brbrainerd/dev/vox`, foreground, prefixed `timeout` (`timeout 300s` for scoped vitest,
  typecheck and single Playwright specs; `timeout 600s` for the full vitest suite and review captures). Exit 124 is a
  STOP. Create `target/visual-lang/` once with `mkdir -p target/visual-lang`.
- The agent never runs `git add`/`git commit`/`git rm`; it deletes files with plain `rm` on the exact paths a step lists
  (never `rm -r` outside `target/`). Claude Code commits by pathspec.
- Existing tests are not edited except exactly these: `ChatSurface.test.tsx` (Task 3 label rename; Task 6 one inserted
  `it`), `lib/chatTranscriptTimeline.test.ts` (Task 1 removes the `buildTranscriptTimeline` block),
  `guards/ipcBoundaries.test.ts` (Task 1 removes three allowlist lines), `components/ui/EmptyState.test.tsx` (Task 6 one
  appended `it`). Deleted with their orphans (Task 1): `ChatModelPicker.test.tsx`, `ResearchSummaryCard.test.tsx`,
  `AttentionStrip.test.tsx`, `Loquela/Transcript.test.tsx`, `Loquela/InlineApprovals.test.tsx`,
  `Loquela/DiffReview.test.tsx`. Any other existing test that fails and did not fail in the Task 1 baseline is a
  STOP: name the file, the test and the assertion.
- Anchors are symbols and quoted strings, not line numbers. If an anchor quoted in a step is missing because Plan 3a
  or the trace plan changed that file, follow the task's "if the anchor is gone" instruction; where a task gives none,
  STOP with the anchor text.
- `Loquela.tsx` (≈1,000 lines) and `ChatSurface.tsx` (≈1,100) are over the 500-line governance limit; this plan only
  swaps class strings and literals in them and adds no lines beyond those shown.
- No versioned cloud model id appears anywhere in this plan's code or fixtures.

<!-- AMENDED: T4/T5/T6 — StatusBarCluster.tsx (the status bar's research chip) joins the scope of every guard and every sed list. -->
**The scope file list** used by every `sed` command in this plan (run from `crates/vox-gui/ui`):

```bash
ls src/components/surfaces/Chat/*.tsx src/components/surfaces/Loquela/*.tsx src/components/layout/BottomStatusBar.tsx src/components/common/StatusBarCluster.tsx | grep -v '\.test\.tsx$'
```

It matches `chatScopeFiles()` in `src/__tests__/sourceScan.ts` exactly.

<!-- AMENDED: T2/T5/T7 — one shared summary script (adds per-entry serious, overflow and icon counts) instead of copies per task. -->
## Shared script (never committed): `target/visual-lang/axe-summary.mjs`

Tasks 2, 5 and 7 each (over)write this file with exactly this content before using it (`mkdir -p target/visual-lang`
first). It reads the entries that `e2e/review/capture.spec.ts` `captureOne` writes (`id`, `state_ok`, `state_error`,
`axe_violations[].{id,impact,nodes[].target}`, `overflow.{bodyHorizontalOverflowPx,scrollHostHorizontalOverflowPx}`,
`icon_issues[]`) — the real producer's output, never a hand-written copy. Usage:
`node target/visual-lang/axe-summary.mjs ENTRIES_DIR ID_REGEX` (two positional arguments).

```js
// Summarise review-capture entries. Lives only under target/ (gitignored); never committed.
// Usage: node target/visual-lang/axe-summary.mjs ENTRIES_DIR ID_REGEX
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const [dir, pattern = '.'] = process.argv.slice(2);
const idRe = new RegExp(pattern);
const lines = [];
let entries = 0;
let stateFailed = 0;
let blocking = 0;
let overflowPx = 0;
let iconIssues = 0;
for (const f of readdirSync(dir).filter((n) => /^entries-.*\.jsonl$/.test(n))) {
  for (const line of readFileSync(join(dir, f), 'utf8').split('\n')) {
    if (!line.trim()) continue;
    const e = JSON.parse(line);
    if (!idRe.test(e.id)) continue;
    entries += 1;
    if (!e.state_ok) stateFailed += 1;
    let serious = 0;
    const v = (e.axe_violations ?? []).map((x) => {
      if (x.impact === 'serious' || x.impact === 'critical') serious += x.nodes.length;
      return `${x.id}:${x.impact}x${x.nodes.length}[${x.nodes.map((n) => n.target.join(' ')).join(' | ')}]`;
    });
    blocking += serious;
    const ov = (e.overflow?.bodyHorizontalOverflowPx ?? 0) + (e.overflow?.scrollHostHorizontalOverflowPx ?? 0);
    const icons = (e.icon_issues ?? []).length;
    overflowPx += ov;
    iconIssues += icons;
    const err = e.state_error ? ` (${String(e.state_error).slice(0, 120)})` : '';
    lines.push(`${e.id} state_ok=${e.state_ok}${err} serious=${serious} overflow_px=${ov} icon_issues=${icons} ${v.join(' ; ')}`);
  }
}
for (const l of lines.sort()) console.log(l);
console.log(
  `ENTRIES=${entries} STATE_FAILED=${stateFailed} SERIOUS_OR_CRITICAL_NODES=${blocking} OVERFLOW_PX=${overflowPx} ICON_ISSUES=${iconIssues}`,
);
```

## File Structure

| File (under `crates/vox-gui/ui/`) | Status | Task | Responsibility |
|---|---|---|---|
| `src/components/surfaces/Chat/ChatModelPicker.tsx` + test | delete | 1 | orphan (model picking lives in Loquela's model-tier trigger) |
| `src/components/surfaces/Chat/ChatAgentEventRow.tsx` | delete | 1 | orphan (the trace plan revives only `PhaseChip`) |
| `src/components/chat/ChatMessage.tsx`, `ResearchSummaryCard.tsx` + test | delete | 1 | orphans (the directory becomes empty and is removed) |
| `src/components/layout/AttentionStrip.tsx` + test | delete | 1 | orphan |
| `src/components/surfaces/Loquela/Transcript.tsx`, `InlineApprovals.tsx`, `DiffReview.tsx` + tests | delete | 1 | orphaned by the `chatDock` removal |
| `src/lib/chatTranscriptTimeline.ts` + `.test.ts` | modify | 1 | drop `buildTranscriptTimeline`, its helpers and the agent/token row types |
| `src/App.tsx` | modify | 1 | drop the dead `chatDock` block, its prop and three imports |
| `src/guards/ipcBoundaries.test.ts` | modify | 1 | drop three deleted files from the allowlist |
| `e2e/review/states.ts` | modify | 2 | chat states target elements that exist |
| `src/__tests__/sourceScan.ts` | create | 3 | shared scan helpers (`SRC_ROOT`, `chatScopeFiles`, `readSrc`, `stripComments`, `visibleStrings`, `propertyStrings`) |
| `src/__tests__/englishVocabulary.test.ts` | create | 3 | one label source + no Latin/code names + retired terms |
| `src/lib/lexicon.ts`, `src/lib/navigation.ts`, `src/lib/federatedSearchIndex.ts` | modify | 3 | two English labels; `NAV_LABELS` derived; palette label from LEXICON |
| `src/components/surfaces/Chat/SecretaryToast.tsx` | modify | 3 (+ mutation anchor in 4–6) | "Suggested task" |
| `src/components/surfaces/Loquela/Loquela.tsx` (`LQ_TIERS`, `ROUTING_TIERS`), `RiskPopover.tsx` (`COPY`) | modify | 3 | tier labels Auto/Local/Mesh/Cloud; "check replies" copy |
| `src/components/surfaces/Chat/ChatSurface.test.tsx` | modify | 3, 6 | Panels menu reads "Market"; empty-state heading level test |
| `src/__tests__/statusColorTokens.test.ts` | create | 4 | status colour + hover-collapse guard |
| the scope file list (above) | modify | 4, 5, 6 | class rewrites |
| `src/__tests__/chatTypeScale.test.ts` | create | 5 | type size, tracking pairing, glow guard |
| `src/index.css`, `../ds/components.css`, `../ds/conventions.md` | modify | 5 | range-thumb glow removed; section head 11 px / 0.13em in both copies; micro caps documented |
| `src/__tests__/chatContrast.test.ts` | create | 6 | token contrast pairs + low-contrast class guard |
| `src/components/ui/EmptyState.tsx` + `.test.tsx` | modify | 6 | optional `headingLevel` |
| `src/components/surfaces/Chat/ChatSurface.tsx` | modify | 4, 5, 6 | ring exception; tracking; `headingLevel={2}` |
| `src/components/surfaces/Activity/ActivitySurface.tsx`, `ActivitySurface.a11y.test.tsx` (new) | modify/create | 7 | two select labels; one contrast class |
| `/Users/brbrainerd/dev/vox/target/visual-lang/axe-summary.mjs` | create (gitignored, never committed) | 2, 5, 7 | the shared script above |

---

### Task 1: Delete verified orphans and the dead `chatDock` block

<!-- AMENDED: T1 — also delete ChatAgentEventRow (no importer; the trace plan revives only PhaseChip) and the three Loquela components orphaned by the chatDock removal, so they are not restyled for nothing; chatDock is removed BEFORE the importer check; loadTaskDiff stays (the /diff slash command still calls it); the non-existent "trace plan Task 18" reference is gone. -->

**Files:**
- Delete: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx`, `crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.test.tsx`, `crates/vox-gui/ui/src/components/surfaces/Chat/ChatAgentEventRow.tsx`, `crates/vox-gui/ui/src/components/chat/ChatMessage.tsx`, `crates/vox-gui/ui/src/components/chat/ResearchSummaryCard.tsx`, `crates/vox-gui/ui/src/components/chat/ResearchSummaryCard.test.tsx`, `crates/vox-gui/ui/src/components/layout/AttentionStrip.tsx`, `crates/vox-gui/ui/src/components/layout/AttentionStrip.test.tsx`, `crates/vox-gui/ui/src/components/surfaces/Loquela/Transcript.tsx`, `crates/vox-gui/ui/src/components/surfaces/Loquela/Transcript.test.tsx`, `crates/vox-gui/ui/src/components/surfaces/Loquela/InlineApprovals.tsx`, `crates/vox-gui/ui/src/components/surfaces/Loquela/InlineApprovals.test.tsx`, `crates/vox-gui/ui/src/components/surfaces/Loquela/DiffReview.tsx`, `crates/vox-gui/ui/src/components/surfaces/Loquela/DiffReview.test.tsx`
- Modify: `crates/vox-gui/ui/src/lib/chatTranscriptTimeline.ts`, `crates/vox-gui/ui/src/lib/chatTranscriptTimeline.test.ts`, `crates/vox-gui/ui/src/App.tsx`, `crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts`
- **Do NOT delete** `PhaseChip.tsx` (+ test): the trace plan revives it. It keeps only its own test as an importer after this task, and that is expected.

**Interfaces:**
- Consumes: nothing.
- Produces: no new API. Removed: `buildTranscriptTimeline`; types `TranscriptTimelineRow`, `TranscriptAgentRow`, `TranscriptTokenGroupRow` (their only consumer was `ChatAgentEventRow.tsx`); components `ChatModelPicker`, `ChatAgentEventRow`, `ChatMessage` (the component in `components/chat/`, not the `ChatMessage` type in `lib/chatCorrelation.ts`), `ResearchSummaryCard`, `AttentionStrip`, `Transcript` (Loquela), `InlineApprovals`, `DiffReview`. Kept: `isTokenStreamEvent`, `TranscriptMessageRow`, everything `buildChatOnlyTimeline` uses, and `App.tsx`'s `loadTaskDiff` with its three `diff*` state hooks (the `/diff` slash command in `handleLoquelaSlash` calls it; see Deferred).

This task adds no behaviour, so it has no RED step. Its gate is the importer check (Step 3), and its adversarial cases
are importers the check could miss: one in `e2e/`, a dynamic `import('…')`, or a type-only import of the row types.

- [ ] **Step 1: Record the vitest baseline** (so later tasks can tell a new failure from an old one)

Run: `mkdir -p target/visual-lang && timeout 600s pnpm --dir crates/vox-gui/ui exec vitest run > target/visual-lang/vitest-baseline.txt 2>&1; grep -E "^ FAIL |Test Files|Tests " target/visual-lang/vitest-baseline.txt | sort -u | tail -30`
Expected: a `Test Files` summary line. Paste the list of ` FAIL ` files (it may be empty). These pre-existing failures are not STOP conditions in any task of this plan; any other failing file is.

- [ ] **Step 2: Delete the dead `chatDock` block** in `crates/vox-gui/ui/src/App.tsx` (it never renders: `chatDocked` is hard-coded `false`). This comes first because it is the only non-test importer of `Transcript`, `InlineApprovals` and `DiffReview`. Precondition: `rg -n "const chatDock = \(|chatDock=\{chatDock\}" crates/vox-gui/ui/src/App.tsx` shows exactly two lines; if it shows none, Plan 3a already removed it — say so and go on to sub-step 3.
  1. Delete this block (and the blank line after it):
     ```tsx
       const chatDock = (
         <>
           <InlineApprovals pushToast={pushToast} onViewAll={() => navigateTo('approvals')} />
           {diffOpen && (
             <DiffReview
               diff={diffText}
               loading={diffLoading}
               onClose={() => setDiffOpen(false)}
             />
           )}
           <Transcript messages={activeChatMessages} />
           {loquelaComposer}
         </>
       );
     ```
  2. Delete the JSX prop line `        chatDock={chatDock}` (keep `chatDocked={chatDocked}`; `AppShell` requires it).
  3. Delete these three imports if present, then confirm each name has no other use: `rg -n "\bInlineApprovals\b|\bDiffReview\b|<Transcript\b|\{ Transcript \}" crates/vox-gui/ui/src/App.tsx` → no output.
     ```ts
     import { Transcript } from './components/surfaces/Loquela/Transcript';
     import { DiffReview } from './components/surfaces/Loquela/DiffReview';
     import { InlineApprovals } from './components/surfaces/Loquela/InlineApprovals';
     ```
  Leave `const chatDocked = false;`, `loadTaskDiff` and the `diffOpen`/`diffText`/`diffLoading` state alone.

- [ ] **Step 3: Importer check — a STOP condition per file.** Run each command and compare with the expected output exactly:

```bash
rg -n "from ['\"][^'\"]*/ChatModelPicker['\"]" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected exactly: crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.test.tsx:12:import { ChatModelPicker } from './ChatModelPicker';
rg -n "from ['\"][^'\"]*/ChatAgentEventRow['\"]" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected: no output
rg -n "from ['\"][^'\"]*/ChatMessage['\"]" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected exactly: crates/vox-gui/ui/src/components/chat/ResearchSummaryCard.test.tsx:6:import { ChatMessage } from './ChatMessage';
rg -n "from ['\"][^'\"]*/ResearchSummaryCard['\"]" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected exactly two lines: components/chat/ChatMessage.tsx:3 and components/chat/ResearchSummaryCard.test.tsx:5
rg -n "from ['\"][^'\"]*/AttentionStrip['\"]" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected exactly: crates/vox-gui/ui/src/components/layout/AttentionStrip.test.tsx:4:import { AttentionStrip } from './AttentionStrip';
rg -n "from ['\"][^'\"]*/Transcript['\"]" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected exactly: crates/vox-gui/ui/src/components/surfaces/Loquela/Transcript.test.tsx:5:import { Transcript } from './Transcript';
rg -n "from ['\"][^'\"]*/InlineApprovals['\"]" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected exactly: crates/vox-gui/ui/src/components/surfaces/Loquela/InlineApprovals.test.tsx:10:import { InlineApprovals } from './InlineApprovals';
rg -n "from ['\"][^'\"]*/DiffReview['\"]" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected exactly: crates/vox-gui/ui/src/components/surfaces/Loquela/DiffReview.test.tsx:5:import { DiffReview } from './DiffReview';
rg -n "\bbuildTranscriptTimeline\b|\bTranscriptTimelineRow\b|\bTranscriptAgentRow\b|\bTranscriptTokenGroupRow\b" crates/vox-gui/ui/src crates/vox-gui/ui/e2e
# expected: only lines in src/lib/chatTranscriptTimeline.ts, src/lib/chatTranscriptTimeline.test.ts and src/components/surfaces/Chat/ChatAgentEventRow.tsx
rg -n "import\(" crates/vox-gui/ui/src crates/vox-gui/ui/e2e | rg "ChatModelPicker|ChatAgentEventRow|/ChatMessage|ResearchSummaryCard|AttentionStrip|Loquela/Transcript|InlineApprovals|DiffReview|chatTranscriptTimeline"
# expected: no output (no dynamic importer)
```

Every importer must be a file this task deletes. Any other line (including one under `e2e/`) is a STOP: end with `DRIVE: STOPPED step 3:` followed by the importing file and the orphan it imports.

- [ ] **Step 4: Delete the orphan files** (plain `rm`, one command):

Run: `rm crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.test.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatAgentEventRow.tsx crates/vox-gui/ui/src/components/chat/ChatMessage.tsx crates/vox-gui/ui/src/components/chat/ResearchSummaryCard.tsx crates/vox-gui/ui/src/components/chat/ResearchSummaryCard.test.tsx crates/vox-gui/ui/src/components/layout/AttentionStrip.tsx crates/vox-gui/ui/src/components/layout/AttentionStrip.test.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/Transcript.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/Transcript.test.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/InlineApprovals.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/InlineApprovals.test.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/DiffReview.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/DiffReview.test.tsx && rmdir crates/vox-gui/ui/src/components/chat`
Expected: no output. (`rmdir` fails if the directory still holds a file — that is a STOP: list what is left.)

- [ ] **Step 5: Remove `buildTranscriptTimeline` and the row types** from `crates/vox-gui/ui/src/lib/chatTranscriptTimeline.ts`. Delete exactly these declarations and nothing else:
  1. `export type TranscriptAgentRow = { … };` and `export type TranscriptTokenGroupRow = { … };` (whole type literals),
  2. the union `export type TranscriptTimelineRow = | TranscriptMessageRow | TranscriptAgentRow | TranscriptTokenGroupRow;`,
  3. `function itemTimestampMs(item: StreamItem): number { … }`, `function itemAgentId(item: StreamItem): string | undefined { … }` and the `type RawTimelineEntry = … ;` declaration,
  4. the doc comment `/** Merge chat bubbles and agent stream items into a single time-ordered transcript. */` and the whole `export function buildTranscriptTimeline(…): TranscriptTimelineRow[] { … }` after it.

  Keep `TranscriptMessageRow`, `isTokenStreamEvent`, the `ChatMessage`/`StreamItem` imports and everything from `export type TranscriptStatusRow` down. Then, in the doc comment of `buildChatOnlyTimeline`, replace

  ```ts
   * event (CHECKPOINT/TASK/PHASE/COST/TOKEN) that used to render as its own
   * row via `ChatAgentEventRow` is excluded here — full detail stays available
   * via `buildTranscriptTimeline` for the Flow panel.
  ```

  with

  ```ts
   * event (CHECKPOINT/TASK/PHASE/COST/TOKEN) is excluded here.
  ```

  If the trace plan already changed this file, keep its edits and apply only the deletions whose declarations still exist; say which.

- [ ] **Step 6: Remove the matching tests** from `crates/vox-gui/ui/src/lib/chatTranscriptTimeline.test.ts`: delete the line `  buildTranscriptTimeline,` from the import list at the top, and delete the whole `describe('buildTranscriptTimeline', () => { … });` block (it starts at `describe('buildTranscriptTimeline', () => {` and ends at the `});` just before `function msg(`). Keep `agentItem`, the `isTokenStreamEvent` block and everything from `function msg(` down.

- [ ] **Step 7: Drop the allowlist entries** in `crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts`: delete exactly these three lines from `ALLOW_DIRECT_INVOKE` (the allowlist "shrinks per wave"; the files no longer exist):

```ts
      'components/surfaces/Chat/ChatAgentEventRow.tsx',
      'components/surfaces/Chat/ChatModelPicker.tsx',
      'components/surfaces/Loquela/InlineApprovals.tsx',
```

- [ ] **Step 8: Verify**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -5`
Expected: exit 0, no errors.

Run: `timeout 600s pnpm --dir crates/vox-gui/ui exec vitest run > target/visual-lang/vitest-task1.txt 2>&1; grep -E "^ FAIL |Test Files|Tests " target/visual-lang/vitest-task1.txt | sort -u | tail -30`
Expected: the ` FAIL ` files are a subset of Step 1's list (the six deleted test files are gone from the run). A new failing file → STOP.

Run: `git status --short -- crates/vox-gui/ui` and `git diff --numstat -- crates/vox-gui/ui`
Expected: 14 deleted files (`D`), 4 modified files; `chatTranscriptTimeline.ts` about 120 lines removed, `App.tsx` about 18 removed, `ipcBoundaries.test.ts` 3 removed.

- [ ] **Step 9: Commit (Claude Code)**

```bash
git add -- crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.test.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatAgentEventRow.tsx crates/vox-gui/ui/src/components/chat crates/vox-gui/ui/src/components/layout/AttentionStrip.tsx crates/vox-gui/ui/src/components/layout/AttentionStrip.test.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/Transcript.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/Transcript.test.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/InlineApprovals.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/InlineApprovals.test.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/DiffReview.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/DiffReview.test.tsx crates/vox-gui/ui/src/lib/chatTranscriptTimeline.ts crates/vox-gui/ui/src/lib/chatTranscriptTimeline.test.ts crates/vox-gui/ui/src/App.tsx crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts
git commit -m "chore(gui): delete orphaned chat modules and the dead chatDock block"
```

---

### Task 2: Chat review states target elements that exist

**Files:**
- Modify: `crates/vox-gui/ui/e2e/review/states.ts` (the `SURFACE_STATES['chat'] = [ … ];` assignment only)
- Create (never committed; `target/` is gitignored): `target/visual-lang/axe-summary.mjs` (§Shared script), `target/visual-lang/states-axe.txt`

**Interfaces:**
- Consumes: the capture entries (see §Shared script).
- Produces: chat review states `default`, `model-picker-open`, `panels-menu-open`, `composer-filled`, `empty`, `error`. `session-menu-open` and `rails-overlay-open` are removed. `target/visual-lang/states-axe.txt` records the per-state axe counts that Task 6/7 plan fixes from.

Why these states are stale (verified 2026-09-28): `ChatModelPicker` (the "model:" button) is never mounted — model choice
is Loquela's `aria-label="Choose model tier"` trigger; sessions moved to the sidebar (`SessionSidebarSection.tsx`),
which has no "Session actions for …" menu and no `chat-session-rail-toggle`. A click on a missing element waits until
the 30 s test timeout, so those tests fail and write no entry at all.

- [ ] **Step 1: Write the summary script** — `mkdir -p target/visual-lang`, then write `target/visual-lang/axe-summary.mjs` with exactly the code in §Shared script.

- [ ] **Step 2: Prove the two stale states fail today; save the output**

Run: `VOX_REVIEW_CAPTURE=1 timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/capture.spec.ts --project=chromium --reporter=line --grep "chat -- (model-picker-open|session-menu-open) -- wide$" > target/visual-lang/states-red.txt 2>&1; tail -15 target/visual-lang/states-red.txt`
Expected: `2 failed`, each with `Test timeout of 30000ms exceeded`. If they pass, an earlier plan already fixed them: STOP and say so.

- [ ] **Step 3: Replace the chat states.** In `crates/vox-gui/ui/e2e/review/states.ts`, replace the whole `SURFACE_STATES['chat'] = [ … ];` assignment with:

```ts
SURFACE_STATES['chat'] = [
  DEFAULT,
  {
    name: 'model-picker-open',
    // ChatModelPicker was deleted (never mounted); model choice is the composer's
    // model-tier trigger. Scoped to the chat layout so transcript text cannot collide.
    // 5 s click timeout: a stale selector records state_ok:false instead of timing the test out.
    setup: async (p) => {
      await p
        .getByTestId('chat-surface-layout')
        .getByRole('button', { name: 'Choose model tier' })
        .click({ timeout: 5_000 });
    },
  },
  {
    name: 'panels-menu-open',
    // Replaces session-menu-open: sessions moved to the sidebar and have no actions
    // menu. The Panels menu lists every dock panel by its lexicon label.
    setup: async (p) => {
      await p.getByRole('button', { name: 'Panels', exact: true }).click({ timeout: 5_000 });
    },
  },
  {
    name: 'composer-filled',
    setup: async (p) => {
      await p.getByLabel('Task composer').fill(
        'A deliberately long composer draft that should wrap across multiple lines and reveal any clipping or overlap issues in the dock '.repeat(2),
      );
    },
  },
  ...VARIANT,
];
```

If `rg -n 'aria-label="Choose model tier"' crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx` finds nothing (Plan 3a renamed the trigger), find the trigger's current label with `rg -n "setTierOpen\(o => !o\)" crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`, use that `aria-label` string instead of `'Choose model tier'`, and say so. Likewise, if `aria-label="Panels"` no longer exists in `ChatSurface.tsx`, use the Panels trigger's current `aria-label` and say so.

<!-- AMENDED: T2 — also record per-state serious axe counts: the two new popover states were never audited, so Task 6/7 must see them early. -->
- [ ] **Step 4: Run every chat state at every viewport; record per-state axe counts**

Run: `VOX_REVIEW_CAPTURE=1 timeout 600s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/capture.spec.ts --project=chromium --reporter=line --grep "chat -- " 2>&1 | tail -8; node target/visual-lang/axe-summary.mjs crates/vox-gui/ui/review-bundle/latest '^chat--' > target/visual-lang/states-axe.txt; cat target/visual-lang/states-axe.txt`
Expected: Playwright `19 passed` (6 states × 3 viewports + 1 theme capture); the last line reads `ENTRIES=19 STATE_FAILED=0 …`. Each line shows `serious=N` for that state: copy the `serious=` values of every `chat--model-picker-open--*` and `chat--panels-menu-open--*` line into your report (these popovers were never audited; Tasks 6–7 must bring the `--wide` ones to 0). Non-zero `serious` does not fail this task. If a new state fails only at `compact`, add `viewports: ['wide', 'laptop'],` to that state (and only that), rerun, expect `17 passed` / `ENTRIES=17 STATE_FAILED=0`, and say so.

<!-- AMENDED: T2 — assert only state_ok=false / STATE_FAILED=1; the script truncates state_error, so the selector text may not appear. -->
- [ ] **Step 5: Mutation proof** — change `'Choose model tier'` in `states.ts` to `'Choose model tierX'`, run `VOX_REVIEW_CAPTURE=1 timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/capture.spec.ts --project=chromium --reporter=line --grep "chat -- model-picker-open -- wide$" 2>&1 | tail -3; node target/visual-lang/axe-summary.mjs crates/vox-gui/ui/review-bundle/latest '^chat--' | tail -2` → expect the entry line to contain `state_ok=false` and the last line `STATE_FAILED=1` (the 5 s timeout records the failure instead of hanging; do not expect the selector text, `state_error` is truncated). Restore the string, rerun the same two commands → `state_ok=true` and `STATE_FAILED=0`. `git diff --stat -- crates/vox-gui/ui/e2e` shows only `states.ts`.

- [ ] **Step 6: Commit (Claude Code)** — Claude first opens `crates/vox-gui/ui/review-bundle/latest/chat--model-picker-open--wide--chromium.png` and `chat--panels-menu-open--wide--chromium.png` (rerun Step 4 first if the mutation run replaced the bundle) and confirms each shows its open menu.

```bash
git add -- crates/vox-gui/ui/e2e/review/states.ts
git commit -m "test(gui): retarget stale chat review states at the composer and Panels menu"
```

---

### Task 3: One label source; English mode never shows Latin or code names

<!-- AMENDED: T3 — retired-term guard also scans object-literal strings in every scope file (so `label: "Auto · Router"` and RiskPopover's "enforce grounding" copy are visible to it); this task renames the tier labels and the risk copy; StatusBarCluster moved into chatScopeFiles; exact-count assertions replaced; wider vitest run. -->

**Files:**
- Create: `crates/vox-gui/ui/src/__tests__/sourceScan.ts`, `crates/vox-gui/ui/src/__tests__/englishVocabulary.test.ts`
- Modify: `crates/vox-gui/ui/src/lib/lexicon.ts` (two entries), `crates/vox-gui/ui/src/lib/navigation.ts` (`NAV_LABELS` + one import), `crates/vox-gui/ui/src/lib/federatedSearchIndex.ts` (one line in `buildFederatedIndex`), `crates/vox-gui/ui/src/components/surfaces/Chat/SecretaryToast.tsx` (two strings), `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx` (`LQ_TIERS` and `ROUTING_TIERS` labels only), `crates/vox-gui/ui/src/components/surfaces/Loquela/RiskPopover.tsx` (`COPY` values only), `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx` (label rename only)

**Interfaces:**
- Consumes: `LEXICON`, `sidebarParentLabel(parentKey: string, lang?: Lang): string` (`lib/lexicon.ts`); `TOP_LEVEL_VIEWS`, `PARENT_CHILD_MAP`, `labelForNavKey(key: string): string` (`lib/navigation.ts`); `buildFederatedIndex(sources: FederatedIndexSources): FederatedIndexEntry[]` (`lib/federatedSearchIndex.ts`); `SURFACE_REGISTRY` (`generated/surfaceRegistry.generated.ts`, the real producer).
- Produces (used by Tasks 4–6): `src/__tests__/sourceScan.ts` exports `SRC_ROOT: string`, `chatScopeFiles(): string[]`, `readSrc(rel: string): string`, `stripComments(src: string): string`, `visibleStrings(src: string): string[]`, `propertyStrings(src: string): string[]`. `NAV_LABELS: Record<string, string>` keeps its name and type but is derived. `labelForNavKey` signature unchanged. Tier ids (`auto|local|mesh|cloud`) unchanged; only their labels change.

- [ ] **Step 1: Write the shared helper** `crates/vox-gui/ui/src/__tests__/sourceScan.ts`:

```ts
/**
 * Source-scanning helpers for the chat visual-language guard tests in this folder
 * (englishVocabulary, statusColorTokens, chatTypeScale, chatContrast).
 * Deliberately NOT named *.test.ts: vitest collects only test files, and tsc
 * type-checks this module the same way it checks lib/dashboardBundleBudgetInProcess.ts.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Absolute path of `crates/vox-gui/ui/src`. */
export const SRC_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

const SCOPE_DIRS = ['components/surfaces/Chat', 'components/surfaces/Loquela'];
/** The status bar and its research chip. */
const SCOPE_EXTRA = ['components/layout/BottomStatusBar.tsx', 'components/common/StatusBarCluster.tsx'];

/** Chat, composer and status-bar component sources (non-test `.tsx`), relative to SRC_ROOT, sorted. */
export function chatScopeFiles(): string[] {
  const fromDirs = SCOPE_DIRS.flatMap((dir) =>
    readdirSync(join(SRC_ROOT, dir))
      .filter((name) => name.endsWith('.tsx') && !name.endsWith('.test.tsx'))
      .map((name) => `${dir}/${name}`),
  );
  return [...fromDirs, ...SCOPE_EXTRA].sort();
}

export function readSrc(rel: string): string {
  return readFileSync(join(SRC_ROOT, rel), 'utf8');
}

/** Removes block and line comments so prose in comments is never scanned; keeps `://` and quoted `//`. */
export function stripComments(src: string): string {
  return src.replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/(^|[^:'"`])\/\/.*$/gm, '$1');
}

const CODE_MARKERS = /[;=]|=>|\b(?:const|let|return|function|import|export)\b/;
const TEXT_ATTRS =
  /\b(?:aria-label|title|placeholder|alt|label)=(?:"([^"]*)"|'([^']*)'|\{\s*["'`]([^"'`]*)["'`]\s*\})/g;

/**
 * User-visible literal text in a TSX source: JSX text children (with `{expr}` parts dropped) and the
 * literal values of aria-label / title / placeholder / alt / label attributes. Fragments that look like
 * code (text between a generic's `>` and the next `<`) are skipped.
 */
export function visibleStrings(src: string): string[] {
  const code = stripComments(src);
  const out: string[] = [];
  for (const m of code.matchAll(/>([^<>]+)</g)) {
    const text = m[1].replace(/\{[^{}]*\}/g, ' ').replace(/\s+/g, ' ').trim();
    if (text && !CODE_MARKERS.test(text)) out.push(text);
  }
  for (const m of code.matchAll(TEXT_ATTRS)) out.push(m[1] ?? m[2] ?? m[3]);
  return out;
}

/**
 * Single-line string values after a colon: object-literal properties such as `label: "Auto · Router"` or
 * `moderate: 'Confirm + enforce grounding.'` (label data that renders as chrome without being JSX text).
 * Ternary else-branches (`: "cls"`) are included too; they are class strings and never contain vocabulary.
 */
export function propertyStrings(src: string): string[] {
  return [...stripComments(src).matchAll(/:\s*(?:'([^'\n]*)'|"([^"\n]*)")/g)].map((m) => m[1] ?? m[2]);
}
```

- [ ] **Step 2: Write the failing test** `crates/vox-gui/ui/src/__tests__/englishVocabulary.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { LEXICON, sidebarParentLabel } from '../lib/lexicon';
import { NAV_LABELS, labelForNavKey, TOP_LEVEL_VIEWS, PARENT_CHILD_MAP } from '../lib/navigation';
import { SURFACE_REGISTRY } from '../generated/surfaceRegistry.generated';
import { buildFederatedIndex } from '../lib/federatedSearchIndex';
import { chatScopeFiles, readSrc, visibleStrings, propertyStrings } from './sourceScan';

/** Latin and internal code names that must never reach English-mode chrome. */
const CODE_NAMES = /\b(?:Loquela|Oratio|Mercatus|Scientia|Axis Inspector|Secretary|Graphify|VoxGraph)\b/i;

/** "Stop using" column of the canonical vocabulary (chat-surface-design-critique-2026-09-28). */
const RETIRED_TERMS =
  /(?:\bIntents\b|\bauto-route\b|Auto · Router|\bCascade\b|\bBudget burn\b|\bOR Spend\b|\bClutch\b|\bEffic\.|\bBal\.|\bgrounding\b)/i;

const VOCAB_FILES = chatScopeFiles();
/** Label data outside the component scope that renders as chrome. */
const LABEL_DATA_FILES = ['lib/driveConsole.ts', 'hooks/useHudTiles.ts'];

describe('vocabulary patterns (self-test, so the guards below cannot be vacuous)', () => {
  it('CODE_NAMES catches every code name and passes plain English', () => {
    for (const s of ['Loquela', 'Oratio', 'Mercatus', 'Scientia', 'Axis Inspector', 'Secretary suggests a task', 'Dismiss secretary toast', 'Graphify Corpus Health', 'VoxGraph']) {
      expect(CODE_NAMES.test(s), s).toBe(true);
    }
    for (const s of ['Market', 'Findings', 'Suggested task', 'Search Index Health', 'Voice', 'Routing Axes', 'Review']) {
      expect(CODE_NAMES.test(s), s).toBe(false);
    }
  });

  it('RETIRED_TERMS catches the stop-using column and passes the canonical terms', () => {
    for (const s of ['Intents', 'auto-route (clear override)', 'Auto · Router', 'Cloud · Cascade', 'Budget burn', 'OR Spend', 'Clutch — how much to spend', 'Effic.', 'Bal.', 'grounding: off', 'Confirm + enforce grounding.']) {
      expect(RETIRED_TERMS.test(s), s).toBe(true);
    }
    for (const s of ['Routing', 'Spend', 'Attention', 'Mode', 'Risk: Moderate', 'Check replies', 'Needs you', 'Efficient', 'Balanced', 'Intent', 'Background task', 'Auto', 'Cloud']) {
      expect(RETIRED_TERMS.test(s), s).toBe(false);
    }
  });

  it('propertyStrings sees object-literal labels in both quote styles', () => {
    expect(propertyStrings(`const T = [{ id: "auto", label: "Auto · Router" }]; const C = { moderate: 'Confirm + enforce grounding.' };`))
      .toEqual(['auto', 'Auto · Router', 'Confirm + enforce grounding.']);
  });
});

describe('one label source: LEXICON', () => {
  it('every English lexicon label is plain English (never its Latin form, never a code name)', () => {
    const offenders = Object.entries(LEXICON)
      .filter(([, e]) => (e.la !== undefined && e.en === e.la) || CODE_NAMES.test(e.en))
      .map(([k, e]) => `${k}: ${e.en}`);
    expect(offenders).toEqual([]);
  });

  it('NAV_LABELS covers every nav key and is derived from LEXICON', () => {
    const navKeys = [...new Set<string>([...TOP_LEVEL_VIEWS, ...Object.keys(PARENT_CHILD_MAP)])].sort();
    expect(Object.keys(NAV_LABELS).sort()).toEqual(navKeys);
    for (const k of navKeys) expect(NAV_LABELS[k], k).toBe(sidebarParentLabel(k, 'en'));
    expect(labelForNavKey('mercatus')).toBe('Market');
    expect(labelForNavKey('runs')).toBe('Review');
    expect(labelForNavKey('vox-search')).toBe('Search Index');
  });

  it('the command palette names each surface with its lexicon label, not the registry navLabel', () => {
    // Two sources that disagree today: the generated registry says "Mercatus", "Agents" (flow),
    // "Commands" (catalog); LEXICON says "Market", "Flow", "Catalog". The lexicon wins.
    const index = buildFederatedIndex({ surfaces: SURFACE_REGISTRY, settings: [], policies: [], docs: [], skills: [] });
    const offenders: string[] = [];
    for (const s of SURFACE_REGISTRY) {
      if (!s.viewKey || !s.navLabel) continue;
      const row = index.find((r) => r.id === `surface:${s.viewKey}`);
      const want = LEXICON[s.viewKey]?.en ?? s.navLabel;
      if (!row || row.label !== want || CODE_NAMES.test(row.label)) offenders.push(`${s.viewKey}: ${row?.label}`);
    }
    expect(offenders).toEqual([]);
  });
});

describe('English-mode chrome in chat, composer and status bar', () => {
  it('scans a real, non-empty file set', () => {
    expect(VOCAB_FILES).toContain('components/layout/BottomStatusBar.tsx');
    expect(VOCAB_FILES).toContain('components/common/StatusBarCluster.tsx');
    expect(VOCAB_FILES).toContain('components/surfaces/Loquela/Loquela.tsx');
    expect(VOCAB_FILES.length).toBeGreaterThanOrEqual(12);
  });

  it('no visible string contains a Latin or code name', () => {
    const offenders = VOCAB_FILES.flatMap((rel) =>
      visibleStrings(readSrc(rel))
        .filter((t) => CODE_NAMES.test(t))
        .map((t) => `${rel}: ${t}`),
    );
    expect(offenders).toEqual([]);
  });

  it('no visible string or label datum uses a retired term', () => {
    const fromChrome = VOCAB_FILES.flatMap((rel) =>
      [...visibleStrings(readSrc(rel)), ...propertyStrings(readSrc(rel))]
        .filter((t) => RETIRED_TERMS.test(t))
        .map((t) => `${rel}: ${t}`),
    );
    const fromData = LABEL_DATA_FILES.flatMap((rel) =>
      propertyStrings(readSrc(rel))
        .filter((t) => RETIRED_TERMS.test(t))
        .map((t) => `${rel}: ${t}`),
    );
    expect([...fromChrome, ...fromData]).toEqual([]);
  });
});
```

(`CODE_NAMES` is applied to visible strings only: property strings include IPC ids such as `oratio.transcribe`, which are not chrome.)

- [ ] **Step 3: Run to verify failure; save the output**

Run: `mkdir -p target/visual-lang && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/englishVocabulary.test.ts > target/visual-lang/vocab-red.txt 2>&1; tail -70 target/visual-lang/vocab-red.txt`
Expected (verified against the tree on 2026-09-28): these FAIL —
- "every English lexicon label…" with `vg-corpus-health: Graphify Corpus Health` and `mat-axis: Axis Inspector`;
- "NAV_LABELS covers…" (`mercatus` is `'Mercatus'`, expected `'Market'`);
- "the command palette…" with at least `mercatus: Mercatus`, `flow: Agents`, `catalog: Commands`;
- "no visible string contains a Latin or code name" with `components/surfaces/Chat/SecretaryToast.tsx: Secretary suggests a task` and `…: Dismiss secretary toast`;
- "no visible string or label datum uses a retired term" with `components/surfaces/Loquela/Loquela.tsx: Auto · Router` and `…: Cloud · Cascade` (twice each) and `components/surfaces/Loquela/RiskPopover.tsx:` the two `COPY` values containing "grounding" — plus anything Plan 3a left (record it; Step 9 decides).

These PASS: the three self-tests and "scans a real, non-empty file set". If one of the expected-FAIL tests passes, STOP and say which.

- [ ] **Step 4: Implement — lexicon.** In `crates/vox-gui/ui/src/lib/lexicon.ts` replace

```ts
  'vg-corpus-health': { en: 'Graphify Corpus Health', la: 'Sanitas Corporis' },
```
with
```ts
  'vg-corpus-health': { en: 'Search Index Health', la: 'Sanitas Corporis' },
```
and replace
```ts
  'mat-axis': { en: 'Axis Inspector', la: 'Inspector Axis' },
```
with
```ts
  'mat-axis': { en: 'Routing Axes', la: 'Inspector Axis' },
```

- [ ] **Step 5: Implement — `NAV_LABELS` derived.** In `crates/vox-gui/ui/src/lib/navigation.ts`, add as the first line of the file:

```ts
import { sidebarParentLabel } from './lexicon';
```

Then replace the whole literal block that starts with `/** Human-readable labels for breadcrumb segments. */` and ends with the `};` just before `export function labelForNavKey` with:

```ts
/**
 * English labels for nav keys (breadcrumb segments, sidebar children, tab chips), derived from
 * LEXICON — the one label source. A top-level parent uses its `nav:` override (runs → Review).
 */
export const NAV_LABELS: Record<string, string> = Object.fromEntries(
  [...TOP_LEVEL_VIEWS, ...Object.keys(PARENT_CHILD_MAP)].map((key) => [key, sidebarParentLabel(key, 'en')]),
);
```

`labelForNavKey` stays exactly as it is.

- [ ] **Step 6: Implement — palette label.** In `crates/vox-gui/ui/src/lib/federatedSearchIndex.ts`, inside `buildFederatedIndex`'s `for (const s of sources.surfaces)` loop, replace the line

```ts
      label: s.navLabel,
```
with
```ts
      label: LEXICON[s.viewKey]?.en ?? s.navLabel,
```

(`LEXICON` is already imported there; `keywords` keeps the Latin form and the registry label, so searching "Mercatus" still finds Market.)

- [ ] **Step 7: Implement — Suggested task, tier labels, risk copy.**
  1. `crates/vox-gui/ui/src/components/surfaces/Chat/SecretaryToast.tsx`: replace the text `Secretary suggests a task` with `Suggested task`, and the attribute value `aria-label="Dismiss secretary toast"` with `aria-label="Dismiss suggested task"`. (Its test clicks `/dismiss/i`, which still matches.)
  2. `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`, inside the two arrays `const LQ_TIERS = [` and `const ROUTING_TIERS = [` only, replace these `label:` values (ids, `detail`, `cost`, `lat` unchanged): `label: "Auto · Router"` → `label: "Auto"`, `label: "Local · Mens"` → `label: "Local"`, `label: "Mesh · Peers"` → `label: "Mesh"`, `label: "Cloud · Cascade"` → `label: "Cloud"`. The trigger shows `tierObj.label.split(' · ')[0]`, so it still reads "Auto"/"Local"/… ; the dropdown now shows the plain tier name. If Plan 3a already renamed these labels, keep its names unless they contain "Router" or "Cascade".
  3. `crates/vox-gui/ui/src/components/surfaces/Loquela/RiskPopover.tsx`, in `const COPY: Record<RiskId, string> = {` only (Plan 3a edits this file earlier, so anchor on the `COPY` symbol, not on lines): set
     ```ts
       moderate: 'Confirm, and check replies. Balanced safety.',
       low: 'Enforce verification and check replies, raise approval, spend safety tokens, lean model up.',
     ```
     leaving `high` unchanged. If 3a already reworded `COPY`, replace only the word "grounding" (and its verb) with "check replies" in each value and say so.

- [ ] **Step 8: Update the one test that asserts the old Panels label.** The Panels menu checkbox and dock tab read `labelForNavKey('mercatus')`, now "Market". Run exactly:

`sed -i '' -E -e 's#/\^mercatus\$/i#/^market$/i#g' -e "s/getAllByText\('Mercatus'\)/getAllByText('Market')/g" crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx`

<!-- AMENDED: T3 — assert that no old label query remains instead of exact counts (3a may have added or removed such tests). -->
Then verify that no old label query remains: `rg -n -F '/^mercatus$/i' crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx` → no output (exit 1), and `rg -n -F "getAllByText('Mercatus')" crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx` → no output (exit 1). Identifiers such as `MercatusDockPanel` and test titles stay unchanged.

- [ ] **Step 9: Run to verify pass**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/englishVocabulary.test.ts src/lib src/hooks src/components/layout src/components/surfaces/Chat src/components/surfaces/Loquela src/App.test.tsx 2>&1 | tail -15; timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -5`
Expected: all pass (including `lexicon.test.ts`, `navigation.test.ts`, `navigation.vox-search.test.ts`, `federatedSearchIndex.test.ts`, `Omnibar.test.tsx` — which exercises `useFederatedSearchIndex`, a hook without its own test — `Sidebar.test.tsx`, `useLanguage.test.tsx`, `ChatSurface.test.tsx`, `SecretaryToast.test.tsx`, `Loquela.test.tsx`, `RiskPopover.test.tsx`, `App.test.tsx`); typecheck exit 0. Any other failure that is not in `target/visual-lang/vitest-baseline.txt` → STOP with its name and assertion. If "no visible string or label datum uses a retired term" still fails, every remaining hit is outside this task's edits (Plan 3a's scope): STOP and list them (`DRIVE: STOPPED step 9: retired terms left by Plan 3a:` followed by the list) — do not rename them here.

- [ ] **Step 10: Mutation proofs** — run `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/englishVocabulary.test.ts 2>&1 | tail -20` after each mutation, confirm the named test FAILS, restore, confirm it passes again:
  1. `lexicon.ts`: `mercatus: { en: 'Market', la: 'Mercatus' }` → `en: 'Mercatus'` ⇒ "every English lexicon label…", "NAV_LABELS…" and "the command palette…" fail.
  2. `federatedSearchIndex.ts`: `label: LEXICON[s.viewKey]?.en ?? s.navLabel,` → `label: s.navLabel,` ⇒ "the command palette…" fails naming `mercatus`, `flow`, `catalog`.
  3. `SecretaryToast.tsx`: `Suggested task` → `Secretary suggests a task` ⇒ "no visible string contains a Latin or code name" fails naming `SecretaryToast.tsx`.
  4. `Loquela.tsx` `LQ_TIERS`: `label: "Cloud"` → `label: "Cloud · Cascade"` ⇒ "no visible string or label datum uses a retired term" fails naming `Loquela.tsx: Cloud · Cascade`.

  Then `git diff --stat -- crates/vox-gui/ui` lists only the Files of this task.

- [ ] **Step 11: Commit (Claude Code)**

```bash
git add -- crates/vox-gui/ui/src/__tests__/sourceScan.ts crates/vox-gui/ui/src/__tests__/englishVocabulary.test.ts crates/vox-gui/ui/src/lib/lexicon.ts crates/vox-gui/ui/src/lib/navigation.ts crates/vox-gui/ui/src/lib/federatedSearchIndex.ts crates/vox-gui/ui/src/components/surfaces/Chat/SecretaryToast.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx crates/vox-gui/ui/src/components/surfaces/Loquela/RiskPopover.tsx crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx
git commit -m "fix(gui): one label source; English mode shows no Latin, code names or retired terms"
```

---

### Task 4: Status colour goes through `--color-status-*`

<!-- AMENDED: T4 — StatusBarCluster in scope; a hover-collapse guard plus a `git diff -U0` review catch recolours that make hover identical to the base (PlanPanel Approve); the stale Loquela "amber tone" JSX comment is fixed. -->

**Files:**
- Create: `crates/vox-gui/ui/src/__tests__/statusColorTokens.test.ts`
- Modify: the scope file list (class strings and one JSX comment only). On 2026-09-28, after Task 1's deletions, the files with changes were `ChatSurface`, `ChatTranscript`, `ChatTurnEventRow`, `ModelBadge`, `PhaseChip`, `PlanPanel`, `DriveConsole`, `Loquela`, `RiskPopover`, `BottomStatusBar`, `StatusBarCluster` (11).
- Read only: `crates/vox-gui/ui/src/styles/tokens.generated.css` (generated; never edited).

**Interfaces:**
- Consumes: `chatScopeFiles`, `readSrc`, `stripComments`, `SRC_ROOT` from `src/__tests__/sourceScan.ts` (Task 3).
- Produces: no API. Class vocabulary: `text-(--color-status-pass|fail|warn|info)` with the same `bg-`/`border-`/`from-`/`to-` forms and `/NN` opacity; a hover that must differ from its base uses `hover:brightness-125`.

Mapping (the default, applied mechanically in Step 5): `emerald` → pass, `rose` and `red` → fail, `amber` → warn,
`cyan` → info; the shade is dropped, a `/NN` opacity is kept. Step 4 first handles the uses that are **not** status
(brand accents and context chips), which must not become warnings. Because the shade is dropped,
`text-emerald-400 hover:text-emerald-300` collapses to two identical classes; Step 6 finds and fixes those.

- [ ] **Step 1: Write the failing guard** `crates/vox-gui/ui/src/__tests__/statusColorTokens.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { SRC_ROOT, chatScopeFiles, readSrc, stripComments } from './sourceScan';

/** Tailwind palette classes that carry status meaning; status goes through --color-status-* instead. */
const PALETTE_STATUS = /\b(?:text|bg|border|ring|from|to|fill|stroke)-(?:amber|rose|red|emerald|cyan)-\d{2,3}\b/g;
/** Raw hex colours such as `bg-[#0b0b0e]`; components use tokens. */
const RAW_HEX = /#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\b/g;
const STATUS_TOKEN_REF = /--color-status-[a-z]+/g;
/** A status class whose hover variant repeats it exactly (the recolour dropped the shade that told them apart). */
const HOVER_COLLAPSE = /(?<![\w:-])((?:text|bg|border)-\(--color-status-[a-z]+\)(?:\/\d+)?)(?=[\s"'`]).*hover:\1(?=[\s"'`])/;

function offendersIn(rel: string): string[] {
  const code = stripComments(readSrc(rel));
  return [...code.matchAll(PALETTE_STATUS), ...code.matchAll(RAW_HEX)].map((m) => `${rel}: ${m[0]}`);
}

describe('status colour guard patterns (self-test)', () => {
  it('flags palette status classes and raw hex, including variants and opacity', () => {
    const src = '<a className="hover:text-rose-300 bg-amber-400/8 border-emerald-400/20 from-cyan-400/40 text-red-400 bg-[#0b0b0e]" />';
    expect([...src.matchAll(PALETTE_STATUS)].map((m) => m[0])).toEqual([
      'text-rose-300',
      'bg-amber-400',
      'border-emerald-400',
      'from-cyan-400',
      'text-red-400',
    ]);
    expect([...src.matchAll(RAW_HEX)].map((m) => m[0])).toEqual(['#0b0b0e']);
  });

  it('passes token classes, brass and unrelated palette utilities', () => {
    const src = '<a className="text-(--color-status-warn) bg-(--color-status-fail)/6 ring-brass text-brass ring-offset-zinc-950 text-violet-300" />';
    expect([...src.matchAll(PALETTE_STATUS)]).toEqual([]);
    expect([...src.matchAll(RAW_HEX)]).toEqual([]);
  });

  it('flags a hover that repeats its base status class, passes distinct hovers', () => {
    expect(HOVER_COLLAPSE.test('className="text-(--color-status-pass) hover:text-(--color-status-pass) disabled:opacity-50"')).toBe(true);
    expect(HOVER_COLLAPSE.test('className="bg-(--color-status-fail)/12 hover:bg-(--color-status-fail)/12"')).toBe(true);
    expect(HOVER_COLLAPSE.test('className="text-(--color-status-pass) hover:brightness-125"')).toBe(false);
    expect(HOVER_COLLAPSE.test('className="bg-(--color-status-fail)/12 hover:bg-(--color-status-fail)/18"')).toBe(false);
    expect(HOVER_COLLAPSE.test('className="text-text-muted hover:text-(--color-status-fail)"')).toBe(false);
  });
});

describe('chat, composer and status bar route status colour through --color-status-* tokens', () => {
  it('scans a real, non-empty file set', () => {
    expect(chatScopeFiles()).toContain('components/layout/BottomStatusBar.tsx');
    expect(chatScopeFiles()).toContain('components/common/StatusBarCluster.tsx');
    expect(chatScopeFiles().length).toBeGreaterThanOrEqual(12);
  });

  it('uses no Tailwind palette status classes and no raw hex', () => {
    expect(chatScopeFiles().flatMap(offendersIn)).toEqual([]);
  });

  it('actually uses the status tokens', () => {
    expect(chatScopeFiles().filter((rel) => readSrc(rel).includes('--color-status-')).length).toBeGreaterThan(0);
  });

  it('never gives a status class a hover that repeats it', () => {
    const offenders = chatScopeFiles().flatMap((rel) =>
      stripComments(readSrc(rel))
        .split('\n')
        .filter((line) => HOVER_COLLAPSE.test(line))
        .map((line) => `${rel}: ${line.trim()}`),
    );
    expect(offenders).toEqual([]);
  });

  it('references only status tokens the generated token sheet defines', () => {
    // Seam with the real producer: Style Dictionary's output, not a copied list.
    const css = readFileSync(join(SRC_ROOT, 'styles/tokens.generated.css'), 'utf8');
    const defined = new Set([...css.matchAll(/(--color-status-[a-z]+):/g)].map((m) => m[1]));
    expect([...defined].sort()).toEqual([
      '--color-status-fail',
      '--color-status-info',
      '--color-status-pass',
      '--color-status-warn',
    ]);
    const missing = chatScopeFiles().flatMap((rel) =>
      [...readSrc(rel).matchAll(STATUS_TOKEN_REF)]
        .map((m) => m[0])
        .filter((t) => !defined.has(t))
        .map((t) => `${rel}: ${t}`),
    );
    expect(missing).toEqual([]);
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/statusColorTokens.test.ts > target/visual-lang/status-red.txt 2>&1; tail -40 target/visual-lang/status-red.txt`
Expected: "uses no Tailwind palette status classes…" FAILS with a long offender list (65 palette classes and 2 hex on 2026-09-28 after Task 1's deletions; it includes `components/surfaces/Chat/PlanPanel.tsx: text-cyan-400` and `components/common/StatusBarCluster.tsx: text-emerald-400`), and "actually uses the status tokens" FAILS. The three self-tests, "scans a real…", "never gives a status class a hover…" and "references only…" pass.

- [ ] **Step 3: Record the offender count** — `grep -c "components/" target/visual-lang/status-red.txt` (paste the number; Step 7 must reach zero).

- [ ] **Step 4: Hand-made exceptions first** (non-status uses; exact find → replace, in these files). If a find string is missing because an earlier plan changed the element, `rg -n` that file for the colour class named in the Find column: if the element is gone, skip the row; if it exists with a different class string, change only its colour classes as the row says. List every skipped or adapted row in your report.

| File (under `crates/vox-gui/ui/src/components/`) | Find | Replace | Why |
|---|---|---|---|
| `surfaces/Chat/ChatSurface.tsx` (2 occurrences, replace both) | `'ring-amber-400'` | `'ring-brass'` | anchor-jump highlight is a brand accent, not a warning |
| `surfaces/Loquela/Loquela.tsx` (`Chip`) | `"border-amber-400/25 text-amber-300 bg-amber-400/5"` | `"border-border-strong text-text-secondary bg-overlay-subtle"` | a file chip is context, not status |
| `surfaces/Loquela/Loquela.tsx` (`Chip`) | `"border-emerald-400/25 text-emerald-300 bg-emerald-400/5"` | `"border-accent-secondary/25 text-accent-secondary bg-accent-secondary/5"` | a branch chip is context (verdigris secondary) |
| `surfaces/Loquela/Loquela.tsx` (`Chip`) | the 6 comment lines starting `// "file" chips used to render as border-cyan-400/text-cyan-300` and ending `// for Doubted/low-confidence states) instead of reusing brass outright.` | `// Context chips are not status: file = neutral, branch = verdigris, skill = brass.` | comment described the old colours |
| `surfaces/Loquela/Loquela.tsx` (file suggestions) | `{/* Matches the file chip's new amber tone above, not the old cyan. */}` | `{/* Neutral, like the file chip above. */}` | comment would be false after the next row |
| `surfaces/Loquela/Loquela.tsx` | `<Icon.file className="size-3 shrink-0 text-amber-300" />` | `<Icon.file className="size-3 shrink-0 text-text-muted" />` | file-suggestion icon |
| `surfaces/Loquela/Loquela.tsx` | `<Icon.cpu className="size-3 text-cyan-300" />` | `<Icon.cpu className="size-3 text-text-muted" />` | model-tier icon |
| `surfaces/Loquela/DriveConsole.tsx` | `bg-linear-to-r from-emerald-400 to-brass` | `bg-brass` | spend meter fill: brand, and no multi-hue gradients (Limes) |
| `surfaces/Chat/PlanPanel.tsx` (`STATUS_COLOR`) | `in_progress: 'text-amber-400',` | `in_progress: 'text-brass',` | in progress is the active state (brass), not a warning |
| `layout/BottomStatusBar.tsx` (achievements trigger) | `text-amber-300/80 hover:bg-overlay-subtle hover:text-amber-200` | `text-brass hover:bg-overlay-subtle hover:text-text-primary` | trophy is a brand accent |
| `surfaces/Chat/ModelBadge.tsx` | `bg-[#0b0b0e]` | `bg-overlay-solid` | raw hex → popover token |
| `surfaces/Loquela/RiskPopover.tsx` | `bg-[#0b0b0e]` | `bg-overlay-solid` | raw hex → popover token |

- [ ] **Step 5: Mechanical mapping for the rest** (one command over the scope file list):

Run:
```bash
cd /Users/brbrainerd/dev/vox/crates/vox-gui/ui && ls src/components/surfaces/Chat/*.tsx src/components/surfaces/Loquela/*.tsx src/components/layout/BottomStatusBar.tsx src/components/common/StatusBarCluster.tsx | grep -v '\.test\.tsx$' | xargs sed -i '' -E -e 's/(text|bg|border|ring|from|to|fill|stroke)-amber-[0-9]{2,3}/\1-(--color-status-warn)/g' -e 's/(text|bg|border|ring|from|to|fill|stroke)-(rose|red)-[0-9]{2,3}/\1-(--color-status-fail)/g' -e 's/(text|bg|border|ring|from|to|fill|stroke)-emerald-[0-9]{2,3}/\1-(--color-status-pass)/g' -e 's/(text|bg|border|ring|from|to|fill|stroke)-cyan-[0-9]{2,3}/\1-(--color-status-info)/g'
```
Expected: no output. Spot-check: `rg -n "status-warn" crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx` shows the system-message tone `border-(--color-status-warn)/20 bg-(--color-status-warn)/6 text-(--color-status-warn)/90`.

- [ ] **Step 6: Review the recolour line by line; fix collapsed hovers by hand**

Run: `git diff -U0 -- crates/vox-gui/ui/src/components | rg "^\+" | rg "hover:" `
Read every printed line. Wherever a `hover:` class is now identical to the element's base class (the guard's "never gives a status class a hover that repeats it" names them; on 2026-09-28 exactly one: the PlanPanel "Approve" button, `className="text-(--color-status-pass) hover:text-(--color-status-pass) disabled:opacity-50"`), replace that `hover:` class with `hover:brightness-125` — e.g. `className="text-(--color-status-pass) hover:brightness-125 disabled:opacity-50"`. Do not touch hovers that already differ (a different opacity such as `/12` → `/18`, or a muted base with a status hover). List every line you changed.

- [ ] **Step 7: Run to verify pass**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/statusColorTokens.test.ts src/components/surfaces/Chat src/components/surfaces/Loquela src/components/layout src/components/common 2>&1 | tail -15; timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -5`
Expected: all pass; typecheck exit 0. A failing test not in the baseline → STOP with name and assertion.

Run: `git diff --numstat -- crates/vox-gui/ui/src/components`
Expected: only files from the scope list, each with a few to ~25 changed lines (none over 60). More than 13 files changed, or a file over 60 lines → STOP and list them (Claude splits the task).

- [ ] **Step 8: Mutation proofs**
  1. In `SecretaryToast.tsx` change `text-text-muted">Suggested task` to `text-amber-300">Suggested task`; run `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/statusColorTokens.test.ts 2>&1 | tail -12` → "uses no Tailwind palette status classes…" FAILS naming `components/surfaces/Chat/SecretaryToast.tsx: text-amber-300`. Restore; rerun → passes.
  2. In `PlanPanel.tsx` change the Approve button's `hover:brightness-125` back to `hover:text-(--color-status-pass)`; rerun → "never gives a status class a hover that repeats it" FAILS naming `PlanPanel.tsx`. Restore; rerun → passes.
  3. In any one file the mapping touched, change one `--color-status-warn` to `--color-status-warning`; rerun → "references only status tokens…" FAILS naming that file. Restore; rerun → passes.

  `git diff --stat` afterwards lists only this task's files.

- [ ] **Step 9: Commit (Claude Code)** — Claude reviews every hunk against the mapping table (no brand accent became a warning; no hover collapsed), then:

```bash
git add -- crates/vox-gui/ui/src/__tests__/statusColorTokens.test.ts crates/vox-gui/ui/src/components/surfaces/Chat crates/vox-gui/ui/src/components/surfaces/Loquela crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx
git commit -m "fix(gui): route chat, composer and status-bar status colour through status tokens"
```

---

### Task 5: Type scale, two tracked-caps styles, no glows

<!-- AMENDED: T5 — StatusBarCluster in scope; tracking is checked per string literal (display = the same className string has font-display), so multi-line classNames are judged correctly; ds/components.css section head updated with index.css; before/after capture requires no new overflow or icon issues from the font bump. -->

**Files:**
- Create: `crates/vox-gui/ui/src/__tests__/chatTypeScale.test.ts`; (never committed) `target/visual-lang/type-before/`, `target/visual-lang/type-after/`, `target/visual-lang/type-before.txt`, `target/visual-lang/type-after.txt`
- Modify: the scope file list (class strings only); `crates/vox-gui/ui/src/index.css` (two utilities); `crates/vox-gui/ds/components.css` (the `.ds-section-head` rule — its header says "keep the two in sync" with `index.css`); `crates/vox-gui/ds/conventions.md` (one table row)

**Interfaces:**
- Consumes: `chatScopeFiles`, `readSrc`, `stripComments`, `SRC_ROOT` (Task 3); §Shared script.
- Produces: the rule set — data text ≥ 11 px (`text-[Npx]` with N < 11 is forbidden in scope); within any one string literal, tracking is `tracking-[0.13em]` when that string contains `font-display` (display caps) and `tracking-[0.08em]` otherwise (micro caps), with `tracking-tight|tighter|normal` always allowed; no `shadow-[…--brass…]` glow in scope; `index.css` `@utility vox-range` has no `box-shadow`; the section head is 11 px at 0.13em in both `index.css` and `ds/components.css`.

- [ ] **Step 1: Write the failing guard** `crates/vox-gui/ui/src/__tests__/chatTypeScale.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { SRC_ROOT, chatScopeFiles, readSrc, stripComments } from './sourceScan';

const MIN_DATA_PX = 11;
const PX_TEXT = /\btext-\[(\d+(?:\.\d+)?)px\]/g;
const TRACKING = /\btracking-(\[[^\]]+\]|[a-z]+)/g;
/** Non-caps tightening, allowed anywhere. */
const NEUTRAL_TRACKING = new Set(['tight', 'tighter', 'normal']);
/** One string literal: a whole template (even multi-line), or a single-line quoted string. */
const STRING_LITERAL = /`[^`]*`|"[^"\n]*"|'[^'\n]*'/g;
const BRASS_GLOW = /shadow-\[[^\]]*--brass[^\]]*\]/g;

/** Tracking offenders in one class string: display caps (font-display) use 0.13em, everything else 0.08em. */
function trackingOffenders(classes: string): string[] {
  const want = classes.includes('font-display') ? '[0.13em]' : '[0.08em]';
  return [...classes.matchAll(TRACKING)]
    .filter((m) => !NEUTRAL_TRACKING.has(m[1]) && m[1] !== want)
    .map((m) => `${m[0]} (want tracking-${want})`);
}

function typeOffenders(rel: string): string[] {
  const code = stripComments(readSrc(rel));
  const out: string[] = [];
  for (const m of code.matchAll(PX_TEXT)) if (parseFloat(m[1]) < MIN_DATA_PX) out.push(`${rel}: ${m[0]}`);
  for (const s of code.matchAll(STRING_LITERAL)) for (const t of trackingOffenders(s[0])) out.push(`${rel}: ${t}`);
  for (const m of code.matchAll(BRASS_GLOW)) out.push(`${rel}: ${m[0]}`);
  return out;
}

function cssBlock(css: string, start: string): string {
  const i = css.indexOf(start);
  expect(i, `${start} missing`).toBeGreaterThan(-1);
  return css.slice(i, css.indexOf('}', i));
}
const appCss = readFileSync(join(SRC_ROOT, 'index.css'), 'utf8');
const dsCss = readFileSync(join(SRC_ROOT, '..', '..', 'ds', 'components.css'), 'utf8');

describe('type-scale guard patterns (self-test)', () => {
  it('flags small text and brass glows; passes 11px and neutral shadows', () => {
    const bad = 'text-[10px] text-[9.5px] shadow-[0_0_24px_-8px_rgb(var(--brass)/0.6)]';
    expect([...bad.matchAll(PX_TEXT)].filter((m) => parseFloat(m[1]) < MIN_DATA_PX)).toHaveLength(2);
    expect([...bad.matchAll(BRASS_GLOW)]).toHaveLength(1);
    const good = 'text-[11px] text-[13px] shadow-lg shadow-[0_1px_0_rgba(255,255,255,0.04)_inset]';
    expect([...good.matchAll(PX_TEXT)].filter((m) => parseFloat(m[1]) < MIN_DATA_PX)).toHaveLength(0);
    expect([...good.matchAll(BRASS_GLOW)]).toHaveLength(0);
  });

  it('pairs tracking with the class string, not the line', () => {
    expect(trackingOffenders('font-display uppercase tracking-[0.13em]')).toEqual([]);
    expect(trackingOffenders('font-mono uppercase tracking-[0.08em]')).toEqual([]);
    expect(trackingOffenders('font-display uppercase tracking-[0.08em]')).toHaveLength(1);
    expect(trackingOffenders('uppercase tracking-[0.13em]')).toHaveLength(1);
    expect(trackingOffenders('uppercase tracking-widest')).toHaveLength(1);
    expect(trackingOffenders('text-[13px] tracking-tight')).toEqual([]);
    // A multi-line template literal is one class string: font-display on the first line governs line two.
    const multi = '<a className={`font-display text-[11px]\n  uppercase tracking-[0.08em] ${x ? "a" : "b"}`} />';
    expect([...multi.matchAll(STRING_LITERAL)].flatMap((s) => trackingOffenders(s[0]))).toHaveLength(1);
  });
});

describe('chat, composer and status bar type scale', () => {
  it('scans a real, non-empty file set', () => {
    expect(chatScopeFiles()).toContain('components/surfaces/Loquela/Loquela.tsx');
    expect(chatScopeFiles()).toContain('components/common/StatusBarCluster.tsx');
    expect(chatScopeFiles().length).toBeGreaterThanOrEqual(12);
  });

  it('has no data text under 11px, tracking paired to its caps style, and no brass glow', () => {
    expect(chatScopeFiles().flatMap(typeOffenders)).toEqual([]);
  });
});

describe('Limes stylesheets (no glows; section heads are display caps at data size)', () => {
  it('the range-slider thumb has no glow', () => {
    expect(cssBlock(appCss, '@utility vox-range')).not.toMatch(/box-shadow/);
  });

  it('the app section head is 11px display caps at 0.13em', () => {
    const block = cssBlock(appCss, '@utility ds-section-head');
    expect(block).toContain('font-size: 11px;');
    expect(block).toContain('letter-spacing: 0.13em;');
  });

  it('the design-system copy of the section head matches (ds/components.css says keep in sync)', () => {
    const block = cssBlock(dsCss, '.ds-section-head {');
    expect(block).toContain('font-size: 11px;');
    expect(block).toContain('letter-spacing: 0.13em;');
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/chatTypeScale.test.ts > target/visual-lang/type-red.txt 2>&1; tail -40 target/visual-lang/type-red.txt`
Expected: "has no data text under 11px…" FAILS (on 2026-09-28 after Task 1's deletions: 72 small-text, 29 tracking and 2 glow offenders; the list includes `components/surfaces/Chat/SecretaryToast.tsx: text-[10px]` and `components/common/StatusBarCluster.tsx: tracking-wider`); all three stylesheet tests FAIL (`box-shadow` present; `font-size: 9px` / `letter-spacing: 0.28em` in both files). Both self-tests and "scans a real…" pass.

- [ ] **Step 3: BEFORE layout capture** (write `target/visual-lang/axe-summary.mjs` from §Shared script first if it is missing):

Run: `VOX_REVIEW_CAPTURE=1 timeout 600s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/capture.spec.ts --project=chromium --reporter=line --grep "chat -- " 2>&1 | tail -5; rm -rf target/visual-lang/type-before && cp -R crates/vox-gui/ui/review-bundle/latest target/visual-lang/type-before && node target/visual-lang/axe-summary.mjs target/visual-lang/type-before '^chat--' > target/visual-lang/type-before.txt; tail -1 target/visual-lang/type-before.txt`
Expected: `19 passed` (or `17 passed` if Task 2 restricted a state); the last line with `OVERFLOW_PX=` and `ICON_ISSUES=` values — paste it. (This BEFORE is the state after Task 4.)

- [ ] **Step 4: Mechanical rewrite** (one command over the scope file list):

Run:
```bash
cd /Users/brbrainerd/dev/vox/crates/vox-gui/ui && ls src/components/surfaces/Chat/*.tsx src/components/surfaces/Loquela/*.tsx src/components/layout/BottomStatusBar.tsx src/components/common/StatusBarCluster.tsx | grep -v '\.test\.tsx$' | xargs sed -i '' -E -e 's/text-\[(7|8|9|10)(\.5)?px\]/text-[11px]/g' -e 's/ shadow-\[0_0_[^]]*--brass[^]]*\]//g' -e '/font-display/ s/tracking-(\[[0-9.]+em\]|widest|wider|wide)/tracking-[0.13em]/g' -e '/font-display/! s/tracking-(\[[0-9.]+em\]|widest|wider|wide)/tracking-[0.08em]/g'
```
Expected: no output. The tracking rule here is per line; the guard judges per class string. Spot-check: `rg -n "shadow-\[0_0" crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx` → no output (the composer-focus and Run-button glows are gone; the focus ring `ring-1 ring-brass/30` stays).

- [ ] **Step 5: Stylesheets.**
  1. `crates/vox-gui/ui/src/index.css`: inside `@utility vox-range { … }` delete the line `    box-shadow: 0 0 10px rgb(var(--brass) / 0.5);`; inside `@utility ds-section-head { … }` replace `  font-size: 9px;` with `  font-size: 11px;` and `  letter-spacing: 0.28em;` with `  letter-spacing: 0.13em;`. (`src/index.css.test.ts` checks this block's border and font family only; both stay.)
  2. `crates/vox-gui/ds/components.css`: inside `.ds-section-head { … }` make the same two replacements (`font-size: 9px;` → `font-size: 11px;`, `letter-spacing: 0.28em;` → `letter-spacing: 0.13em;`). Nothing else in that file changes.

- [ ] **Step 6: Document micro caps.** In `crates/vox-gui/ds/conventions.md`, in the Typography table, insert after the row that starts `| Mono | system mono |`:

```md
| Micro caps | Inter or mono, ≥ 11 px | Chip and metadata labels — all-caps with `letter-spacing: 0.08em`. Display (0.13em) and micro (0.08em) are the only tracked-caps styles; data text is never below 11 px. |
```

- [ ] **Step 7: Run to verify pass; fix per-string tracking by hand**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__ src/index.css.test.ts src/components/surfaces/Chat src/components/surfaces/Loquela src/components/layout src/components/common 2>&1 | tail -15; timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -5`
Expected: all pass (including Tasks 3–4's guards); typecheck exit 0. If "…tracking paired to its caps style…" names offenders (a className string split across lines, where the per-line `sed` could not see `font-display`), fix each named class by hand to the `want` value the message gives, and rerun. A failure not in the baseline and not this guard → STOP.

Run: `git diff --numstat -- crates/vox-gui/ui/src crates/vox-gui/ds`
Expected: only this task's files; `Loquela.tsx` is the largest (roughly 30–40 changed lines); none over 60.

- [ ] **Step 8: AFTER layout capture — no new overflow or icon issues**

Run: `VOX_REVIEW_CAPTURE=1 timeout 600s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/capture.spec.ts --project=chromium --reporter=line --grep "chat -- " 2>&1 | tail -5; rm -rf target/visual-lang/type-after && cp -R crates/vox-gui/ui/review-bundle/latest target/visual-lang/type-after && node target/visual-lang/axe-summary.mjs target/visual-lang/type-after '^chat--' > target/visual-lang/type-after.txt; tail -1 target/visual-lang/type-before.txt; tail -1 target/visual-lang/type-after.txt; diff target/visual-lang/type-before.txt target/visual-lang/type-after.txt | rg "overflow_px|icon_issues" | head -40`
Expected: same pass count as Step 3; `OVERFLOW_PX` and `ICON_ISSUES` in the AFTER line are each ≤ the BEFORE line, and no single entry's `overflow_px=` or `icon_issues=` went up (compare the paired `<`/`>` lines). If any went up, STOP and paste both lines for those entries (Claude decides whether a container needs room or a class must stay smaller).

- [ ] **Step 9: Mutation proofs** — after each, run `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/chatTypeScale.test.ts 2>&1 | tail -12`, confirm the FAIL, restore, confirm pass:
  1. `SecretaryToast.tsx`: `text-[11px] text-text-muted">Suggested task` → `text-[10px] text-text-muted">Suggested task` ⇒ "has no data text under 11px…" fails naming `SecretaryToast.tsx: text-[10px]`.
  2. same line: add ` uppercase tracking-[0.13em]` after `text-[11px]` (no `font-display` in that string) ⇒ same test fails naming `tracking-[0.13em] (want tracking-[0.08em])`.
  3. `index.css` `@utility vox-range`: re-add `    box-shadow: 0 0 10px rgb(var(--brass) / 0.5);` inside the thumb block ⇒ "the range-slider thumb has no glow" fails.
  4. `ds/components.css`: set `.ds-section-head`'s `font-size: 11px;` back to `font-size: 9px;` ⇒ "the design-system copy of the section head matches…" fails.

- [ ] **Step 10: Commit (Claude Code)** — Claude views `target/visual-lang/type-after/chat--default--compact--chromium.png` and `chat--composer-filled--compact--chromium.png` for clipping before committing.

```bash
git add -- crates/vox-gui/ui/src/__tests__/chatTypeScale.test.ts crates/vox-gui/ui/src/components/surfaces/Chat crates/vox-gui/ui/src/components/surfaces/Loquela crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx crates/vox-gui/ui/src/index.css crates/vox-gui/ds/components.css crates/vox-gui/ds/conventions.md
git commit -m "fix(gui): 11px minimum data text, two tracked-caps styles, no glows in chat"
```

---

### Task 6: Contrast classes and heading order (was Task 6a)

<!-- AMENDED: T6 — split: this task is the contrast sed, the chatContrast guard (now also text-text-muted/NN and text-white/NN) and the heading fix; Activity and the capture acceptance moved to Task 7. -->

**Files:**
- Create: `crates/vox-gui/ui/src/__tests__/chatContrast.test.ts`
- Modify: the scope file list (class strings only); `crates/vox-gui/ui/src/components/ui/EmptyState.tsx`; `crates/vox-gui/ui/src/components/ui/EmptyState.test.tsx` (one appended `it`); `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` (one prop); `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx` (one inserted `it`)

**Interfaces:**
- Consumes: `contrastRatio(fg: string, bg: string): number` (`src/lib/contrast.ts`); `tokens` (`src/styles/tokens.generated.ts`, generated — read only); `chatScopeFiles`, `readSrc`, `stripComments` (Task 3).
- Produces: `EmptyStateProps.headingLevel?: 2 | 3` (default 3; existing callers unchanged). Replacement classes: `text-zinc-500|600|700` and `text-text-muted/NN` → `text-text-muted`; `text-brass/NN` → `text-brass`; `text-white/NN` → `text-text-secondary`.

Baseline quoted from the critique (review capture, 2026-09-28, before this plan): every chat state had
`color-contrast` serious ×2 — `.text-zinc-500.font-mono` (`/50.00`, 11 px on `#11151a`, 3.79:1) and `.text-brass/70`
("Skill", 10 px, 4.12:1); `chat--empty` and `chat--error` also had `heading-order` (moderate) on the `EmptyState` `h3`
"No messages yet" directly under the page's `h1`. Task 7 measures the result.

- [ ] **Step 1: Write the failing tests.**

(a) `crates/vox-gui/ui/src/__tests__/chatContrast.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { contrastRatio } from '../lib/contrast';
import { tokens } from '../styles/tokens.generated';
import { chatScopeFiles, readSrc, stripComments } from './sourceScan';

/**
 * Text classes that fall below 4.5:1 on the chat surfaces: zinc 500-700 (axe, 2026-09-28) and any
 * opacity-reduced text (translucent brass, muted or white text) — tokens are used at full strength.
 */
const LOW_CONTRAST_TEXT = /\btext-(?:zinc-(?:500|600|700)|brass\/\d+|text-muted\/\d+|white\/\d+)(?![\w/])/g;
const AA_TEXT = 4.5;

const SURFACES: Record<string, string> = {
  'bg.base': tokens.color.bg.base,
  'bg.surface': tokens.color.bg.surface,
  'bg.elevated': tokens.color.bg.elevated,
  'overlay.solid': tokens.color.overlay.solid,
};

/** The token colours this plan puts on chat text in place of zinc, translucent brass and palette status classes. */
const TEXT_TOKENS: Record<string, string> = {
  'text.muted': tokens.color.text.muted,
  'text.secondary': tokens.color.text.secondary,
  'accent.default (brass)': tokens.color.accent.default,
  'status.pass': tokens.color.status.pass,
  'status.fail': tokens.color.status.fail,
  'status.warn': tokens.color.status.warn,
  'status.info': tokens.color.status.info,
};

describe('replacement text tokens meet WCAG AA on every basalt surface', () => {
  for (const [fgName, fg] of Object.entries(TEXT_TOKENS)) {
    for (const [bgName, bg] of Object.entries(SURFACES)) {
      it(`${fgName} on ${bgName} is at least 4.5:1`, () => {
        expect(contrastRatio(fg, bg)).toBeGreaterThanOrEqual(AA_TEXT);
      });
    }
  }

  it('the replaced pairs really fail, so this suite discriminates', () => {
    // text-zinc-500 on the composer (overlay.solid): axe measured 3.79:1.
    expect(contrastRatio('#71717b', tokens.color.overlay.solid)).toBeLessThan(AA_TEXT);
    // text-brass/70 on the skill chip: axe measured #957a3c on #1c1d1d, 4.12:1.
    expect(contrastRatio('#957a3c', '#1c1d1d')).toBeLessThan(AA_TEXT);
  });
});

describe('low-contrast text guard', () => {
  it('flags zinc 500-700 and opacity-reduced text, passes the token classes (self-test)', () => {
    const bad = 'text-zinc-500 hover:text-zinc-600 text-zinc-700 text-brass/70 hover:text-brass/80 text-text-muted/70 text-white/40';
    expect([...bad.matchAll(LOW_CONTRAST_TEXT)]).toHaveLength(7);
    const good = 'text-text-muted text-text-secondary text-brass text-zinc-300 text-white bg-brass/6 border-brass/25 ring-brass/30 bg-white/5';
    expect([...good.matchAll(LOW_CONTRAST_TEXT)]).toHaveLength(0);
  });

  it('chat, composer and status bar use no low-contrast text classes', () => {
    const offenders = chatScopeFiles().flatMap((rel) =>
      [...stripComments(readSrc(rel)).matchAll(LOW_CONTRAST_TEXT)].map((m) => `${rel}: ${m[0]}`),
    );
    expect(offenders).toEqual([]);
  });
});
```

(b) Append inside the top-level `describe('EmptyState Primitive', () => { … })` of `crates/vox-gui/ui/src/components/ui/EmptyState.test.tsx`, after its last `it`:

```tsx
  it('renders the title as h3 by default and as h2 when headingLevel is 2', () => {
    const { unmount } = render(<EmptyState title="Default level" />);
    expect(screen.getByRole('heading', { level: 3, name: 'Default level' })).toBeInTheDocument();
    unmount();
    render(<EmptyState title="Raised" headingLevel={2} />);
    expect(screen.getByRole('heading', { level: 2, name: 'Raised' })).toBeInTheDocument();
  });
```

(c) In `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx`, insert directly after the whole `it('renders an empty state when the session has no messages', async () => { … });` block:

```tsx
  it('gives the empty-state title the heading level after the h1 (axe heading-order)', async () => {
    render(<LanguageProvider><ChatSurface pushToast={noopToast} activeSessionId="s1" messages={[]} /></LanguageProvider>);
    expect(await screen.findByRole('heading', { level: 2, name: /no messages yet/i })).toBeDefined();
  });
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__/chatContrast.test.ts src/components/ui/EmptyState.test.tsx src/components/surfaces/Chat/ChatSurface.test.tsx > target/visual-lang/contrast-red.txt 2>&1; grep -E "✓|×|FAIL|passed|failed" target/visual-lang/contrast-red.txt | tail -40`
Expected FAIL: "chat, composer and status bar use no low-contrast text classes" (13 offenders on 2026-09-28, e.g. `components/surfaces/Loquela/Loquela.tsx: text-brass/70`), the new EmptyState `it` (no level-2 heading) and the new ChatSurface `it` (the title is `h3`). Expected PASS: all 28 token-pair tests, "the replaced pairs really fail", the self-test, and every pre-existing `EmptyState`/`ChatSurface` test. A token-pair failure is a STOP (the tokens changed; do not edit generated files).

- [ ] **Step 3: Implement — class rewrite** (one command over the scope file list):

Run:
```bash
cd /Users/brbrainerd/dev/vox/crates/vox-gui/ui && ls src/components/surfaces/Chat/*.tsx src/components/surfaces/Loquela/*.tsx src/components/layout/BottomStatusBar.tsx src/components/common/StatusBarCluster.tsx | grep -v '\.test\.tsx$' | xargs sed -i '' -E -e 's/text-zinc-(500|600|700)/text-text-muted/g' -e 's/text-text-muted\/[0-9]+/text-text-muted/g' -e 's/text-brass\/[0-9]+/text-brass/g' -e 's/text-white\/[0-9]+/text-text-secondary/g'
```
Expected: no output.

- [ ] **Step 4: Implement — `EmptyState` heading level.** In `crates/vox-gui/ui/src/components/ui/EmptyState.tsx`:
  1. In `interface EmptyStateProps`, after `  children?: React.ReactNode;` add:
     ```ts
       /** Heading level of the title: 2 when the surface's own h1 is the only heading above it (axe heading-order). */
       headingLevel?: 2 | 3;
     ```
  2. In the destructured parameters of `export function EmptyState({ … })`, replace
     ```tsx
       children
     }: EmptyStateProps) {
     ```
     with
     ```tsx
       children,
       headingLevel = 3,
     }: EmptyStateProps) {
     ```
  3. After `  const actualPrimary = primaryAction || action;` add:
     ```ts
       const Heading = headingLevel === 2 ? 'h2' : 'h3';
     ```
  4. Replace `<h3 className="font-display text-sm tracking-widest uppercase text-text-secondary">{title}</h3>` with `<Heading className="font-display text-sm tracking-widest uppercase text-text-secondary">{title}</Heading>`.

- [ ] **Step 5: Implement — chat empty state.** In `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`, in the `<EmptyState` element whose `title="No messages yet"`, add the prop line `            headingLevel={2}` directly after `            title="No messages yet"`.

- [ ] **Step 6: Run to verify pass**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__ src/components/ui src/components/surfaces/Chat src/components/surfaces/Loquela src/components/layout src/components/common 2>&1 | tail -15; timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -5`
Expected: all pass; typecheck exit 0. A failure not in the baseline → STOP.

- [ ] **Step 7: Mutation proofs** — after each, rerun the test file(s) that mutation names with `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run` followed by those paths and `2>&1 | tail -12`, confirm the FAIL, restore, confirm pass:
  1. `SecretaryToast.tsx`: `text-text-muted">Suggested task` → `text-text-muted/70">Suggested task` ⇒ `src/__tests__/chatContrast.test.ts` "…use no low-contrast text classes" fails naming `SecretaryToast.tsx: text-text-muted/70`.
  2. `EmptyState.tsx`: `const Heading = headingLevel === 2 ? 'h2' : 'h3';` → `const Heading = 'h3';` ⇒ both `src/components/ui/EmptyState.test.tsx` (new `it`) and `src/components/surfaces/Chat/ChatSurface.test.tsx` (new `it`) fail.

  Then `git diff --stat -- crates/vox-gui/ui` lists only this task's files.

- [ ] **Step 8: Commit (Claude Code)**

```bash
git add -- crates/vox-gui/ui/src/__tests__/chatContrast.test.ts crates/vox-gui/ui/src/components/surfaces/Chat crates/vox-gui/ui/src/components/surfaces/Loquela crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx crates/vox-gui/ui/src/components/ui/EmptyState.tsx crates/vox-gui/ui/src/components/ui/EmptyState.test.tsx
git commit -m "fix(gui): chat text meets contrast; empty-state heading follows the h1"
```

---

### Task 7: Activity filter labels and the capture acceptance (was Task 6b)

<!-- AMENDED: T6 — split out of the old Task 6; the BEFORE capture is labelled honestly (taken after Tasks 4–6). -->

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.a11y.test.tsx`; (never committed) `target/visual-lang/before/`, `target/visual-lang/after/`, `target/visual-lang/axe-before.txt`, `target/visual-lang/axe-after.txt`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx` (two attributes, one class); and, only if Step 6 names a node, the scope file list (class strings only)

**Interfaces:**
- Consumes: §Shared script; the capture entries (real producer); Tasks 4–6's class vocabulary.
- Produces: nothing new beyond the two `aria-label`s.

- [ ] **Step 1: Write the summary script** — `mkdir -p target/visual-lang`, then (over)write `target/visual-lang/axe-summary.mjs` with exactly the code in §Shared script.

- [ ] **Step 2: BEFORE capture** — this is the state after Tasks 1–6, **not** the 2026-09-28 baseline (that baseline is quoted in Task 6 from the critique). Its purpose is the Activity numbers and anything Tasks 4–6 did not reach.

Run: `VOX_REVIEW_CAPTURE=1 timeout 600s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/capture.spec.ts --project=chromium --reporter=line --grep "(chat|activity) -- [a-z-]+ -- wide$" 2>&1 | tail -5; rm -rf target/visual-lang/before && cp -R crates/vox-gui/ui/review-bundle/latest target/visual-lang/before && node target/visual-lang/axe-summary.mjs target/visual-lang/before '^(chat|activity)--.*--wide--chromium$' > target/visual-lang/axe-before.txt; cat target/visual-lang/axe-before.txt`
Expected: `7 passed`; 6 `chat--…` lines and 1 `activity--…` line. The activity line shows `select-name:critical x2` and `color-contrast:serious x1`. Paste the file. (The capture wipes `review-bundle/latest` on every run, which is why BEFORE is copied under `target/`.)

- [ ] **Step 3: Write the failing test** `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.a11y.test.tsx`:

```tsx
// @vitest-environment jsdom
import { render, screen, act } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';

vi.mock('../../../transport', () => ({
  activityQuery: vi.fn().mockResolvedValue([]),
  listenActivityAppended: vi.fn().mockResolvedValue(() => {}),
  listenAgentEvents: vi.fn().mockResolvedValue(() => {}),
}));

import { ActivitySurface } from './ActivitySurface';

describe('ActivitySurface filter selects are named (axe select-name)', () => {
  it('names both filter selects distinctly, matching their visible labels', async () => {
    render(<ActivitySurface pushToast={vi.fn()} />);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByRole('combobox', { name: 'Agent' })).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Event Type' })).toBeInTheDocument();
    expect(screen.getAllByRole('combobox')).toHaveLength(2);
  });
});
```

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Activity/ActivitySurface.a11y.test.tsx > target/visual-lang/activity-red.txt 2>&1; tail -15 target/visual-lang/activity-red.txt`
Expected: FAIL — no combobox named "Agent" (the `<label>` elements are not associated with their selects).

- [ ] **Step 4: Implement — Activity.** In `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx`:
  1. On the `<select` whose `onChange={(e) => setAgentFilter(e.target.value)}`, add the attribute `aria-label="Agent"` (new line after `value={agentFilter}`).
  2. On the `<select` whose `onChange={(e) => setKindFilter(e.target.value)}`, add `aria-label="Event Type"` (new line after `value={kindFilter}`).
  3. Replace `<p className="text-xs text-zinc-500">` (the "Durable log of high-signal events…" description) with `<p className="text-xs text-text-muted">`.

- [ ] **Step 5: Run to verify pass**

Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Activity src/components/surfaces/Discovery 2>&1 | tail -10; timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -5`
Expected: all pass; typecheck exit 0.

- [ ] **Step 6: AFTER capture — the acceptance**

Run: `VOX_REVIEW_CAPTURE=1 timeout 600s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/capture.spec.ts --project=chromium --reporter=line --grep "(chat|activity) -- [a-z-]+ -- wide$" 2>&1 | tail -5; rm -rf target/visual-lang/after && cp -R crates/vox-gui/ui/review-bundle/latest target/visual-lang/after && node target/visual-lang/axe-summary.mjs target/visual-lang/after '^chat--.*--wide--chromium$' > target/visual-lang/axe-after.txt; node target/visual-lang/axe-summary.mjs target/visual-lang/after '^activity--.*--wide--chromium$' >> target/visual-lang/axe-after.txt; cat target/visual-lang/axe-after.txt; diff target/visual-lang/axe-before.txt target/visual-lang/axe-after.txt`
Expected: `7 passed`. The chat block's summary line reads `ENTRIES=6 STATE_FAILED=0 SERIOUS_OR_CRITICAL_NODES=0`, every chat line shows `serious=0`, and no chat line contains `heading-order`. The activity line shows only `page-has-heading-one:moderate` (deferred) — no `select-name` and no `color-contrast`.

If a chat line still lists a `serious`/`critical` node (most likely inside the `model-picker-open` or `panels-menu-open` popovers, first audited in Task 2): find the node's class in the scope files and apply only these rules (the same as Task 6's `sed`) — `text-zinc-*` or `text-text-muted/NN` text → `text-text-muted`; `text-white/NN` → `text-text-secondary`; `text-brass/NN` → `text-brass`; a palette status colour → the Task 4 mapping. Rerun Task 6's guard (`timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/__tests__`) and this step. After 3 rounds, or for a node these rules do not cover, STOP and paste `axe-after.txt`.

- [ ] **Step 7: Mutation proof** — delete `aria-label="Agent"` from `ActivitySurface.tsx`; `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Activity/ActivitySurface.a11y.test.tsx 2>&1 | tail -10` → FAILS. Restore; rerun → passes. `git diff --stat -- crates/vox-gui/ui` lists only this task's files.

- [ ] **Step 8: Commit (Claude Code)** — Claude opens `target/visual-lang/after/chat--default--wide--chromium.png`, `chat--empty--wide--chromium.png` and `chat--model-picker-open--wide--chromium.png`, checks nothing clips, and keeps `axe-before.txt` / `axe-after.txt` for the PR description.

```bash
git add -- crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.a11y.test.tsx
git status --short -- crates/vox-gui/ui/src/components   # add any scope file Step 6 changed, by path
git commit -m "fix(gui): label Activity filters; chat review states pass axe"
```

---

### Task 8: Verification sweep — Owner: Claude

- [ ] `timeout 600s pnpm --dir crates/vox-gui/ui exec vitest run` — the failing-file set equals Task 1's baseline minus the deleted tests; `timeout 300s pnpm --dir crates/vox-gui/ui typecheck` — clean.
- [ ] `timeout 600s pnpm --dir crates/vox-gui/ui exec playwright test e2e/chat-trust-chips.spec.ts e2e/status-bar-surfaces.spec.ts e2e/chat-interactions.spec.ts e2e/chat-composer-dock.spec.ts e2e/workbench-tabs.spec.ts e2e/model-picker-interactions.spec.ts --project=chromium --reporter=line` — green (these assert chat, composer, status-bar and tab labels).
- [ ] Full review capture at all three viewports for chat: `VOX_REVIEW_CAPTURE=1 timeout 600s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/capture.spec.ts --project=chromium --reporter=line --grep "chat -- "`, then `node target/visual-lang/axe-summary.mjs crates/vox-gui/ui/review-bundle/latest '^chat--'` → `STATE_FAILED=0`, zero serious/critical at `wide`, and `OVERFLOW_PX`/`ICON_ISSUES` no higher than Task 5's `type-before.txt`; record any serious at `laptop`/`compact` as follow-ups. Look at every chat screenshot (larger type must not clip chips, the rail or the status bar).
- [ ] `vox ci pre-push` fast tier; `pnpm --dir crates/vox-gui/ui test:e2e` is left to fleet CI.

---

## Decisions (resolved 2026-09-28; the user delegated open decisions to Claude)

1. **Palette labels come from LEXICON (English).** `buildFederatedIndex` shows `LEXICON[viewKey].en`, falling back to the registry `navLabel`; search keywords keep the Latin form and the registry label. The generated `surfaceRegistry.generated.ts` (from its contract) is not edited here.
2. **`labelForNavKey` stays parent-aware** (`runs` → "Review" via `nav:runs`); `NAV_LABELS` is its derived projection over every nav key. The key set is unchanged (36 keys), so existing callers see the same labels except `mercatus` ("Market").
3. **Retired terms are enforced** by Task 3's guard over the visible strings **and** object-literal strings of every scope file, plus two label-data files. <!-- AMENDED: T3 --> This task fixes the ones in its own files (model tiers → Auto/Local/Mesh/Cloud; risk copy → "check replies"); any other leftover is Plan 3a's and is a STOP. `CODE_NAMES` applies to visible strings only (property strings carry IPC ids such as `oratio.transcribe`).
4. **Status mapping:** emerald → pass, rose/red → fail, amber → warn, cyan → info. Non-status uses (anchor highlight, context chips, file/model icons, spend meter, in-progress plan step, achievements trophy) become brass, neutral or verdigris per Task 4's table. `red` is guarded too (a superset of the critique's four families). <!-- AMENDED: T4 --> A hover that the recolour made identical to its base becomes `hover:brightness-125`, and a guard keeps it that way.
5. **Tracking rule:** display caps (the class string contains `font-display`) use 0.13em; everything else uses micro caps 0.08em. <!-- AMENDED: T5 --> The guard judges each string literal (a multi-line template is one string); the `sed` is per line and the guard catches what it misses. `ds-section-head` moves to 11 px / 0.13em in both `index.css` and `ds/components.css`.
6. **Heading order:** `EmptyState` gains an optional `headingLevel` (default 3, so its other callers are unchanged); chat passes 2.
7. **Stale review states:** `model-picker-open` retargets the composer's model-tier trigger; `session-menu-open` is replaced by `panels-menu-open` (the menu that shows the lexicon labels); `rails-overlay-open` is dropped (no rail toggle exists). Setup clicks carry a 5 s timeout so a stale selector records `state_ok:false` instead of silently producing no entry.
8. **Activity and Approvals:** the two unlabeled selects get `aria-label`s and the Activity description gets a token colour (both small). The missing `h1`s are deferred: adding one inside `ActivitySurface` would duplicate the chat page's `h1` when Discovery is docked in chat, and five surfaces share the problem.
9. <!-- AMENDED: T1 --> **Orphans:** delete `ChatAgentEventRow` (the trace plan revives only `PhaseChip`) and the row types only it used, plus `Loquela/Transcript`, `InlineApprovals` and `DiffReview`, which the `chatDock` removal orphans — otherwise Tasks 4–6 would restyle dead files. `PhaseChip` stays. `loadTaskDiff` and its state stay because the `/diff` slash command calls it.

## Deferred

- **Page `h1` for Activity, Approvals, Console, Coverage and CodeRabbit** (`page-has-heading-one`, moderate): the root fix is one shell-level `sr-only` `h1` from `AppShell`'s `surfaceLabel`, with surface-owned `h1`s (Chat, Dashboard, Tasks, Settings, Browser) removed — not small.
- **Approvals contrast** (serious ×3): the shared `components/ui/Segment.tsx` renders inactive options `text-zinc-500`; it is used app-wide, so fix it with its own review. CodeRabbit contrast ×6, Coverage `scrollable-region-focusable`, Dashboard `heading-order`.
- <!-- AMENDED: T1 --> **`/diff` has no renderer:** after `DiffReview` is deleted, `App.tsx`'s `loadTaskDiff` (called by the `/diff` slash command) still sets `diffOpen`/`diffText`/`diffLoading`, which nothing reads — as it was before, since the dock never rendered. Either wire a diff viewer into the chat surface or remove `/diff`, `loadTaskDiff` and the three state hooks together.
- **`e2e/session-rail-actions.spec.ts`** targets the removed session rail (`chat-session-rail`, "Session actions for …"); retarget it to the sidebar session tabs (double-click rename, Archive button) or delete it.
- **VoxGraph panel strings** ("Graphify status unavailable", "No graphify data", and a stale `vox graphify rebuild --corpus` hint — the CLI is now `vox graph`): fix, then add `components/surfaces/VoxGraph/VoxGraphStatusPanel.tsx` to `SCOPE_EXTRA`.
- **`index.css` `.attention-budget-meter`** hardcodes zinc hexes (`#71717a` fails contrast when expanded) and a brass→rose gradient; the `--color-amber-glow` theme entry is unused.
- **Registry contract labels** (`mercatus` "Mercatus", `flow` "Agents", `catalog` "Commands"): align the contract behind `vox ci gui-surface-registry --write` so the generated file agrees with LEXICON.
- **Wider guard scope:** `styles/tokens.ts` `STATUS_TONE`/`STATUS_BADGE_CLASS` (emerald/red/amber classes behind `StatusPill`, `Pill`, `Toasts`), `lib/visualTokens.ts` viz hexes, and every other surface.
- **Travertine contrast** for the status tokens is not unit-tested (`tokens.generated.ts` holds the dark scope only).
- **Child labels:** the sidebar child row for `runs` reads "Review" (parent override) while `ParentSurface` tabs read "Runs"; breadcrumbs and sidebar children use English even in Latin mode. A child-label helper settles both.
- **Stale comment:** `ChatTurnEventRow.tsx`'s header comment still contrasts itself with the deleted `ChatAgentEventRow`.

## Execution Order

- **Prerequisites:** Phase 5 complete; **Plan 3a (`2026-09-28-chat-surfaces-consolidation.md`) fully committed** — it edits `Loquela.tsx`, `DriveConsole.tsx`, `RiskPopover.tsx`, `BottomStatusBar.tsx`, `StatusBarCluster.tsx`, `ChatExecutionRail.tsx`, `lib/driveConsole.ts`, `hooks/useHudTiles.ts`, and this plan's rewrites and Task 3's retired-term guard run over 3a's final text.
- **Sequential, strictly 1 → 8** (shared files): `SecretaryToast.tsx` (3 → 4 → 5 → 6 mutation anchors), `Loquela.tsx` and `RiskPopover.tsx` (3 → 4 → 5 → 6), `ChatSurface.tsx` (4 → 5 → 6), `ChatSurface.test.tsx` (3 → 6), every scope `.tsx` (4 → 5 → 6 → 7), `src/__tests__/sourceScan.ts` (created in 3, used by 4–6), `target/visual-lang/axe-summary.mjs` (2, 5, 7). Task 1 first so deleted files are never rewritten.
- **Trace plan interplay (independent):** it touches `ChatTranscript.tsx`, `ChatTurnEventRow.tsx`, `ChatExecutionRail.tsx`, a new `TurnTrace.tsx`, `PhaseChip.tsx` and `lib/chatTranscriptTimeline.ts`. Whichever lands second: the guards scan the whole `Chat/` directory, so trace files written after this plan must pass them (the guards say what to change), and `chatTranscriptTimeline.ts` edits are sequential with Task 1. If the trace plan wants the deleted row types or `ChatAgentEventRow` back, it recreates them from git history.
- **Pre-flight per task:** `git status --short -- crates/vox-gui/ui crates/vox-gui/ds` clean for the task's files; HEAD on the branch Claude is committing to; the Vite dev server on :1420 up (Playwright reuses it).
- **SDD ledger:**
  - One label source = `LEXICON`; `NAV_LABELS` derived; palette reads LEXICON — ruling: settled
  - Code-name list (Loquela, Oratio, Mercatus, Scientia, Axis Inspector, Secretary, Graphify, VoxGraph) and retired-term list from the critique — ruling: settled
  - Retired-term enforcement (visible + object-literal strings; tier labels Auto/Local/Mesh/Cloud; "check replies" risk copy) — ruling: **settled only once the surfaces plan (3a) has landed**; until then a leftover is a STOP
  - Status family mapping + non-status exception table + `hover:brightness-125` for collapsed hovers (Task 4) — ruling: settled
  - Guard families `amber|rose|red|emerald|cyan` on `text|bg|border|ring|from|to|fill|stroke`, raw 6/8-digit hex — ruling: settled
  - Scope = `Chat/*.tsx`, `Loquela/*.tsx`, `BottomStatusBar.tsx`, `StatusBarCluster.tsx` (non-test) — ruling: settled
  - Minimum data text 11 px; tracking paired per class string (display 0.13em / micro 0.08em); no new overflow or icon issues from the bump — ruling: settled
  - Section head 11 px / 0.13em in `index.css` and `ds/components.css` together — ruling: settled
  - Low-contrast replacements `text-text-muted` / `text-text-secondary` / `text-brass`, backed by the 28 token-pair tests — ruling: settled
  - `EmptyState.headingLevel` default 3 — ruling: settled
  - Review states: retarget model picker, `panels-menu-open` replaces `session-menu-open`, drop `rails-overlay-open`, 5 s setup timeouts; per-state axe recorded in Task 2 — ruling: settled
  - Activity select labels in scope; `h1`s and Approvals `Segment` contrast deferred — ruling: settled
  - Orphan deletion list incl. `ChatAgentEventRow` and the three chatDock-only Loquela components; `PhaseChip` and `loadTaskDiff` kept — ruling: settled
  - Shared files: `SecretaryToast.tsx` 3 → 4 → 5 → 6; `Loquela.tsx`/`RiskPopover.tsx` 3 → 4 → 5 → 6; `ChatSurface.tsx` 4 → 5 → 6; `ChatSurface.test.tsx` 3 → 6; scope `.tsx` 4 → 5 → 6 → 7 — sequential — settled
