# Tab 5B: Frontend Research View Bar: Lane Switcher, Quota Badges & Honesty Guard

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Files modified in Tab 5B are strictly disjoint from Tab 5A and Tab 5C.
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.

---

## 1. Handoff Contract

- **Upstream Dependencies:** Requires Tab 5A (`startResearchAsync` lane forwarding).
- **Downstream Deliverables:**
  1. `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`: Segmented lane switch (`⚡ Fast` vs `🔬 Deep Research`), active source badge strip with dynamic quota, and honesty-sentry-compliant low-evidence guard.
  2. `crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts`: Typed action wrappers passing `lane: 'fast' | 'deep'`.
- **Handoff Consumer:** Tab 6 (E2E verification) exercises the lane switcher and honesty guard.

---

## 2. Context & Technical Specification

### 2.1 Segmented Lane Switcher
Placed above the query input in `ResearchView.tsx`:
- `⚡ Fast Lane (Sub-second Keyless)`: Default. Direct keyless retrieval across Wikipedia, OpenAlex, and arXiv. Max 1,500 ms.
- `🔬 Deep Research (Multi-hop Synthesis)`: Deep recursive CRAG retrieval, subquery fan-out, multi-perspective verification.

### 2.2 Dynamic Source Quota Badges
Renders a compact horizontal strip:
- `Wikipedia: Free` (emerald)
- `arXiv: Free` (emerald)
- `OpenAlex: Free` (emerald)
- `Tavily: {used}/{limit}` (emerald if remaining $> 200$, amber if $\le 200$, rose if $0$)
- Action: `[ ⚙️ Configure Sources & Free Keys ]` which triggers `onOpenEngineDrawer()`.

### 2.3 Honesty Sentry Compliant Low-Evidence Guard
When a fast or deep research run produces evidence confidence $< 0.35$ or zero verified claims:
- Must render `data-testid="empty-results-notice"` with an amber notice:
  *"Low evidence retrieved for this topic. [ Re-run in Deep Research Lane ]"*
- Satisfies `sentryHonesty.ts` so low evidence is acknowledged honestly rather than masking as a fake success.

---

## 3. Step-by-Step Implementation

### Step 1: Pre-flight Verification
Run `rg` to verify existing `ResearchView.tsx` props and exports:
```bash
rg "export function ResearchView" crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx
```

### Step 2: Write Failing Unit Tests for Lane Switcher and Honesty Guard
In `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`:
```typescript
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ResearchView } from './ResearchView';

describe('ResearchView Lane Switch & Honesty Guard', () => {
  it('renders Fast Lane by default and switches to Deep Lane on click', () => {
    render(<ResearchView />);
    const fastBtn = screen.getByTestId('lane-switch-fast');
    const deepBtn = screen.getByTestId('lane-switch-deep');
    expect(fastBtn).toHaveAttribute('aria-selected', 'true');

    fireEvent.click(deepBtn);
    expect(deepBtn).toHaveAttribute('aria-selected', 'true');
  });

  it('renders empty-results-notice when research has low evidence', () => {
    render(<ResearchView initialLowEvidence={true} />);
    expect(screen.getByTestId('empty-results-notice')).toBeInTheDocument();
  });
});
```

### Step 3: Run Failing Vitest Tests
```bash
pnpm --dir crates/vox-gui/ui test src/components/surfaces/Research/ResearchView.test.tsx
```
Expected: FAIL (missing `lane-switch-fast` testid).

### Step 4: Update `researchActions.ts` and `ResearchView.tsx`
1. In `researchActions.ts`, update `startResearchAsync` to accept `lane?: 'fast' | 'deep'`.
2. In `ResearchView.tsx`, add state `lane: 'fast' | 'deep' = 'fast'`.
3. Render segmented buttons (`data-testid="lane-switch-fast"`, `data-testid="lane-switch-deep"`).
4. Render source quota badge strip and trigger `onOpenEngineDrawer`.
5. Render `[data-testid="empty-results-notice"]` banner when low evidence is flagged.

### Step 5: Verify Vitest Tests Pass
```bash
pnpm --dir crates/vox-gui/ui test src/components/surfaces/Research/ResearchView.test.tsx
```
Expected: PASS.

### Step 6: Atomic Commit
```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx
git commit -m "feat(ui): add segmented lane switcher, quota badges, and honesty low-evidence guard"
```
