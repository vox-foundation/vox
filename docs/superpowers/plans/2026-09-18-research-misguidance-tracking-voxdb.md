# Research Misguidance & Code Defect Tracking via VoxDb: Implementation & Handoff Plan

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc`.
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all`.
> - **Strict File Disjointness:** Tasks executed in parallel in the same wave must have strictly non-overlapping file sets ($F_A \cap F_B = \emptyset$).
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately.

**Goal:** Provide an end-to-end telemetry and feedback loop in `vox-db` to track when external research misleads AI agents or human developers into writing inelegant code (bloat/over-engineered), code that fails to compile or run, or code requiring user correction. Penalize and suppress bad domains in `vox-search` rank fusion.

**Architecture:** 
1. `vox-db` stores `research_misguidance_events` and aggregates `research_domain_reputation` under the Scientia domain schema.
2. `vox-search` ingests domain penalties from VoxDb during `rank_and_dedupe_results`, applying authority multipliers and suppressing blacklisted domains.
3. `vox-research-shim` provides lightweight failure-attribution helpers matching compiler/test error tokens to retrieved citation snippets.
4. `vox-gui` and `vox-cli` expose surfaces for developers to flag misleading research citations and view domain reputation health.

**Tech Stack:** Rust 2024 (Tokio, Turso/libSQL, Serde JSON), TypeScript 5.5+ (React 19, Tailwind CSS, Vitest).

---

## Global Constraints & Architectural Invariants

1. **Scientia Domain Cohesion:** `research_misguidance_events` and `research_domain_reputation` belong to `crates/vox-db/src/schema/domains/scientia.rs` alongside `scientia_research_sessions` and `scientia_research_artifacts`.
2. **Schema Migration Integrity:** Bumps `BASELINE_VERSION` in `crates/vox-db/src/schema/manifest.rs` to ensure fresh and existing databases apply the DDL.
3. **Decoupled Architecture:** Forbid direct dependencies from `vox-search` to `vox-db`. Search policy carries domain penalties and blacklist sets provided by the caller.
4. **Offline Determinism in CI:** All tests must run against in-memory databases (`DbConfig::Memory`) with zero network dependencies.
5. **No Secret Leaks:** Failure diagnostics and excerpts must sanitize any sensitive environment variables or authorization headers before persistence.

---

## File Manifest & Concurrency Map

| Wave | Task ID | Execution Mode | Target Files | Primary Responsibility |
| :--- | :--- | :--- | :--- | :--- |
| **Wave 1** | **Task 1** | `[SEQUENTIAL]` | `crates/vox-db-types/src/research.rs`<br/>`crates/vox-db/src/schema/domains/scientia.rs`<br/>`crates/vox-db/src/schema/manifest.rs`<br/>`crates/vox-db/tests/research_misguidance_schema_test.rs` | Define DTOs, DDL tables, indexes, and baseline migration |
| **Wave 2** | **Task 2** | `[SEQUENTIAL]` | `crates/vox-db/src/research_pipeline.rs`<br/>`crates/vox-db/tests/research_misguidance_store_test.rs` | Implement `record_research_misguidance`, exponential decaying reputation aggregation, and penalty queries |
| **Wave 3** | **Task 3** | `[SEQUENTIAL]` | `crates/vox-search/src/policy.rs`<br/>`crates/vox-search/src/web_dispatcher.rs`<br/>`crates/vox-search/tests/domain_penalty_test.rs` | Ingest domain penalties into `SearchPolicy` and apply authority degradation/blacklisting in True RRF |
| **Wave 4** | **Task 4** | `[SEQUENTIAL]` | `crates/vox-research-shim/src/research/misguidance.rs`<br/>`crates/vox-research-shim/src/research/mod.rs`<br/>`crates/vox-research-shim/tests/misguidance_attribution_test.rs` | Implement lightweight token-matching attribution correlating diagnostics to citations |
| **Wave 5** | **Task 5** | `[SEQUENTIAL]` | `crates/vox-gui/src/commands/research.rs`<br/>`crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`<br/>`crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx` | Tauri IPC command `flag_research_misleading` and GUI "Flag Misleading" citation modal |

---

## Tasks

### Task 1: VoxDb Schema & Types: `research_misguidance_events` `[SEQUENTIAL]`

<!-- AMENDED: #1 — Use db.connection().query() to avoid privacy compile error on db.conn -->
<!-- AMENDED: #2 — Stubs created in vox-db-types before writing test to preserve TDD red-green integrity -->
**Files:**
- Modify: `crates/vox-db-types/src/research.rs`
- Modify: `crates/vox-db/src/schema/domains/scientia.rs`
- Modify: `crates/vox-db/src/schema/manifest.rs`
- Test: `crates/vox-db/tests/research_misguidance_schema_test.rs`

**Interfaces:**
- Produces:
  - `ResearchDefectClass` enum (`InelegantCode`, `FailsToRun`, `UserCorrection`, `HallucinatedApi`)
  - `MisguidanceReporter` enum (`Compiler`, `TestRunner`, `Linter`, `User`, `PonytailAudit`)
  - `ResearchMisguidanceRecord` struct
  - `DomainReputationRecord` struct
  - `RecordMisguidanceParams` struct
  - Tables: `research_misguidance_events`, `research_domain_reputation`

**Pre-flight Verification:**
Run: `rg "pub const BASELINE_VERSION" crates/vox-db/src/schema/manifest.rs` to verify the baseline schema version.

- [ ] **Step 1: Declare DTO stubs in `vox-db-types` and write failing schema test**
In `crates/vox-db-types/src/research.rs`, declare `ResearchDefectClass`, `MisguidanceReporter`, `ResearchMisguidanceRecord`, `DomainReputationRecord`, and `RecordMisguidanceParams`.
Then write the integration test:

```rust
// crates/vox-db/tests/research_misguidance_schema_test.rs
use vox_db::{DbConfig, VoxDb};

#[tokio::test]
async fn test_research_misguidance_tables_exist() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("connect memory db");
    // AMENDED #1: Use public db.connection() accessor instead of private db.conn
    let mut rows = db.connection().query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('research_misguidance_events', 'research_domain_reputation')",
        turso::params![],
    ).await.expect("query tables");

    let mut found = Vec::new();
    while let Some(row) = rows.next().await.expect("row") {
        found.push(row.get::<String>(0).expect("name"));
    }
    assert!(found.contains(&"research_misguidance_events".to_string()));
    assert!(found.contains(&"research_domain_reputation".to_string()));
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-db --test research_misguidance_schema_test`  
Expected: FAIL (assertion fails because tables do not yet exist in SQLite schema).

- [ ] **Step 3: Implement DDL in Scientia domain and bump migration baseline**
In `crates/vox-db/src/schema/domains/scientia.rs`, add:
```sql
CREATE TABLE IF NOT EXISTS research_misguidance_events (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id             INTEGER REFERENCES scientia_research_sessions(id) ON DELETE SET NULL,
    defect_class           TEXT    NOT NULL,
    culprit_url            TEXT,
    culprit_domain         TEXT    NOT NULL,
    claim_id               INTEGER,
    research_query         TEXT    NOT NULL,
    misleading_excerpt     TEXT,
    generated_code_snippet TEXT,
    failure_diagnostic     TEXT,
    correction_diff        TEXT,
    reporter               TEXT    NOT NULL,
    domain_penalty         REAL    NOT NULL DEFAULT 0.1,
    created_at_ms          INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_misguidance_domain ON research_misguidance_events(culprit_domain);
CREATE INDEX IF NOT EXISTS idx_misguidance_session ON research_misguidance_events(session_id);

CREATE TABLE IF NOT EXISTS research_domain_reputation (
    domain                 TEXT    PRIMARY KEY,
    incident_count         INTEGER NOT NULL DEFAULT 0,
    penalty_score          REAL    NOT NULL DEFAULT 0.0,
    last_incident_at_ms    INTEGER NOT NULL,
    is_blacklisted         INTEGER NOT NULL DEFAULT 0,
    updated_at_ms          INTEGER NOT NULL
);
```
In `crates/vox-db/src/schema/manifest.rs`, bump `BASELINE_VERSION` by 1.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-db --test research_misguidance_schema_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-db-types/src/research.rs crates/vox-db/src/schema/domains/scientia.rs crates/vox-db/src/schema/manifest.rs crates/vox-db/tests/research_misguidance_schema_test.rs
git commit -m "feat(vox-db): add research misguidance and domain reputation tables"
```

---

### Task 2: VoxDb Store Operations: Decaying Reputation & Penalties `[SEQUENTIAL]`

<!-- AMENDED: #3 — Explicit turso::Value mapping for Option<String> parameters -->
<!-- AMENDED: #4 — Exponential decay calculation with 30-day half-life on penalty rollup -->
**Files:**
- Modify: `crates/vox-db/src/research_pipeline.rs`
- Test: `crates/vox-db/tests/research_misguidance_store_test.rs`

**Interfaces:**
- Produces:
  - `VoxDb::record_research_misguidance(&self, params: &RecordMisguidanceParams) -> Result<i64, StoreError>`
  - `VoxDb::list_research_misguidance(&self, session_id: Option<i64>, limit: u32) -> Result<Vec<ResearchMisguidanceRecord>, StoreError>`
  - `VoxDb::get_domain_penalties(&self) -> Result<HashMap<String, f64>, StoreError>`
  - `VoxDb::get_blacklisted_domains(&self) -> Result<HashSet<String>, StoreError>`

**Pre-flight Verification:**
Run: `rg "pub async fn record_research_metric" crates/vox-db/src/research_pipeline.rs` to review store method patterns.

- [ ] **Step 1: Declare method stubs on VoxDb and write failing store test**
In `crates/vox-db/src/research_pipeline.rs`, declare `record_research_misguidance` returning `Err(StoreError::Internal("unimplemented".into()))`.

```rust
// crates/vox-db/tests/research_misguidance_store_test.rs
use vox_db::{DbConfig, VoxDb};
use vox_db_types::{RecordMisguidanceParams, ResearchDefectClass, MisguidanceReporter};

#[tokio::test]
async fn test_record_misguidance_updates_reputation_with_decay() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let params = RecordMisguidanceParams {
        session_id: None,
        defect_class: ResearchDefectClass::FailsToRun,
        culprit_url: Some("https://bad-docs.example.com/api".into()),
        culprit_domain: "bad-docs.example.com".into(),
        claim_id: None,
        research_query: "How to use example API".into(),
        misleading_excerpt: Some("Call api.v1.old_method()".into()),
        generated_code_snippet: Some("example::old_method();".into()),
        failure_diagnostic: Some("error[E0425]: cannot find function `old_method`".into()),
        correction_diff: None,
        reporter: MisguidanceReporter::Compiler,
        domain_penalty: 0.2,
    };

    let id = db.record_research_misguidance(&params).await.expect("record");
    assert!(id > 0);

    let penalties = db.get_domain_penalties().await.expect("penalties");
    assert_eq!(penalties.get("bad-docs.example.com").copied(), Some(0.2));
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-db --test research_misguidance_store_test`  
Expected: FAIL (unimplemented error from stub).

- [ ] **Step 3: Implement store operations with decay in `research_pipeline.rs`**
Implement:
1. `record_research_misguidance`:
   - Bind `Option<String>` safely:
     ```rust
     let culprit_url = params.culprit_url.as_deref().map(turso::Value::Text).unwrap_or(turso::Value::Null);
     let excerpt = params.misleading_excerpt.as_deref().map(turso::Value::Text).unwrap_or(turso::Value::Null);
     let code = params.generated_code_snippet.as_deref().map(turso::Value::Text).unwrap_or(turso::Value::Null);
     let diag = params.failure_diagnostic.as_deref().map(turso::Value::Text).unwrap_or(turso::Value::Null);
     let diff = params.correction_diff.as_deref().map(turso::Value::Text).unwrap_or(turso::Value::Null);
     ```
   - Insert into `research_misguidance_events`.
   - Read current `penalty_score` and `last_incident_at_ms` from `research_domain_reputation`.
   - Calculate decay:
     `let elapsed_days = ((now_ms - last_incident_at_ms) as f64 / 86_400_000.0).max(0.0);`
     `let decayed = current_penalty * (-elapsed_days / 30.0).exp();`
     `let new_penalty = (decayed + params.domain_penalty).min(2.0);`
   - Upsert into `research_domain_reputation`:
     `incident_count = incident_count + 1, penalty_score = new_penalty, is_blacklisted = CASE WHEN incident_count >= 5 AND new_penalty >= 1.0 THEN 1 ELSE 0 END`.
2. `get_domain_penalties`: Returns `HashMap<String, f64>` of domains with `penalty_score > 0.0`.
3. `get_blacklisted_domains`: Returns `HashSet<String>` where `is_blacklisted = 1`.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-db --test research_misguidance_store_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-db/src/research_pipeline.rs crates/vox-db/tests/research_misguidance_store_test.rs
git commit -m "feat(vox-db): implement misguidance recording with exponential decay rollup"
```

---

### Task 3: Search Dispatcher Authority Degradation & Blacklisting `[SEQUENTIAL]`

<!-- AMENDED: #5 — Fixed SearxngResult field from 'snippet' to 'content' -->
<!-- AMENDED: #6 — Update SearchPolicy::default() manual implementation -->
<!-- AMENDED: #7 — Robust domain extraction via url crate; integrate in search_with_registry and search_with_lane -->
**Files:**
- Modify: `crates/vox-search/src/policy.rs`
- Modify: `crates/vox-search/src/web_dispatcher.rs`
- Test: `crates/vox-search/tests/domain_penalty_test.rs`

**Interfaces:**
- Produces:
  - `SearchPolicy.domain_penalties: HashMap<String, f64>`
  - `SearchPolicy.blacklisted_domains: HashSet<String>`
  - `extract_registrable_domain(url: &str) -> Option<String>`
  - Authority scaling degraded by penalty: `base * (1.0 - penalty).clamp(0.05, 1.0)`

**Pre-flight Verification:**
Run: `rg "pub struct SearxngResult" crates/vox-search/src/searxng.rs` to verify fields (`content` vs `snippet`).

- [ ] **Step 1: Add stubs to `policy.rs` and write failing test**
In `crates/vox-search/src/policy.rs`, add fields `domain_penalties: HashMap<String, f64>` and `blacklisted_domains: HashSet<String>` to `SearchPolicy` and update `impl Default for SearchPolicy`.

```rust
// crates/vox-search/tests/domain_penalty_test.rs
use std::collections::{HashMap, HashSet};
use vox_search::policy::SearchPolicy;
use vox_search::searxng::SearxngResult;
use vox_search::web_dispatcher::WebSearchDispatcher;

#[test]
fn test_blacklisted_domain_results_are_pruned() {
    let mut policy = SearchPolicy::default();
    policy.blacklisted_domains.insert("malicious-docs.com".to_string());

    let mut results = vec![
        SearxngResult {
            url: "https://malicious-docs.com/guide".into(),
            title: "Bad guide".into(),
            content: "...".into(), // AMENDED #5: uses content, not snippet
            score: Some(1.0),
            engine: Some("searxng".into()),
        },
        SearxngResult {
            url: "https://docs.rs/tokio".into(),
            title: "Tokio docs".into(),
            content: "...".into(),
            score: Some(0.8),
            engine: Some("searxng".into()),
        },
    ];

    WebSearchDispatcher::filter_and_penalize_results(&mut results, &policy);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://docs.rs/tokio");
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-search --test domain_penalty_test`  
Expected: FAIL (`filter_and_penalize_results` missing).

- [ ] **Step 3: Implement domain penalty logic in `web_dispatcher.rs`**
1. Implement `extract_registrable_domain`:
   ```rust
   pub fn extract_registrable_domain(url_str: &str) -> Option<String> {
       let parsed = url::Url::parse(url_str).ok()?;
       let host = parsed.host_str()?;
       Some(host.trim_start_matches("www.").to_ascii_lowercase())
   }
   ```
2. Implement `filter_and_penalize_results`:
   - Retain only results where `extract_registrable_domain(&r.url)` is not in `policy.blacklisted_domains`.
   - For each remaining result, scale `score`:
     `let penalty = domain.and_then(|d| policy.domain_penalties.get(&d)).copied().unwrap_or(0.0);`
     `r.score = Some(r.score.unwrap_or(0.5) * (1.0 - penalty).clamp(0.05, 1.0));`
3. Hook `filter_and_penalize_results` before `rank_and_dedupe_results` in `search_with_registry` (and `search_with_lane`).

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-search --test domain_penalty_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-search/src/policy.rs crates/vox-search/src/web_dispatcher.rs crates/vox-search/tests/domain_penalty_test.rs
git commit -m "feat(vox-search): add domain penalty scaling and blacklist filtering to search dispatcher"
```

---

### Task 4: Research Shim Attribution: Fuzzy Token Diagnostic Matching `[SEQUENTIAL]`

<!-- AMENDED: #8 — Lightweight token intersection heuristic replacing brittle regex parser -->
**Files:**
- Create: `crates/vox-research-shim/src/research/misguidance.rs`
- Modify: `crates/vox-research-shim/src/research/mod.rs`
- Test: `crates/vox-research-shim/tests/misguidance_attribution_test.rs`

**Interfaces:**
- Produces:
  - `correlate_diagnostic_to_citations(diagnostic: &str, citations: &[Citation]) -> Option<String>`

**Pre-flight Verification:**
Run: `rg "pub struct Citation" crates/vox-research-shim/src/research/types.rs` to check citation fields.

- [ ] **Step 1: Write failing correlation test**

```rust
// crates/vox-research-shim/tests/misguidance_attribution_test.rs
use vox_research_shim::research::types::Citation;
use vox_research_shim::research::misguidance::correlate_diagnostic_to_citations;

#[test]
fn test_correlates_compile_error_to_culprit_citation() {
    let citations = vec![
        Citation {
            source_id: 1,
            url: "https://docs.rs/good/latest".into(),
            title: "Good docs".into(),
            snippet: "fn healthy_api()".into(),
            confidence: 0.9,
        },
        Citation {
            source_id: 2,
            url: "https://obsolete-blog.com/tips".into(),
            title: "Broken tips".into(),
            snippet: "call legacy_unstable_feature() in your code".into(),
            confidence: 0.6,
        },
    ];

    let error_log = "error[E0425]: cannot find function `legacy_unstable_feature` in module";
    let culprit = correlate_diagnostic_to_citations(error_log, &citations);
    assert_eq!(culprit.as_deref(), Some("https://obsolete-blog.com/tips"));
}
```

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p vox-research-shim --test misguidance_attribution_test`  
Expected: FAIL (`misguidance` module not found).

- [ ] **Step 3: Implement token-intersection heuristic in `misguidance.rs`**
In `crates/vox-research-shim/src/research/misguidance.rs`:
Extract word tokens ($\ge 4$ characters, alphanumeric + `_`) from `diagnostic`. Score each citation by count of overlapping tokens between diagnostic and `citation.snippet` / `citation.title`. Return the URL with the highest positive overlap, or `None` if no tokens match. Export in `research/mod.rs`.

- [ ] **Step 4: Verify test passes**
Run: `cargo test -p vox-research-shim --test misguidance_attribution_test`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-research-shim/src/research/misguidance.rs crates/vox-research-shim/src/research/mod.rs crates/vox-research-shim/tests/misguidance_attribution_test.rs
git commit -m "feat(research-shim): implement diagnostic-to-citation misguidance attribution"
```

---

### Task 5: GUI Flagging Interface & Tauri IPC `[SEQUENTIAL]`

<!-- AMENDED: #9 — Mock Tauri IPC invoke in Vitest test setup -->
<!-- AMENDED: #10 — Modal styled with semantic tokens, z-50, focus trap, and User Corrected option -->
**Files:**
- Modify: `crates/vox-gui/src/commands/research.rs`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`
- Test: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx`

**Interfaces:**
- Produces:
  - Tauri command: `flag_research_misleading(params: RecordMisguidanceDto) -> Result<i64, String>`
  - UI button: `[data-testid="flag-citation-misleading"]`
  - Modal with semantic classes: `z-50`, `bg-overlay-subtle`, `border-border-subtle`, `text-text-primary`

**Pre-flight Verification:**
Run: `rg "pub async fn get_research_engine_status" crates/vox-gui/src/commands/research.rs` to review command registrations.

- [ ] **Step 1: Write failing UI test with Tauri IPC mocks**

```typescript
// crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ResearchView } from './ResearchView';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'get_research_session_detail') {
      return {
        session_id: 1,
        citations: [{ source_id: 1, url: 'https://flawed-docs.com', title: 'Flawed', snippet: 'code' }],
      };
    }
    return null;
  }),
}));

describe('ResearchView Misguidance Flagging', () => {
  it('opens misguidance modal when Flag Citation button is clicked', async () => {
    render(<ResearchView />);
    const flagBtn = await screen.findByTestId('flag-citation-misleading');
    fireEvent.click(flagBtn);

    expect(screen.getByText(/Flag Misleading Research/i)).toBeInTheDocument();
    expect(screen.getByTestId('defect-class-selector')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test to verify failure**
Run: `pnpm --dir crates/vox-gui/ui test src/components/surfaces/Research/ResearchView.test.tsx`  
Expected: FAIL (`flag-citation-misleading` missing).

- [ ] **Step 3: Implement Tauri command and UI modal**
1. In `crates/vox-gui/src/commands/research.rs`, implement `flag_research_misleading` calling `db.record_research_misguidance`.
2. In `ResearchView.tsx`, render `[data-testid="flag-citation-misleading"]` on each citation item.
3. On click, display a modal at `z-50` with focus trap, allowing user to select `Inelegant Code`, `Fails to Run`, or `User Corrected`, add an optional note or diff, and submit via Tauri `invoke('flag_research_misleading', ...)`.

- [ ] **Step 4: Verify test passes**
Run: `pnpm --dir crates/vox-gui/ui test src/components/surfaces/Research/ResearchView.test.tsx`  
Expected: PASS.

- [ ] **Step 5: Atomic Commit**
```bash
git add crates/vox-gui/src/commands/research.rs crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx
git commit -m "feat(gui): add citation misguidance flagging modal and Tauri IPC handler"
```

---

## Execution Order

### Sequential Constraints:
- **Task 1 $\rightarrow$ Task 2**: `vox-db` store operations in Task 2 depend on types and DDL created in Task 1.
- **Task 2 $\rightarrow$ Task 3**: `vox-search` tests require `policy.rs` types and will load penalties from `vox-db`.
- **Task 2 $\rightarrow$ Task 5**: Tauri commands in Task 5 call `record_research_misguidance` on `VoxDb`.

### Pre-Flight Checklist:
- [ ] Working branch confirmed (`feat/axis-gui-debugger-deep-research`).
- [ ] `BASELINE_VERSION` checked in `crates/vox-db/src/schema/manifest.rs`.
- [ ] In-memory test database confirmed functional.

### Recommended Task Sequence:
```
Wave 1: Task 1 (Sequential)
Wave 2: Task 2 (Sequential)
Wave 3: Task 3 (Sequential)
Wave 4: Task 4 (Sequential)
Wave 5: Task 5 (Sequential)
```

### SDD Ledger Pre-Population:
```markdown
## Pre-Resolved Review Rulings
- Domain Placement: research_misguidance_events and research_domain_reputation live in scientia.rs — ruling: settled
- Connection Accessor (A-1/B-1): Integration tests must use db.connection().query(), not private db.conn — ruling: settled
- SearxngResult Field (A-2): Uses content, not snippet — ruling: settled
- Penalty Decay (A-4): Decays with 30-day half-life before adding new incident penalty — ruling: settled
- LibSQL Value Mapping (B-3): Map Option<String> to turso::Value explicitly in parameters — ruling: settled
- Domain Extraction (B-4): Use url::Url::parse(url) to extract host safely — ruling: settled
- Disjointness: All 5 tasks are sequential to preserve layer dependencies (types -> store -> search/shim -> GUI) — ruling: settled
```

## Deferred Minor Issues
- None currently. All Track A, B, and C findings addressed inline.
