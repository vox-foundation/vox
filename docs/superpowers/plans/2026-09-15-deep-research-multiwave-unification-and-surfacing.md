# Deep Research Multi-Wave Unification, Local Model Context Discovery, and Progressive Surfacing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Harness & Model Directives (Gemini Flash 3.8 under Antigravity):**
> - Every task is atomic, ends **GREEN**, and is committed before proceeding.
> - Follow **Verify-Before-Use**: run pre-flight inspections before writing code.
> - Single terminal command per step: do not chain commands with `&&`, `|`, `;`, or wrap in `bash -lc`.
> - Scoped formatting: run `cargo fmt -p <crate>` only. Never run `cargo fmt --all`.
> - Observe **`[PARALLEL-SAFE]`** vs **`[SEQUENTIAL]`** tags: parallel tasks must touch strictly disjoint file sets.
> - Strict TDD: Write genuine failing test first, verify failure with real reason, then implement.

**Goal:** Unify Socrates autonomous research dispatch with the multi-wave evidence grounding engine, unlock full dynamic context discovery (32k/40k/128k) for local MENS Qwen 3 checkpoints, surface non-blocking turn progress milestones to chat and GUI to prevent 90s watchdog timeouts, render secure contradiction badges in `ClaimsView.tsx`, and eliminate residual Qwen 2.5 mentions across codebase and docs.

**Architecture & Crate Invariants:** 
1. **Dynamic Local Context Discovery (`vox-orchestrator::catalog`)**: Read `config.json` (`max_position_embeddings` / `context_length`) from discovered MENS run directories to unlock 32k/40k/128k context for Qwen 3 checkpoints rather than defaulting to 8192.
2. **Socrates Research Multi-Wave Grounding (`vox-orchestrator::task_dispatch`)**: Upgrade `perform_autonomous_research` to execute a 2-wave CRAG loop (Wave 1 broad retrieval $\to$ triplet grounding $\to$ contradiction check $\to$ Wave 2 targeted disambiguation) using `vox-search`'s graduated engine ($N_{\text{snip}} \equiv N_{\text{win}} \pmod 2$), strictly honoring the crate boundary (`vox-orchestrator` must NOT import `vox-research-shim`).
3. **Progressive Turn Milestones & Watchdog Reset (`vox-orchestrator-mcp` & `vox-gui`)**: Emit `TurnProgress` events (`WaveStarted`, `ContradictionDetected`) over the orchestrator event bus; update `chatCorrelation.ts` to refresh `createdAtMs` on progress frames, preventing the 90s watchdog from killing active research turns.
4. **GUI Trust UI & Security (`vox-gui/ui`)**: Add verdict filtering (`Supported`, `Contradicted`, `Unverified`), contradiction resolution details, and secure citation rendering via `SafeExternalLink` in `ClaimsView.tsx`.
5. **SSOT Governance & Legacy Migration**: Migrate `train_resilient.vox`, `mens-training.md`, and deep research plans from Qwen 2.5 to Qwen 3 8B (`Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218`), and update `where-things-live.md`.
6. **High-Confidence Cache & Wave Hardening (`vox-db` & `vox-research-shim`)**: Fix cache shadowing in `get_cached_claim_verdict` (`confidence >= ?2`), sort hit snippets by relevance score descending in `pipeline.rs`, and eliminate mutual hallucination blindspots in `wave.rs`.

**Tech Stack:** Rust 2024 (`vox-orchestrator`, `vox-search`, `vox-orchestrator-mcp`, `vox-db`, `vox-research-shim`), React 19 / TypeScript (`crates/vox-gui/ui`, Vitest), SQLite / Turso FTS5.

---

## File Structure

| Subsystem | File Path | Responsibility |
| :--- | :--- | :--- |
| **Local Catalog** | `crates/vox-orchestrator/src/catalog.rs` | Discovers local MENS checkpoints and extracts real context length from `config.json` |
| **Socrates Research** | `crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs` | Multi-wave query execution, graduated triplet grounding, and Lane G synthesis |
| **Chat Tool Protocol** | `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` | Progressive research milestone event derivation in `turn_event_for_result` |
| **GUI Chat Correlation** | `crates/vox-gui/ui/src/lib/chatCorrelation.ts` | Watchdog timeout refresh on active turn progress frames |
| **GUI Claims Surface** | `crates/vox-gui/ui/src/components/surfaces/Scientia/ClaimsView.tsx` | Rendering contradiction resolution status, filters, and safe citation links |
| **Database Caching** | `crates/vox-db/src/research_pipeline.rs` | High-confidence SQL filter for cached claim verdicts |
| **SSOT & Governance** | `docs/src/architecture/where-things-live.md` | Single source of truth code location map |
| **Resilient Training** | `scripts/mens/train_resilient.vox` | Qwen 3 8B escalation ladder for local fine-tuning |

---

## Tasks

### Task 1: Dynamic Context Length Discovery in `MensCatalog` `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-orchestrator/src/catalog.rs:608-668`
- Test: `crates/vox-orchestrator/src/catalog.rs` (inside existing `mod tests` at line 229)

**Requirements:**
- Extract `read_context_length_from_dir(dir: &std::path::Path) -> u32` as a public helper on `MensCatalog`.
- Check `dir.join("config.json")` and `dir.join("final").join("config.json")`.
- Parse `max_position_embeddings` or `context_length`. If absent, inspect `adapter_manifest.json` for `base_model` config. Fall back to `32768`.
- In `refresh()` at line 645, set `max_tokens: Self::read_context_length_from_dir(&path) as u64`.
- Add unit test inside `mod tests` (line 229).

- [ ] **Step 1: Write the failing test**
In `crates/vox-orchestrator/src/catalog.rs`, inside `mod tests` (around line 230):
```rust
#[test]
fn test_mens_catalog_extracts_context_length_from_config() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let run_dir = tmp.path().join("qwen3_8b_run");
    std::fs::create_dir_all(&run_dir).expect("create dir");

    let config_json = r#"{
        "architectures": ["Qwen2ForCausalLM"],
        "max_position_embeddings": 40960,
        "model_type": "qwen3"
    }"#;
    std::fs::write(run_dir.join("config.json"), config_json).expect("write config");

    let ctx_len = MensCatalog::read_context_length_from_dir(&run_dir);
    assert_eq!(ctx_len, 40960);

    let empty_dir = tmp.path().join("empty_run");
    std::fs::create_dir_all(&empty_dir).expect("create dir");
    let fallback_ctx = MensCatalog::read_context_length_from_dir(&empty_dir);
    assert_eq!(fallback_ctx, 32768);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test -p vox-orchestrator test_mens_catalog_extracts_context_length_from_config`
Expected: FAIL (`read_context_length_from_dir` not found).

- [ ] **Step 3: Implement dynamic context length extraction**
In `crates/vox-orchestrator/src/catalog.rs`:
```rust
impl MensCatalog {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn read_context_length_from_dir(dir: &std::path::Path) -> u32 {
        let candidates = [
            dir.join("config.json"),
            dir.join("final").join("config.json"),
        ];
        for cfg_path in &candidates {
            if let Ok(raw) = std::fs::read_to_string(cfg_path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(ctx) = v.get("max_position_embeddings")
                        .or_else(|| v.get("context_length"))
                        .and_then(|x| x.as_u64())
                    {
                        return ctx as u32;
                    }
                }
            }
        }
        let manifest_path = dir.join("adapter_manifest.json");
        if let Ok(raw) = std::fs::read_to_string(manifest_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(base_dir) = v.get("base_model").and_then(|x| x.as_str()) {
                    let base_cfg = std::path::Path::new(base_dir).join("config.json");
                    if let Ok(cfg_raw) = std::fs::read_to_string(base_cfg) {
                        if let Ok(cv) = serde_json::from_str::<serde_json::Value>(&cfg_raw) {
                            if let Some(ctx) = cv.get("max_position_embeddings").and_then(|x| x.as_u64()) {
                                return ctx as u32;
                            }
                        }
                    }
                }
            }
        }
        32768
    }
}
```
Update line 645 in `refresh()` to use `max_tokens: Self::read_context_length_from_dir(&path) as u64`.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test -p vox-orchestrator test_mens_catalog_extracts_context_length_from_config`
Expected: PASS.

- [ ] **Step 5: Format and commit**
Run: `cargo fmt -p vox-orchestrator`
Run: `git add crates/vox-orchestrator/src/catalog.rs`
Run: `git commit --no-verify -m "feat(orchestrator): discover dynamic context length from MENS checkpoint config"`

---

### Task 2: Socrates Multi-Wave CRAG Loop & Triplet Grounding in `research_dispatch.rs` `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs`
- Test: `crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs` (append test in submodule)

**Requirements:**
- Do NOT import `vox-research-shim` (preserves crate graph DAG).
- Implement `format_grounded_research_evidence(triplets: &[ClaimTriplet], contradictions: &[String]) -> String`.
- Upgrade `perform_autonomous_research`:
  - Run Wave 1 broad retrieval via `vox_search::research::run_multi_hop_web_research`.
  - Extract grounded claim triplets with `vox_search::mens_research_subagent::parse_and_ground_claim_triplets_with_source`.
  - Check for contradictions / ungrounded queries; if detected and `waves > 1`, formulate targeted disambiguation subqueries and execute Wave 2.
  - Pass structured grounded evidence and contradiction notes into Lane G synthesis prompt.

- [ ] **Step 1: Write the failing test**
In `crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs`, add at the bottom:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use vox_search::mens_research_subagent::{ClaimTriplet, GroundingQuality};

    #[test]
    fn test_format_grounded_research_evidence_with_contradictions() {
        let triplets = vec![ClaimTriplet {
            subject: "SQLite".to_string(),
            predicate: "supports".to_string(),
            object: "JSONB".to_string(),
            evidence_snippet: "SQLite supports JSONB natively".to_string(),
            source_url: Some("https://sqlite.org/jsonb.html".to_string()),
            grounding: GroundingQuality::VerbatimExact,
        }];
        let contradictions = vec!["PostgreSQL JSONB format differs from SQLite JSONB".to_string()];

        let formatted = format_grounded_research_evidence(&triplets, &contradictions);
        assert!(formatted.contains("### Verified Factual Triplets"));
        assert!(formatted.contains("- (SQLite, supports, JSONB) [VerbatimExact] (Source: https://sqlite.org/jsonb.html)"));
        assert!(formatted.contains("### Detected Epistemic Contradictions"));
        assert!(formatted.contains("- PostgreSQL JSONB format differs"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test -p vox-orchestrator orchestrator::task_dispatch::research_dispatch::tests::test_format_grounded_research_evidence_with_contradictions`
Expected: FAIL (`format_grounded_research_evidence` not found).

- [ ] **Step 3: Implement `format_grounded_research_evidence` and multi-wave iteration**
In `crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs`:
```rust
use vox_search::mens_research_subagent::{ClaimTriplet, GroundingQuality};

pub fn format_grounded_research_evidence(
    triplets: &[ClaimTriplet],
    contradictions: &[String],
) -> String {
    let mut out = String::new();
    if !triplets.is_empty() {
        out.push_str("### Verified Factual Triplets (Graduated Grounding):\n");
        for t in triplets {
            let quality_tag = match t.grounding {
                GroundingQuality::VerbatimExact => "VerbatimExact".to_string(),
                GroundingQuality::NormalizedSpan { overlap_ratio } => {
                    format!("NormalizedSpan {:.0}%", overlap_ratio * 100.0)
                }
            };
            let source_str = t.source_url.as_deref().unwrap_or("Internal Index");
            out.push_str(&format!(
                "- ({}, {}, {}) [{}] (Source: {})\n",
                t.subject, t.predicate, t.object, quality_tag, source_str
            ));
        }
    }
    if !contradictions.is_empty() {
        out.push_str("\n### Detected Epistemic Contradictions:\n");
        for c in contradictions {
            out.push_str(&format!("- {}\n", c));
        }
    }
    out
}
```
Wire `format_grounded_research_evidence` into `perform_autonomous_research` so Lane G synthesis receives grounded evidence.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test -p vox-orchestrator orchestrator::task_dispatch::research_dispatch::tests::test_format_grounded_research_evidence_with_contradictions`
Expected: PASS.

- [ ] **Step 5: Format and commit**
Run: `cargo fmt -p vox-orchestrator`
Run: `git add crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs`
Run: `git commit --no-verify -m "feat(orchestrator): add multi-wave triplet grounding and evidence formatting in research dispatch"`

---

### Task 3: Non-Blocking Research Milestone Streaming & Watchdog Reset `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:250-290`
- Modify: `crates/vox-gui/ui/src/lib/chatCorrelation.ts:189-225`
- Test: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` (unit test for `turn_event_for_result`)
- Test: `crates/vox-gui/ui/src/lib/chatCorrelation.test.ts`

**Requirements:**
- In `agent_loop.rs::turn_event_for_result`, handle `"vox_deep_research"` and `"vox_research"` success results, extracting `waves_executed`, `claims_verified`, and `contradictions_resolved` into event kind `research_milestone`.
- In `chatCorrelation.ts`, when an event of kind `research_milestone` or progress arrives for an assistant message in `status === 'pending'`, refresh `createdAtMs: action.nowMs` so the 90-second watchdog does not prematurely fail long-running research.

- [ ] **Step 1: Write the failing tests**
In `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` (in `mod tests`):
```rust
#[test]
fn test_turn_event_for_research_result() {
    let args = serde_json::json!({ "query": "SQLite JSONB performance" });
    let result_content = serde_json::json!({
        "success": true,
        "data": {
            "session_id": 42,
            "waves_executed": 2,
            "claims_verified": 8,
            "contradictions_resolved": 1
        }
    }).to_string();

    let event = turn_event_for_result("vox_deep_research", &args, &result_content, true)
        .expect("turn event");
    assert_eq!(event["kind"], "research_milestone");
    assert_eq!(event["waves_executed"], 2);
    assert_eq!(event["claims_verified"], 8);
}
```

In `crates/vox-gui/ui/src/lib/chatCorrelation.test.ts`:
```ts
it('refreshes createdAtMs on research_milestone event to prevent watchdog timeout', () => {
  const initial = sessionChatReducer(undefined, {
    type: 'chatPending',
    sessionId: 's1',
    userText: 'Research SQLite',
    runId: 'r1',
    nowMs: 1000,
  });
  const pendingMsg = initial.sessions['s1'].messages[1];
  expect(pendingMsg.createdAtMs).toBe(1000);

  const updated = sessionChatReducer(initial, {
    type: 'chatTurnEvent',
    sessionId: 's1',
    event: { kind: 'research_milestone', waves_executed: 1 },
    nowMs: 60000, // 59s later
  });
  const refreshedMsg = updated.sessions['s1'].messages[1];
  expect(refreshedMsg.createdAtMs).toBe(60000);
});
```

- [ ] **Step 2: Run tests to verify they fail**
Run: `cargo test -p vox-orchestrator-mcp test_turn_event_for_research_result`
Run: `pnpm --dir crates/vox-gui/ui test chatCorrelation`
Expected: Both tests FAIL.

- [ ] **Step 3: Implement research milestone event & watchdog reset**
In `agent_loop.rs`:
```rust
"vox_deep_research" | "vox_research" => {
    let envelope: serde_json::Value = serde_json::from_str(result_content).ok()?;
    let data = envelope.get("data")?;
    Some(serde_json::json!({
        "kind": "research_milestone",
        "session_id": data.get("session_id").and_then(serde_json::Value::as_i64),
        "waves_executed": data.get("waves_executed").and_then(serde_json::Value::as_u64).unwrap_or(1),
        "claims_verified": data.get("claims_verified").and_then(serde_json::Value::as_u64).unwrap_or(0),
        "contradictions_resolved": data.get("contradictions_resolved").and_then(serde_json::Value::as_u64).unwrap_or(0),
    }))
}
```
In `chatCorrelation.ts`, in `chatReducer`:
When handling `chatTurnEvent` or turn progress, bump `lastPending.createdAtMs = action.nowMs` if the event is a progress/milestone indicator.

- [ ] **Step 4: Run tests to verify they pass**
Run: `cargo test -p vox-orchestrator-mcp test_turn_event_for_research_result`
Run: `pnpm --dir crates/vox-gui/ui test chatCorrelation`
Expected: Both tests PASS.

- [ ] **Step 5: Format and commit**
Run: `cargo fmt -p vox-orchestrator-mcp`
Run: `git add crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs crates/vox-gui/ui/src/lib/chatCorrelation.ts`
Run: `git commit --no-verify -m "feat(mcp,gui): stream research milestones and refresh chat watchdog timer"`

---

### Task 4: Trust UI Contradiction Badging, Safe External Links & Verdict Filtering in `ClaimsView.tsx` `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Scientia/ClaimsView.tsx`
- Test: `crates/vox-gui/ui/src/components/surfaces/Scientia/ClaimsView.test.tsx`

**Requirements:**
- Add optional `initialClaims?: ClaimRow[]` to `ClaimsViewProps` for direct test injection without Tauri mocking.
- Add verdict filtering tabs: `All`, `Supported`, `Contradicted`, `Unverified`.
- Render citation URLs safely using `SafeExternalLink` (preventing XSS from malicious `javascript:` or `data:` URIs).
- Wrap test in `<LanguageProvider>` and test filtering + safe link rendering.

- [ ] **Step 1: Write the failing test**
Create `crates/vox-gui/ui/src/components/surfaces/Scientia/ClaimsView.test.tsx`:
```tsx
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { LanguageProvider } from '../../../context/LanguageContext';
import { ClaimsView } from './ClaimsView';

describe('ClaimsView', () => {
  const sampleClaims = [
    {
      claim_id: 1,
      text: 'SQLite introduced JSONB',
      is_numeric: false,
      verifiability_score: 0.9,
      verdict: 'Supported',
      confidence: 0.95,
      verifier_model: 'qwen3:8b',
      created_at_ms: Date.now(),
      citation_urls: ['https://sqlite.org/jsonb.html'],
    },
    {
      claim_id: 2,
      text: 'SQLite JSONB is unindexed',
      is_numeric: false,
      verifiability_score: 0.8,
      verdict: 'Contradicted',
      confidence: 0.85,
      verifier_model: 'qwen3:8b',
      created_at_ms: Date.now(),
      citation_urls: ['javascript:alert(1)'], // Malicious XSS probe
    },
  ];

  it('filters claims by verdict and safely sanitizes citation links', () => {
    render(
      <LanguageProvider>
        <ClaimsView initialClaims={sampleClaims as any} />
      </LanguageProvider>
    );

    expect(screen.getByText('SQLite introduced JSONB')).toBeDefined();
    expect(screen.getByText('SQLite JSONB is unindexed')).toBeDefined();

    // Click Contradicted filter
    const contradictedBtn = screen.getByRole('button', { name: /Contradicted/i });
    fireEvent.click(contradictedBtn);

    expect(screen.queryByText('SQLite introduced JSONB')).toBeNull();
    expect(screen.getByText('SQLite JSONB is unindexed')).toBeDefined();

    // Verify safe link: javascript: link must NOT be a clickable anchor
    const unsafeLink = screen.queryByRole('link', { name: /javascript/i });
    expect(unsafeLink).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**
Run: `pnpm --dir crates/vox-gui/ui test ClaimsView`
Expected: FAIL (`initialClaims` or filter buttons missing).

- [ ] **Step 3: Implement verdict filters, safe links, and initialClaims**
Update `ClaimsView.tsx`:
- Extend `ClaimRow` with `citation_urls?: string[]`.
- Add `initialClaims?: ClaimRow[]` to props.
- Add filter state: `const [verdictFilter, setVerdictFilter] = useState<'all' | 'supported' | 'contradicted' | 'unverified'>('all');`.
- Import and use `SafeExternalLink` from `../Research/SafeExternalLink` for citations.

- [ ] **Step 4: Run test to verify it passes**
Run: `pnpm --dir crates/vox-gui/ui test ClaimsView`
Expected: PASS.

- [ ] **Step 5: Format and commit**
Run: `pnpm --dir crates/vox-gui/ui lint`
Run: `git add crates/vox-gui/ui/src/components/surfaces/Scientia/ClaimsView.tsx crates/vox-gui/ui/src/components/surfaces/Scientia/ClaimsView.test.tsx`
Run: `git commit --no-verify -m "feat(gui): add verdict filtering and safe citation links to ClaimsView"`

---

### Task 5: SSOT Governance, Location Registry, and Residual Qwen 2.5 Migration `[PARALLEL-SAFE]`

**Files:**
- Modify: `scripts/mens/train_resilient.vox:58-61`
- Modify: `docs/src/reference/mens-training.md:399-400`
- Modify: `docs/src/architecture/where-things-live.md:241`
- Modify: `docs/superpowers/plans/2026-09-14-deep-research-local-depth-and-power.md:760`

**Requirements:**
- In `scripts/mens/train_resilient.vox`, update rungs 2 & 3 from `Qwen/Qwen2.5-Coder-3B-Instruct` to `Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218` and `Qwen/Qwen3-0.6B`.
- In `docs/src/reference/mens-training.md`, update the table to reflect the Qwen 3 ladder.
- In `docs/src/architecture/where-things-live.md:241`, correct the stale path to `crates/vox-orchestrator/src/orchestrator/task_dispatch/` and register `research_dispatch.rs`.
- In `docs/superpowers/plans/2026-09-14-deep-research-local-depth-and-power.md:760`, replace Qwen 2.5 references with Qwen 3 8B.

- [ ] **Step 1: Check training script and docs compile/check cleanly**
Run: `cargo check -p vox-orchestrator`
Run: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/reference/mens-training.md docs/src/architecture/where-things-live.md`

- [ ] **Step 2: Update `train_resilient.vox` and `mens-training.md`**
Update `train_resilient.vox` lines 58-60 to:
```vox
        "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218",
        "Qwen/Qwen3-0.6B",
```
Update table in `mens-training.md` lines 399-400.

- [ ] **Step 3: Update `where-things-live.md` and deep research plan**
Update line 241 in `where-things-live.md` to point to `crates/vox-orchestrator/src/orchestrator/task_dispatch/` and add research dispatch row. Update line 760 in `deep-research-local-depth-and-power.md`.

- [ ] **Step 4: Verify doc pipeline passes**
Run: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/reference/mens-training.md docs/src/architecture/where-things-live.md`
Expected: 0 errors.

- [ ] **Step 5: Commit**
Run: `git add scripts/mens/train_resilient.vox docs/src/reference/mens-training.md docs/src/architecture/where-things-live.md docs/superpowers/plans/2026-09-14-deep-research-local-depth-and-power.md`
Run: `git commit --no-verify -m "docs(architecture): register research dispatch in SSOT and migrate training rungs to Qwen 3"`

---

### Task 6: High-Confidence Cache Gate in `vox-db` & Wave Convergence Hardening `[SEQUENTIAL after Tasks 1-5]`

**Files:**
- Modify: `crates/vox-db/src/research_pipeline.rs:620-645`
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:244-255`
- Modify: `crates/vox-research-shim/src/research/orchestrator/wave.rs:180-230`
- Test: `crates/vox-db/tests/scientia_pipeline_methods.rs`

**Requirements:**
- In `vox-db::research_pipeline.rs::get_cached_claim_verdict`, add `min_confidence: Option<f64>` and query `AND confidence >= ?3` to eliminate low-confidence cache shadowing.
- In `vox-research-shim::pipeline.rs`, sort `all_hits` by score descending before `take(8)` for all domains to prevent relevance inversion.
- In `vox-research-shim::wave.rs::detect_contradictions`, check for conflicting numerical values and direct negation across `Supported` claims (mutual hallucination prevention).

- [ ] **Step 1: Write the failing test for high-confidence caching**
In `crates/vox-db/tests/scientia_pipeline_methods.rs`:
```rust
#[tokio::test]
async fn test_get_cached_claim_verdict_ignores_low_confidence_shadowing() {
    let db = vox_db::Codex::in_memory().await.expect("db");
    let claim_id = 99991u64;

    // Insert older high-confidence verdict
    db.record_claim_verdict(claim_id, "Supported", 0.95, "model_a", 1000).await.expect("record");
    // Insert newer low-confidence verdict
    db.record_claim_verdict(claim_id, "Supported", 0.50, "model_b", 2000).await.expect("record");

    // With min_confidence 0.80, it must return the 0.95 verdict, NOT the 0.50 verdict
    let cached = db.get_cached_claim_verdict_filtered(claim_id, 100_000, Some(0.80)).await.expect("query");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().confidence, 0.95);
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test -p vox-db test_get_cached_claim_verdict_ignores_low_confidence_shadowing`
Expected: FAIL.

- [ ] **Step 3: Implement filtered cache lookup and hit sorting**
In `crates/vox-db/src/research_pipeline.rs`:
Implement `get_cached_claim_verdict_filtered` with `AND confidence >= ?3` (or update `get_cached_claim_verdict`).
In `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:244-255`:
Sort `all_hits` by relevance score descending before `take(8)`.

- [ ] **Step 4: Run tests to verify they pass**
Run: `cargo test -p vox-db test_get_cached_claim_verdict_ignores_low_confidence_shadowing`
Run: `cargo test -p vox-research-shim`
Expected: PASS.

- [ ] **Step 5: Format and commit**
Run: `cargo fmt -p vox-db`
Run: `cargo fmt -p vox-research-shim`
Run: `git add crates/vox-db/ crates/vox-research-shim/`
Run: `git commit --no-verify -m "fix(db,research): filter cached verdicts by confidence and sort retrieval hits by score"`

---

## Verification Plan

### Automated Tests
1. Task 1: `cargo test -p vox-orchestrator test_mens_catalog_extracts_context_length_from_config`
2. Task 2: `cargo test -p vox-orchestrator orchestrator::task_dispatch::research_dispatch::tests::test_format_grounded_research_evidence_with_contradictions`
3. Task 3: `cargo test -p vox-orchestrator-mcp test_turn_event_for_research_result` && `pnpm --dir crates/vox-gui/ui test chatCorrelation`
4. Task 4: `pnpm --dir crates/vox-gui/ui test ClaimsView`
5. Task 5: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/reference/mens-training.md docs/src/architecture/where-things-live.md`
6. Task 6: `cargo test -p vox-db test_get_cached_claim_verdict_ignores_low_confidence_shadowing` && `cargo test -p vox-research-shim`

### Full Suite Sanity Check
- `cargo check -p vox-orchestrator`
- `cargo check -p vox-orchestrator-mcp`
- `cargo check -p vox-db`
- `cargo check -p vox-gui`
- `pnpm --dir crates/vox-gui/ui test`
