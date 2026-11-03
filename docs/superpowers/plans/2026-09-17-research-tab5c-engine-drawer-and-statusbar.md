# Tab 5C: Slide-Out ResearchEngineDrawer (`z-50`), Drive Profile Selector, Status Bar System Cluster & Placeholder Audit

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Files modified in Tab 5C are strictly disjoint from Tab 5A and Tab 5B.
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.

---

## 1. Handoff Contract

- **Upstream Dependencies:** Requires Tab 5A (`get_research_engine_status`, `save_research_engine_config`).
- **Downstream Deliverables:**
  1. `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx`: Slide-out source configuration drawer at `z-50` with circular focus trap and isolated `Escape` handling.
  2. `crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx`: Clustered status bar popover for Web, Models, Indices, and Workspace health.
  3. `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`: `VOX_CHAT_RESEARCH_ENABLED` toggle under Orchestrator settings.
  4. `crates/vox-gui/ui/src/debugger/usePipelineStepper.ts`: Audited default payloads with dynamic URLs and realistic scholarly metadata replacing all placeholder mocks.
- **Handoff Consumer:** Tab 6 (E2E Playwright verification) drives the drawer, status bar, and settings.

---

## 2. Context & Technical Specification

### 2.1 Multi-Layer Drawer Hierarchy (`z-50`)
- `ResearchEngineDrawer` is layered at `z-50` with a full-screen backdrop (`fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex justify-end`).
- Circular Focus Trap: Keeps keyboard focus cycled between the close button, source checkboxes, and key input fields.
- `Escape` Key Isolation: Calls `e.stopPropagation()` when dismissed via `Escape` so that it never simultaneously dismisses the underlying `InspectorDrawer` (`z-45`).

### 2.2 Free Key Acquisition Hub
Renders verified cards for free-tier providers:
- **Tavily**: 1,000 monthly searches free. Direct link: `https://app.tavily.com/sign-up` (triggers `open_url`).
- **Google Gemini**: Free API key at `https://aistudio.google.com/app/apikey`.
- **OpenRouter**: Free models at `https://openrouter.ai/keys`.
- **Semantic Scholar**: Free academic API at `https://www.semanticscholar.org/product/api`.

### 2.3 Status Bar Cluster Popover
A compact global pill in the bottom bar (`● Systems 4/4 Ready` + `[ ⚖️ Balanced ]`):
- Clicking opens a 4-quadrant health popover:
  1. **Web & Retrieval**: OpenAlex (200 OK), arXiv (200 OK), Wikipedia (200 OK), Tavily (Quota remaining).
  2. **Inference & Models**: Active model, fallback readiness, token velocity.
  3. **Indices & Storage**: Tantivy index, Qdrant vector store, SQLite quota DB.
  4. **Workspace & Runtime**: Actor daemon, Git VCS status, approval queue.

### 2.4 Placeholder Audit & Dynamic Realism
Replaces all dummy strings (`example.com`, `hit 1`, `hit 2`) in `usePipelineStepper.ts` and `tauriMockShared.ts` with real scholarly URLs, authentic paper titles, domain authority badges, and verified citation snippets.

---

## 3. Step-by-Step Implementation

### Step 1: Pre-flight Verification
Run `rg` to verify `InspectorDrawer` overlay styling and z-index:
```bash
rg "z-45" crates/vox-gui/ui/src/debugger/InspectorDrawer.tsx
```

### Step 2: Write Failing Unit Tests for `ResearchEngineDrawer`
Create `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx`:
```typescript
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ResearchEngineDrawer } from './ResearchEngineDrawer';

describe('ResearchEngineDrawer', () => {
  it('renders Zero-Key Guarantee and free signup links', () => {
    render(<ResearchEngineDrawer isOpen={true} onClose={vi.fn()} />);
    expect(screen.getByText(/Zero-Key Guarantee/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Claim Free Key/i })).toBeInTheDocument();
  });

  it('stops propagation on Escape keypress', () => {
    const onClose = vi.fn();
    render(<ResearchEngineDrawer isOpen={true} onClose={onClose} />);
    const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
    const stopSpy = vi.spyOn(event, 'stopPropagation');
    window.dispatchEvent(event);
    expect(stopSpy).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });
});
```

### Step 3: Run Failing Vitest Tests
```bash
pnpm --dir crates/vox-gui/ui test src/components/surfaces/Research/ResearchEngineDrawer.test.tsx
```
Expected: FAIL (component does not exist).

### Step 4: Implement `ResearchEngineDrawer.tsx`
Create `crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx`:
- Layered at `z-50`.
- Circular focus trap using `useRef` and Tab key handler.
- Stop-propagation on `Escape`.
- Source toggles for Wikipedia, OpenAlex, arXiv, Tavily, SearXNG.
- Free tier cards with `open_url` triggers.

### Step 5: Implement `StatusBarCluster.tsx`
Create `crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx`:
- Surfaces the 4-cluster popover (Web, Models, Indices, Workspace).

### Step 6: Add Orchestrator Chat Research Toggle to `SettingsView.tsx`
In `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`:
Under `section === 'orchestrator'`, add `Autonomous chat research` toggle bound to `VOX_CHAT_RESEARCH_ENABLED`. Register in `settingsIndex.ts`.

### Step 7: Audit and Replace Placeholders in `usePipelineStepper.ts`
Update `DEFAULT_STAGE_PAYLOADS` with authentic research data:
- Titles: *"Attention Is All You Need"*, *"FlashAttention: Fast and Memory-Efficient Exact Attention"*.
- Domains: `arxiv.org`, `openalex.org`, `en.wikipedia.org`.

### Step 8: Verify Vitest Tests Pass
```bash
pnpm --dir crates/vox-gui/ui test src/components/surfaces/Research/ResearchEngineDrawer.test.tsx
```
Expected: PASS.

### Step 9: Atomic Commit
```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.tsx crates/vox-gui/ui/src/components/surfaces/Research/ResearchEngineDrawer.test.tsx crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx crates/vox-gui/ui/src/config/settingsIndex.ts crates/vox-gui/ui/src/debugger/usePipelineStepper.ts crates/vox-gui/ui/e2e/lib/tauriMockShared.ts
git commit -m "feat(ui): implement ResearchEngineDrawer at z-50, status bar cluster, and audited stepper payloads"
```
