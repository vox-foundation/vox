# Model Routing: Latest Models, Honest Scores, Mode Guarantees — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task <this file> <N>`), per [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, reviews the diff and commits. Tasks marked
> **Owner: Claude** or **Owner: user** are not driven. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Every routing mode picks the newest member of the right model family, ranked by real quality evidence, and
the default Efficient mode can no longer pick a flagship (Opus-class) model when a cheaper candidate exists.

**Architecture:** Parse OpenRouter's `created` date into the model spec; derive a model *family* from each slug and
drop superseded versions at the one filter every selection path shares (`ModelRegistry::best_for_internal`); replace
the paid/free quality proxy with a dated, family-keyed seed contract; enforce each clutch mode's promise with a
two-pass mode-aware selector used by task dispatch; move the provider-key check into the shared filter.

**Tech Stack:** Rust (`vox-orchestrator`, `vox-research-shim`), serde/serde_json, JSON contracts under `contracts/orchestration/`.

**Spec:** [`docs/src/architecture/chat-surface-design-critique-2026-09-28.md`](../../src/architecture/chat-surface-design-critique-2026-09-28.md)
(priority recommendation 1) and the evidence section of
[`2026-09-28-chat-surface-trace-and-latest-models.md`](2026-09-28-chat-surface-trace-and-latest-models.md).

## Global Constraints

- All LLM selection stays inside `vox-orchestrator::models` (AGENTS.md §Model-Agnostic LLM Boundary). No new crate, no new crate edge, no new dependency.
- No versioned cloud model id may be introduced as a literal in non-test code. Local MENS revisions are exempt (AGENTS.md).
- `contracts/orchestration/model-pins.v1.yaml` is council-ratified (2026-09-15); this plan does **not** edit it (Task 8 is a user decision).
- New code goes in new files under `crates/vox-orchestrator/src/models/`; `registry.rs` (1,396 non-blank lines), `catalog.rs` and `runtime.rs` get only the minimal edits shown.
- Test-first for every new `pub fn` (lefthook `tdd-guard`). Format only the files you changed: `rustfmt --edition 2024 <file>`. Never `cargo fmt`.
- Every build/test command is foreground and prefixed with `timeout 1500s`. Scope cargo with `-p vox-orchestrator`.
- The agent never runs `git add` / `git commit`; each task's commit block is run by Claude Code after review.

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `crates/vox-orchestrator/src/models/spec.rs` | modify | `ModelCapabilities.released_at: Option<u64>` (the OpenRouter `created` unix time) |
| `crates/vox-research-shim/src/selection/virtual_models.rs` | modify | add `released_at: None` to its two literal `ModelCapabilities` |
| `crates/vox-orchestrator/src/catalog.rs` | modify | parse `created` / `canonical_slug`; extract `specs_from_openrouter_json`; stamp seed tier |
| `crates/vox-orchestrator/src/models/family.rs` | create | `family_key`, `version_tuple`, `newest_per_family`, `is_superseded` (pure) |
| `contracts/orchestration/model-seed-scores.v1.json` | create (Claude) | dated, sourced, family-keyed intelligence / responsiveness / tier |
| `crates/vox-orchestrator/src/models/seed.rs` | create | load the seed contract; `seed_for(family)`, `tier_for(slug)` |
| `crates/vox-orchestrator/src/models/scoring.rs` | modify | `quality_score` uses the seed when present |
| `crates/vox-orchestrator/src/models/registry.rs` | modify | two one-line gates in `best_for_internal` (superseded, provider key) |
| `crates/vox-orchestrator/src/models/key_guard.rs` | modify | test-only key-availability override (mirrors `route_policy::set_test_privacy_override`) |
| `crates/vox-orchestrator/src/models/mode_select.rs` | create | `ModeSelection`, `best_for_task_in_mode`, `best_for_in_mode` (two-pass, Elite excluded first for Efficiency/Balanced) |
| `crates/vox-orchestrator/src/runtime.rs` | modify | `resolve_task_cost_policy` also returns the clutch; dispatch calls the mode-aware selector |
| `crates/vox-orchestrator/src/models/mod.rs` | modify | `pub mod family; pub mod seed; pub mod mode_select;` |

---

### Task 1: Parse OpenRouter `created` and `canonical_slug`

**Files:**
- Modify: `crates/vox-orchestrator/src/models/spec.rs` (struct `ModelCapabilities`, after `uptime_score`)
- Modify: `crates/vox-research-shim/src/selection/virtual_models.rs` (the two `ModelCapabilities {` literals, ~:39 and ~:84)
- Modify: `crates/vox-orchestrator/src/catalog.rs:36-55` (`OpenRouterModelData`), `:116-227` (`refresh`)
- Test: `crates/vox-orchestrator/src/catalog.rs` (existing `mod tests`)

**Interfaces:**
- Produces: `ModelCapabilities::released_at: Option<u64>`; `pub(crate) fn specs_from_openrouter_json(json: &str) -> anyhow::Result<Vec<ModelSpec>>` in `catalog.rs`.

- [ ] **Step 1: Write the failing test** — append inside `mod tests` in `catalog.rs`:

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

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib catalog::tests::specs_from_openrouter_json 2>&1 | tail -20`
Expected: FAIL to compile — `cannot find function specs_from_openrouter_json` / `no field released_at`.

- [ ] **Step 3: Add the capability field.** In `spec.rs`, inside `pub struct ModelCapabilities`, directly after the `uptime_score` field:

```rust
    /// Unix time the provider published this model (OpenRouter `/models` `created`).
    /// `None` when the source does not report it; never defaulted to 0 or "now",
    /// because recency ordering (`models::family`) must not invent a date.
    #[serde(default)]
    pub released_at: Option<u64>,
```

Then run `rg -n "ModelCapabilities \{" crates --type rust` and, for every literal that does **not** end with `..Default::default()` or `..ModelCapabilities::default()`, add `released_at: None,`. Expected sites: two in `crates/vox-research-shim/src/selection/virtual_models.rs` and one in `crates/vox-orchestrator/src/models/spec.rs`. If more than these three need it, STOP and list them.

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

Inside the moved loop make exactly two edits: in the `ModelCapabilities { ... }` literal add `released_at: m.created,` before `..Default::default()`; and in the `ModelSpec { ... }` literal replace `canonical_slug: m.id.clone(),` with `canonical_slug: m.canonical_slug.clone().unwrap_or_else(|| m.id.clone()),`. Do not change anything else in the loop.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib catalog 2>&1 | tail -20 && timeout 1500s cargo check -p vox-research-shim 2>&1 | tail -3`
Expected: all `catalog::tests` pass (including the pre-existing `infer_strengths_*`); `vox-research-shim` checks clean.

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
- Consumes: `ModelSpec.canonical_slug`, `ModelCapabilities.released_at` (Task 1).
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
        assert_eq!(family_key("qwen/qwen3-coder:free"), "qwen/qwen-coder");
        assert_eq!(family_key("openai/gpt-5-mini"), "openai/gpt-mini");
        assert_eq!(family_key("google/gemini-3.1-flash-lite-preview"), "google/gemini-flash-lite");
        assert_eq!(family_key("google/gemma-4-31b-it"), "google/gemma-it");
        assert_eq!(family_key("Anthropic/Claude-Opus-5.5"), "anthropic/claude-opus");
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
    fn undated_specs_are_never_superseded_and_never_supersede() {
        let undated_old_name = dated("anthropic/claude-opus-4.8", None);
        let dated_new = dated("anthropic/claude-opus-5.5", Some(1_760_000_000));
        let newest = newest_per_family([&undated_old_name, &dated_new]);
        assert!(!is_superseded(&undated_old_name, &newest), "no date, no verdict");
        let only_undated = newest_per_family([&undated_old_name]);
        assert!(only_undated.is_empty());
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

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::family 2>&1 | tail -15`
Expected: FAIL to compile — `cannot find function family_key`.

- [ ] **Step 3: Implement** — insert above `#[cfg(test)]` in `family.rs`:

```rust
use std::collections::HashMap;

use crate::models::ModelSpec;

/// Qualifier tokens that mark a variant, not a different family.
const QUALIFIERS: &[&str] = &["preview", "beta", "exp", "experimental", "latest"];

/// Version-free family id: `org/name-words`, lowercased.
///
/// Per `-`-separated token of the name part (after `:variant` is removed):
/// pure numbers/dots (`4.6`, `20260101`) and `v4`-style markers are dropped;
/// size tokens (`31b`, `8b`, `a4b`) are dropped; qualifiers (`preview`, `beta`,
/// `exp`, `latest`) are dropped; mixed tokens keep only their letters
/// (`qwen3` → `qwen`, `k2.6` → `k`, `4o` → `o`).
// ponytail: heuristic slug parse; if providers publish an explicit family field, prefer it.
#[must_use]
pub fn family_key(slug: &str) -> String {
    let slug = slug.to_ascii_lowercase();
    let slug = slug.split(':').next().unwrap_or_default();
    let (org, name) = slug.split_once('/').unwrap_or(("", slug));
    let words: Vec<String> = name
        .split('-')
        .filter_map(|tok| {
            if tok.is_empty() || QUALIFIERS.contains(&tok) {
                return None;
            }
            let has_digit = tok.chars().any(|c| c.is_ascii_digit());
            if !has_digit {
                return Some(tok.to_string());
            }
            let is_version = tok.chars().all(|c| c.is_ascii_digit() || c == '.');
            let is_v_marker = tok.starts_with('v') && tok[1..].chars().all(|c| c.is_ascii_digit() || c == '.');
            let is_size = tok.ends_with('b')
                && tok[..tok.len() - 1].trim_start_matches('a').chars().all(|c| c.is_ascii_digit() || c == '.');
            if is_version || is_v_marker || is_size {
                return None;
            }
            let letters: String = tok.chars().filter(|c| c.is_ascii_alphabetic()).collect();
            (!letters.is_empty()).then_some(letters)
        })
        .collect();
    if org.is_empty() {
        words.join("-")
    } else {
        format!("{org}/{}", words.join("-"))
    }
}

/// Every run of digits in the slug, in order (`claude-opus-4.8` → `[4, 8]`).
#[must_use]
pub fn version_tuple(slug: &str) -> Vec<u32> {
    slug.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn slug_of(m: &ModelSpec) -> &str {
    if m.canonical_slug.is_empty() { &m.id } else { &m.canonical_slug }
}

/// Newest `(released_at, version)` per family, over dated specs only.
#[must_use]
pub fn newest_per_family<'a>(
    specs: impl IntoIterator<Item = &'a ModelSpec>,
) -> HashMap<String, (u64, Vec<u32>)> {
    let mut newest: HashMap<String, (u64, Vec<u32>)> = HashMap::new();
    for m in specs {
        let Some(at) = m.capabilities.released_at else { continue };
        let key = family_key(slug_of(m));
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

/// True when a newer dated member of the same family exists. Undated specs
/// (bootstrap, local MENS) are never superseded.
#[must_use]
pub fn is_superseded(m: &ModelSpec, newest: &HashMap<String, (u64, Vec<u32>)>) -> bool {
    let Some(at) = m.capabilities.released_at else { return false };
    newest
        .get(&family_key(slug_of(m)))
        .is_some_and(|best| *best > (at, version_tuple(&m.id)))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::family 2>&1 | tail -15`
Expected: 5 passed. If a `family_key` assertion fails, fix the function, not the assertion; if an assertion looks wrong, STOP and say which.

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
    { "family": "anthropic/claude-opus", "intelligence": 0, "responsiveness": 0, "tier": "elite" }
  ]
}
```

`family` uses Task 2's `family_key`; `intelligence` and `responsiveness` are 0–100 or `null`; `tier` is one of the
`ModelTier` snake_case names (`elite`, `pro`, `fast`, `light`, `free`, `local`, `unknown`). Every family present in
`model-catalog.bootstrap.v1.json` must appear (null scores allowed).

- [ ] **Step 1:** Research warp.dev's published agent-model defaults (per mode) and one machine-readable public quality/latency index whose terms permit automated use; record URLs and retrieval dates in `sources`.
- [ ] **Step 2:** Write the file. Flagship lines (Opus-class, GPT-*-pro, Gemini *-pro) are `elite`; strong mid lines `pro`; Flash/mini/Haiku/DeepSeek-flash-class `fast`; `:free` pools `free`.
- [ ] **Step 3:** `jq -e '.families | length > 0' contracts/orchestration/model-seed-scores.v1.json`
- [ ] **Step 4: Commit** `feat(models): dated family seed scores (warp.dev defaults + <index>)`, body citing sources.

---

### Task 4: Seed loader, honest quality, seeded tiers

**Files:**
- Create: `crates/vox-orchestrator/src/models/seed.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs` (add `pub mod seed;`)
- Modify: `crates/vox-orchestrator/src/models/scoring.rs:152-162` (`quality_score`)
- Modify: `crates/vox-orchestrator/src/catalog.rs` (in `specs_from_openrouter_json`, one line before `models.push`)

**Interfaces:**
- Consumes: `family::family_key` (Task 2); the contract (Task 3).
- Produces: `pub struct SeedScore { pub intelligence: Option<u8>, pub responsiveness: Option<u8>, pub tier: crate::models::ModelTier }`; `pub fn seed_for(family: &str) -> Option<&'static SeedScore>`; `pub fn seed_for_model(slug: &str) -> Option<&'static SeedScore>`.

- [ ] **Step 1: Write the failing tests** — create `seed.rs` with the test module only:

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
        let fam = seeds().keys().next().expect("at least one family").clone();
        assert!(seed_for(&fam).is_some());
        assert!(seed_for("nobody/no-such-family").is_none(), "unknown family is None, not a panic");
    }

    #[test]
    fn every_bootstrap_family_has_a_seed_row() {
        let raw = include_str!("../../../../contracts/orchestration/model-catalog.bootstrap.v1.json");
        let v: serde_json::Value = serde_json::from_str(raw).expect("bootstrap json");
        let mut missing = Vec::new();
        for m in v["models"].as_array().into_iter().flatten() {
            let Some(id) = m["id"].as_str() else { continue };
            if m["provider_type"].as_str().is_some_and(|p| p.eq_ignore_ascii_case("ollama") || p.eq_ignore_ascii_case("voxlocal")) {
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

Before writing the implementation, run `jq '.models[0] | keys' contracts/orchestration/model-catalog.bootstrap.v1.json`. If the top-level array is not `models` or the id field is not `id`, adjust only those two names in the test and say so in your final message.

- [ ] **Step 2: Run to verify failure**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::seed 2>&1 | tail -15`
Expected: FAIL to compile — `cannot find function seeds`.

- [ ] **Step 3: Implement** (above the tests):

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
pub fn seed_for_model(slug: &str) -> Option<&'static SeedScore> {
    seed_for(&crate::models::family::family_key(slug))
}
```

Add `pub mod seed;` to `models/mod.rs`.

- [ ] **Step 4: Honest `quality_score`** — add this test at the end of `scoring.rs`'s test module (find it with `rg -n "mod tests" crates/vox-orchestrator/src/models/scoring.rs`), using that module's existing spec helper (`make_spec`) — set `id`/`canonical_slug` on the returned spec to a family present in the seed contract with a non-null intelligence (pick two rows of different intelligence with `jq`):

```rust
    #[test]
    fn quality_score_uses_seed_intelligence_when_present() {
        let mut hi = make_spec(ProviderType::OpenRouter, 1.0, false);
        hi.id = "<high-intelligence family slug>".into();
        hi.canonical_slug = hi.id.clone();
        let mut lo = make_spec(ProviderType::OpenRouter, 1.0, false);
        lo.id = "<lower-intelligence family slug>".into();
        lo.canonical_slug = lo.id.clone();
        assert!(quality_score(&hi) > quality_score(&lo), "paid models must no longer score equal");
        let mut unknown = make_spec(ProviderType::OpenRouter, 1.0, false);
        unknown.id = "nobody/no-such-family".into();
        unknown.canonical_slug = unknown.id.clone();
        assert!(quality_score(&unknown) > 0.0, "no seed falls back to the old proxy, never panics");
    }
```

Replace the two `<…>` placeholders with the literal family slugs you picked (e.g. `"anthropic/claude-opus"` vs `"openai/gpt-mini"`) — the test must contain real strings when you run it. Then replace the body of `quality_score` with:

```rust
pub(super) fn quality_score(m: &ModelSpec) -> f64 {
    let slug = if m.canonical_slug.is_empty() { &m.id } else { &m.canonical_slug };
    if let Some(i) = crate::models::seed::seed_for_model(slug).and_then(|s| s.intelligence) {
        return (f64::from(i) / 100.0).clamp(0.0, 1.0);
    }
    let token_component = (m.max_tokens as f64).log10().clamp(1.0, 7.0) / 7.0;
    let paid_component = if m.is_free { QUALITY_FREE_PAID_COMPONENT } else { QUALITY_PAID_COMPONENT };
    ((token_component * QUALITY_TOKEN_WEIGHT) + (paid_component * QUALITY_PAID_WEIGHT)).clamp(0.0, 1.0)
}
```

- [ ] **Step 5: Seeded tier on catalog specs** — in `catalog.rs` `specs_from_openrouter_json`, directly before `models.push(ModelSpec {`, add:

```rust
            if let Some(seed) = crate::models::seed::seed_for_model(&m.id) {
                capabilities.tier = seed.tier;
            }
```

and add to `catalog.rs`'s tests (the JSON uses a family that must exist in the contract; replace the slug with one whose contract tier is `elite`):

```rust
    #[test]
    fn catalog_specs_take_their_tier_from_the_seed_contract() {
        let json = r#"{"data":[{"id":"<elite family slug>-9.9","created":1800000000,
            "pricing":{"prompt":"0.00001","completion":"0.00005"},"context_length":200000}]}"#;
        let specs = specs_from_openrouter_json(json).unwrap();
        assert_eq!(specs[0].capabilities.tier, crate::models::ModelTier::Elite);
    }
```

(Replace `<elite family slug>` with the literal family, e.g. `anthropic/claude-opus`, before running.)

- [ ] **Step 6: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::seed models::scoring catalog 2>&1 | tail -25`
Expected: all pass. If a pre-existing `scoring` test now fails because it asserted the old paid/free quality, STOP and name it — do not edit it.

- [ ] **Step 7: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/seed.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/models/scoring.rs crates/vox-orchestrator/src/catalog.rs
git commit -m "feat(models): seed-backed quality scores and tiers instead of a paid/free proxy"
```

---

### Task 5: Every selection path skips superseded versions

**Files:**
- Modify: `crates/vox-orchestrator/src/models/registry.rs:871-940` (`best_for_internal`)
- Test: `crates/vox-orchestrator/src/models/tests.rs` (new module at end)

**Interfaces:**
- Consumes: `family::{newest_per_family, is_superseded}` (Task 2).

- [ ] **Step 1: Write the failing test** — append to `models/tests.rs`:

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

    #[test]
    fn older_member_of_a_family_is_never_selected() {
        let mut r = ModelRegistry::default();
        // The older member is made cheaper so it would win on cost if it were not filtered.
        r.register(dated("acme/widget-4.8", Some(1_700_000_000), 0.1));
        r.register(dated("acme/widget-5.5", Some(1_760_000_000), 5.0));
        let pick = r
            .best_for_with_filter(TaskCategory::CodeGen, 5, CostPreference::Economy, false, |_| true, None)
            .expect("a candidate");
        assert_eq!(pick.id, "acme/widget-5.5");
    }

    #[test]
    fn undated_specs_still_compete() {
        let mut r = ModelRegistry::default();
        r.register(dated("acme/widget-4.8", None, 0.1));
        let pick = r.best_for_with_filter(TaskCategory::CodeGen, 5, CostPreference::Economy, false, |_| true, None);
        assert_eq!(pick.map(|m| m.id), Some("acme/widget-4.8".to_string()));
    }
}
```

`ProviderType::Ollama` is used so the provider-key gate (Task 7) does not interfere. If `TaskCategory::CodeGen` is not the variant name, use `rg -n "enum TaskCategory" -A12 crates/vox-orchestrator-types/src` to find the codegen variant and use it.

- [ ] **Step 2: Run to verify failure**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::tests::superseded_selection_tests 2>&1 | tail -15`
Expected: `older_member_of_a_family_is_never_selected` FAILS (the cheaper 4.8 wins); `undated_specs_still_compete` passes.

- [ ] **Step 3: Implement** — in `best_for_internal`, add as the first statement of the function body:

```rust
        // ponytail: recomputed per call (O(models)); cache on registry mutation if selection gets hot.
        let newest = super::family::newest_per_family(self.models.values());
```

and inside `.filter(|m| { ... })`, as its first check:

```rust
                if super::family::is_superseded(m, &newest) {
                    return false;
                }
```

- [ ] **Step 4: Run to verify pass, plus the existing model tests**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models:: 2>&1 | tail -20`
Expected: all pass.

- [ ] **Step 5: Mutation proof** — comment out the three `is_superseded` lines, re-run the Step 2 command into `target/models-mutant-superseded.txt`, confirm `older_member_of_a_family_is_never_selected ... FAILED`, restore the lines, confirm `git diff` shows only the Step 3 additions.

- [ ] **Step 6: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/registry.rs crates/vox-orchestrator/src/models/tests.rs
git commit -m "fix(models): selection never picks a superseded version of a model family"
```

---

### Task 6: Mode guarantees on the dispatch path

**Files:**
- Create: `crates/vox-orchestrator/src/models/mode_select.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs` (add `pub mod mode_select;`)
- Modify: `crates/vox-orchestrator/src/runtime.rs:66-94` (`resolve_task_cost_policy`), `:714-715`, `:744`, `:756`, and the second caller near `:2173`

**Interfaces:**
- Consumes: `ModelRegistry::best_for_task_with_filter` / `best_for_with_filter` (existing); `crate::mode::ClutchProfile` (existing); seeded `capabilities.tier` (Task 4).
- Produces: `pub struct ModeSelection { pub spec: ModelSpec, pub only_candidate: bool }`; `impl ModelRegistry { pub fn best_for_task_in_mode(&self, task: &AgentTask, preference: CostPreference, clutch: ClutchProfile, pred: impl FnMut(&ModelSpec) -> bool) -> Option<ModeSelection>; pub fn best_for_in_mode(&self, task_type: TaskCategory, complexity: u8, preference: CostPreference, clutch: ClutchProfile, pred: impl FnMut(&ModelSpec) -> bool) -> Option<ModeSelection> }`; `resolve_task_cost_policy` returns `(CostPreference, bool, RiskPosture, ClutchProfile)`.

- [ ] **Step 1: Write the failing tests** — create `mode_select.rs` with tests only:

```rust
//! Mode guarantees: Efficiency and Balanced never pick an Elite (flagship)
//! model while any non-Elite candidate passes every other filter.

#[cfg(test)]
mod tests {
    use crate::config::CostPreference;
    use crate::mode::ClutchProfile;
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType, StrengthTag};
    use crate::types::TaskCategory;

    fn tiered(id: &str, tier: ModelTier, cost: f64, max_tokens: u64) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: ProviderType::Ollama,
            max_tokens,
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

    fn registry() -> ModelRegistry {
        let mut r = ModelRegistry::default();
        // The flagship has the biggest context, which is what wins high-complexity scoring today.
        r.register(tiered("acme/flagship-9", ModelTier::Elite, 0.02, 1_000_000));
        r.register(tiered("acme/workhorse-9", ModelTier::Pro, 0.003, 200_000));
        r.register(tiered("acme/quick-9", ModelTier::Fast, 0.0005, 128_000));
        r
    }

    #[test]
    fn efficiency_on_a_hard_task_never_picks_elite() {
        let pick = registry()
            .best_for_in_mode(TaskCategory::CodeGen, 10, CostPreference::Economy, ClutchProfile::Efficiency, |_| true)
            .expect("a candidate");
        assert_ne!(pick.spec.capabilities.tier, ModelTier::Elite, "picked {}", pick.spec.id);
        assert!(!pick.only_candidate);
    }

    #[test]
    fn balanced_never_picks_elite_when_alternatives_exist() {
        let pick = registry()
            .best_for_in_mode(TaskCategory::CodeGen, 10, CostPreference::Economy, ClutchProfile::Balanced, |_| true)
            .expect("a candidate");
        assert_ne!(pick.spec.capabilities.tier, ModelTier::Elite);
    }

    #[test]
    fn efficiency_falls_back_to_elite_only_when_it_is_the_only_candidate() {
        let mut r = ModelRegistry::default();
        r.register(tiered("acme/flagship-9", ModelTier::Elite, 0.02, 1_000_000));
        let pick = r
            .best_for_in_mode(TaskCategory::CodeGen, 10, CostPreference::Economy, ClutchProfile::Efficiency, |_| true)
            .expect("must not return None when the only candidate is Elite");
        assert_eq!(pick.spec.id, "acme/flagship-9");
        assert!(pick.only_candidate);
    }

    #[test]
    fn genius_may_pick_elite() {
        let pick = registry()
            .best_for_in_mode(TaskCategory::CodeGen, 10, CostPreference::Performance, ClutchProfile::Genius, |_| true)
            .expect("a candidate");
        assert!(!pick.only_candidate);
        // Genius applies no tier exclusion; which model wins is the scorer's call.
    }

    #[test]
    fn caller_predicate_still_applies_in_both_passes() {
        let pick = registry().best_for_in_mode(
            TaskCategory::CodeGen, 10, CostPreference::Economy, ClutchProfile::Efficiency,
            |m| m.id != "acme/workhorse-9" && m.id != "acme/quick-9",
        );
        let pick = pick.expect("falls back to the Elite model the predicate allows");
        assert_eq!(pick.spec.id, "acme/flagship-9");
        assert!(pick.only_candidate);
    }
}
```

Add `pub mod mode_select;` to `models/mod.rs`. (Use the same `TaskCategory` codegen variant name you found in Task 5.)

- [ ] **Step 2: Run to verify failure**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::mode_select 2>&1 | tail -15`
Expected: FAIL to compile — `no method named best_for_in_mode`.

- [ ] **Step 3: Implement** (above the tests in `mode_select.rs`):

```rust
use crate::config::CostPreference;
use crate::mode::ClutchProfile;
use crate::models::{ModelRegistry, ModelSpec, ModelTier};
use crate::types::{AgentTask, TaskCategory};

/// A mode-aware pick. `only_candidate` is true when the mode's preferred set
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
    /// Task-level entry point used by dispatch; mirrors [`Self::best_for_task_with_filter`].
    pub fn best_for_task_in_mode(
        &self,
        task: &AgentTask,
        preference: CostPreference,
        clutch: ClutchProfile,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModeSelection> {
        if excludes_elite(clutch) {
            if let Some(spec) = self.best_for_task_with_filter(task, preference, |m| {
                m.capabilities.tier != ModelTier::Elite && pred(m)
            }) {
                return Some(ModeSelection { spec, only_candidate: false });
            }
            return self
                .best_for_task_with_filter(task, preference, pred)
                .map(|spec| ModeSelection { spec, only_candidate: true });
        }
        self.best_for_task_with_filter(task, preference, pred)
            .map(|spec| ModeSelection { spec, only_candidate: false })
    }

    /// Category/complexity entry point (tests, non-task callers).
    pub fn best_for_in_mode(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
        clutch: ClutchProfile,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModeSelection> {
        if excludes_elite(clutch) {
            if let Some(spec) = self.best_for_with_filter(task_type, complexity, preference, false, |m| {
                m.capabilities.tier != ModelTier::Elite && pred(m)
            }, None) {
                return Some(ModeSelection { spec, only_candidate: false });
            }
            return self
                .best_for_with_filter(task_type, complexity, preference, false, pred, None)
                .map(|spec| ModeSelection { spec, only_candidate: true });
        }
        self.best_for_with_filter(task_type, complexity, preference, false, pred, None)
            .map(|spec| ModeSelection { spec, only_candidate: false })
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::mode_select 2>&1 | tail -15`
Expected: 5 passed. If `efficiency_on_a_hard_task_never_picks_elite` fails before Step 3 (the RED run), that is the bug this task fixes — record its failure line in your final message.

- [ ] **Step 5: Wire dispatch.** In `runtime.rs` change `resolve_task_cost_policy` to also return the clutch that applies (the default clutch when none is configured, so the default Efficiency guarantee also covers unconfigured tasks):

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
        return (global_default, false, risk, crate::mode::ClutchProfile::default());
    }
    let rc = clutch.resolve();
    (rc.cost_preference, rc.force_free_pool, risk, clutch)
}
```

At `:714` destructure the fourth value: `let (mut cost_pref, force_free_pool, resolved_risk, clutch) =`. At `:744` and `:756` replace `registry.best_for_task_with_filter(&task, cost_pref, |m| {` with `registry.best_for_task_in_mode(&task, cost_pref, clutch, |m| {` (the closure bodies stay identical). The result type changes from `Option<ModelSpec>` to `Option<ModeSelection>`: at the point `routed` is consumed (read the next ~40 lines after `:776`), map it back with `.map(|sel| { if sel.only_candidate { tracing::info!(model = %sel.spec.id, ?clutch, "mode fallback: only candidate outside the mode's preferred tiers"); } sel.spec })` so downstream code is unchanged. At the second caller near `:2173` add `_` for the new tuple element. Then update `resolve_task_cost_policy`'s existing unit tests (`rg -n "resolve_task_cost_policy" crates/vox-orchestrator/src/runtime.rs`) to destructure four values; add one assertion to one of them: an unconfigured task returns `ClutchProfile::Efficiency`.

- [ ] **Step 6: Run the dispatch tests**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib runtime models::mode_select 2>&1 | tail -20`
Expected: all pass.

- [ ] **Step 7: Mutation proof** — in `mode_select.rs` change `matches!(clutch, ClutchProfile::Efficiency | ClutchProfile::Balanced)` to `false`, run the Step 4 command into `target/models-mutant-elite.txt`, confirm `efficiency_on_a_hard_task_never_picks_elite ... FAILED`, restore, confirm with `git diff`.

- [ ] **Step 8: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/mode_select.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/runtime.rs
git commit -m "fix(models): Efficiency and Balanced never pick a flagship model when a cheaper one fits"
```

---

### Task 7: The provider-key check lives in the shared filter

**Files:**
- Modify: `crates/vox-orchestrator/src/models/key_guard.rs` (test-only override)
- Modify: `crates/vox-orchestrator/src/models/registry.rs:370-372` (`key_is_present_for`) and `best_for_internal`'s filter
- Test: `crates/vox-orchestrator/src/models/tests.rs` (new module)

**Interfaces:**
- Produces: `#[cfg(any(test, feature = "test-support"))] pub fn set_test_key_availability(providers: Option<Vec<ProviderType>>)` in `key_guard.rs`.

- [ ] **Step 1: Write the failing test** — append to `models/tests.rs`:

```rust
#[cfg(test)]
mod key_gate_tests {
    use crate::config::CostPreference;
    use crate::models::key_guard::set_test_key_availability;
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ProviderType, StrengthTag};
    use crate::types::TaskCategory;
    use serial_test::file_serial;

    fn on(id: &str, provider_type: ProviderType, cost: f64) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 8192,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Codegen, StrengthTag::Generalist],
            capabilities: ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    #[test]
    #[file_serial]
    fn direct_best_for_never_returns_a_keyless_provider() {
        set_test_key_availability(Some(vec![ProviderType::DeepSeek]));
        let mut r = ModelRegistry::default();
        r.register(on("openrouter/cheap", ProviderType::OpenRouter, 0.0001));
        r.register(on("deepseek/deepseek-v4-flash", ProviderType::DeepSeek, 0.001));
        let pick = r.best_for(TaskCategory::CodeGen, 5, CostPreference::Economy);
        set_test_key_availability(None);
        assert_eq!(pick.map(|m| m.id), Some("deepseek/deepseek-v4-flash".to_string()));
    }

    #[test]
    #[file_serial]
    fn no_cloud_keys_leaves_only_local() {
        set_test_key_availability(Some(vec![]));
        let mut r = ModelRegistry::default();
        r.register(on("openrouter/cheap", ProviderType::OpenRouter, 0.0001));
        r.register(on("local/qwen", ProviderType::Ollama, 0.0));
        let pick = r.best_for(TaskCategory::CodeGen, 5, CostPreference::Economy);
        set_test_key_availability(None);
        assert_eq!(pick.map(|m| m.id), Some("local/qwen".to_string()));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::tests::key_gate_tests 2>&1 | tail -15`
Expected: FAIL to compile — `cannot find function set_test_key_availability`.

- [ ] **Step 3: Implement the override** — append to `key_guard.rs` above its `#[cfg(test)] mod avail_tests`:

```rust
/// Test-only provider-key availability. `None` (the default) means every
/// provider counts as keyed, so registry tests that build cloud specs without
/// real secrets keep working; `Some(list)` restricts to exactly `list`.
/// Mirrors `route_policy::set_test_privacy_override`.
static TEST_KEY_OVERRIDE: std::sync::Mutex<Option<Vec<ProviderType>>> = std::sync::Mutex::new(None);

#[cfg(any(test, feature = "test-support"))]
pub fn set_test_key_availability(providers: Option<Vec<ProviderType>>) {
    *TEST_KEY_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) = providers;
}

/// Key availability as seen by model selection.
#[must_use]
pub(crate) fn selection_key_available(ptype: &ProviderType) -> bool {
    if cfg!(any(test, feature = "test-support")) {
        return match &*TEST_KEY_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) {
            Some(list) => list.contains(ptype) || matches!(ptype, ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal),
            None => true,
        };
    }
    provider_secret_is_available(ptype)
}
```

In `registry.rs` change `key_is_present_for` to call `super::key_guard::selection_key_available(&m.provider_type)`, and in `best_for_internal`'s filter add, directly after the `is_superseded` check from Task 5:

```rust
                if !Self::key_is_present_for(m) {
                    return false;
                }
```

- [ ] **Step 4: Run the new and the existing model tests**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models:: 2>&1 | tail -25`
Expected: all pass. If an existing test fails because it relied on a keyless cloud spec being excluded (or included), STOP and list the test names — do not edit them.

- [ ] **Step 5: Mutation proof** — remove the three gate lines from `best_for_internal`, run the Step 2 command into `target/models-mutant-keygate.txt`, confirm `direct_best_for_never_returns_a_keyless_provider ... FAILED`, restore, `git diff` check.

- [ ] **Step 6: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/key_guard.rs crates/vox-orchestrator/src/models/registry.rs crates/vox-orchestrator/src/models/tests.rs
git commit -m "fix(models): every selection path checks the provider key, not only callers' filters"
```

---

### Task 8: Council-pinned aliases — Owner: user (decision)

`contracts/orchestration/model-pins.v1.yaml` and the `premium_alias` block of `model-routing.v1.yaml` map task
categories to exact ids (`anthropic/claude-3-7-sonnet`, `google/gemini-2.0-flash`, `openai/o3-mini`). The pins file was
council-ratified on 2026-09-15 and changes need council sign-off plus an entry in
`docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md`. After Task 5, a pinned id that is
superseded in the live catalog is still returned by `select_via_premium_alias` (it calls `registry.get(&alias)`
directly, not `best_for_internal`).

- [ ] **Decision:** approve changing pins from exact ids to family selectors resolved with `family::newest_per_family`
  at call time (the council keeps choosing the *family*; the version tracks the catalog), or keep exact pins and have
  `select_via_premium_alias` skip a pin when `family::is_superseded` says a newer member exists. Claude writes the
  follow-up task once decided.

---

### Task 9: Verification sweep — Owner: Claude

- [ ] `timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5` — all pass.
- [ ] `timeout 1500s cargo clippy -p vox-orchestrator -p vox-research-shim --all-targets -- -D warnings` — clean for touched files.
- [ ] `timeout 1500s cargo run -q -p vox-cli -- ci pre-push` (fast tier), regenerate and commit any inventory it names.
- [ ] Record in the plan index which follow-ups (literal defaults in `vox-config`, id-pinned tests, the research engine's premium path, family-keyed scoreboard + bandit wiring, the offline catalog refresh) move to the next plan.

---

## Self-review (2026-09-28)

- **Spec coverage:** latest-in-family (Tasks 1, 2, 5), honest quality from a dated seed seeded by warp.dev defaults (Tasks 3, 4), Efficient never flagship (Task 6), key-aware branching on every path (Task 7), council pins (Task 8, user). Deferred to the next plan, listed in Task 9: vox-config literal defaults, id-pinned tests and the routing drift gate, research-engine unification, family-keyed scoreboard + bandit wiring, offline catalog refresh, and the GUI/`routing_decision` surfacing (trace plan).
- **Placeholder scan:** the only `<…>` markers are in Task 4 Steps 4–5 and name contract slugs that do not exist until Task 3; the step instructs replacing them with literals before running.
- **Type consistency:** `released_at: Option<u64>` (Tasks 1, 2, 5); `newest_per_family` → `HashMap<String, (u64, Vec<u32>)>` used by `is_superseded` (Tasks 2, 5); `ModeSelection { spec, only_candidate }` (Task 6); `selection_key_available` / `set_test_key_availability` (Task 7); `ClutchProfile::{Efficiency, Balanced, Genius, Free}` and `ModelTier::Elite` match the existing enums.
