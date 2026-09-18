# Tab 4: Orchestrator Dual Lanes, Auto-Escalation Engine, Epistemic Zero-Hit Halt & Deep Synthesis

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Files modified in Tab 4 are localized to `crates/vox-research-shim/` and `crates/vox-orchestrator/`.
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.

---

## 1. Handoff Contract

- **Upstream Dependencies:** Requires Tab 2 (`VoxChatResearchEnabled`) and Tab 3 (`ResearchLane`, `search_with_lane`).
- **Downstream Deliverables:**
  1. `crates/vox-research-shim/src/research/types.rs`: `lane: ResearchLane` on `ResearchQuery`, `low_grounding_evidence: bool` on `ResearchMetadata`.
  2. `crates/vox-research-shim/src/research/orchestrator/pipeline.rs`: Fast lane bypass of query planner, zero-evidence hard halt (`ResearchStage::Failed`), auto-escalation trigger on high entropy / conflict.
  3. `crates/vox-research-shim/src/research/orchestrator/stages.rs`: Comprehensive multi-section synthesis prompt (Executive Summary, Technical Analysis, Contested Claims, Implications) eliminating short stubs.
  4. `crates/vox-orchestrator`: `is_chat_research_enabled()` respecting `VOX_CHAT_RESEARCH_ENABLED`.
- **Handoff Consumers:** Tab 5A (Tauri IPC) and Tab 5B (Research View) consume updated DTOs and lane execution.

---

## 2. Context & Technical Specification

### 2.1 Fast vs. Deep Lane Orchestration
- **Fast Lane (`⚡ Fast`)**:
  - Bypasses LLM planner/decomposition completely.
  - Calls `web_gather::gather_web_hits_for_plan` with single-hop `search_with_lane(ResearchLane::Fast)`.
  - Runs fast claim extraction; skips multi-wave loops.
- **Deep Research Lane (`🔬 Deep`)**:
  - Runs full planner decomposition into orthogonal subqueries.
  - Multi-hop CRAG loop with citation corroboration ($N \ge 2$).
  - Full NLI log-odds verification and resample stability scoring.

### 2.2 Dynamic Auto-Escalation Engine
When executing in default **Balanced** mode:
1. Calculates query entropy $\mathcal{H}(Q)$ and verifier contradiction entropy $\mathcal{H}_{\text{verdict}}$.
2. If $\mathcal{H}_{\text{verdict}} > 0.90$ (evidence from sources directly contradicts on facts or numbers), the pipeline logs an escalation event:
   ```rust
   tracing::info!(target: "vox_research::escalate", "High contradiction entropy ({:.2}); escalating to Deep Investigation", h_verdict);
   ```
3. Dynamically appends 2 targeted verification subqueries and upgrades the verifier model tier.

### 2.3 Epistemic Zero-Hit Hard Halt
$$\forall s \in \text{Stages},\quad \text{status}(s) = \text{Completed} \implies |\text{hits}| > 0$$
If `all_hits.is_empty()`, the pipeline terminates with:
```rust
return Err(anyhow::anyhow!("Zero research hits retrieved across all enabled providers. Halting to prevent hallucinated synthesis."));
```
Removes all fallbacks claiming success from "internal knowledge only" when web research was invoked.

### 2.4 Comprehensive Multi-Section Synthesis
Replaces short stub generation with a structured synthesis prompt requiring:
1. Executive Summary
2. Architectural & Practical Tradeoffs
3. Grounded Core Claims with Inline Citations `[1]`, `[2]`
4. Contested & Ambiguous Findings
5. Implementation Implications

---

## 3. Step-by-Step Implementation

### Step 1: Pre-flight Verification
Run `rg` to verify `ResearchQuery` in `vox-research-shim`:
```bash
rg "pub struct ResearchQuery" crates/vox-research-shim/src/research/types.rs
```

### Step 2: Write Failing Unit Test for Orchestrator Lane Routing
Create `crates/vox-research-shim/tests/lane_orchestrator_test.rs`:
```rust
use vox_research_shim::research::types::{ResearchQuery, ResearchScope};
use vox_search::policy::ResearchLane;

#[test]
fn test_query_lane_defaults_and_metadata_serialization() {
    let q = ResearchQuery {
        query: "What is quantum annealing?".to_string(),
        scope: ResearchScope::All,
        max_sources: 5,
        persist_to_docs: false,
        verify_claims: true,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: ResearchLane::Fast,
    };
    assert_eq!(q.lane, ResearchLane::Fast);
}
```

### Step 3: Run Failing Orchestrator Test
```bash
cargo test -p vox-research-shim --test lane_orchestrator_test
```
Expected: FAIL (`lane` field missing on `ResearchQuery`).

### Step 4: Add `lane` and Metadata Fields to `types.rs`
1. Add `pub lane: vox_search::policy::ResearchLane` to `ResearchQuery`.
2. Add `pub low_grounding_evidence: bool` and `pub suggested_lane: Option<vox_search::policy::ResearchLane>` to `ResearchMetadata`.

### Step 5: Implement Pipeline Dual Lanes and Zero-Hit Halt in `pipeline.rs`
1. In `pipeline.rs`, if `query.lane == Fast`, skip `decompose_query_with_config`.
2. In `pipeline.rs`, enforce `if all_hits.is_empty() { return Err(...); }`.
3. If confidence $< 0.35$, set `metadata.low_grounding_evidence = true`.
4. In `stages.rs`, update `synthesize_answer_with_llm` with comprehensive 5-section markdown prompt.

### Step 6: Implement Chat Research Killswitch in `vox-orchestrator`
In `crates/vox-orchestrator/src/orchestrator/core/mod.rs`:
```rust
pub fn is_chat_research_enabled() -> bool {
    std::env::var("VOX_CHAT_RESEARCH_ENABLED")
        .map(|v| v.trim() != "0" && v.trim().to_lowercase() != "false")
        .unwrap_or(true)
}
```
Short-circuit research tool calls in chat turns when `!is_chat_research_enabled()`.

### Step 7: Verify Orchestrator Tests Pass
```bash
cargo test -p vox-research-shim --test lane_orchestrator_test
```
Expected: PASS.

### Step 8: Format Code
```bash
cargo fmt -p vox-research-shim -p vox-orchestrator
```

### Step 9: Atomic Commit
```bash
git add crates/vox-research-shim/src/research/types.rs crates/vox-research-shim/src/research/orchestrator/pipeline.rs crates/vox-research-shim/src/research/orchestrator/web_gather.rs crates/vox-research-shim/src/research/orchestrator/stages.rs crates/vox-orchestrator/src/orchestrator/core/mod.rs crates/vox-research-shim/tests/lane_orchestrator_test.rs
git commit -m "feat(orchestrator): add Fast/Deep lane routing, epistemic zero-hit halt, and comprehensive synthesis"
```
