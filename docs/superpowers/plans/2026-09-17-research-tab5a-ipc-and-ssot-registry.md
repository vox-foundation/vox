# Tab 5A: Tauri IPC Commands & SSOT Plain-Language Explainer Registry

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Files modified in Tab 5A are strictly disjoint from Tab 5B (`ResearchView.tsx`) and Tab 5C (`ResearchEngineDrawer.tsx`).
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.

---

## 1. Handoff Contract

- **Upstream Dependencies:** Requires Tab 1 (`quota.rs`), Tab 2 (`FreeTierOffer`), and Tab 4 (`ResearchLane`).
- **Downstream Deliverables:**
  1. `crates/vox-gui/src/commands/search_probe.rs`: Tauri commands `get_research_engine_status` and `save_research_engine_config`.
  2. `crates/vox-gui/src/commands/research.rs`: Updated `start_research_async` accepting `lane: Option<String>`.
  3. `crates/vox-gui/ui/src/debugger/researchExplainerRegistry.ts`: Declarative SSOT dictionary mapping every research stage, metric, and action to plain-English *why/how* descriptions and progressive *mathematical details*.
- **Handoff Consumers:**
  - Tab 5B (Research View) and Tab 5C (Drawer) consume IPC commands and the explainer registry.

---

## 2. Context & Technical Specification

### 2.1 Tauri IPC Commands
- `get_research_engine_status`: Returns current status of all providers (Wikipedia, OpenAlex, arXiv, Tavily, SearXNG), remaining quotas from SQLite, and free tier offers.
- `save_research_engine_config`: Persists provider toggles and base URLs into local application configuration.
- `probe_search_provider`: Updated to accept `"openalex"` and `"arxiv"` and reject/stub `"duckduckgo"`.
- `start_research_async`: Accepts `lane: Option<String>` (`"fast"` vs `"deep"`) and forwards to backend `dei_method::RESEARCH_RUN`.

### 2.2 Declarative SSOT Explainer Registry (`researchExplainerRegistry.ts`)
```typescript
export interface StageExplainer {
  id: string;
  plainTitle: string;
  plainSummary: string;
  whyItMatters: string;
  howItWorks: string;
  technicalMath?: {
    formulaLatex: string;
    parameters: Record<string, string | number>;
    description: string;
  };
  actions: Array<{
    id: string;
    label: string;
    tooltip: string;
    role: 'step' | 'audit' | 'inspect';
  }>;
}

export const RESEARCH_STAGE_EXPLAINERS: Record<string, StageExplainer> = {
  planning: {
    id: 'planning',
    plainTitle: 'Analyzing & Planning Sub-Topics',
    plainSummary: 'Splitting your question into focused sub-queries to ensure complete coverage.',
    whyItMatters: 'Prevents shallow answers by examining different perspectives and tradeoffs independently.',
    howItWorks: 'Measures topic breadth (information entropy) and picks discriminating technical keywords.',
    technicalMath: {
      formulaLatex: '\\mathcal{H}(Q) = -\\sum_{i=1}^n P(w_i) \\log_2 P(w_i)',
      parameters: { threshold: '2.5 bits', maxSubqueries: 8 },
      description: 'Query Information Entropy determines multi-query fan-out breadth.'
    },
    actions: [
      { id: 'step-next', label: 'Step to Retrieval', tooltip: 'Dispatch search queries across enabled engines', role: 'step' }
    ]
  },
  // ... entries for queued, retrieving, verifying_claims, synthesizing, auditing_citations, persisting, completed
};
```

---

## 3. Step-by-Step Implementation

### Step 1: Pre-flight Verification
Run `rg` to verify `probe_search_provider` in `crates/vox-gui/src/commands/search_probe.rs`:
```bash
rg "probe_search_provider" crates/vox-gui/src/commands/search_probe.rs
```

### Step 2: Write Failing Unit Tests for Tauri Search Commands
Create `crates/vox-gui/tests/search_probe_test.rs`:
```rust
#[tokio::test]
async fn test_probe_openalex_and_arxiv_accepted() {
    let res_oa = vox_gui::commands::search_probe::probe_search_provider("openalex".into(), "rust".into()).await;
    assert!(res_oa.is_ok());
    let res_ax = vox_gui::commands::search_probe::probe_search_provider("arxiv".into(), "rust".into()).await;
    assert!(res_ax.is_ok());
}

#[tokio::test]
async fn test_get_research_engine_status_payload() {
    let status = vox_gui::commands::search_probe::get_research_engine_status().await.unwrap();
    assert!(status.providers.iter().any(|p| p.id == "openalex" && p.is_keyless));
    assert!(status.free_key_offers.iter().any(|o| o.provider_id == "tavily" && o.signup_url.contains("tavily.com")));
}
```

### Step 3: Run Failing Rust Tests
```bash
cargo test -p vox-gui --test search_probe_test
```
Expected: FAIL (missing commands and probe arms).

### Step 4: Implement IPC Commands in `search_probe.rs` and `main.rs`
1. Define `ProviderStatusDto`, `FreeKeyOfferDto`, `ResearchEngineStatusDto`, `ResearchEngineConfigDto`.
2. Implement `get_research_engine_status` reading from `vox_db` quota table and `vox_secrets` catalog.
3. Update `probe_search_provider` to call `OpenAlexClient` and `ArXivClient`.
4. Update `start_research_async` in `crates/vox-gui/src/commands/research.rs` with `lane: Option<String>`.
5. Register in `crates/vox-gui/src/main.rs`.

### Step 5: Verify Rust Tests Pass
```bash
cargo test -p vox-gui --test search_probe_test
```
Expected: PASS.

### Step 6: Write Failing Unit Test for `researchExplainerRegistry.ts`
Create `crates/vox-gui/ui/src/debugger/researchExplainerRegistry.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { RESEARCH_STAGE_EXPLAINERS } from './researchExplainerRegistry';

describe('researchExplainerRegistry', () => {
  it('defines all canonical research stages with plain English and actions', () => {
    const requiredStages = ['queued', 'planning', 'retrieving', 'verifying_claims', 'synthesizing', 'auditing_citations', 'persisting', 'completed'];
    for (const s of requiredStages) {
      const explainer = RESEARCH_STAGE_EXPLAINERS[s];
      expect(explainer).toBeDefined();
      expect(explainer.plainTitle.length).toBeGreaterThan(0);
      expect(explainer.whyItMatters.length).toBeGreaterThan(0);
      expect(explainer.actions.length).toBeGreaterThan(0);
    }
  });
});
```

### Step 7: Run Failing TypeScript Test
```bash
pnpm --dir crates/vox-gui/ui test src/debugger/researchExplainerRegistry.test.ts
```
Expected: FAIL (file does not exist).

### Step 8: Implement `researchExplainerRegistry.ts`
Create `crates/vox-gui/ui/src/debugger/researchExplainerRegistry.ts` with all 8 stages, plain descriptions, and LaTeX math.

### Step 9: Verify TypeScript Test Passes
```bash
pnpm --dir crates/vox-gui/ui test src/debugger/researchExplainerRegistry.test.ts
```
Expected: PASS.

### Step 10: Format Code
```bash
cargo fmt -p vox-gui
```

### Step 11: Atomic Commit
```bash
git add crates/vox-gui/src/commands/search_probe.rs crates/vox-gui/src/commands/research.rs crates/vox-gui/src/main.rs crates/vox-gui/tests/search_probe_test.rs crates/vox-gui/ui/src/debugger/researchExplainerRegistry.ts crates/vox-gui/ui/src/debugger/researchExplainerRegistry.test.ts
git commit -m "feat(gui): implement get_research_engine_status IPC commands and SSOT explainer registry"
```
