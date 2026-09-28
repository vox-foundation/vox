# Model Routing: Latest Models, Honest Scores, Mode Guarantees — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task <this file> <N>`), per [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, reviews the diff and commits. Tasks marked
> **Owner: Claude** or **Owner: user** are not driven. Steps use checkbox (`- [ ]`) syntax.
>
> **Review status (2026-09-28):** three-track review (correctness, tests pre-mortem, simplicity/safety) applied; see
> `.superpowers/review/2026-09-28-model-routing-latest-and-honest-*.md`. The grill stage was not run (the `grill-me`
> skill needs explicit user invocation). Amendments are marked `<!-- AMENDED: R<n> — reason -->`.

**Goal:** On the task-dispatch path, every clutch mode picks the newest member of the right model family, ranked by
real quality evidence, and the default Efficient mode can no longer pick a flagship (Opus-class) model when a cheaper
candidate fits.

**Architecture:** Parse OpenRouter's `created` date; derive a model *family* from each id and drop superseded
versions among the candidates that already passed every filter in `ModelRegistry::best_for_internal`; replace the
paid/free quality proxy with a dated, family-keyed seed contract and stamp seeded tiers at registration; enforce each
clutch mode's promise and the provider-key check with a mode-aware selector used by task dispatch.

**Tech Stack:** Rust (`vox-orchestrator`, `vox-research-shim`), serde/serde_json, JSON contracts under `contracts/orchestration/`.

**Spec:** [`docs/src/architecture/chat-surface-design-critique-2026-09-28.md`](../../src/architecture/chat-surface-design-critique-2026-09-28.md)
(priority recommendation 1) and the evidence section of
[`2026-09-28-chat-surface-trace-and-latest-models.md`](2026-09-28-chat-surface-trace-and-latest-models.md).

## Global Constraints

- All LLM selection stays inside `vox-orchestrator::models` (AGENTS.md §Model-Agnostic LLM Boundary). No new crate, no new crate edge, no new dependency.
- No versioned cloud model id may be introduced as a literal in non-test code. Local MENS revisions are exempt (AGENTS.md).
- `contracts/orchestration/model-pins.v1.yaml` is council-ratified (2026-09-15); this plan does **not** edit it (Task 8 is a user decision).
- New code goes in new files under `crates/vox-orchestrator/src/models/`; `registry.rs`, `catalog.rs`, `runtime.rs` get only the edits shown.
- Test-first for every new `pub fn` (lefthook `tdd-guard`, which wants a test in the same file). Record the RED output **before** touching implementation code. Format only changed files: `rustfmt --edition 2024 <file>`. Never `cargo fmt`.
- Every build/test command is foreground and prefixed with `timeout 1500s`. Cargo takes one test filter before `--`; pass several filters after `--` (e.g. `cargo test -p vox-orchestrator --lib -- a b c`). <!-- AMENDED: R14 — cargo rejects multiple TESTNAME args -->
- Family, seed and quality lookups key on `ModelSpec.id`; `canonical_slug` is parsed and stored but not used for family identity (bootstrap canonical slugs are irregular, e.g. `anthropic/sonnet`). <!-- AMENDED: R10 — mixed slug sources missed seeds -->
- Existing tests are not edited except the three `resolve_task_cost_policy` tests in `runtime.rs` (mechanical four-value destructure, Task 6). Any other existing test that breaks is a STOP.
- The agent never runs `git add` / `git commit`; each task's commit block is run by Claude Code after review.

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `crates/vox-orchestrator/src/models/spec.rs` | modify | `ModelCapabilities.released_at: Option<u64>` |
| `crates/vox-research-shim/src/selection/virtual_models.rs` | modify | `released_at: None` in its two literal `ModelCapabilities` |
| `crates/vox-orchestrator/src/catalog.rs` | modify | parse `created` / `canonical_slug`; extract `specs_from_openrouter_json` |
| `crates/vox-orchestrator/src/models/family.rs` | create | `family_key`, `version_tuple`, `newest_per_family`, `is_superseded` (pure) |
| `contracts/orchestration/model-seed-scores.v1.json` | create (Claude) | dated, sourced, family-keyed intelligence / responsiveness / tier |
| `crates/vox-orchestrator/src/models/seed.rs` | create | load the seed contract; `seed_for`, `seed_for_model` |
| `crates/vox-orchestrator/src/models/scoring.rs` | modify | `quality_score` uses the seed; unseeded models capped |
| `crates/vox-orchestrator/src/models/registry.rs` | modify | `register` stamps seeded tier when `Unknown`; `best_for_internal` drops superseded candidates |
| `crates/vox-orchestrator/src/models/key_guard.rs` | modify | `selection_key_available` with a thread-local test override |
| `crates/vox-orchestrator/src/models/mode_select.rs` | create | `ModeSelection`, `best_for_task_in_mode` (Elite excluded first for Efficiency/Balanced; provider-key gate) |
| `crates/vox-orchestrator/src/runtime.rs` | modify | `resolve_task_cost_policy` also returns the clutch; dispatch calls `best_for_task_in_mode` |
| `crates/vox-orchestrator/src/models/mod.rs` | modify | `pub mod family; pub mod seed; pub mod mode_select;` |

---

### Task 1: Parse OpenRouter `created` and `canonical_slug`

**Files:**
- Modify: `crates/vox-orchestrator/src/models/spec.rs` (struct `ModelCapabilities`, after `uptime_score`)
- Modify: `crates/vox-research-shim/src/selection/virtual_models.rs` (the two `ModelCapabilities {` literals)
- Modify: `crates/vox-orchestrator/src/catalog.rs` (`OpenRouterModelData`, `OpenRouterCatalog::refresh`)
- Test: `crates/vox-orchestrator/src/catalog.rs` (existing `mod tests`)

**Interfaces:**
- Produces: `ModelCapabilities::released_at: Option<u64>`; `pub(crate) fn specs_from_openrouter_json(json: &str) -> anyhow::Result<Vec<ModelSpec>>` in `catalog.rs`.

- [ ] **Step 1: Write the failing tests** — append inside `mod tests` in `catalog.rs`:

```rust
    const SAMPLE_MODELS_JSON: &str = r#"{"data":[
      {"id":"anthropic/claude-sonnet-4.6","canonical_slug":"anthropic/claude-4.6-sonnet-20260101","created":1767225600,
       "pricing":{"prompt":"0.000003","completion":"0.000015"},"context_length":200000},
      {"id":"qwen/qwen3-coder:free","created":1760000000,
       "pricing":{"prompt":"0","completion":"0"},"context_length":131072},
      {"id":"acme/undated-model",
       "pricing":{"prompt":"0.000001","completion":"0.000002"},"context_length":32000}
    ]}"#;

    #[test]
    fn specs_from_openrouter_json_reads_created_and_canonical_slug() {
        let specs = specs_from_openrouter_json(SAMPLE_MODELS_JSON).expect("parse");
        assert_eq!(specs.len(), 3);
        let sonnet = specs.iter().find(|s| s.id == "anthropic/claude-sonnet-4.6").unwrap();
        assert_eq!(sonnet.capabilities.released_at, Some(1_767_225_600));
        assert_eq!(sonnet.canonical_slug, "anthropic/claude-4.6-sonnet-20260101");
        let free = specs.iter().find(|s| s.id == "qwen/qwen3-coder:free").unwrap();
        assert!(free.is_free);
        assert_eq!(free.canonical_slug, "qwen/qwen3-coder:free", "missing canonical_slug falls back to id");
    }

    #[test]
    fn specs_from_openrouter_json_keeps_missing_created_as_none() {
        let specs = specs_from_openrouter_json(SAMPLE_MODELS_JSON).expect("parse");
        let undated = specs.iter().find(|s| s.id == "acme/undated-model").unwrap();
        assert_eq!(undated.capabilities.released_at, None, "absent created must not become 0 or now");
        assert_eq!(undated.capabilities.max_context, 32_000);
    }
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib catalog::tests::specs_from_openrouter_json > target/models-t1-red.txt 2>&1; tail -20 target/models-t1-red.txt`
Expected: FAIL to compile — `cannot find function specs_from_openrouter_json` / `no field released_at`.

- [ ] **Step 3: Add the capability field.** In `spec.rs`, inside `pub struct ModelCapabilities`, directly after the `uptime_score` field:

```rust
    /// Unix time the provider published this model (OpenRouter `/models` `created`).
    /// `None` when the source does not report it; never defaulted to 0 or "now",
    /// because recency ordering (`models::family`) must not invent a date.
    #[serde(default)]
    pub released_at: Option<u64>,
```

Then add `released_at: None,` to the two `ModelCapabilities {` literals in `crates/vox-research-shim/src/selection/virtual_models.rs` (the only literals in the workspace without `..Default::default()`). If `cargo check` names any other site, STOP and list it. <!-- AMENDED: R19 — two sites, not three; spec.rs:388 already uses ..Default -->

- [ ] **Step 4: Parse the fields.** In `catalog.rs`, add to `struct OpenRouterModelData` (after `latency`):

```rust
    /// Provider-published unix time; the only recency signal OpenRouter exposes.
    #[serde(default)]
    created: Option<u64>,
    /// Stable dated slug when OpenRouter provides one; falls back to `id`.
    #[serde(default)]
    canonical_slug: Option<String>,
```

Move the body of `refresh` from `let body: OpenRouterModelsResponse = resp.json().await?;` through `Ok(models)` into a new free function, and make `refresh` call it:

```rust
/// Map an OpenRouter `/api/v1/models` JSON body to specs. Pure; used by
/// [`OpenRouterCatalog::refresh`] and by tests.
pub(crate) fn specs_from_openrouter_json(json: &str) -> anyhow::Result<Vec<ModelSpec>> {
    let body: OpenRouterModelsResponse = serde_json::from_str(json)?;
    let mut models = Vec::new();
    for m in body.data {
        // ... the existing loop body, unchanged, except the two edits below ...
    }
    Ok(models)
}
```

and in `refresh` replace the moved lines with:

```rust
        let text = resp.text().await?;
        specs_from_openrouter_json(&text)
```

Inside the moved loop make exactly two edits: in the `ModelCapabilities { ... }` literal add `released_at: m.created,` before `..Default::default()`; in the `ModelSpec { ... }` literal replace `canonical_slug: m.id.clone(),` with `canonical_slug: m.canonical_slug.clone().unwrap_or_else(|| m.id.clone()),`. Change nothing else in the loop.

- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib catalog 2>&1 | tail -20 && timeout 1500s cargo check -p vox-research-shim 2>&1 | tail -3`
Expected: all `catalog::tests` pass (including `infer_strengths_*`); `vox-research-shim` checks clean.

- [ ] **Step 6: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/spec.rs crates/vox-research-shim/src/selection/virtual_models.rs crates/vox-orchestrator/src/catalog.rs
git commit -m "feat(models): read OpenRouter created and canonical_slug into the catalog"
```

---

### Task 2: Model families and superseded versions

**Files:**
- Create: `crates/vox-orchestrator/src/models/family.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs` (add `pub mod family;` in alphabetical position)

**Interfaces:**
- Consumes: `ModelSpec.id`, `ModelCapabilities.released_at` (Task 1).
- Produces: `pub fn family_key(slug: &str) -> String`; `pub fn version_tuple(slug: &str) -> Vec<u32>`; `pub fn newest_per_family<'a>(specs: impl IntoIterator<Item = &'a ModelSpec>) -> HashMap<String, (u64, Vec<u32>)>`; `pub fn is_superseded(m: &ModelSpec, newest: &HashMap<String, (u64, Vec<u32>)>) -> bool`.

- [ ] **Step 1: Write the failing tests** — create `family.rs` with only the test module:

```rust
//! Model families: the version-free identity of a model line (e.g. every
//! `anthropic/claude-opus-*`), so routing can prefer the newest member.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ModelCapabilities, ModelSpec, ProviderType};

    fn dated(slug: &str, released_at: Option<u64>) -> ModelSpec {
        ModelSpec {
            id: slug.into(),
            canonical_slug: slug.into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 8192,
            cost_per_1k: 1.0,
            cost_per_1k_input: 1.0,
            cost_per_1k_output: 1.0,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![],
            capabilities: ModelCapabilities { released_at, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: crate::models::spec::PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn family_key_strips_versions_across_naming_shapes() {
        assert_eq!(family_key("anthropic/claude-opus-4.8"), "anthropic/claude-opus");
        assert_eq!(family_key("anthropic/claude-sonnet-4.6"), "anthropic/claude-sonnet");
        assert_eq!(family_key("anthropic/claude-3-7-sonnet"), "anthropic/claude-sonnet");
        assert_eq!(family_key("deepseek/deepseek-v4-flash"), "deepseek/deepseek-flash");
        assert_eq!(family_key("openai/gpt-5-mini"), "openai/gpt-mini");
        assert_eq!(family_key("google/gemini-3.1-flash-lite-preview"), "google/gemini-flash-lite");
        assert_eq!(family_key("Anthropic/Claude-Opus-5.5"), "anthropic/claude-opus");
    }

    #[test]
    fn family_key_keeps_free_variants_and_sizes_apart() {
        // <!-- AMENDED: R2/R3 — :free and parameter sizes are distinct families -->
        assert_eq!(family_key("qwen/qwen3-coder:free"), "qwen/qwen-coder:free");
        assert_eq!(family_key("qwen/qwen3-coder"), "qwen/qwen-coder");
        assert_eq!(family_key("qwen/qwen3-coder:beta"), "qwen/qwen-coder");
        assert_eq!(family_key("google/gemma-4-31b-it"), "google/gemma-31b-it");
        assert_ne!(family_key("meta-llama/llama-3.1-8b"), family_key("meta-llama/llama-3.1-70b"));
        assert_eq!(family_key("qwen/qwen3-235b-a22b"), "qwen/qwen-235b-a22b");
    }

    #[test]
    fn version_tuple_orders_numeric_parts() {
        assert_eq!(version_tuple("anthropic/claude-opus-4.8"), vec![4, 8]);
        assert!(version_tuple("anthropic/claude-opus-5.5") > version_tuple("anthropic/claude-opus-4.8"));
        assert_eq!(version_tuple("acme/plain-model"), Vec::<u32>::new());
    }

    #[test]
    fn older_dated_member_is_superseded_newest_is_not() {
        let old = dated("anthropic/claude-opus-4.8", Some(1_700_000_000));
        let new = dated("anthropic/claude-opus-5.5", Some(1_760_000_000));
        let other = dated("deepseek/deepseek-v4-flash", Some(1_650_000_000));
        let newest = newest_per_family([&old, &new, &other]);
        assert!(is_superseded(&old, &newest));
        assert!(!is_superseded(&new, &newest));
        assert!(!is_superseded(&other, &newest), "sole member of its family is never superseded");
    }

    #[test]
    fn free_variant_is_not_superseded_by_a_newer_paid_member() {
        let free = dated("qwen/qwen3-coder:free", Some(1_700_000_000));
        let paid = dated("qwen/qwen3.5-coder", Some(1_760_000_000));
        let newest = newest_per_family([&free, &paid]);
        assert!(!is_superseded(&free, &newest));
    }

    #[test]
    fn undated_specs_are_never_superseded_and_never_supersede() {
        let undated_old_name = dated("anthropic/claude-opus-4.8", None);
        let dated_new = dated("anthropic/claude-opus-5.5", Some(1_760_000_000));
        let newest = newest_per_family([&undated_old_name, &dated_new]);
        assert!(!is_superseded(&undated_old_name, &newest), "no date, no verdict");
        assert!(newest_per_family([&undated_old_name]).is_empty());
    }

    #[test]
    fn same_date_tie_breaks_on_version() {
        let a = dated("google/gemini-3-flash", Some(1_750_000_000));
        let b = dated("google/gemini-3.1-flash", Some(1_750_000_000));
        let newest = newest_per_family([&a, &b]);
        assert!(is_superseded(&a, &newest));
        assert!(!is_superseded(&b, &newest));
    }
}
```

Add `pub mod family;` to `models/mod.rs`.

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::family > target/models-t2-red.txt 2>&1; tail -15 target/models-t2-red.txt`
Expected: FAIL to compile — `cannot find function family_key`.

- [ ] **Step 3: Implement** — insert above `#[cfg(test)]` in `family.rs`:

```rust
use std::collections::HashMap;

use crate::models::ModelSpec;

/// Qualifier tokens that mark a release stage, not a different family.
const QUALIFIERS: &[&str] = &["preview", "beta", "exp", "experimental", "latest"];

fn is_size_token(tok: &str) -> bool {
    tok.len() > 1
        && tok.ends_with('b')
        && tok[..tok.len() - 1].trim_start_matches('a').chars().all(|c| c.is_ascii_digit() || c == '.')
        && tok[..tok.len() - 1].trim_start_matches('a').chars().any(|c| c.is_ascii_digit())
}

/// Version-free family id: `org/name-words[:free]`, lowercased.
///
/// Per `-`-separated token of the name: pure numbers/dots (`4.6`, `20260101`) and
/// `v4`-style markers are dropped; release-stage words (`preview`, `beta`, `exp`,
/// `latest`) are dropped; parameter sizes (`8b`, `70b`, `a22b`) are KEPT, because a
/// smaller size is a different cost point, not an older version; other mixed tokens
/// keep only their letters (`qwen3` → `qwen`, `k2.6` → `k`, `4o` → `o`). A `:free`
/// variant is its own family (it is a different price pool); other `:variant`
/// suffixes are dropped.
// ponytail: heuristic slug parse; if providers publish an explicit family field, prefer it.
#[must_use]
pub fn family_key(slug: &str) -> String {
    let slug = slug.to_ascii_lowercase();
    let (base, variant) = slug.split_once(':').unwrap_or((slug.as_str(), ""));
    let (org, name) = base.split_once('/').unwrap_or(("", base));
    let words: Vec<String> = name
        .split('-')
        .filter_map(|tok| {
            if tok.is_empty() || QUALIFIERS.contains(&tok) {
                return None;
            }
            if !tok.chars().any(|c| c.is_ascii_digit()) {
                return Some(tok.to_string());
            }
            if is_size_token(tok) {
                return Some(tok.to_string());
            }
            let is_version = tok.chars().all(|c| c.is_ascii_digit() || c == '.');
            let is_v_marker =
                tok.starts_with('v') && tok[1..].chars().all(|c| c.is_ascii_digit() || c == '.');
            if is_version || is_v_marker {
                return None;
            }
            let letters: String = tok.chars().filter(|c| c.is_ascii_alphabetic()).collect();
            (!letters.is_empty()).then_some(letters)
        })
        .collect();
    let mut key = if org.is_empty() { words.join("-") } else { format!("{org}/{}", words.join("-")) };
    if variant == "free" {
        key.push_str(":free");
    }
    key
}

/// Every run of digits in the slug, in order (`claude-opus-4.8` → `[4, 8]`).
#[must_use]
pub fn version_tuple(slug: &str) -> Vec<u32> {
    slug.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Newest `(released_at, version)` per family, over dated specs only.
#[must_use]
pub fn newest_per_family<'a>(
    specs: impl IntoIterator<Item = &'a ModelSpec>,
) -> HashMap<String, (u64, Vec<u32>)> {
    let mut newest: HashMap<String, (u64, Vec<u32>)> = HashMap::new();
    for m in specs {
        let Some(at) = m.capabilities.released_at else { continue };
        let key = family_key(&m.id);
        let cand = (at, version_tuple(&m.id));
        match newest.get(&key) {
            Some(cur) if *cur >= cand => {}
            _ => {
                newest.insert(key, cand);
            }
        }
    }
    newest
}

/// True when a newer dated member of the same family is in `newest`. Undated
/// specs (bootstrap, local MENS) are never superseded.
#[must_use]
pub fn is_superseded(m: &ModelSpec, newest: &HashMap<String, (u64, Vec<u32>)>) -> bool {
    let Some(at) = m.capabilities.released_at else { return false };
    newest
        .get(&family_key(&m.id))
        .is_some_and(|best| *best > (at, version_tuple(&m.id)))
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::family 2>&1 | tail -15`
Expected: 7 passed. If a `family_key` assertion fails, fix the function, not the assertion; if an assertion looks wrong, STOP and say which.

- [ ] **Step 5: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/family.rs crates/vox-orchestrator/src/models/mod.rs
git commit -m "feat(models): family keys and superseded-version detection"
```

---

### Task 3: Seed-score contract — Owner: Claude

**Files:**
- Create: `contracts/orchestration/model-seed-scores.v1.json`

**Interfaces:**
- Produces (shape consumed by Task 4, exact):

```json
{
  "schema": "vox.orchestration.seed-scores/v1",
  "as_of": "YYYY-MM-DD",
  "sources": [{ "name": "…", "url": "https://…", "retrieved": "YYYY-MM-DD" }],
  "families": [
    { "family": "anthropic/claude-opus", "intelligence": 90, "responsiveness": 40, "tier": "elite" }
  ]
}
```

`intelligence` and `responsiveness` are **integers** 0–100 or `null`; `tier` is a `ModelTier` snake_case name
(`elite`, `pro`, `fast`, `light`, `free`, `local`, `unknown`). <!-- AMENDED: R8 — Option<u8> rejects decimals -->

- [ ] **Step 1:** After Task 2 is committed, generate the family list by running `family_key` over every non-local id in `model-catalog.bootstrap.v1.json` (a top-level JSON array; skip `provider_type` `ollama` and `vox_local`) with a throwaway test or `vox run` script, not by hand, so the contract uses the exact keys the code computes (e.g. `openai/o-mini`, `moonshot/kimi-k-thinking`, `zhipu/glm`). <!-- AMENDED: R8 — hand-written keys would not match -->
- [ ] **Step 2:** Research warp.dev's published agent-model defaults (per mode) and one machine-readable public quality/latency index whose terms permit automated use; record URLs and retrieval dates in `sources`.
- [ ] **Step 3:** Write the file with every generated family. Flagship lines (Opus-class, GPT-*-pro, Gemini *-pro) are `elite`; strong mid lines `pro`; Flash/mini/Haiku/DeepSeek-flash-class `fast`; `…:free` families `free`.
- [ ] **Step 4:** `jq -e '[.families[] | select((.intelligence|type) != "number" and .intelligence != null)] | length == 0' contracts/orchestration/model-seed-scores.v1.json`
- [ ] **Step 5 (pre-drive for Task 4):** edit Task 4 of this plan, replacing `@@HIGH@@`, `@@LOW@@` and `@@ELITE@@` with literal family keys from the contract (two families with non-null, different intelligence, and one `elite` family). <!-- AMENDED: R8 — Flash must not choose rows -->
- [ ] **Step 6: Commit** `feat(models): dated family seed scores (warp.dev defaults + <index>)`, body citing sources.

---

### Task 4: Seed loader, honest quality, seeded tiers

**Pre-drive gate:** Task 3 Step 5 has replaced every `@@…@@` marker below with a literal family key.

**Files:**
- Create: `crates/vox-orchestrator/src/models/seed.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs` (add `pub mod seed;`)
- Modify: `crates/vox-orchestrator/src/models/scoring.rs` (`quality_score`, and a test in its test module)
- Modify: `crates/vox-orchestrator/src/models/registry.rs` (`ModelRegistry::register`)
- Test: `crates/vox-orchestrator/src/models/tests.rs` (new module at end)

**Interfaces:**
- Consumes: `family::family_key` (Task 2); the contract (Task 3).
- Produces: `pub struct SeedScore { pub intelligence: Option<u8>, pub responsiveness: Option<u8>, pub tier: crate::models::ModelTier }`; `pub fn seed_for(family: &str) -> Option<&'static SeedScore>`; `pub fn seed_for_model(id: &str) -> Option<&'static SeedScore>`.

- [ ] **Step 1: Write the failing seed tests** — create `seed.rs` with the test module only:

```rust
//! Dated, family-keyed prior scores from `contracts/orchestration/model-seed-scores.v1.json`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_parses_and_has_families() {
        assert!(!seeds().is_empty());
    }

    #[test]
    fn seed_for_model_resolves_through_family_key() {
        // <!-- AMENDED: R8 — the old test never called seed_for_model -->
        let direct = seed_for("@@ELITE@@").expect("elite family row");
        let via_model = seed_for_model("@@ELITE@@-9.9").expect("versioned id resolves to its family");
        assert_eq!(direct.tier, via_model.tier);
        assert!(seed_for("nobody/no-such-family").is_none(), "unknown family is None, not a panic");
    }

    #[test]
    fn every_bootstrap_family_has_a_seed_row() {
        // <!-- AMENDED: R11 — bootstrap is a top-level array; local type is vox_local -->
        let raw = include_str!("../../../../contracts/orchestration/model-catalog.bootstrap.v1.json");
        let v: serde_json::Value = serde_json::from_str(raw).expect("bootstrap json");
        let rows = v.as_array().expect("bootstrap catalog is a top-level array");
        assert!(!rows.is_empty());
        let mut missing = Vec::new();
        for m in rows {
            let Some(id) = m["id"].as_str() else { continue };
            let ptype = m["provider_type"].as_str().unwrap_or_default();
            if ptype.eq_ignore_ascii_case("ollama") || ptype.eq_ignore_ascii_case("vox_local") {
                continue;
            }
            let fam = crate::models::family::family_key(id);
            if seed_for(&fam).is_none() {
                missing.push(fam);
            }
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "bootstrap families with no seed row: {missing:?}");
    }
}
```

Add `pub mod seed;` to `models/mod.rs`.

- [ ] **Step 2: Write the failing quality and tier tests.** Append to the test module of `scoring.rs` (it has a `make_spec(provider_type, cost, is_free)` helper; `rg -n "fn make_spec" crates/vox-orchestrator/src/models/scoring.rs`):

```rust
    #[test]
    fn quality_score_uses_seed_intelligence_when_present() {
        let mut hi = make_spec(ProviderType::OpenRouter, 1.0, false);
        hi.id = "@@HIGH@@".into();
        let mut lo = make_spec(ProviderType::OpenRouter, 1.0, false);
        lo.id = "@@LOW@@".into();
        assert!(quality_score(&hi) > quality_score(&lo), "paid models must no longer score equal");
    }

    #[test]
    fn unseeded_quality_is_capped_below_strong_seeded_families() {
        // <!-- AMENDED: R4 — unseeded long-tail models must not outrank seeded ones -->
        let mut unknown = make_spec(ProviderType::OpenRouter, 1.0, false);
        unknown.id = "nobody/no-such-family".into();
        unknown.max_tokens = 1_000_000;
        let q = quality_score(&unknown);
        assert!(q > 0.0 && q <= UNSEEDED_QUALITY_CAP, "got {q}");
    }
```

Append to `crates/vox-orchestrator/src/models/tests.rs`:

```rust
#[cfg(test)]
mod seeded_tier_tests {
    use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType};
    use crate::models::spec::PricingSource;

    fn unknown_tier(id: &str) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 8192,
            cost_per_1k: 1.0,
            cost_per_1k_input: 1.0,
            cost_per_1k_output: 1.0,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![],
            capabilities: ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::OpenRouter,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn register_stamps_the_seeded_tier_on_unknown_specs() {
        // <!-- AMENDED: R5 — stamp once at registration so every catalog (OpenRouter, direct) is covered -->
        let mut r = ModelRegistry::default();
        r.register(unknown_tier("@@ELITE@@-9.9"));
        assert_eq!(r.get("@@ELITE@@-9.9").unwrap().capabilities.tier, ModelTier::Elite);
    }

    #[test]
    fn register_keeps_an_explicit_tier() {
        let mut r = ModelRegistry::default();
        let mut s = unknown_tier("@@ELITE@@-9.9");
        s.capabilities.tier = ModelTier::Pro;
        r.register(s);
        assert_eq!(r.get("@@ELITE@@-9.9").unwrap().capabilities.tier, ModelTier::Pro);
    }
}
```

(`ModelRegistry::get` returns an owned or borrowed spec; if `.unwrap().capabilities` does not compile, adapt only the accessor in these two tests and say so.)

- [ ] **Step 3: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::seed models::scoring models::tests::seeded_tier_tests > target/models-t4-red.txt 2>&1; tail -20 target/models-t4-red.txt`
Expected: FAIL to compile (`seeds`, `UNSEEDED_QUALITY_CAP` not found). Then comment out nothing — proceed to implementation only after saving this output.

- [ ] **Step 4: Implement `seed.rs`** (above the tests):

```rust
use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::models::ModelTier;

#[derive(Debug, Clone, Deserialize)]
pub struct SeedScore {
    pub intelligence: Option<u8>,
    pub responsiveness: Option<u8>,
    pub tier: ModelTier,
}

#[derive(Deserialize)]
struct SeedRow {
    family: String,
    #[serde(flatten)]
    score: SeedScore,
}

#[derive(Deserialize)]
struct SeedFile {
    families: Vec<SeedRow>,
}

fn seeds() -> &'static HashMap<String, SeedScore> {
    static SEEDS: OnceLock<HashMap<String, SeedScore>> = OnceLock::new();
    SEEDS.get_or_init(|| {
        let raw = include_str!("../../../../contracts/orchestration/model-seed-scores.v1.json");
        let file: SeedFile = serde_json::from_str(raw).expect("model-seed-scores.v1.json is valid");
        file.families.into_iter().map(|r| (r.family, r.score)).collect()
    })
}

#[must_use]
pub fn seed_for(family: &str) -> Option<&'static SeedScore> {
    seeds().get(family)
}

#[must_use]
pub fn seed_for_model(id: &str) -> Option<&'static SeedScore> {
    seed_for(&crate::models::family::family_key(id))
}
```

- [ ] **Step 5: Implement `quality_score`.** In `scoring.rs` add next to the other constants:

```rust
/// Ceiling for models with no seed row: an unknown model must not outrank a
/// seeded family on the paid/free proxy alone.
// ponytail: flat cap; replace with measured quality once the scoreboard is family-keyed.
pub(super) const UNSEEDED_QUALITY_CAP: f64 = 0.65;
```

and replace the body of `quality_score` with:

```rust
pub(super) fn quality_score(m: &ModelSpec) -> f64 {
    if let Some(i) = crate::models::seed::seed_for_model(&m.id).and_then(|s| s.intelligence) {
        return (f64::from(i) / 100.0).clamp(0.0, 1.0);
    }
    let token_component = (m.max_tokens as f64).log10().clamp(1.0, 7.0) / 7.0;
    let paid_component = if m.is_free { QUALITY_FREE_PAID_COMPONENT } else { QUALITY_PAID_COMPONENT };
    ((token_component * QUALITY_TOKEN_WEIGHT) + (paid_component * QUALITY_PAID_WEIGHT))
        .clamp(0.0, UNSEEDED_QUALITY_CAP)
}
```

- [ ] **Step 6: Stamp the tier at registration.** In `registry.rs`, change `pub fn register(&mut self, spec: ModelSpec)` to `pub fn register(&mut self, mut spec: ModelSpec)` and make its first statement:

```rust
        if spec.capabilities.tier == super::ModelTier::Unknown {
            if let Some(seed) = super::seed::seed_for_model(&spec.id) {
                spec.capabilities.tier = seed.tier;
            }
        }
```

- [ ] **Step 7: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models:: catalog 2>&1 | tail -25`
Expected: all pass. If a pre-existing `scoring` or `models::tests` test now fails (e.g. it asserted the old uncapped paid/free quality), STOP and name it — do not edit it.

- [ ] **Step 8: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/seed.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/models/scoring.rs crates/vox-orchestrator/src/models/registry.rs crates/vox-orchestrator/src/models/tests.rs
git commit -m "feat(models): seed-backed quality scores and tiers instead of a paid/free proxy"
```

---

### Task 5: Selection drops superseded versions among eligible candidates

**Files:**
- Modify: `crates/vox-orchestrator/src/models/registry.rs` (`best_for_internal`)
- Test: `crates/vox-orchestrator/src/models/tests.rs` (new module at end)

**Interfaces:**
- Consumes: `family::{newest_per_family, is_superseded}` (Task 2).

- [ ] **Step 1: Write the failing tests** — append to `models/tests.rs`:

```rust
#[cfg(test)]
mod superseded_selection_tests {
    use crate::config::CostPreference;
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ProviderType, StrengthTag};
    use crate::types::TaskCategory;

    fn dated(id: &str, released_at: Option<u64>, cost: f64) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 8192,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Codegen, StrengthTag::Generalist],
            capabilities: ModelCapabilities { released_at, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    fn pick(r: &ModelRegistry, pred: impl FnMut(&ModelSpec) -> bool) -> Option<String> {
        r.best_for_with_filter(TaskCategory::CodeGen, 5, CostPreference::Economy, false, pred, None)
            .map(|m| m.id)
    }

    #[test]
    fn older_member_of_a_family_is_never_selected() {
        // <!-- AMENDED: R7 — 5.5 at cost 5.0 exceeded the 5.0 safety cap; 1.0 keeps it eligible -->
        let mut r = ModelRegistry::default();
        // The older member is cheaper, so it wins on cost today unless recency filters it.
        r.register(dated("acme/widget-4.8", Some(1_700_000_000), 0.1));
        r.register(dated("acme/widget-5.5", Some(1_760_000_000), 1.0));
        assert_eq!(pick(&r, |_| true), Some("acme/widget-5.5".to_string()));
    }

    #[test]
    fn a_filtered_out_newest_member_does_not_hide_the_family() {
        // <!-- AMENDED: R1 — recency is judged among eligible candidates only -->
        let mut r = ModelRegistry::default();
        r.register(dated("acme/widget-4.8", Some(1_700_000_000), 0.1));
        r.register(dated("acme/widget-5.5", Some(1_760_000_000), 1.0));
        assert_eq!(pick(&r, |m| m.id != "acme/widget-5.5"), Some("acme/widget-4.8".to_string()));
    }

    #[test]
    fn undated_specs_still_compete() {
        let mut r = ModelRegistry::default();
        r.register(dated("acme/widget-4.8", None, 0.1));
        assert_eq!(pick(&r, |_| true), Some("acme/widget-4.8".to_string()));
    }
}
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::tests::superseded_selection_tests > target/models-t5-red.txt 2>&1; tail -15 target/models-t5-red.txt`
Expected: `older_member_of_a_family_is_never_selected` FAILS (the cheaper 4.8 wins); the other two pass.

- [ ] **Step 3: Implement** — in `best_for_internal`, collect the filtered candidates, then drop superseded ones among them. Replace the opening

```rust
        self.models
            .values()
            .filter(|m| {
```

with

```rust
        let candidates: Vec<&ModelSpec> = self
            .models
            .values()
            .filter(|m| {
```

and replace the lines that close the filter and open the ranking

```rust
                Self::matches_strength(m, strength) && pred(m)
            })
            .max_by(|a, b| {
```

with

```rust
                Self::matches_strength(m, strength) && pred(m)
            })
            .collect();
        // ponytail: recomputed per call (O(candidates)); the filter above already loads config per model.
        let newest = super::family::newest_per_family(candidates.iter().copied());
        candidates
            .into_iter()
            .filter(|m| !super::family::is_superseded(m, &newest))
            .max_by(|a, b| {
```

Leave the rest of the chain (the `max_by` body and whatever follows it) unchanged.

- [ ] **Step 4: Run to verify pass, plus all model tests**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models:: 2>&1 | tail -20`
Expected: all pass.

- [ ] **Step 5: Mutation proof** — replace `.filter(|m| !super::family::is_superseded(m, &newest))` with `.filter(|_| true)`, run the Step 2 command into `target/models-mutant-superseded.txt`, confirm `older_member_of_a_family_is_never_selected ... FAILED`, restore the line, confirm `git diff` shows only the Step 3 edits.

- [ ] **Step 6: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/registry.rs crates/vox-orchestrator/src/models/tests.rs
git commit -m "fix(models): selection never picks a superseded version among eligible models"
```

---

### Task 6: Mode guarantees and the provider-key gate on the dispatch path

<!-- AMENDED: R6 — Task 6 and the old Task 7 merged: the key gate lives in the dispatch selector, not in best_for_internal, so existing key-gate tests (select.rs key_gate_*), cloud-spec registry tests and tests/economy_test.rs are untouched. -->

**Files:**
- Create: `crates/vox-orchestrator/src/models/mode_select.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs` (add `pub mod mode_select;`)
- Modify: `crates/vox-orchestrator/src/models/key_guard.rs` (append `selection_key_available`, its override and a test)
- Modify: `crates/vox-orchestrator/src/runtime.rs` (`resolve_task_cost_policy`; the `let routed = { … };` block in `AiTaskProcessor::process`; the three `resolve_task_cost_policy` tests)

**Interfaces:**
- Consumes: `ModelRegistry::best_for_task_with_filter` (existing); `crate::mode::ClutchProfile` (existing); seeded tiers (Task 4).
- Produces: `pub struct ModeSelection { pub spec: ModelSpec, pub only_candidate: bool }`; `impl ModelRegistry { pub fn best_for_task_in_mode(&self, task: &AgentTask, preference: CostPreference, clutch: ClutchProfile, pred: impl FnMut(&ModelSpec) -> bool) -> Option<ModeSelection> }`; `pub(crate) fn selection_key_available(ptype: &ProviderType) -> bool`; `#[cfg(any(test, feature = "test-support"))] pub fn set_test_key_availability(providers: Option<Vec<ProviderType>>)`; `resolve_task_cost_policy` returns `(CostPreference, bool, RiskPosture, ClutchProfile)`.

- [ ] **Step 1: Write the failing mode tests** — create `mode_select.rs` with tests only:

```rust
//! Dispatch-path selection: Efficiency and Balanced never pick an Elite
//! (flagship) model while a non-Elite candidate fits, and a provider without
//! a resolvable key is never picked.

#[cfg(test)]
mod tests {
    use crate::config::CostPreference;
    use crate::mode::ClutchProfile;
    use crate::models::key_guard::set_test_key_availability;
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType, StrengthTag};
    use crate::types::{AgentTask, TaskCategory, TaskId, TaskPriority};

    fn spec(id: &str, provider_type: ProviderType, tier: ModelTier, cost: f64) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 200_000,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Codegen, StrengthTag::Generalist],
            capabilities: ModelCapabilities { tier, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    fn hard_task() -> AgentTask {
        let mut t = AgentTask::new(TaskId(1), "hard codegen", TaskPriority::Normal, vec![]);
        t.task_category = TaskCategory::CodeGen;
        t.estimated_complexity = 10;
        t
    }

    /// <!-- AMENDED: R6 — the Elite model is the CHEAPEST with equal context, so without the
    /// guard it wins under Economy; the old fixture let the Fast model win regardless. -->
    fn registry() -> ModelRegistry {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/flagship-9", ProviderType::Ollama, ModelTier::Elite, 0.0005));
        r.register(spec("acme/workhorse-9", ProviderType::Ollama, ModelTier::Pro, 0.02));
        r.register(spec("acme/quick-9", ProviderType::Ollama, ModelTier::Fast, 0.02));
        r
    }

    #[test]
    fn efficiency_on_a_hard_task_never_picks_elite() {
        let pick = registry()
            .best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, |_| true)
            .expect("a candidate");
        assert_ne!(pick.spec.capabilities.tier, ModelTier::Elite, "picked {}", pick.spec.id);
        assert!(!pick.only_candidate);
    }

    #[test]
    fn balanced_never_picks_elite_when_alternatives_exist() {
        let pick = registry()
            .best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Balanced, |_| true)
            .expect("a candidate");
        assert_ne!(pick.spec.capabilities.tier, ModelTier::Elite);
    }

    #[test]
    fn genius_applies_no_tier_exclusion() {
        let pick = registry()
            .best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Genius, |_| true)
            .expect("a candidate");
        assert_eq!(pick.spec.id, "acme/flagship-9", "same inputs, no exclusion: the cheaper Elite wins");
    }

    #[test]
    fn efficiency_falls_back_to_elite_only_when_it_is_the_only_candidate() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/flagship-9", ProviderType::Ollama, ModelTier::Elite, 0.0005));
        let pick = r
            .best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, |_| true)
            .expect("must not return None when the only candidate is Elite");
        assert_eq!(pick.spec.id, "acme/flagship-9");
        assert!(pick.only_candidate);
    }

    #[test]
    fn caller_predicate_still_applies_in_both_passes() {
        let pick = registry()
            .best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, |m| {
                m.capabilities.tier == ModelTier::Elite
            })
            .expect("falls back to the Elite model the predicate allows");
        assert_eq!(pick.spec.id, "acme/flagship-9");
        assert!(pick.only_candidate);
    }

    #[test]
    fn a_provider_without_a_key_is_never_picked() {
        let mut r = ModelRegistry::default();
        // The keyless provider is far cheaper, so it would win without the gate.
        r.register(spec("acme/cheap-cloud", ProviderType::OpenRouter, ModelTier::Fast, 0.0001));
        r.register(spec("acme/keyed-model", ProviderType::DeepSeek, ModelTier::Fast, 0.02));
        set_test_key_availability(Some(vec![ProviderType::DeepSeek]));
        let pick = r.best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, |_| true);
        set_test_key_availability(None);
        assert_eq!(pick.map(|s| s.spec.id), Some("acme/keyed-model".to_string()));
    }

    #[test]
    fn with_no_cloud_keys_only_local_remains() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/cheap-cloud", ProviderType::OpenRouter, ModelTier::Fast, 0.0001));
        r.register(spec("acme/local", ProviderType::Ollama, ModelTier::Fast, 0.05));
        set_test_key_availability(Some(vec![]));
        let pick = r.best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, |_| true);
        set_test_key_availability(None);
        assert_eq!(pick.map(|s| s.spec.id), Some("acme/local".to_string()));
    }
}
```

Add `pub mod mode_select;` to `models/mod.rs`. If `AgentTask::task_category` is an `Option<TaskCategory>` or `estimated_complexity` is not a plain `u8` field on `AgentTask`, adapt only `hard_task()` and say so.

- [ ] **Step 2: Write the failing key-override test** — append inside `key_guard.rs`'s `mod avail_tests`:

```rust
    #[test]
    fn selection_key_override_restricts_and_resets() {
        set_test_key_availability(Some(vec![ProviderType::DeepSeek]));
        assert!(selection_key_available(&ProviderType::DeepSeek));
        assert!(!selection_key_available(&ProviderType::OpenRouter));
        assert!(selection_key_available(&ProviderType::Ollama), "local providers never need a key");
        set_test_key_availability(None);
    }
```

- [ ] **Step 3: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::mode_select models::key_guard > target/models-t6-red.txt 2>&1; tail -20 target/models-t6-red.txt`
Expected: FAIL to compile — `no method named best_for_task_in_mode`, `cannot find function set_test_key_availability`.

- [ ] **Step 4: Implement the key check** — append to `key_guard.rs` above `#[cfg(test)] mod avail_tests`:

```rust
// Test-only provider-key availability, per thread (selection is synchronous and runs on
// the calling thread), so tests that set it cannot leak into tests running in parallel.
// `None` means "use the real Clavis check", exactly like
// `route_policy::set_test_privacy_override`.
thread_local! {
    static TEST_KEY_OVERRIDE: std::cell::RefCell<Option<Vec<ProviderType>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "test-support"))]
pub fn set_test_key_availability(providers: Option<Vec<ProviderType>>) {
    TEST_KEY_OVERRIDE.with(|o| *o.borrow_mut() = providers);
}

/// Provider-key availability as seen by dispatch-path model selection.
#[must_use]
pub(crate) fn selection_key_available(ptype: &ProviderType) -> bool {
    if matches!(ptype, ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal) {
        return true;
    }
    TEST_KEY_OVERRIDE
        .with(|o| o.borrow().as_ref().map(|list| list.contains(ptype)))
        .unwrap_or_else(|| provider_secret_is_available(ptype))
}
```

<!-- AMENDED: R6 — thread-local, and None falls through to the real check (the old draft returned true for everything under cfg(test)/test-support, disabling key-gate assertions in other crates' test builds). -->

- [ ] **Step 5: Implement the selector** (above the tests in `mode_select.rs`):

```rust
use crate::config::CostPreference;
use crate::mode::ClutchProfile;
use crate::models::key_guard::selection_key_available;
use crate::models::{ModelRegistry, ModelSpec, ModelTier};
use crate::types::AgentTask;

/// A dispatch-path pick. `only_candidate` is true when the mode's preferred set
/// was empty and the pick came from the unrestricted fallback pass.
#[derive(Debug, Clone)]
pub struct ModeSelection {
    pub spec: ModelSpec,
    pub only_candidate: bool,
}

fn excludes_elite(clutch: ClutchProfile) -> bool {
    matches!(clutch, ClutchProfile::Efficiency | ClutchProfile::Balanced)
}

impl ModelRegistry {
    /// Task dispatch entry point: [`Self::best_for_task_with_filter`] plus the clutch's tier
    /// guarantee and the provider-key gate.
    pub fn best_for_task_in_mode(
        &self,
        task: &AgentTask,
        preference: CostPreference,
        clutch: ClutchProfile,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModeSelection> {
        let mut eligible = |m: &ModelSpec| selection_key_available(&m.provider_type) && pred(m);
        if excludes_elite(clutch) {
            if let Some(spec) =
                self.best_for_task_with_filter(task, preference, |m| m.capabilities.tier != ModelTier::Elite && eligible(m))
            {
                return Some(ModeSelection { spec, only_candidate: false });
            }
            return self
                .best_for_task_with_filter(task, preference, eligible)
                .map(|spec| ModeSelection { spec, only_candidate: true });
        }
        self.best_for_task_with_filter(task, preference, eligible)
            .map(|spec| ModeSelection { spec, only_candidate: false })
    }
}
```

<!-- AMENDED: R12 — best_for_in_mode removed (no production caller); tests drive the task-level entry point that dispatch uses. -->

- [ ] **Step 6: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::mode_select models::key_guard 2>&1 | tail -20`
Expected: all pass.

- [ ] **Step 7: Wire dispatch.** In `runtime.rs`, `resolve_task_cost_policy` returns the clutch that `resolve_task_policy` produced (it already falls back to its own default when nothing is configured, so there is one source of truth):

```rust
fn resolve_task_cost_policy(
    task: &crate::types::AgentTask,
    overrides: &crate::config::TaskPolicyOverrides,
    global_default: crate::config::CostPreference,
) -> (
    crate::config::CostPreference,
    bool,
    crate::mode::RiskPosture,
    crate::mode::ClutchProfile,
) {
    // ... existing body unchanged up to and including the `resolve_task_policy` call ...
    if task.clutch_profile.is_none() && category_clutch.is_none() && source_clutch.is_none() {
        return (global_default, false, risk, clutch);
    }
    let rc = clutch.resolve();
    (rc.cost_preference, rc.force_free_pool, risk, clutch)
}
```

<!-- AMENDED: R13 — return resolve_task_policy's clutch, not ClutchProfile::default(), so there is one default. -->

In `AiTaskProcessor::process`: destructure the fourth value (`let (mut cost_pref, force_free_pool, resolved_risk, clutch) =`); in both branches of the `let routed = { … };` block replace `registry.best_for_task_with_filter(&task, cost_pref, |m| {` with `registry.best_for_task_in_mode(&task, cost_pref, clutch, |m| {` (closure bodies unchanged); and change the block's closing `};` to:

```rust
        }
        .map(|sel| {
            if sel.only_candidate {
                tracing::info!(model = %sel.spec.id, ?clutch, "mode fallback: only candidate outside the mode's preferred tiers");
            }
            sel.spec
        });
```

so `routed` stays `Option<ModelSpec>` for all later uses. <!-- AMENDED: R15 — exact anchor: the block that ends before the "Code-review fix: `routed == None`" comment -->

Update the three existing `resolve_task_cost_policy` tests in `runtime.rs` (the only callers besides dispatch; `rg -n "resolve_task_cost_policy" crates/vox-orchestrator/src/runtime.rs`) to destructure four values — this is the one sanctioned edit to existing tests — and add one assertion to the first of them: an unconfigured task returns the same clutch `crate::mode::resolve_task_policy(None, None, None, None, None, None).0` returns.

- [ ] **Step 8: Run the dispatch and model tests**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- runtime models:: 2>&1 | tail -20 && timeout 1500s cargo test -p vox-orchestrator --tests 2>&1 | grep -E "test result|FAILED" | tail -10`
Expected: all pass, including integration tests such as `tests/economy_test.rs`. <!-- AMENDED: R14 — integration tests were never run -->

- [ ] **Step 9: Mutation proofs** — (a) change `matches!(clutch, ClutchProfile::Efficiency | ClutchProfile::Balanced)` to `false`, run `timeout 1500s cargo test -p vox-orchestrator --lib models::mode_select > target/models-mutant-elite.txt 2>&1`, confirm `efficiency_on_a_hard_task_never_picks_elite ... FAILED`, restore; (b) change `selection_key_available(&m.provider_type) && pred(m)` to `pred(m)`, run the same command into `target/models-mutant-keygate.txt`, confirm `a_provider_without_a_key_is_never_picked ... FAILED`, restore; confirm `git diff` shows only the intended edits.

- [ ] **Step 10: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/mode_select.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/models/key_guard.rs crates/vox-orchestrator/src/runtime.rs
git commit -m "fix(models): dispatch keeps Efficiency/Balanced off flagships and never picks a keyless provider"
```

---

### Task 7: (merged into Task 6)

<!-- AMENDED: R6 — the separate best_for_internal key gate was removed: it broke select.rs key_gate_* tests and tests/economy_test.rs, and a global override raced parallel tests. The gate now lives in the dispatch selector (Task 6). Direct best_for / explain / cheapest / best_free callers are listed under Open Decisions. -->

---

### Task 8: Council-pinned aliases — Owner: user (decision)

`contracts/orchestration/model-pins.v1.yaml` and the `premium_alias` block of `model-routing.v1.yaml` map task
categories to exact ids (`anthropic/claude-3-7-sonnet`, `google/gemini-2.0-flash`, `openai/o3-mini`). The pins file was
council-ratified on 2026-09-15 and changes need council sign-off plus an entry in
`docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md`. After Task 5, a pinned id that is
superseded in the live catalog is still returned by `select.rs#select_via_premium_alias` (it calls `registry.get(&alias)`
directly, not `best_for_internal`).

- [ ] **Decision:** approve changing pins from exact ids to family selectors resolved with `family::newest_per_family`
  at call time (the council keeps choosing the *family*; the version tracks the catalog), or keep exact pins and have
  `select_via_premium_alias` skip a pin when a newer member of its family exists. Claude writes the follow-up task once decided.

---

### Task 9: Verification sweep — Owner: Claude

- [ ] `timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5` and `timeout 1500s cargo test -p vox-orchestrator --tests 2>&1 | grep -E "test result|FAILED"` — all pass.
- [ ] `timeout 1500s cargo test -p vox-orchestrator-mcp --lib model_route_policy 2>&1 | tail -5` — the MCP key-gate tests still pass.
- [ ] `timeout 1500s cargo clippy -p vox-orchestrator -p vox-research-shim --all-targets -- -D warnings` — clean for touched files.
- [ ] `timeout 1500s cargo run -q -p vox-cli -- ci pre-push` (fast tier); regenerate and commit any inventory it names.
- [ ] Carry the Open Decisions and Deferred items below into the next plan's scope.

---

## Open Decisions

1. **Task 8 — council pins:** family selectors vs. skip-superseded pins (user + council).
2. **Risk "Low" with the default clutch:** `runtime.rs` forces `CostPreference::Performance` when the risk lean is Intelligence, but an unconfigured task still gets the Efficiency/Balanced Elite exclusion. Should a Low-risk task lift the exclusion? (user) <!-- AMENDED: R16 — surfaced by Tracks B and C -->
3. **Paths not yet covered by recency and the key gate** (next plan): the **chat lane** (`vox-orchestrator-mcp` `llm_bridge/model_route_policy/resolve.rs` resolves its clutch and never calls `best_for_task_in_mode` — highest priority, it is what the chat GUI shows); `registry.rs#explain_selection` (must match the real path); `best_free_for*`, `cheapest*` (never reach `best_for_internal`); the Thompson fallback in `registry_model_resolve.rs` that re-admits models after `best_for_with_filter` returns `None`; GUI `suggest_model_for_task`. <!-- AMENDED: R17 — "every path" was not met; now stated -->
4. **Research-shim scoring shift:** stamping seeded tiers on previously `Unknown` specs changes `vox-research-shim` tier-based scores (Free −0.8, Pro +1.0). Accept, or have the shim ignore seeded tiers? (user)
5. **Index the seed contract** in `contracts/index.yaml` alongside the other orchestration contracts. (Claude, next plan)

## Deferred Minor Issues

- `SeedScore.responsiveness` is loaded but not used yet (Spec Feature 5 partly covered); wire it with the family-keyed scoreboard.
- `seeds()` panics via `expect` on malformed JSON; the contract test catches it in CI.
- `ModeSelection` could be `(ModelSpec, bool)`; kept as a struct because the trace plan will add a reason field.
- `select.rs#select_via_scorer`'s own key predicate becomes redundant once the chat lane uses the mode selector; delete then.

## Execution Order

- **Sequential constraints:** Task 1 → Task 2 (`family.rs` reads `released_at`); Task 2 → Task 3 (keys generated with `family_key`); Task 3 → Task 4 (`include_str!` of the contract; placeholders filled); Task 4 → Task 5 (both modify `registry.rs` and `models/tests.rs`); Task 5 → Task 6 (dispatch relies on superseded filtering and seeded tiers); `models/mod.rs` is touched by Tasks 2, 4, 6 — sequential.
- **Phase 5 interplay:** no shared files with plans 05-03…05-07; drive this plan after the Phase 5 task in flight finishes (shared working tree and index).
- **Pre-flight:** working tree clean for the task's files; HEAD on `main` with Phase 5's latest commit; no schema change (baseline 93 untouched); VoxDb not needed.
- **Sequence:** 1 → 2 → 3 (Claude) → 4 → 5 → 6 → 9 (Claude); 8 whenever the user decides.
- **SDD ledger (copy into progress):**
  - R1 supersession computed over eligible candidates — ruling: settled
  - R2 `:free` is its own family — ruling: settled
  - R3 parameter sizes are part of the family — ruling: settled
  - R4 unseeded quality capped at 0.65 — ruling: settled
  - R5 seeded tier stamped in `register` when `Unknown` — ruling: settled
  - R6 key gate in the dispatch selector with a thread-local override; `best_for_internal` untouched for keys — ruling: settled
  - R7 Task 5 fixture cost within the safety cap — ruling: settled
  - `registry.rs` Tasks 4 → 5 — sequential — settled; `models/tests.rs` Tasks 4 → 5 — sequential — settled; `models/mod.rs` Tasks 2 → 4 → 6 — sequential — settled
