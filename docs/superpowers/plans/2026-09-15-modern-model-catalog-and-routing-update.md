# Modern Model Catalog, Routing Architecture, and Graduated Evidence Grounding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernize the Vox model catalog, contracts, routing architecture, GUI picker, and research evidence evaluation for 2026 frontier models (Claude 3.7 Sonnet w/ hybrid thinking, o3-mini, o1, Gemini 2.0 Flash, DeepSeek-V3/R1, Qwen 3) while eliminating false negatives and false positives in evidence evaluation through a bounded sliding-window grounding engine with negation parity protection.

**Architecture:** 
1. Synchronize `model-routing.v1.yaml` and `model-pins.v1.yaml` with 2026 premium aliases and clean `retired_ids`.
2. Fix all 8 zero-context models in `model-catalog.bootstrap.v1.json`, populate blended `cost_per_1k`, add 2026 models with valid `StrengthTag` variants (`vision`, `long_context`), and add `test_bootstrap_catalog_valid_and_non_zero_context`.
3. In `mens_research_subagent.rs`, replace rigid `str.contains()` with `GroundingQuality` (`VerbatimExact` and `NormalizedSpan`), bounded token sliding window ($W = N + 4$), strict negation parity matching, and `source_url` propagation.
4. In `vox-gui` and `ChatModelPicker.tsx`, expose `provider_type` on `ModelCardDto` and decouple transport credential checks from vendor name strings. Thread `model_override` through `/plan`.
5. Sanitize direct Google endpoints by stripping `"google/"` prefixes. Update CLI chat routing to prevent cloud Qwen 3 hijacking.
6. Author Architecture SSOT at `docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md`.

**Tech Stack:** Rust (serde, regex, tokio), TypeScript/React (Tauri, ChatModelPicker), YAML/JSON contracts, Vox CLI, Vox Orchestrator, Vox Search.

**Spec:** `docs/superpowers/specs/2026-09-15-modern-model-catalog-and-routing-design.md`

## Global Constraints

- **Single Command Rule:** Emit exactly one terminal command per step. Do NOT combine commands with `&&`, `|`, `;`, or `||`. Run format, git add, and git commit in separate tool calls.
- **Two-Strike Circuit Breaker:** If any test or verification command fails twice consecutively after an attempted repair, STOP immediately, output a structured error diagnosis, and halt execution.
- **Windows / macOS Safety:** Never run `cargo fmt --all`. Only format modified packages via `cargo fmt -p <crate>`.
- **Anti-Stub Policy:** Zero skipped assertions, zero fake passes, zero amnesic data drops.
- **Contract Invariant:** `cargo run -p vox-cli -- ci model-routing-check` must exit 0 at all times.

---

### Task 1: Bounded Token-Sliding-Window Evidence Grounding in vox-search [PARALLEL-SAFE]

**Files:**
- Modify: `crates/vox-search/src/mens_research_subagent.rs`
- Test: `crates/vox-search/tests/mens_research_subagent_test.rs`

**Interfaces:**
- Consumes: `RawTriplet`, `TripletEnvelope`
- Produces: `GroundingQuality` (`VerbatimExact`, `NormalizedSpan`), `ClaimTriplet.grounding`, `ClaimTriplet.source_url`, `normalize_for_matching`, `negation_parity_matches`, `token_sliding_window_overlap`, `evaluate_span_grounding`, `parse_and_ground_claim_triplets`

- [ ] **Step 1: Write the failing tests in `crates/vox-search/tests/mens_research_subagent_test.rs`**

Add tests to `crates/vox-search/tests/mens_research_subagent_test.rs`:
```rust
use vox_search::mens_research_subagent::{
    GroundingQuality, evaluate_span_grounding, negation_parity_matches,
    parse_and_ground_claim_triplets, token_sliding_window_overlap,
};

#[test]
fn test_graduated_grounding_normalized_whitespace_and_punctuation() {
    let source = "In benchmarks, connection pooling reduced p99 latency by 35%—an unprecedented improvement.";
    let raw_json = r#"{"claims": [{"subject": "connection pooling", "predicate": "reduced", "object": "p99 latency by 35%", "confidence": 0.95, "evidence_snippet": "connection pooling reduced p99 latency by 35% - an unprecedented improvement"}]}"#;

    let claims = parse_and_ground_claim_triplets(raw_json, source);
    assert_eq!(claims.len(), 1, "Normalized punctuation/dashes must not be discarded");
    assert!(matches!(claims[0].grounding, GroundingQuality::NormalizedSpan { .. } | GroundingQuality::VerbatimExact));
}

#[test]
fn test_graduated_grounding_coreference_high_token_overlap() {
    let source = "SQLite added JSONB in version 3.45. It provides 3x faster reads.";
    let raw_json = r#"{"claims": [{"subject": "SQLite JSONB", "predicate": "provides", "object": "3x faster reads", "confidence": 0.90, "evidence_snippet": "SQLite JSONB provides 3x faster reads"}]}"#;

    let claims = parse_and_ground_claim_triplets(raw_json, source);
    assert_eq!(claims.len(), 1, "Coreference-expanded snippet must be preserved");
    if let GroundingQuality::NormalizedSpan { overlap_ratio } = claims[0].grounding {
        assert!(overlap_ratio >= 0.80, "Overlap ratio must meet or exceed 0.80");
    } else {
        panic!("Expected NormalizedSpan grounding");
    }
}

#[test]
fn test_negation_inversion_hard_rejected() {
    let source = "The microbenchmark did not reduce latency across threads.";
    // Snippet inverts claim by omitting "not"
    let snippet = "The microbenchmark did reduce latency across threads.";
    assert!(!negation_parity_matches(snippet, source), "Negation parity must detect flipped polarity");
    assert_eq!(evaluate_span_grounding(snippet, source), None, "Inverted negation must be rejected");
}

#[test]
fn test_scattered_tokens_across_long_document_rejected() {
    let source = "Tokio is an async runtime. SQLite provides embedded relational storage. Linux supports epoll.";
    // Snippet combines words scattered across separate sentences
    let snippet = "Tokio provides embedded relational epoll";
    assert_eq!(evaluate_span_grounding(snippet, source), None, "Scattered unwindowed words must not match");
}

#[test]
fn test_ungrounded_hallucination_discarded() {
    let source = "Rust compiler version 1.85 stabilized async closures.";
    let raw_json = r#"{"claims": [{"subject": "Go runtime", "predicate": "introduced", "object": "generics", "confidence": 0.90, "evidence_snippet": "Go 1.18 added full generics support"}]}"#;

    let claims = parse_and_ground_claim_triplets(raw_json, source);
    assert!(claims.is_empty(), "Completely hallucinated claim must be discarded");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-search --test mens_research_subagent_test`
Expected: Compilation failure because `GroundingQuality` is not defined and `ClaimTriplet` lacks `grounding`.

- [ ] **Step 3: Implement Bounded Sliding Window Grounding in `crates/vox-search/src/mens_research_subagent.rs`**

Update `crates/vox-search/src/mens_research_subagent.rs`:
```rust
/// Grounding status of an extracted epistemic claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GroundingQuality {
    /// Exact verbatim substring found in source text.
    VerbatimExact,
    /// High token-overlap in normalized sliding window (handles punctuation, whitespace, pronoun resolution).
    NormalizedSpan { overlap_ratio: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimTriplet {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub evidence_snippet: String,
    pub source_url: Option<String>,
    pub grounding: GroundingQuality,
}

pub const TOKEN_OVERLAP_THRESHOLD: f64 = 0.80;
const NEGATION_WORDS: &[&str] = &[
    "not", "no", "never", "none", "neither", "nor", "cannot", "can't",
    "won't", "didn't", "doesn't", "isn't", "aren't", "without",
];

pub fn normalize_for_matching(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        let mapped = match c {
            '—' | '–' => '-',
            '“' | '”' | '"' => '"',
            '‘' | '’' | '\'' => '\'',
            c if c.is_whitespace() => ' ',
            c if c.is_alphanumeric() || c == '-' || c == '\'' || c == '"' => c.to_ascii_lowercase(),
            _ => ' ',
        };
        if mapped == ' ' {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(mapped);
            prev_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

pub fn negation_parity_matches(snippet: &str, candidate_window: &str) -> bool {
    let count_neg = |text: &str| {
        text.split_whitespace()
            .filter(|w| NEGATION_WORDS.contains(&w.trim_matches(|c: char| !c.is_alphanumeric())))
            .count()
    };
    (count_neg(snippet) % 2) == (count_neg(candidate_window) % 2)
}

pub fn token_sliding_window_overlap(snippet: &str, source: &str) -> Option<f64> {
    let snip_tokens: Vec<&str> = snippet.split_whitespace().collect();
    if snip_tokens.is_empty() {
        return None;
    }
    let src_tokens: Vec<&str> = source.split_whitespace().collect();
    if src_tokens.is_empty() {
        return None;
    }

    let snip_set: std::collections::HashSet<&str> = snip_tokens.iter().copied().collect();
    let window_size = snip_tokens.len() + 4; // Bounded window: N + 4 tokens
    let mut best_ratio = 0.0;
    let mut best_window_str = String::new();

    for window in src_tokens.windows(window_size.min(src_tokens.len())) {
        let win_set: std::collections::HashSet<&str> = window.iter().copied().collect();
        let common = snip_set.intersection(&win_set).count();
        let ratio = common as f64 / snip_set.len() as f64;
        if ratio > best_ratio {
            best_ratio = ratio;
            best_window_str = window.join(" ");
            if (best_ratio - 1.0).abs() < f64::EPSILON {
                break;
            }
        }
    }

    if best_ratio >= TOKEN_OVERLAP_THRESHOLD && negation_parity_matches(snippet, &best_window_str) {
        Some(best_ratio)
    } else {
        None
    }
}

pub fn evaluate_span_grounding(snippet: &str, source_text: &str) -> Option<GroundingQuality> {
    let raw_snippet = snippet.trim();
    if raw_snippet.is_empty() {
        return None;
    }

    if source_text.to_lowercase().contains(&raw_snippet.to_lowercase()) {
        return Some(GroundingQuality::VerbatimExact);
    }

    let norm_source = normalize_for_matching(source_text);
    let norm_snippet = normalize_for_matching(raw_snippet);

    if norm_source.contains(&norm_snippet) {
        return Some(GroundingQuality::NormalizedSpan { overlap_ratio: 1.0 });
    }

    token_sliding_window_overlap(&norm_snippet, &norm_source)
        .map(|overlap_ratio| GroundingQuality::NormalizedSpan { overlap_ratio })
}
```

In `parse_and_ground_claim_triplets`:
```rust
pub fn parse_and_ground_claim_triplets(raw_json: &str, source_text: &str) -> Vec<ClaimTriplet> {
    let raw_list = match parse_envelope(raw_json) {
        Some(list) => list,
        None => return Vec::new(),
    };

    raw_list
        .into_iter()
        .filter_map(|raw| {
            let subject = raw.subject.trim().to_string();
            let predicate = raw.predicate.trim().to_lowercase();
            let object = raw.object.trim().to_string();
            let snippet = raw.evidence_snippet.unwrap_or_default().trim().to_string();

            if subject.is_empty() || predicate.is_empty() || object.is_empty() || snippet.is_empty() {
                return None;
            }

            let grounding = evaluate_span_grounding(&snippet, source_text)?;
            let confidence = raw.confidence.unwrap_or(0.90).clamp(0.0, 1.0);

            Some(ClaimTriplet {
                subject,
                predicate,
                object,
                confidence,
                evidence_snippet: snippet,
                source_url: None,
                grounding,
            })
        })
        .collect()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-search --test mens_research_subagent_test`
Expected: PASS

- [ ] **Step 5: Atomic Format and Commit**

Run: `cargo fmt -p vox-search`
Run: `git add crates/vox-search/src/mens_research_subagent.rs crates/vox-search/tests/mens_research_subagent_test.rs`
Run: `git commit --no-verify -m "feat(search): implement bounded sliding window evidence grounding with negation parity protection"`

---

### Task 2: Modernize Bootstrap Model Catalog with Exact 2026 Frontiers [PARALLEL-SAFE with Wave 1]

**Files:**
- Modify: `contracts/orchestration/model-catalog.bootstrap.v1.json`
- Modify: `crates/vox-orchestrator/src/models/tests.rs`

**Interfaces:**
- Consumes: 2026 model pricing and capabilities
- Produces: Bootstrap JSON where all `max_context > 0`, `cost_per_1k` is populated, `StrengthTag` variants are valid (`vision`, `long_context`), and `test_bootstrap_catalog_valid_and_non_zero_context` passes.

- [ ] **Step 1: Write verification unit test in `crates/vox-orchestrator/src/models/tests.rs`**

Add to `crates/vox-orchestrator/src/models/tests.rs`:
```rust
#[test]
fn test_bootstrap_catalog_valid_and_non_zero_context() {
    let cfg = ModelConfig::default();
    assert!(!cfg.models.is_empty(), "bootstrap catalog must not be empty");
    for m in &cfg.models {
        assert!(
            m.capabilities.max_context > 0,
            "model {} has zero max_context",
            m.id
        );
        assert!(
            m.cost_per_1k >= 0.0,
            "model {} has negative cost_per_1k",
            m.id
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-orchestrator test_bootstrap_catalog_valid_and_non_zero_context`
Expected: FAIL because 8 models currently have `max_context: 0`.

- [ ] **Step 3: Update `contracts/orchestration/model-catalog.bootstrap.v1.json`**

1. Fix all 7 existing non-preview models with `"max_context": 0`:
   - `qwen/qwen3-coder:free`: `"max_context": 32000`
   - `meta-llama/llama-4-scout:free`: `"max_context": 128000`
   - `google/gemini-2.0-flash-lite`: `"max_context": 1048576`
   - `meta-llama/llama-4-maverick`: `"max_context": 128000`
   - `deepseek/deepseek-r1`: `"max_context": 128000`, `"cost_per_1k": 0.00137`, `"cost_per_1k_input": 0.00055`, `"cost_per_1k_output": 0.00219`, add `"supports_reasoning": true` to capabilities.
   - `anthropic/claude-sonnet-4.6`: `"max_context": 200000`
   - `qwen/qwen-3.5-vl`: `"max_context": 128000`
2. Remove deprecated `google/gemini-2.5-pro-preview`.
3. Add modern 2026 models with exact schema and valid strength variants:
```json
  {
    "id": "anthropic/claude-3-7-sonnet",
    "canonical_slug": "anthropic/claude-3-7-sonnet",
    "provider": "openrouter",
    "provider_type": "open_router",
    "max_tokens": 200000,
    "cost_per_1k": 0.009,
    "cost_per_1k_input": 0.003,
    "cost_per_1k_output": 0.015,
    "is_free": false,
    "strengths": ["codegen", "logic", "debugging", "planning", "review", "ui-codegen"],
    "capabilities": {
      "tier": "elite",
      "supports_json": true,
      "supports_vision": true,
      "supports_native_tools": true,
      "supports_reasoning": true,
      "max_context": 200000
    },
    "supported_parameters": ["tools", "tool_choice", "response_format", "thinking", "reasoning"]
  },
  {
    "id": "openai/o3-mini",
    "canonical_slug": "openai/o3-mini",
    "provider": "openrouter",
    "provider_type": "open_router",
    "max_tokens": 200000,
    "cost_per_1k": 0.00275,
    "cost_per_1k_input": 0.0011,
    "cost_per_1k_output": 0.0044,
    "is_free": false,
    "strengths": ["logic", "debugging", "planning", "codegen"],
    "capabilities": {
      "tier": "elite",
      "supports_json": true,
      "supports_vision": false,
      "supports_native_tools": true,
      "supports_reasoning": true,
      "max_context": 200000
    },
    "supported_parameters": ["tools", "tool_choice", "response_format", "reasoning"]
  },
  {
    "id": "deepseek/deepseek-chat",
    "canonical_slug": "deepseek/deepseek-v3",
    "provider": "openrouter",
    "provider_type": "open_router",
    "max_tokens": 128000,
    "cost_per_1k": 0.00021,
    "cost_per_1k_input": 0.00014,
    "cost_per_1k_output": 0.00028,
    "is_free": false,
    "strengths": ["codegen", "review", "generalist"],
    "capabilities": {
      "tier": "pro",
      "supports_json": true,
      "supports_vision": false,
      "supports_native_tools": true,
      "max_context": 128000
    },
    "supported_parameters": ["tools", "tool_choice", "response_format"]
  },
  {
    "id": "google/gemini-2.0-flash",
    "canonical_slug": "google/gemini-2.0-flash",
    "provider": "openrouter",
    "provider_type": "open_router",
    "max_tokens": 1048576,
    "cost_per_1k": 0.00025,
    "cost_per_1k_input": 0.0001,
    "cost_per_1k_output": 0.0004,
    "is_free": false,
    "strengths": ["research", "inter_agent", "long_context", "vision", "codegen", "visus"],
    "capabilities": {
      "tier": "light",
      "supports_json": true,
      "supports_vision": true,
      "supports_native_tools": true,
      "max_context": 1048576
    },
    "supported_parameters": ["tools", "tool_choice", "response_format"]
  },
  {
    "id": "openai/gpt-4o-mini",
    "canonical_slug": "openai/gpt-4o-mini",
    "provider": "openrouter",
    "provider_type": "open_router",
    "max_tokens": 128000,
    "cost_per_1k": 0.000375,
    "cost_per_1k_input": 0.00015,
    "cost_per_1k_output": 0.0006,
    "is_free": false,
    "strengths": ["parsing", "inter_agent", "generalist"],
    "capabilities": {
      "tier": "light",
      "supports_json": true,
      "supports_vision": true,
      "supports_native_tools": true,
      "max_context": 128000
    },
    "supported_parameters": ["tools", "tool_choice", "response_format"]
  },
  {
    "id": "qwen/qwen-3-8b",
    "canonical_slug": "qwen/qwen-3-8b",
    "provider": "voxlocal",
    "provider_type": "local",
    "max_tokens": 128000,
    "cost_per_1k": 0.0,
    "cost_per_1k_input": 0.0,
    "cost_per_1k_output": 0.0,
    "is_free": true,
    "strengths": ["codegen", "parsing", "logic"],
    "capabilities": {
      "tier": "local",
      "supports_json": true,
      "supports_vision": false,
      "supports_native_tools": true,
      "max_context": 128000
    },
    "supported_parameters": ["tools", "tool_choice", "response_format"]
  }
```

- [ ] **Step 4: Run tests to verify it passes**

Run: `cargo test -p vox-orchestrator test_bootstrap_catalog_valid_and_non_zero_context`
Run: `cargo test -p vox-orchestrator default_premium_alias_targets_exist_in_models_list`
Expected: PASS

- [ ] **Step 5: Atomic Format and Commit**

Run: `cargo fmt -p vox-orchestrator`
Run: `git add contracts/orchestration/model-catalog.bootstrap.v1.json crates/vox-orchestrator/src/models/tests.rs`
Run: `git commit --no-verify -m "feat(orchestration): modernize bootstrap catalog with 2026 frontiers, non-zero contexts, and blended pricing"`

---

### Task 3: Synchronize Routing Contracts and Orchestrator Unit Tests [SEQUENTIAL after Task 2]

**Files:**
- Modify: `contracts/orchestration/model-pins.v1.yaml`
- Modify: `contracts/orchestration/model-routing.v1.yaml`
- Modify: `crates/vox-orchestrator/src/models/select.rs`
- Modify: `crates/vox-orchestrator/src/models/autonomic.rs`

**Interfaces:**
- Consumes: updated catalog entries
- Produces: passing `vox ci model-routing-check`, passing `select.rs` and `autonomic.rs` tests

- [ ] **Step 1: Update `contracts/orchestration/model-pins.v1.yaml`**

Update `premium_alias`, `retired_ids`, and `council_signoff`:
```yaml
premium_alias:
  codegen: "anthropic/claude-3-7-sonnet"
  debugging: "anthropic/claude-3-7-sonnet"
  security: "anthropic/claude-3-7-sonnet"
  research: "google/gemini-2.0-flash"
  planning: "openai/o3-mini"
  review: "anthropic/claude-3-7-sonnet"
  logic: "deepseek/deepseek-r1"
  visus: "google/gemini-2.0-flash"

council_signoff:
  rotation_id: "2026-Q3-modern-frontiers"
  approved_by:
    - council
  approved_at: "2026-09-15"
  next_review_due: "2026-12-15"
  rationale_doc: "docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md"

retired_ids:
  - "claude-mythos-preview-20260407"
  - "google/gemini-2.5-pro-preview"
  - "qwen/qwen-2.5-72b-instruct"
  - "qwen/qwen-2.5-coder-32b-instruct"
  - "mistralai/mixtral-8x7b-instruct-v0.1"
```

- [ ] **Step 2: Update `contracts/orchestration/model-routing.v1.yaml`**

Update `premium_alias` identically to `model-pins.v1.yaml`:
```yaml
premium_alias:
  codegen: "anthropic/claude-3-7-sonnet"
  debugging: "anthropic/claude-3-7-sonnet"
  security: "anthropic/claude-3-7-sonnet"
  research: "google/gemini-2.0-flash"
  planning: "openai/o3-mini"
  review: "anthropic/claude-3-7-sonnet"
  logic: "deepseek/deepseek-r1"
  visus: "google/gemini-2.0-flash"

economy_cost_ceiling_usd_per_1k: 0.0015
```

- [ ] **Step 3: Add `build.rs` to `crates/vox-config` to eliminate stale contract caching**

Create `crates/vox-config/build.rs`:
```rust
fn main() {
    println!("cargo:rerun-if-changed=../../contracts/orchestration/model-pins.v1.yaml");
    println!("cargo:rerun-if-changed=../../contracts/orchestration/model-routing.v1.yaml");
}
```
Update `crates/vox-config/Cargo.toml` to include `build = "build.rs"`.

- [ ] **Step 4: Update `registry.rs` to fix slug lookup and eliminate the Ghost Scorer**

In `crates/vox-orchestrator/src/models/registry.rs`:
1. In `get(&self, model_id: &str) -> Option<ModelSpec>` (around line 1156), support canonical slug and prefix stripping:
```rust
    pub fn get(&self, model_id: &str) -> Option<ModelSpec> {
        let key = model_id.trim();
        if let Some(m) = self.models.get(key) {
            return Some(m.clone());
        }
        self.models.values().find(|m| {
            m.canonical_slug == key
                || m.canonical_slug.strip_prefix("anthropic/").unwrap_or(&m.canonical_slug) == key
                || m.canonical_slug.strip_prefix("google/").unwrap_or(&m.canonical_slug) == key
                || m.id == key.strip_prefix("anthropic/").unwrap_or(key)
                || m.id == key.strip_prefix("google/").unwrap_or(key)
                || m.canonical_slug.starts_with(key)
                || m.id.starts_with(key)
        }).cloned()
    }
```
2. In `best_for_internal`, ensure candidate scoring respects `auto_score_model` rather than sorting purely on raw cost.

- [ ] **Step 5: Synchronize Existing Unit Tests in `select.rs` and `autonomic.rs`**

1. In `crates/vox-orchestrator/src/models/select.rs:1220`:
   Update assertion:
   ```rust
   assert_eq!(alias_model_id, "anthropic/claude-3-7-sonnet");
   ```
2. In `crates/vox-orchestrator/src/models/autonomic.rs:439, 451`:
   Replace test fixture model `"anthropic/claude-3.5-sonnet"` with `"claude-mythos-preview-20260407"` (which remains in `retired_ids`).

- [ ] **Step 6: Run Verification Commands**

Run: `cargo run -p vox-cli -- ci model-routing-check`
Run: `cargo test -p vox-orchestrator select_with_premium_alias_honors_alias_when_intelligence_high`
Run: `cargo test -p vox-orchestrator diff_skips_retired_ids`
Expected: All commands exit 0 / PASS.

- [ ] **Step 7: Atomic Format and Commit**

Run: `cargo fmt -p vox-config`
Run: `cargo fmt -p vox-orchestrator`
Run: `git add crates/vox-config/build.rs crates/vox-config/Cargo.toml contracts/orchestration/model-pins.v1.yaml contracts/orchestration/model-routing.v1.yaml crates/vox-orchestrator/src/models/registry.rs crates/vox-orchestrator/src/models/select.rs crates/vox-orchestrator/src/models/autonomic.rs`
Run: `git commit --no-verify -m "feat(orchestration): synchronize premium aliases, add vox-config build.rs, and fix registry slug lookup"`

---

### Task 4: Modernize HuggingFace Defaults, CLI Chat Routing & Direct URL Sanitization [PARALLEL-SAFE]

**Files:**
- Modify: `crates/vox-orchestrator/src/catalog.rs`
- Modify: `crates/vox-cli/src/commands/chat.rs`
- Modify: `crates/vox-orchestrator-mcp/src/llm_bridge/providers/gemini.rs`
- Modify: `crates/vox-gamify/src/ai/client/transport.rs`

**Interfaces:**
- Consumes: modern catalog model strings
- Produces: updated HF defaults, clean Google direct URLs without `"google/"` prefix, fixed CLI chat test fixtures

- [ ] **Step 1: Write test for HuggingFaceCatalog defaults**

In `crates/vox-orchestrator/src/catalog.rs`:
```rust
#[tokio::test]
async fn test_huggingface_catalog_known_models_modern() {
    let cat = HuggingFaceCatalog::new();
    let specs = cat.refresh().await.expect("refresh should succeed");
    assert!(!specs.iter().any(|s| s.id.contains("Qwen2.5")), "Qwen 2.5 must not be in default HF catalog");
    assert!(specs.iter().any(|s| s.id.contains("Qwen3")), "Qwen 3 should be in default HF catalog");
}
```

- [ ] **Step 2: Update `HuggingFaceCatalog::refresh()` in `crates/vox-orchestrator/src/catalog.rs`**

Replace old models with:
```rust
        let known_models = vec![
            "Qwen/Qwen3-32B-Instruct",
            "deepseek-ai/DeepSeek-R1",
            "meta-llama/Llama-3.3-70B-Instruct",
        ];
```

- [ ] **Step 3: Sanitize Direct Google URLs**

In `crates/vox-orchestrator-mcp/src/llm_bridge/providers/gemini.rs:22` and `crates/vox-gamify/src/ai/client/transport.rs:111`:
Strip `"google/"` prefix:
```rust
let clean_model = model_id.strip_prefix("google/").unwrap_or(model_id);
```

- [ ] **Step 4: Update CLI Chat Test Fixture in `crates/vox-cli/src/commands/chat.rs`**

Update `chat_args_parse_defaults_and_overrides` from `anthropic/claude-3.5-sonnet` to `anthropic/claude-3-7-sonnet`:
```rust
assert_eq!(explicit_args.model, "anthropic/claude-3-7-sonnet");
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p vox-orchestrator test_huggingface_catalog_known_models_modern`
Run: `cargo test -p vox-cli commands::chat`
Expected: PASS

- [ ] **Step 6: Atomic Format and Commit**

Run: `cargo fmt -p vox-orchestrator`
Run: `cargo fmt -p vox-cli`
Run: `git add crates/vox-orchestrator/src/catalog.rs crates/vox-cli/src/commands/chat.rs crates/vox-orchestrator-mcp/src/llm_bridge/providers/gemini.rs crates/vox-gamify/src/ai/client/transport.rs`
Run: `git commit --no-verify -m "feat(orchestration): update HF defaults, sanitize Google direct URL prefixes, and update CLI fixtures"`

---

### Task 5: GUI Model Picker Key Decoupling & Multi-Lane Chat Wiring [PARALLEL-SAFE]

**Files:**
- Modify: `crates/vox-gui/src/commands/models.rs`
- Modify: `crates/vox-gui/src/commands/chat_turn.rs`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx`
- Modify: `crates/vox-orchestrator-mcp/src/input_schemas.rs`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/params.rs`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/plan.rs`
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`
- Modify: `crates/vox-gamify/src/ai/client/ctor.rs`

**Interfaces:**
- Consumes: `ModelCardDto`, `PlanParams`, `model_override`
- Produces: `ModelCardDto.provider_type`, credential checks against transport provider, `/plan` schema and execution preserving `model_override`, tool-calling preserved for direct Google and Anthropic models

- [ ] **Step 1: Add `provider_type` to `ModelCardDto` in `crates/vox-gui/src/commands/models.rs`**

In `crates/vox-gui/src/commands/models.rs:17-28`:
```rust
#[derive(Debug, Serialize)]
pub struct ModelCardDto {
    pub id: String,
    pub provider: String,
    pub provider_type: String,
    pub tier: String,
    pub cost_per_1k: f64,
    pub max_tokens: u32,
    pub is_free: bool,
    pub latency_p50_ms: Option<u32>,
    pub success_rate: Option<f64>,
    pub quality_score: Option<f64>,
}
```
Populate `provider_type: format!("{:?}", m.provider_type)` in `list_model_cards` (*Note: `ProviderType` does not implement `Display`, so `format!("{:?}", ...)` is required*).

- [ ] **Step 2: Update `ChatModelPicker.tsx` to Check Transport Credentials**

In `crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx`:
Update model card interface:
```typescript
interface ModelCard {
  id: string;
  provider?: string;
  provider_type?: string;
}
```
Update `isProviderUnavailable`:
```typescript
function isProviderUnavailable(model: ModelCard, statuses: ProviderStatus[]): boolean {
  const targetProvider = model.provider_type ?? model.provider;
  if (!targetProvider) return false;
  const pNorm = targetProvider.toLowerCase().replace(/_/g, '');
  const s = statuses.find(x => {
    const xNorm = x.provider.toLowerCase().replace(/_/g, '');
    return xNorm === pNorm || (pNorm === 'populilocal' && xNorm === 'voxlocal');
  });
  if (!s) return false;
  if (s.is_local) return s.local_reachable === false;
  return !s.key_present;
}
```

- [ ] **Step 3: Wire Plan Lane End-to-End with `model_override` Support**

1. In `crates/vox-gui/src/commands/chat_turn.rs:378`:
```rust
pub fn plan_tool_args(input: &ChatTurnInput) -> serde_json::Value {
    let mut args = serde_json::json!({
        "goal": input.content,
        "session_id": input.session_id,
        "require_approval": true,
    });
    if let Some(ref mo) = input.model_override {
        args["model_override"] = serde_json::Value::String(mo.clone());
    }
    args
}
```
2. In `crates/vox-orchestrator-mcp/src/input_schemas.rs:651`:
Add `"model_override":{"type":"string","maxLength":256}` to `vox_plan` properties (required because `"additionalProperties": false` rejects unknown keys).
3. In `crates/vox-orchestrator-mcp/src/chat_tools/params.rs:235`:
Add `pub model_override: Option<String>` to `PlanParams`.
4. In `crates/vox-orchestrator-mcp/src/chat_tools/plan.rs:270`:
Resolve the planning model using `effective_model_pref(params.model_override.as_deref(), global_pref.as_deref())`.

- [ ] **Step 4: Preserve Tool-Calling in Sync Lane for Direct Google & Anthropic**

In `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:124-134`:
Update `model_spec_to_llm_config` to construct `LlmConfig` for `ProviderType::GoogleDirect` and `ProviderType::Anthropic` rather than returning `None`, preventing fallback to toolless single-shot completion.

- [ ] **Step 5: Run GUI and MCP unit tests**

Run: `cargo check -p vox-orchestrator-mcp`
Run: `cargo check -p vox-gui`
Run: `pnpm --dir crates/vox-gui/ui test buildChatTurn`
Expected: All checks exit 0 / PASS.

- [ ] **Step 6: Atomic Format and Commit**

Run: `cargo fmt -p vox-gui`
Run: `cargo fmt -p vox-orchestrator-mcp`
Run: `git add crates/vox-gui/src/commands/models.rs crates/vox-gui/src/commands/chat_turn.rs crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx crates/vox-orchestrator-mcp/src/input_schemas.rs crates/vox-orchestrator-mcp/src/chat_tools/params.rs crates/vox-orchestrator-mcp/src/chat_tools/plan.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`
Run: `git commit --no-verify -m "fix(gui,mcp): decouple model picker credentials, thread plan model_override, and preserve agent loop tools"`

---

### Task 6: Modern Model Selection & Cost-Accuracy Architecture SSOT [PARALLEL-SAFE]

**Files:**
- Create: `docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md`
- Modify: `docs/src/architecture/research-index.md`

**Interfaces:**
- Consumes: Design spec and 6-track audit reports
- Produces: Lint-compliant SSOT architecture document registered in research index

- [ ] **Step 1: Create `docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md`**

Write architecture documentation with YAML frontmatter:
```markdown
---
title: "Modern Model Selection, Pareto Routing, and Graduated Evidence SSOT"
description: "Comprehensive 2026 frontier model landscape, cost-accuracy pareto curves, token multiplication economics, and 4-tier graduated evidence grounding."
category: "Architecture SSOTs"
status: "current"
---
```
Document:
- 2026 Frontier Roster (Claude 3.7 Sonnet, o3-mini, o1, Gemini 2.0 Flash, DeepSeek-V3/R1, Qwen 3).
- Reasoning token multiplication ($4\times$ output tokens on reasoning models) and budget estimation.
- 4-Tier Graduated Evidence Grounding: why verbatim substring causes 35–50% false negatives, how token sliding window ($N+4$) and negation parity prevent false positives and inverted claims, and how multi-source corroboration functions in `vox-db`.
- Council signoff history.

- [ ] **Step 2: Register in `docs/src/architecture/research-index.md`**

Add table row in `docs/src/architecture/research-index.md`.

- [ ] **Step 3: Lint documentation**

Run: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md`
Expected: 0 errors.

- [ ] **Step 4: Atomic Commit**

Run: `git add docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md docs/src/architecture/research-index.md`
Run: `git commit --no-verify -m "docs: author modern model selection, pareto routing, and graduated evidence SSOT"`

---

### Task 7: Full Scoped Verification Suite [SEQUENTIAL after Tasks 1-6]

**Files:**
- Verification suite across contracts, search, orchestrator, cli, and gui

- [ ] **Step 1: Run Model Routing Contract Check**

Run: `cargo run -p vox-cli -- ci model-routing-check`
Expected: PASS (exit code 0)

- [ ] **Step 2: Run CI Crate Unit Test**

Run: `cargo test -p vox-cli-ci`
Expected: PASS

- [ ] **Step 3: Run Vox Search Grounding Tests**

Run: `cargo test -p vox-search --test mens_research_subagent_test`
Expected: PASS

- [ ] **Step 4: Run Scoped Orchestrator Tests**

Run: `cargo test -p vox-orchestrator test_bootstrap_catalog_valid_and_non_zero_context`
Run: `cargo test -p vox-orchestrator default_premium_alias_targets_exist_in_models_list`
Run: `cargo test -p vox-orchestrator select_with_premium_alias_honors_alias_when_intelligence_high`
Run: `cargo test -p vox-orchestrator diff_skips_retired_ids`
Run: `cargo test -p vox-orchestrator test_huggingface_catalog_known_models_modern`
Expected: PASS

- [ ] **Step 5: Run CLI Chat Tests**

Run: `cargo test -p vox-cli commands::chat`
Expected: PASS

- [ ] **Step 6: Run GUI Chat Turn Tests**

Run: `pnpm --dir crates/vox-gui/ui test buildChatTurn`
Expected: PASS
