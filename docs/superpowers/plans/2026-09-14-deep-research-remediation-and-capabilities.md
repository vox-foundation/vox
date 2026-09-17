# Deep Research Full-Surface Remediation and Capabilities Implementation Plan (Hardened)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 
> **Harness & Model Directives (Gemini Flash 3.8 under Antigravity):**
> - Every task is atomic, ends **GREEN**, and is committed before proceeding.
> - Follow **Verify-Before-Use**: run pre-flight `rg` commands to inspect exact signatures before writing code.
> - Adhere to the **Two-Strike Circuit Breaker**: if a verification step fails twice, STOP and write a handoff note.
> - Observe **`[PARALLEL-SAFE]`** vs **`[SEQUENTIAL]`** tags: parallel tasks MUST touch strictly disjoint file sets. Never dispatch two subagents that write to the same file.

**Goal:** Eliminate silent failure modes, false positives, false negatives, and amnesic storage in Vox Deep Research. Establish an asymmetric local/cloud model pipeline, auto-activate Tavily, connect persistent FTS5 research storage and claim deduplication loops, fix the evaluation harness to benchmark real end-to-end pipeline accuracy, and deliver first-party domain engines for shopping comparisons and code generation.

**Architecture:** Seven tightly focused tasks arranged across 4 execution waves.
- **Wave 1 (Search & Storage Foundations):** Task 3 `[PARALLEL-SAFE]` fixes Tavily auto-activation, DDG serde crashes, and RRF test races. Task 4 `[PARALLEL-SAFE]` connects the persistent storage loop in `vox-db`, adds `scientia_research_fts`, and implements cross-session claim deduplication.
- **Wave 2 (Model Cascades & Orchestration):** Task 5 `[SEQUENTIAL]` fixes the synthesis bypass bug, adds schema-aware cascade fallback, enables local-first NLI/extraction, and introduces adaptive early-exit resampling.
- **Wave 3 (Domain Engines & Evaluation):** Task 6 `[PARALLEL-SAFE]` transforms `vox research eval` from raw snippet concatenation into a real end-to-end benchmark. Task 7 `[PARALLEL-SAFE]` implements first-party shopping comparison and code generation engines with sandbox verification.
- **Wave 4 (Surfaces: GUI & Chat):** Task 1 `[PARALLEL-SAFE]` repairs the GUI trust UI, fixes `Contradicted` omissions and corroboration multi-counting, and mounts `ResearchReportMarkdown`. Task 2 `[PARALLEL-SAFE]` plumbs chat parameters, adds `/research` slash routing, and prevents 30–60s synchronous UI lockups.

**Tech Stack:** Rust 2024 (`vox-research-shim`, `vox-search`, `vox-db`, `vox-orchestrator`, `vox-gui`, `vox-cli-research`), React 19 + TypeScript 5 (`crates/vox-gui/ui`, Vitest), SQLite / Turso FTS5 (`vox-db`), OpenRouter + MENS / Ollama LLM facades (`vox-actor-runtime`).

**Spec:** [`docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md`](file:///Users/brbrainerd/dev/vox/docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md)

---

## Global Constraints

1. **Model-Agnostic Facade:** All LLM calls MUST route through `vox_actor_runtime::llm` or `vox_orchestrator::models`. Hardcoded vendor hostnames and direct SDK instantiations are prohibited.
2. **VoxScript-First Glue:** No new `.sh`, `.ps1`, or `.py` scripts. Use `vox run scripts/*.vox`.
3. **No `cargo fmt --all`:** Format only dirty crates or packages with `cargo fmt -p <crate>` or `vox run scripts/fmt.vox`.
4. **Markdown Frontmatter:** Any `.md` created under `docs/src/` requires YAML frontmatter; plans under `docs/superpowers/plans/` follow plan standards.
5. **No Placeholders:** Every step must contain complete, copy-pasteable code, exact paths, and verified signatures.

---

## Wave 1: Search & Storage Substrates

### Task 3: Tavily Auto-Activation, Tier Priority, DDG Serde Fix & Search Test Race Fix `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-search/src/tavily_research.rs:39-65`
- Modify: `crates/vox-search/src/tavily.rs:15-25`
- Modify: `crates/vox-search/src/web_dispatcher.rs:16-120`
- Modify: `crates/vox-search/src/duckduckgo.rs:10-60`
- Modify: `crates/vox-search/src/policy.rs:188-220, 616-645`
- Modify: `crates/vox-search/src/novelty.rs:14-30`
- Test: `crates/vox-search/src/policy.rs`
- Test: `crates/vox-search/src/tavily_research.rs`
- Test: `crates/vox-search/src/duckduckgo.rs`

**Pre-flight Verification:**
Run: `rg "pub fn tavily_research_enabled" crates/vox-search/src/`
Run: `rg "RelatedTopics" crates/vox-search/src/duckduckgo.rs`

**Interfaces:**
- Consumes: `resolve_secret(SecretId::TavilyApiKey)` and `resolve_secret(SecretId::VoxTavilyResearch)`.
- Produces: Auto-activated Tavily web & research clients when `TavilyApiKey` is set. Correct tier ordering: SearXNG → Tavily → DuckDuckGo. Heterogeneous DDG parsing. Zero-allocation novelty shingling.

- [ ] **Step 1: Write the failing unit tests**

In `crates/vox-search/src/duckduckgo.rs` (under `mod tests`):
```rust
#[tokio::test]
async fn test_ddg_parses_heterogeneous_related_topics() {
    let raw_json = r#"{
        "RelatedTopics": [
            {
                "FirstURL": "https://duckduckgo.com/Rust",
                "Text": "Rust programming language"
            },
            {
                "Name": "Other Topics",
                "Topics": [
                    {
                        "FirstURL": "https://duckduckgo.com/Iron_Oxide",
                        "Text": "Iron oxide compound"
                    }
                ]
            }
        ]
    }"#;
    let resp: DdgResponse = serde_json::from_str(raw_json).expect("must parse heterogeneous related topics");
    let hits = resp.flatten_topics(5);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].url, "https://duckduckgo.com/Rust");
    assert_eq!(hits[1].url, "https://duckduckgo.com/Iron_Oxide");
}
```

In `crates/vox-search/src/tavily_research.rs` (under `mod tests`):
```rust
#[test]
fn test_tavily_research_auto_activation() {
    assert!(tavily_research_enabled_with_values(Some("tvly-key"), None));
    assert!(!tavily_research_enabled_with_values(Some("tvly-key"), Some("0")));
    assert!(!tavily_research_enabled_with_values(None, None));
    assert!(tavily_research_enabled_with_values(None, Some("1")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --lib test_ddg_parses_heterogeneous_related_topics`
Expected: FAIL with missing method `flatten_topics` or serde deserialization error.

- [ ] **Step 3: Implement minimal fixes**

1. **Fix DDG Heterogeneous Deserialization** in `crates/vox-search/src/duckduckgo.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DdgTopicItem {
    Single(DdgResult),
    Category {
        #[serde(rename = "Name")]
        name: Option<String>,
        #[serde(rename = "Topics")]
        topics: Vec<DdgResult>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdgResponse {
    #[serde(rename = "RelatedTopics", default)]
    pub related_topics: Vec<DdgTopicItem>,
}

impl DdgResponse {
    pub fn flatten_topics(self, limit: usize) -> Vec<DdgResult> {
        let mut out = Vec::new();
        for item in self.related_topics {
            if out.len() >= limit {
                break;
            }
            match item {
                DdgTopicItem::Single(res) => out.push(res),
                DdgTopicItem::Category { topics, .. } => {
                    for sub in topics {
                        if out.len() >= limit {
                            break;
                        }
                        out.push(sub);
                    }
                }
            }
        }
        out
    }
}
```

2. **Fix Tavily Auto-Activation** in `crates/vox-search/src/tavily_research.rs`:
```rust
pub fn tavily_research_enabled_with_values(api_key: Option<&str>, explicit_override: Option<&str>) -> bool {
    if let Some(v) = explicit_override {
        let v = v.trim();
        return matches!(v, "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON");
    }
    api_key.map(|k| !k.trim().is_empty()).unwrap_or(false)
}

#[must_use]
pub fn tavily_research_enabled() -> bool {
    let key = resolve_secret(SecretId::TavilyApiKey).expose();
    let explicit = resolve_secret(SecretId::VoxTavilyResearch).expose();
    tavily_research_enabled_with_values(key.as_deref(), explicit.as_deref())
}
```

3. **Fix Policy & Auto-enable in `crates/vox-search/src/policy.rs`**:
```rust
// lines 190-195:
tavily_enabled: match resolve_secret(vox_secrets::SecretId::VoxSearchTavilyEnabled).expose() {
    Some(v) => parse_bool_str(v),
    None => resolve_secret(vox_secrets::SecretId::TavilyApiKey).expose().is_some(),
},
```

4. **Reorder Tiers in `crates/vox-search/src/web_dispatcher.rs`**:
SearXNG -> Tavily (if enabled and key present) -> DuckDuckGo fallback.
Also fix line 114: separate robots.txt from text density:
```rust
if doc.text_density >= policy.scraper_min_text_density {
    // accept scraped markdown
} else {
    // fallback to original search engine snippet
}
```

5. **Fix Test Race Condition in `crates/vox-search/src/policy.rs`**:
Replace `std::env::set_var` test mutations with testing a pure string parser or using an atomic test mutex guard.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search`
Expected: PASS with 0 failures and 0 flakes.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-search/src/
git commit -m "fix(search): auto-enable Tavily, parse heterogeneous DDG topics, and fix test races"
```

---

### Task 4: Persistent Research Storage Loop, FTS5 Indexing & Claim Deduplication `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-research-shim/src/research/orchestrator/web_gather.rs:180-270`
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:320-370`
- Modify: `crates/vox-db/src/schema_extensions.rs:35-120`
- Modify: `crates/vox-db/src/research_pipeline.rs:440-520`
- Modify: `crates/vox-db-types/src/research.rs` or `types/research.rs`
- Modify: `crates/vox-cli-research/src/lib.rs:80-140`
- Test: `crates/vox-db/tests/research_search_test.rs`
- Test: `crates/vox-research-shim/tests/claim_dedup_test.rs`

**Pre-flight Verification:**
Run: `rg "ingest_research_document_async" crates/vox-db/src/`
Run: `rg "apply_scientia_research_fts_cutover" crates/vox-db/src/`

**Interfaces:**
- Consumes: `_db` and `_session_id` in `web_gather.rs`.
- Produces: `scientia_research_fts` virtual table, `search_research_artifacts(query, limit)`, `get_cached_claim_verdict(claim_id, max_age_ms)`, and CLI `vox research search <query>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-db/tests/research_search_test.rs`:
```rust
use vox_db::{DbConfig, VoxDb};

#[tokio::test]
async fn test_fts5_research_search_and_deduplication() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("connect");
    let session_id = db.create_research_session("test:sess1", "Rust memory safety guarantees").await.expect("create");

    db.store_claim(session_id, 12345, "Rust prevents data races at compile time", false, false, false).await.expect("claim");
    db.store_claim_verdict(12345, "Supported", 0.95, "mock-verifier").await.expect("verdict");
    db.store_research_artifact(session_id, "{}", "# Rust Invariants\nOwnership guarantees no data races.").await.expect("artifact");

    // 1. Test FTS search
    let search_hits = db.search_research_artifacts("data races", 5).await.expect("search");
    assert!(!search_hits.is_empty(), "must find matching research artifact");
    assert_eq!(search_hits[0].session_id, session_id);

    // 2. Test Claim Cache lookup
    let cached = db.get_cached_claim_verdict(12345, 0).await.expect("lookup");
    assert!(cached.is_some(), "must find recently verified claim");
    let c = cached.unwrap();
    assert_eq!(c.verdict, "Supported");
    assert!(c.confidence >= 0.90);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-db --test research_search_test`
Expected: FAIL with "no method named `get_cached_claim_verdict`".

- [ ] **Step 3: Implement database schema cutover, FTS5 search, and claim caching**

1. In `crates/vox-db/src/schema_extensions.rs`, add `apply_scientia_research_fts_cutover`:
```rust
pub async fn apply_scientia_research_fts_cutover(conn: &turso::Connection) -> Result<(), StoreError> {
    if !has_fts5_support(conn).await? {
        return Ok(());
    }
    let sql = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS scientia_research_fts USING fts5(
    session_id UNINDEXED,
    query_text,
    report_markdown,
    claims_text,
    tokenize = 'porter unicode61'
);
"#;
    exec_optional_batch(conn, sql).await;
    Ok(())
}
```

2. In `crates/vox-db/src/research_pipeline.rs`:
- Implement `get_cached_claim_verdict(claim_id, max_age_ms)`.
- Implement `search_research_artifacts(query, limit)` with FTS5 `snippet()` + BM25 ranking, falling back to LIKE if FTS5 is not compiled.
- In `store_research_artifact`, insert into `scientia_research_fts`.

3. In `crates/vox-research-shim/src/research/orchestrator/web_gather.rs`:
- Stop ignoring `_db` and `_session_id`.
- Iterate through accepted web hits and call `db.create_research_source(session_id, &hit.url, Some(&hit.title))` and `db.ingest_research_document_async(&mut req)`.

4. In `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:320`:
- Before running `verify_claims_with_config`, check `db.get_cached_claim_verdict` for each claim.
- Re-use high-confidence verdicts ($\ge 0.80$, age $\le 14$ days) directly, bypassing 3x LLM resampling.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-db --test research_search_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-db/ crates/vox-research-shim/src/research/orchestrator/web_gather.rs crates/vox-research-shim/src/research/orchestrator/pipeline.rs crates/vox-cli-research/
git commit -m "feat(research): wire web evidence ingestion, FTS5 search, and claim deduplication"
```

---

## Wave 2: Model Cascades & Asymmetric Routing

### Task 5: Asymmetric Local/Cloud Dispatch, Schema Fallback & Adaptive Resampling `[SEQUENTIAL]`

**Files:**
- Modify: `crates/vox-orchestrator/src/models/select.rs:435-520`
- Modify: `crates/vox-actor-runtime/src/llm/cascade.rs:25-80`
- Modify: `crates/vox-research-shim/src/research/orchestrator/stages.rs:180-220, 440-470`
- Modify: `crates/vox-research-shim/src/research/verifier.rs:210-285`
- Modify: `crates/vox-research-shim/src/research/claims.rs:100-150`
- Test: `crates/vox-orchestrator/src/models/select.rs`
- Test: `crates/vox-research-shim/src/research/verifier.rs`

**Pre-flight Verification:**
Run: `rg "synthesize_answer_with_llm" crates/vox-research-shim/src/research/orchestrator/stages.rs`
Run: `rg "pub fn nli_classifier" crates/vox-orchestrator/src/models/select.rs`

**Interfaces:**
- Consumes: `SelectionIntent::snippet_triage()`, `SelectionIntent::claim_extraction()`.
- Produces: Local-preferred NLI/extraction dispatch, adaptive 1-3x sequential resampling, schema-aware cascade fallback `chat_with_cascade_parsed`.

- [ ] **Step 1: Write the failing unit tests**

In `crates/vox-orchestrator/src/models/select.rs` (under `mod tests`):
```rust
#[test]
fn test_local_first_research_intents() {
    let nli = SelectionIntent::nli_classifier();
    assert!(nli.prefer_local, "NLI classifier must prefer local execution");

    let triage = SelectionIntent::snippet_triage();
    assert!(triage.prefer_local, "snippet triage must prefer local execution");

    let extract = SelectionIntent::claim_extraction();
    assert!(extract.prefer_local, "claim extraction must prefer local execution");

    let synth = SelectionIntent::research();
    assert!(!synth.prefer_local, "synthesis must allow cloud frontier models");
}
```

In `crates/vox-research-shim/src/research/verifier.rs` (under `mod tests`):
```rust
#[test]
fn test_adaptive_resampling_early_exit_logic() {
    let s1 = Sample { verdict: Verdict::Supported, confidence: 0.95 };
    assert!(should_early_exit_after_sample_1(&s1), "High confidence on sample 1 must exit immediately");

    let s1_low = Sample { verdict: Verdict::Supported, confidence: 0.70 };
    let s2_same = Sample { verdict: Verdict::Supported, confidence: 0.75 };
    assert!(should_early_exit_after_sample_2(&s1_low, &s2_same), "Agreement on sample 2 locks 2/2 majority; must exit");

    let s2_diff = Sample { verdict: Verdict::Contradicted, confidence: 0.75 };
    assert!(!should_early_exit_after_sample_2(&s1_low, &s2_diff), "Disagreement requires sample 3 tie-breaker");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-orchestrator --lib test_local_first_research_intents`
Expected: FAIL with "no function `snippet_triage`".

- [ ] **Step 3: Implement minimal fixes**

1. **Update `crates/vox-orchestrator/src/models/select.rs`**:
   - Set `prefer_local: true` on `nli_classifier()`.
   - Add `snippet_triage()` and `claim_extraction()` with `prefer_local: true`.

2. **Fix Synthesizer Bypass Bug in `crates/vox-research-shim/src/research/orchestrator/stages.rs:180-195`**:
   Remove the dead `if let (Some(_ep), Some(_key)) = (params.endpoint, params.api_key)` requirement so `call_synthesis_llm` uses the resolved cascade models instead of silently dropping to dumb template concatenation.

3. **Implement `chat_with_cascade_parsed` in `crates/vox-actor-runtime/src/llm/cascade.rs`**:
   When candidate 1 (e.g. local Ollama) returns malformed JSON, try candidate 2 (e.g. OpenRouter cloud) instead of silently returning empty results.

4. **Implement Adaptive Resampling in `crates/vox-research-shim/src/research/verifier.rs`**:
   - Sample 1: if confidence $\ge 0.92$, exit immediately.
   - Sample 2: if sample 2 agrees with sample 1, exit immediately (2/2 majority locked).
   - Sample 3: tie-breaker only when sample 1 and 2 disagree.

5. **Route CoVE Step 2 to Fast/Local Model**:
   In `stages.rs:453`, map `ResearchStage::SelfVerification` to `SelectionIntent::nli_classifier()` or `SelectionIntent::snippet_triage()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-orchestrator --lib test_local_first_research_intents`
Run: `cargo test -p vox-research-shim`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-orchestrator/src/models/select.rs \
        crates/vox-actor-runtime/src/llm/cascade.rs \
        crates/vox-research-shim/src/research/orchestrator/stages.rs \
        crates/vox-research-shim/src/research/verifier.rs \
        crates/vox-research-shim/src/research/claims.rs
git commit -m "feat(orchestrator): asymmetric local/cloud routing, schema-aware fallback, and adaptive resampling"
```

---

## Wave 3: Domain Engines & Evaluation Harness

### Task 6: Real Pipeline Evaluation Harness in `vox research eval` `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-cli-research/src/eval.rs:40-120, 240-270`
- Create: `crates/vox-cli-research/tests/eval_e2e_test.rs`

**Pre-flight Verification:**
Run: `rg "execute_search_plan" crates/vox-cli-research/src/eval.rs`

**Interfaces:**
- Consumes: `vox_research_shim::research::run_research_with_context`.
- Produces: Comprehensive eval sample with ground truth recall, real citation precision, claim verdict counts, and abstention metrics.

- [ ] **Step 1: Write the failing test**

Create `crates/vox-cli-research/tests/eval_e2e_test.rs`:
```rust
use vox_cli_research::eval::{evaluate_single_query_pipeline, GoldenQueryItem};
use vox_search::context::SearchRuntimeContext;

#[tokio::test]
async fn test_eval_executes_real_research_pipeline_not_web_lines() {
    let current_dir = std::env::current_dir().expect("cwd");
    let ctx = SearchRuntimeContext::new(
        current_dir.clone(),
        None,
        current_dir.clone(),
        current_dir.join("memory.md"),
    );
    let config = vox_research_shim::research::ResearchConfig::default();

    let item = GoldenQueryItem {
        query: "What is the memory safety model of Rust?".into(),
        gold_answer: Some("Rust enforces memory safety through ownership, borrowing, and lifetimes at compile time.".into()),
        domain_mode: None,
        expected_sources: None,
        is_adversarial: None,
    };

    let sample = evaluate_single_query_pipeline("run-001", &item, &ctx, None, &config)
        .await
        .expect("must execute real pipeline");

    assert!(!sample.model_answer.is_empty(), "model answer must not be empty");
    assert!(!sample.model_answer.starts_with("engine:"), "model answer must not be raw engine lines");
    assert!(sample.evidence.get("total_claims").is_some(), "must record verified claims");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-cli-research --test eval_e2e_test`
Expected: FAIL with "unresolved function `evaluate_single_query_pipeline`".

- [ ] **Step 3: Implement `evaluate_single_query_pipeline` in `crates/vox-cli-research/src/eval.rs`**

Replace lines 47–67 in `crates/vox-cli-research/src/eval.rs` with `evaluate_single_query_pipeline`:
- Execute `vox_research_shim::research::run_research_with_context`.
- Calculate true citation precision via `result.research_metadata.citation_audit`.
- Compute true recall against `item.gold_answer`.
- Parse `gold_answer`, `domain_mode`, and `is_adversarial` in `GoldenQueryItem`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-cli-research --test eval_e2e_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-cli-research/src/eval.rs \
        crates/vox-cli-research/tests/eval_e2e_test.rs
git commit -m "fix(eval): benchmark real multi-stage research pipeline and eliminate evaluation illusion"
```

---

### Task 7: Shopping & Code Generation Domain Engines `[PARALLEL-SAFE]`

**Files:**
- Create: `crates/vox-research-shim/src/research/domain/mod.rs`
- Create: `crates/vox-research-shim/src/research/domain/shopping.rs`
- Create: `crates/vox-research-shim/src/research/domain/codegen.rs`
- Modify: `crates/vox-research-shim/src/research/types.rs:20-35`
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:135-155, 445-470`
- Test: `crates/vox-research-shim/tests/domain_engines_test.rs`

**Pre-flight Verification:**
Run: `rg "pub struct ResearchQuery" crates/vox-research-shim/src/research/types.rs`

**Interfaces:**
- Consumes: `ResearchDomainMode` (`General`, `Shopping`, `CodeGen`) on `ResearchQuery`.
- Produces: Shopping comparison matrix synthesis, Reddit/RTINGS de-biasing, CodeGen docs.rs signature extraction, and sandboxed `cargo check` verification.

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-research-shim/tests/domain_engines_test.rs`:
```rust
use vox_research_shim::research::domain::shopping::generate_shopping_subqueries;
use vox_research_shim::research::domain::codegen::{generate_codegen_subqueries, verify_rust_code_in_sandbox};

#[test]
fn test_shopping_subqueries() {
    let qs = generate_shopping_subqueries("Sony WH-1000XM5");
    assert!(qs.iter().any(|q| q.contains("specifications") || q.contains("battery")));
    assert!(qs.iter().any(|q| q.contains("price") || q.contains("discounts")));
    assert!(qs.iter().any(|q| q.contains("complaints") || q.contains("reddit")));
}

#[tokio::test]
async fn test_codegen_sandbox_verification() {
    let valid_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
    let res = verify_rust_code_in_sandbox(valid_code, &[]).await.expect("check");
    assert!(res.passed, "valid code must pass compiler check");

    let invalid_code = "pub fn bad() { let x: i32 = \"string\"; }";
    let res_bad = verify_rust_code_in_sandbox(invalid_code, &[]).await.expect("check");
    assert!(!res_bad.passed, "invalid type must fail compiler check");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-research-shim --test domain_engines_test`
Expected: FAIL with "unresolved module `domain`".

- [ ] **Step 3: Implement domain types and engines**

1. Add `ResearchDomainMode` to `crates/vox-research-shim/src/research/types.rs`.
2. Implement `crates/vox-research-shim/src/research/domain/shopping.rs` with `generate_shopping_subqueries`, anti-affiliate filtering, and product comparison matrix instructions.
3. Implement `crates/vox-research-shim/src/research/domain/codegen.rs` with `generate_codegen_subqueries`, docs.rs signature fetching instructions, and `verify_rust_code_in_sandbox`.
4. In `pipeline.rs`: seed subqueries and append domain synthesis instructions when `query.domain_mode` is set.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-research-shim --test domain_engines_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-research-shim/src/research/domain/ \
        crates/vox-research-shim/src/research/types.rs \
        crates/vox-research-shim/src/research/orchestrator/pipeline.rs \
        crates/vox-research-shim/tests/domain_engines_test.rs
git commit -m "feat(research): implement first-party shopping and code generation domain engines"
```

---

## Wave 4: User-Facing Surfaces (GUI & Chat)

### Task 1: GUI Trust UI Remediation, Contradiction Support & Interactive Markdown `[PARALLEL-SAFE]`

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchReportMarkdown.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchReportMarkdown.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/HeadlineVerdictBanner.tsx:1-55`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchClaimAccordion.tsx:25-70`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx:120-230`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts:1-40`
- Modify: `crates/vox-gui/ui/src/lib/pipeline.ts:15-50`
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:425-440`
- Modify: `crates/vox-gui/src/commands/scientia.rs:180-190`
- Test: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`
- Test: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchReportMarkdown.test.tsx`

**Pre-flight Verification:**
Run: `rg "HeadlineVerdictBanner" crates/vox-gui/ui/src/`
Run: `rg "citations.dedup" crates/vox-gui/src/commands/scientia.rs`

**Interfaces:**
- Consumes: `startResearchAsync({ query, verifyClaims: true, scope, maxSources })`.
- Produces: Fixed `HeadlineVerdictBanner` (displaying Contradicted, distinct corroborating sources, no false "High confidence"), `ResearchReportMarkdown` with clickable citation buttons that highlight accordion rows, progressive `deriveStages` handling.

- [ ] **Step 1: Write the failing tests**

In `crates/vox-gui/ui/src/components/surfaces/Research/ResearchReportMarkdown.test.tsx`:
```tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { ResearchReportMarkdown } from './ResearchReportMarkdown';

describe('ResearchReportMarkdown', () => {
  it('renders markdown structure and interactive citation badges', () => {
    const onCitationClick = vi.fn();
    const md = "# Findings\n\n- Fact one [1]\n- Fact two [2]";
    render(<ResearchReportMarkdown markdown={md} onCitationClick={onCitationClick} />);

    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Findings');
    const badge = screen.getByRole('button', { name: 'Citation 1' });
    fireEvent.click(badge);
    expect(onCitationClick).toHaveBeenCalledWith(1);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd crates/vox-gui/ui && pnpm test src/components/surfaces/Research/ResearchReportMarkdown.test.tsx`
Expected: FAIL with "Cannot find module './ResearchReportMarkdown'".

- [ ] **Step 3: Implement minimal fixes**

1. **Create `ResearchReportMarkdown.tsx`**: zero-dependency parser rendering headings, lists, code blocks, and clickable citation buttons `[N]`.
2. **Fix `HeadlineVerdictBanner.tsx`**:
   - Add `contradictedClaims` and `supportedClaims` props.
   - If `contradictedClaims > 0`: render red banner (`Contradicted — N claims refuted by evidence`).
   - If `corroboratingSources === 0`: render `Preliminary` or `Unverified`, NEVER `High confidence`.
3. **Fix `ResearchClaimAccordion.tsx`**:
   - Add `contradictedCount` to header summary.
   - Accept `highlightedClaimId` and assign `id={`claim-${c.claimId}`}` for smooth scrolling.
4. **Fix `ResearchView.tsx`**:
   - Pass `verifyClaims: true` in `startResearchAsync`.
   - Calculate distinct corroborating source URLs via `Set<string>` rather than summing per-claim citations.
   - Replace `<pre>` with `ResearchReportMarkdown`. Clicking `[N]` opens the accordion, scrolls to the claim, and highlights it.
   - Read actual session status for `deriveStages`. Handle `'orphaned'`.
5. **Fix Backend Evidence Span Truncation (`pipeline.rs:429`)**:
   Do not truncate `citations` to 10 if higher index hits are cited by evidence spans.
6. **Fix Unsorted Dedup (`scientia.rs:184`)**:
   Call `citation_urls.sort()` before `citation_urls.dedup()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd crates/vox-gui/ui && pnpm test src/components/surfaces/Research/`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/ \
        crates/vox-gui/ui/src/lib/pipeline.ts \
        crates/vox-gui/src/commands/scientia.rs \
        crates/vox-research-shim/src/research/orchestrator/pipeline.rs
git commit -m "fix(gui): restore trust UI, handle contradicted claims, and add interactive markdown report"
```

---

### Task 2: Chat Research Wiring, Slash Routing & Non-Blocking Execution `[PARALLEL-SAFE]`

**Files:**
- Modify: `crates/vox-gui/src/commands/chat_turn.rs:36-80, 166-220`
- Modify: `crates/vox-gui/ui/src/transport.ts:995-1025`
- Modify: `crates/vox-gui/ui/src/lib/buildChatTurn.ts:10-75`
- Modify: `crates/vox-gui/ui/src/lib/slashCommands.ts:1-40`
- Modify: `crates/vox-gui/ui/src/lib/slashRouter.ts:1-25`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx:20-85`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:650-685`
- Test: `crates/vox-gui/src/commands/chat_turn.rs`

**Pre-flight Verification:**
Run: `rg "force_research" crates/vox-gui/src/commands/chat_turn.rs`
Run: `rg "APP_SLASH_COMMANDS" crates/vox-gui/ui/src/lib/slashRouter.ts`

**Interfaces:**
- Consumes: `force_research` and `research_scope` from `ChatTurnInput`.
- Produces: Forwarded research controls to `vox_chat_message`, `/research` slash command routing to non-blocking background execution, research summary card in `ChatTranscript.tsx`.

- [ ] **Step 1: Write the failing test**

In `crates/vox-gui/src/commands/chat_turn.rs` (under `mod tests`):
```rust
#[test]
fn test_sync_tool_args_carries_research_parameters() {
    let input: ChatTurnInput = serde_json::from_value(serde_json::json!({
        "session_id": "s-chat",
        "content": "Analyze memory safety in C vs Rust",
        "force_research": true,
        "research_scope": "web"
    }))
    .expect("deserialize");

    let args = sync_tool_args(&input);
    assert_eq!(args["force_research"], true);
    assert_eq!(args["research_scope"], "web");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-gui --lib test_sync_tool_args_carries_research_parameters`
Expected: FAIL with "no field `force_research`".

- [ ] **Step 3: Implement minimal fixes**

1. In `crates/vox-gui/src/commands/chat_turn.rs`:
   - Add `force_research: Option<bool>` and `research_scope: Option<String>` to `ChatTurnInput`.
   - Forward them in `sync_tool_args`.
2. In `crates/vox-gui/ui/src/transport.ts`:
   - Add `force_research?: boolean | null;` and `research_scope?: 'local' | 'web' | 'both' | null;` to `ChatTurnInput`.
3. In `crates/vox-gui/ui/src/lib/slashRouter.ts` & `slashCommands.ts`:
   - Register `/research` and `/deepresearch`.
4. In `crates/vox-gui/ui/src/lib/buildChatTurn.ts`:
   - Parse `/research <query>` and strip the command prefix.
   - When `/research` is detected, set `execution: 'background'` (non-blocking) and `force_research: true`.
5. In `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:667`:
   - Pass valid `task_id` into `perform_autonomous_research` so progress events correlate to the session.
6. In `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx`:
   - Render `HeadlineVerdictBanner` and `ResearchClaimAccordion` inside `MessageBubble` when research summary metadata is attached.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-gui --lib test_sync_tool_args_carries_research_parameters`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/src/commands/chat_turn.rs \
        crates/vox-gui/ui/src/transport.ts \
        crates/vox-gui/ui/src/lib/ \
        crates/vox-gui/ui/src/components/surfaces/Chat/ \
        crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
git commit -m "feat(chat): plumb research arguments, route /research to background, and mount summary cards"
```

---

## Verification & Execution Gates

### Automated Regression Matrix
```bash
# 1. Search, Secrets & Policy Gates
cargo test -p vox-search

# 2. Storage & FTS5 Gates
cargo test -p vox-db --test research_search_test

# 3. Model Orchestrator & Asymmetric Cascades
cargo test -p vox-orchestrator --lib test_local_first_research_intents
cargo test -p vox-research-shim

# 4. Evaluation Harness & Domain Engines
cargo test -p vox-cli-research --test eval_e2e_test
cargo test -p vox-research-shim --test domain_engines_test

# 5. GUI & Chat Wire Parity
cargo test -p vox-gui --lib test_sync_tool_args_carries_research_parameters
cd crates/vox-gui/ui && pnpm test

# 6. Global Workspace Architecture Check
cargo run -p vox-arch-check
vox run scripts/fmt.vox
```
