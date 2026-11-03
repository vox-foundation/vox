# Model Routing: Latest Models, Live Scores, Mode Guarantees — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task <this file> <N>`), per [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, reviews the diff and commits. Tasks marked
> **Owner: Claude** are not driven. Steps use checkbox (`- [ ]`) syntax.
>
> **Revision 2 (2026-09-28).** Rewritten after (a) a three-track review of revision 1 (see
> `.superpowers/review/2026-09-28-model-routing-latest-and-honest-*.md`) and (b) inspecting OpenRouter's live
> `/api/v1/models` (460 models). The live catalog already publishes what revision 1 planned to hand-curate:
> `benchmarks.artificial_analysis.intelligence_index` (98 of the 387 non-batch models), `created`, `expiration_date`,
> and 18 `~vendor/family-latest` aliases with `alias_target`. The static seed contract is therefore **dropped**;
> quality and recency come from the live catalog, tier from price. Amendments from revision 1's review keep their
> `<!-- AMENDED: R<n> -->` markers.

**Goal:** On the task-dispatch path, every clutch mode picks the newest member of the right model family, ranked by
live benchmark evidence, and the default Efficient mode can no longer pick a flagship (Opus-class) model when a
cheaper candidate fits.

**Architecture:** Read `created`, `expiration_date` and the Artificial Analysis intelligence index from OpenRouter's
catalog (skipping `:batch` and `~alias` entries and expired models); derive a model *family* from each id and drop
superseded versions among the candidates that already passed every filter in `ModelRegistry::best_for_internal`;
score quality from the live index (proxy for unbenchmarked models is scaled down) and derive the flagship tier from
completion price; enforce each clutch mode's promise and the provider-key check in a mode-aware selector used by
task dispatch.

**Tech Stack:** Rust (`vox-orchestrator`, `vox-research-shim`), serde/serde_json.

**Spec:** [`docs/src/architecture/chat-surface-design-critique-2026-09-28.md`](../../src/architecture/chat-surface-design-critique-2026-09-28.md)
(priority recommendation 1) and the evidence section of
[`2026-09-28-chat-surface-trace-and-latest-models.md`](2026-09-28-chat-surface-trace-and-latest-models.md).
Warp's published Auto modes (docs.warp.dev, updated 2026-09-24: Responsive [default], Cost-efficient, Genius,
Open-weights) are the reference vocabulary: Efficient ≈ Cost-efficient, Genius ≈ Genius, Free ≈ Open-weights. Warp
publishes no per-mode default model, so no default is copied from it.

## Global Constraints

- All LLM selection stays inside `vox-orchestrator::models` (AGENTS.md §Model-Agnostic LLM Boundary). No new crate, no new crate edge, no new dependency.
- No versioned cloud model id may be introduced as a literal in non-test code. Local MENS revisions are exempt (AGENTS.md).
- `contracts/orchestration/model-pins.v1.yaml` is council-ratified (2026-09-15); this plan does **not** edit it. A pinned alias that is superseded in the live catalog is skipped instead (Task 6).
- New code goes in new files under `crates/vox-orchestrator/src/models/`; `registry.rs`, `catalog.rs`, `select.rs`, `runtime.rs` get only the edits shown.
- Test-first for every new `pub fn` (lefthook `tdd-guard`, which wants a test in the same file). Record the RED output **before** touching implementation code. Format only changed files: `rustfmt --edition 2024 <file>`. Never `cargo fmt`.
- Every build/test command is foreground and prefixed with `timeout 1500s`. Cargo takes one test filter before `--`; pass several after `--` (e.g. `cargo test -p vox-orchestrator --lib -- a b c`).
- Family and quality lookups key on `ModelSpec.id`; `canonical_slug` is parsed and stored but not used for family identity.
- Existing tests are not edited except the three `resolve_task_cost_policy` tests in `runtime.rs` (mechanical four-value destructure, Task 5). Any other existing test that breaks is a STOP: list its name and the assertion.
- The agent never runs `git add` / `git commit`; each task's commit block is run by Claude Code after review.

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `crates/vox-orchestrator/src/models/spec.rs` | modify | `ModelCapabilities.released_at`, `.intelligence_index` |
| `crates/vox-research-shim/src/selection/virtual_models.rs` | modify | add the two new fields (`None`) to its two literal `ModelCapabilities` |
| `crates/vox-orchestrator/src/catalog.rs` | modify | parse `created`, `canonical_slug`, `expiration_date`, `benchmarks`; skip `:batch`, `~alias`, expired; extract `specs_from_openrouter_json[_at]` |
| `crates/vox-orchestrator/src/models/family.rs` | create | `family_key`, `version_tuple`, `newest_per_family`, `is_superseded` (pure) |
| `crates/vox-orchestrator/src/models/tiering.rs` | create | `derive_tier(is_free, cost_per_1k_output)` |
| `crates/vox-orchestrator/src/models/scoring.rs` | modify | `quality_score` from the live index; unbenchmarked proxy scaled down |
| `crates/vox-orchestrator/src/models/registry.rs` | modify | `register` derives tier when `Unknown`; `best_for_internal` drops superseded candidates |
| `crates/vox-orchestrator/src/models/key_guard.rs` | modify | `selection_key_available` with a thread-local test override |
| `crates/vox-orchestrator/src/models/mode_select.rs` | create | `ModeSelection`, `best_for_task_in_mode` (Elite excluded first for Efficiency/Balanced; key gate) |
| `crates/vox-orchestrator/src/runtime.rs` | modify | `resolve_task_cost_policy` also returns the clutch; dispatch calls `best_for_task_in_mode` |
| `crates/vox-orchestrator/src/models/select.rs` | modify | `select_via_premium_alias` skips a superseded pin |
| `crates/vox-orchestrator/src/models/mod.rs` | modify | `pub mod family; pub mod tiering; pub mod mode_select;` |

---

### Task 1: Read recency, benchmarks and expiry from OpenRouter; skip batch, alias and expired entries

**Files:**
- Modify: `crates/vox-orchestrator/src/models/spec.rs` (struct `ModelCapabilities`, after `uptime_score`)
- Modify: `crates/vox-research-shim/src/selection/virtual_models.rs` (the two `ModelCapabilities {` literals)
- Modify: `crates/vox-orchestrator/src/catalog.rs` (`OpenRouterModelData`, `OpenRouterCatalog::refresh`)
- Test: `crates/vox-orchestrator/src/catalog.rs` (existing `mod tests`)

**Interfaces:**
- Produces: `ModelCapabilities::released_at: Option<u64>`, `ModelCapabilities::intelligence_index: Option<f32>`; `pub(crate) fn specs_from_openrouter_json_at(json: &str, today: &str) -> anyhow::Result<Vec<ModelSpec>>` and `pub(crate) fn specs_from_openrouter_json(json: &str) -> anyhow::Result<Vec<ModelSpec>>` (calls the former with today's UTC date `YYYY-MM-DD`); private `fn utc_date_iso(unix_secs: u64) -> String`.

- [ ] **Step 1: Write the failing tests** — append inside `mod tests` in `catalog.rs`:

```rust
    const SAMPLE_MODELS_JSON: &str = r#"{"data":[
      {"id":"anthropic/claude-sonnet-4.6","canonical_slug":"anthropic/claude-4.6-sonnet-20260101","created":1767225600,
       "pricing":{"prompt":"0.000003","completion":"0.000015"},"context_length":200000,
       "benchmarks":{"artificial_analysis":{"intelligence_index":47.5,"coding_index":null,"agentic_index":null},"design_arena":[]}},
      {"id":"anthropic/claude-sonnet-4.6:batch","created":1767225600,
       "pricing":{"prompt":"0.0000015","completion":"0.0000075"},"context_length":200000},
      {"id":"~deepseek/deepseek-flash-latest","created":1780000000,
       "pricing":{"prompt":"0.0000003","completion":"0.0000012"},"context_length":1048576,
       "alias_target":{"name":"DeepSeek: DeepSeek V4.1 Flash","slug":"deepseek/deepseek-v4.1-flash"}},
      {"id":"qwen/qwen3-coder:free","created":1760000000,
       "pricing":{"prompt":"0","completion":"0"},"context_length":131072},
      {"id":"acme/undated-model",
       "pricing":{"prompt":"0.000001","completion":"0.000002"},"context_length":32000},
      {"id":"acme/expired-model","created":1700000000,"expiration_date":"2026-09-01",
       "pricing":{"prompt":"0.000001","completion":"0.000002"},"context_length":32000},
      {"id":"acme/retiring-later","created":1700000000,"expiration_date":"2026-12-31",
       "pricing":{"prompt":"0.000001","completion":"0.000002"},"context_length":32000}
    ]}"#;

    #[test]
    fn specs_read_created_canonical_slug_and_intelligence_index() {
        let specs = specs_from_openrouter_json_at(SAMPLE_MODELS_JSON, "2026-09-28").expect("parse");
        let sonnet = specs.iter().find(|s| s.id == "anthropic/claude-sonnet-4.6").unwrap();
        assert_eq!(sonnet.capabilities.released_at, Some(1_767_225_600));
        assert_eq!(sonnet.canonical_slug, "anthropic/claude-4.6-sonnet-20260101");
        assert_eq!(sonnet.capabilities.intelligence_index, Some(47.5));
        let free = specs.iter().find(|s| s.id == "qwen/qwen3-coder:free").unwrap();
        assert!(free.is_free);
        assert_eq!(free.canonical_slug, "qwen/qwen3-coder:free", "missing canonical_slug falls back to id");
        assert_eq!(free.capabilities.intelligence_index, None, "no benchmarks block means None, not 0");
    }

    #[test]
    fn missing_created_stays_none() {
        let specs = specs_from_openrouter_json_at(SAMPLE_MODELS_JSON, "2026-09-28").expect("parse");
        let undated = specs.iter().find(|s| s.id == "acme/undated-model").unwrap();
        assert_eq!(undated.capabilities.released_at, None, "absent created must not become 0 or now");
    }

    #[test]
    fn batch_alias_and_expired_entries_are_skipped() {
        let specs = specs_from_openrouter_json_at(SAMPLE_MODELS_JSON, "2026-09-28").expect("parse");
        let ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();
        assert!(!ids.iter().any(|i| i.ends_with(":batch")), "batch variants are asynchronous: {ids:?}");
        assert!(!ids.iter().any(|i| i.starts_with('~')), "alias entries duplicate real models: {ids:?}");
        assert!(!ids.contains(&"acme/expired-model"), "expired model kept: {ids:?}");
        assert!(ids.contains(&"acme/retiring-later"), "a future expiry is still selectable: {ids:?}");
        assert_eq!(specs.len(), 4);
    }

    #[test]
    fn utc_date_iso_formats_known_instants() {
        assert_eq!(utc_date_iso(0), "1970-01-01");
        assert_eq!(utc_date_iso(1_767_225_600), "2026-01-01");
        assert_eq!(utc_date_iso(1_790_618_686), "2026-09-28");
    }
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib catalog::tests > target/models-t1-red.txt 2>&1; tail -20 target/models-t1-red.txt`
Expected: FAIL to compile — `cannot find function specs_from_openrouter_json_at` / `no field intelligence_index`.

- [ ] **Step 3: Add the capability fields.** In `spec.rs`, inside `pub struct ModelCapabilities`, directly after `uptime_score`:

```rust
    /// Unix time the provider published this model (OpenRouter `/models` `created`).
    /// `None` when the source does not report it; never defaulted to 0 or "now",
    /// because recency ordering (`models::family`) must not invent a date.
    #[serde(default)]
    pub released_at: Option<u64>,
    /// Artificial Analysis intelligence index as published in OpenRouter's catalog
    /// (`benchmarks.artificial_analysis.intelligence_index`, ~0–60 today). `None` = unbenchmarked.
    #[serde(default)]
    pub intelligence_index: Option<f32>,
```

Then add `released_at: None, intelligence_index: None,` to the two `ModelCapabilities {` literals in `crates/vox-research-shim/src/selection/virtual_models.rs` (the only literals in the workspace without `..Default::default()`). If `cargo check` names any other site, STOP and list it.

- [ ] **Step 4: Parse.** In `catalog.rs`, add to `struct OpenRouterModelData` (after `latency`):

```rust
    /// Provider-published unix time; the only recency signal OpenRouter exposes.
    #[serde(default)]
    created: Option<u64>,
    /// Stable dated slug when OpenRouter provides one; falls back to `id`.
    #[serde(default)]
    canonical_slug: Option<String>,
    /// `YYYY-MM-DD` after which OpenRouter stops serving the model.
    #[serde(default)]
    expiration_date: Option<String>,
    #[serde(default)]
    benchmarks: Option<OpenRouterBenchmarks>,
```

and above it:

```rust
#[derive(serde::Deserialize, Default)]
struct OpenRouterBenchmarks {
    #[serde(default)]
    artificial_analysis: Option<OpenRouterArtificialAnalysis>,
}

#[derive(serde::Deserialize, Default)]
struct OpenRouterArtificialAnalysis {
    #[serde(default)]
    intelligence_index: Option<f32>,
}
```

Move the body of `refresh` from `let body: OpenRouterModelsResponse = resp.json().await?;` through `Ok(models)` into new free functions, and make `refresh` call the wrapper:

```rust
/// Civil date (`YYYY-MM-DD`, UTC) for a unix time (Howard Hinnant's days-to-civil).
fn utc_date_iso(unix_secs: u64) -> String {
    let z = (unix_secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Map an OpenRouter `/api/v1/models` JSON body to specs (today's UTC date drives expiry).
pub(crate) fn specs_from_openrouter_json(json: &str) -> anyhow::Result<Vec<ModelSpec>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    specs_from_openrouter_json_at(json, &utc_date_iso(now))
}

/// Pure mapping used by [`OpenRouterCatalog::refresh`] and by tests. Skips `:batch`
/// variants (asynchronous), `~vendor/…-latest` alias entries (they duplicate a real
/// model) and models whose `expiration_date` is on or before `today`.
pub(crate) fn specs_from_openrouter_json_at(json: &str, today: &str) -> anyhow::Result<Vec<ModelSpec>> {
    let body: OpenRouterModelsResponse = serde_json::from_str(json)?;
    let mut models = Vec::new();
    for m in body.data {
        if m.id.ends_with(":batch") || m.id.starts_with('~') {
            continue;
        }
        if m.expiration_date.as_deref().is_some_and(|d| d <= today) {
            continue;
        }
        // ... the existing loop body, unchanged, except the three edits below ...
    }
    Ok(models)
}
```

and in `refresh` replace the moved lines with:

```rust
        let text = resp.text().await?;
        specs_from_openrouter_json(&text)
```

Inside the moved loop make exactly three edits: in the `ModelCapabilities { ... }` literal add `released_at: m.created,` and `intelligence_index: m.benchmarks.as_ref().and_then(|b| b.artificial_analysis.as_ref()).and_then(|a| a.intelligence_index),` before `..Default::default()` (compute the index *before* `m.top_provider` / other fields are moved out of `m`, or clone it into a local first); in the `ModelSpec { ... }` literal replace `canonical_slug: m.id.clone(),` with `canonical_slug: m.canonical_slug.clone().unwrap_or_else(|| m.id.clone()),`. Change nothing else in the loop. The struct field `alias_target` in the sample JSON is ignored by serde (no `deny_unknown_fields`).

- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib catalog 2>&1 | tail -20 && timeout 1500s cargo check -p vox-research-shim 2>&1 | tail -3`
Expected: all `catalog::tests` pass (including the pre-existing `infer_strengths_*`); `vox-research-shim` checks clean.

- [ ] **Step 6: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/spec.rs crates/vox-research-shim/src/selection/virtual_models.rs crates/vox-orchestrator/src/catalog.rs
git commit -m "feat(models): read OpenRouter recency, benchmarks and expiry; skip batch, alias and expired entries"
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
        assert_eq!(family_key("deepseek/deepseek-v4.1-flash"), "deepseek/deepseek-flash");
        assert_eq!(family_key("deepseek/deepseek-v4-flash-0731"), "deepseek/deepseek-flash");
        assert_eq!(family_key("openai/gpt-6-luna"), "openai/gpt-luna");
        assert_eq!(family_key("openai/gpt-5.6-luna"), "openai/gpt-luna");
        assert_eq!(family_key("google/gemini-3.8-flash"), "google/gemini-flash");
        assert_eq!(family_key("google/gemini-3.1-flash-lite-preview"), "google/gemini-flash-lite");
        assert_eq!(family_key("Anthropic/Claude-Opus-5.5"), "anthropic/claude-opus");
    }

    #[test]
    fn family_key_keeps_free_variants_and_sizes_apart() {
        // <!-- AMENDED: R2/R3 — :free and parameter sizes are distinct families -->
        assert_eq!(family_key("qwen/qwen3.8-27b:free"), "qwen/qwen-27b:free");
        assert_eq!(family_key("qwen/qwen3.8-27b"), "qwen/qwen-27b");
        assert_eq!(family_key("qwen/qwen3.8-max-0902"), "qwen/qwen-max");
        assert_eq!(family_key("qwen/qwen3.8-max-prime"), "qwen/qwen-max-prime");
        assert_ne!(family_key("meta-llama/llama-3.1-8b"), family_key("meta-llama/llama-3.1-70b"));
        assert_eq!(family_key("qwen/qwen3-235b-a22b"), "qwen/qwen-235b-a22b");
        assert_eq!(family_key("z-ai/glm-5.3-flash"), "z-ai/glm-flash");
        assert_eq!(family_key("z-ai/glm-5.3"), "z-ai/glm");
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
        let free = dated("qwen/qwen3.8-27b:free", Some(1_700_000_000));
        let paid = dated("qwen/qwen3.9-27b", Some(1_760_000_000));
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
    let body = tok.strip_suffix('b').unwrap_or("").trim_start_matches('a');
    !body.is_empty() && body.chars().all(|c| c.is_ascii_digit() || c == '.') && body.chars().any(|c| c.is_ascii_digit())
}

/// Version-free family id: `org/name-words[:free]`, lowercased.
///
/// Per `-`-separated token of the name: pure numbers/dots (`4.6`, `20260101`, `0731`) and
/// `v4`-style markers are dropped; release-stage words (`preview`, `beta`, `exp`, `latest`)
/// are dropped; parameter sizes (`8b`, `70b`, `a22b`) are KEPT, because a smaller size is a
/// different cost point, not an older version; other mixed tokens keep only their letters
/// (`qwen3` → `qwen`, `k2.6` → `k`). A `:free` variant is its own family (a different price
/// pool); other `:variant` suffixes are dropped.
// ponytail: heuristic slug parse; OpenRouter publishes `~vendor/family-latest` alias entries
// (with `alias_target`) for ~18 families — prefer those if a family key ever needs to be authoritative.
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
            if !tok.chars().any(|c| c.is_ascii_digit()) || is_size_token(tok) {
                return Some(tok.to_string());
            }
            let is_version = tok.chars().all(|c| c.is_ascii_digit() || c == '.');
            let is_v_marker = tok.strip_prefix('v').is_some_and(|r| !r.is_empty() && r.chars().all(|c| c.is_ascii_digit() || c == '.'));
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

### Task 3: Selection drops superseded versions among eligible candidates

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

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::tests::superseded_selection_tests > target/models-t3-red.txt 2>&1; tail -15 target/models-t3-red.txt`
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

### Task 4: Quality from the live benchmark index; flagship tier from price

**Files:**
- Create: `crates/vox-orchestrator/src/models/tiering.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs` (add `pub mod tiering;`)
- Modify: `crates/vox-orchestrator/src/models/scoring.rs` (`quality_score`, and tests in its test module)
- Modify: `crates/vox-orchestrator/src/models/registry.rs` (`ModelRegistry::register`)
- Test: `crates/vox-orchestrator/src/models/tests.rs` (new module at end)

**Interfaces:**
- Consumes: `ModelCapabilities.intelligence_index` (Task 1).
- Produces: `pub fn derive_tier(is_free: bool, cost_per_1k_output: f64) -> crate::models::ModelTier`; `pub const ELITE_MIN_OUTPUT_USD_PER_1K: f64 = 0.020;` `pub const PRO_MIN_OUTPUT_USD_PER_1K: f64 = 0.004;` (USD per 1,000 output tokens; `0.020` = $20 per 1M).

- [ ] **Step 1: Write the failing tier tests** — create `tiering.rs` with the test module only:

```rust
//! Flagship detection from price, so the Efficient lane never needs a hand-kept list of
//! "expensive models" that goes stale each release. At time of writing the ≥ $20/M
//! completion band is exactly Opus/Fable/GPT-6-Astra-class.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelTier;

    #[test]
    fn free_models_are_free_tier() {
        assert_eq!(derive_tier(true, 0.0), ModelTier::Free);
    }

    #[test]
    fn output_price_bands_map_to_tiers() {
        assert_eq!(derive_tier(false, 0.050), ModelTier::Elite, "$50/M");
        assert_eq!(derive_tier(false, 0.020), ModelTier::Elite, "$20/M is the boundary");
        assert_eq!(derive_tier(false, 0.0199), ModelTier::Pro);
        assert_eq!(derive_tier(false, 0.004), ModelTier::Pro, "$4/M is the boundary");
        assert_eq!(derive_tier(false, 0.0039), ModelTier::Fast);
        assert_eq!(derive_tier(false, 0.00012), ModelTier::Fast);
    }

    #[test]
    fn unknown_pricing_is_not_guessed() {
        // A non-free model with a 0.0 placeholder price (e.g. Anthropic-direct before the
        // LiteLLM oracle fills it in) must not be called Fast.
        assert_eq!(derive_tier(false, 0.0), ModelTier::Unknown);
        assert_eq!(derive_tier(false, f64::NAN), ModelTier::Unknown);
    }
}
```

Add `pub mod tiering;` to `models/mod.rs`.

- [ ] **Step 2: Write the failing quality and register tests.** Append to the test module of `scoring.rs` (it has a `make_spec(provider_type, cost, is_free)` helper; find it with `rg -n "fn make_spec" crates/vox-orchestrator/src/models/scoring.rs`):

```rust
    #[test]
    fn quality_score_follows_the_live_intelligence_index() {
        let mut hi = make_spec(ProviderType::OpenRouter, 1.0, false);
        hi.capabilities.intelligence_index = Some(57.6);
        let mut lo = make_spec(ProviderType::OpenRouter, 1.0, false);
        lo.capabilities.intelligence_index = Some(37.3);
        assert!(quality_score(&hi) > quality_score(&lo), "paid models must no longer score equal");
        assert!((quality_score(&hi) - 57.6 / QUALITY_INDEX_REFERENCE).abs() < 1e-6);
    }

    #[test]
    fn a_free_model_with_a_high_index_beats_a_paid_one_without() {
        let mut free = make_spec(ProviderType::OpenRouter, 0.0, true);
        free.capabilities.intelligence_index = Some(45.0);
        let paid_unbenchmarked = make_spec(ProviderType::OpenRouter, 5.0, false);
        assert!(quality_score(&free) > quality_score(&paid_unbenchmarked));
    }

    #[test]
    fn unbenchmarked_quality_is_scaled_below_strong_benchmarked_models() {
        // <!-- AMENDED: R4 — unbenchmarked long-tail models must not outrank benchmarked ones -->
        let mut unknown = make_spec(ProviderType::OpenRouter, 1.0, false);
        unknown.max_tokens = 1_000_000;
        assert!(quality_score(&unknown) > 0.0 && quality_score(&unknown) <= UNBENCHMARKED_QUALITY_SCALE);
        let mut strong = make_spec(ProviderType::OpenRouter, 1.0, false);
        strong.capabilities.intelligence_index = Some(47.5);
        assert!(quality_score(&strong) > quality_score(&unknown));
    }
```

Append to `crates/vox-orchestrator/src/models/tests.rs`:

```rust
#[cfg(test)]
mod tier_stamp_tests {
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType};

    fn priced(id: &str, provider_type: ProviderType, out_per_1k: f64, is_free: bool) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 8192,
            cost_per_1k: out_per_1k,
            cost_per_1k_input: out_per_1k / 5.0,
            cost_per_1k_output: out_per_1k,
            is_free,
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
    fn register_derives_the_tier_from_price_when_unknown() {
        // <!-- AMENDED: R5 — stamp once at registration so every catalog is covered -->
        let mut r = ModelRegistry::default();
        r.register(priced("acme/flagship", ProviderType::OpenRouter, 0.025, false));
        r.register(priced("acme/mid", ProviderType::OpenRouter, 0.010, false));
        r.register(priced("acme/cheap", ProviderType::OpenRouter, 0.0005, false));
        assert_eq!(r.get("acme/flagship").unwrap().capabilities.tier, ModelTier::Elite);
        assert_eq!(r.get("acme/mid").unwrap().capabilities.tier, ModelTier::Pro);
        assert_eq!(r.get("acme/cheap").unwrap().capabilities.tier, ModelTier::Fast);
    }

    #[test]
    fn register_keeps_an_explicit_tier_and_leaves_local_models_alone() {
        let mut r = ModelRegistry::default();
        let mut explicit = priced("acme/pinned", ProviderType::OpenRouter, 0.025, false);
        explicit.capabilities.tier = ModelTier::Pro;
        r.register(explicit);
        r.register(priced("local/qwen", ProviderType::Ollama, 0.0, true));
        assert_eq!(r.get("acme/pinned").unwrap().capabilities.tier, ModelTier::Pro);
        assert_ne!(r.get("local/qwen").unwrap().capabilities.tier, ModelTier::Free, "local models are not cloud-free");
    }
}
```

(`ModelRegistry::get` may return an owned or borrowed spec; if `.unwrap().capabilities` does not compile, adapt only the accessor in these tests and say so.)

- [ ] **Step 3: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::tiering models::scoring models::tests::tier_stamp_tests > target/models-t4-red.txt 2>&1; tail -20 target/models-t4-red.txt`
Expected: FAIL to compile (`derive_tier`, `QUALITY_INDEX_REFERENCE`, `UNBENCHMARKED_QUALITY_SCALE` not found). Save this output before implementing.

- [ ] **Step 4: Implement `tiering.rs`** (above the tests):

```rust
use crate::models::ModelTier;

/// USD per 1,000 output tokens at or above which a model is a flagship (Elite): `0.020` = $20 per 1M.
pub const ELITE_MIN_OUTPUT_USD_PER_1K: f64 = 0.020;
/// USD per 1,000 output tokens at or above which a model is Pro (below it: Fast): `0.004` = $4 per 1M.
pub const PRO_MIN_OUTPUT_USD_PER_1K: f64 = 0.004;

/// Tier from price alone. `Unknown` when the price is not known (zero or NaN on a non-free model).
// ponytail: fixed bands; move to model-routing.v1.yaml when the council wants to tune them.
#[must_use]
pub fn derive_tier(is_free: bool, cost_per_1k_output: f64) -> ModelTier {
    if is_free {
        return ModelTier::Free;
    }
    if !cost_per_1k_output.is_finite() || cost_per_1k_output <= 0.0 {
        return ModelTier::Unknown;
    }
    if cost_per_1k_output >= ELITE_MIN_OUTPUT_USD_PER_1K {
        ModelTier::Elite
    } else if cost_per_1k_output >= PRO_MIN_OUTPUT_USD_PER_1K {
        ModelTier::Pro
    } else {
        ModelTier::Fast
    }
}
```

- [ ] **Step 5: Implement `quality_score`.** In `scoring.rs` add next to the other constants:

```rust
/// Artificial Analysis intelligence-index value treated as quality 1.0 (the best model in
/// OpenRouter's catalog scored 57.6 on 2026-09-28).
// ponytail: fixed reference; derive from the registry maximum if the scale drifts.
pub(super) const QUALITY_INDEX_REFERENCE: f64 = 60.0;
/// Scale applied to the paid/free + context-length proxy for unbenchmarked models, so an unknown
/// model cannot outrank a strongly benchmarked one on the proxy alone.
pub(super) const UNBENCHMARKED_QUALITY_SCALE: f64 = 0.6;
```

and replace the body of `quality_score` with:

```rust
pub(super) fn quality_score(m: &ModelSpec) -> f64 {
    if let Some(i) = m.capabilities.intelligence_index {
        return (f64::from(i) / QUALITY_INDEX_REFERENCE).clamp(0.0, 1.0);
    }
    let token_component = (m.max_tokens as f64).log10().clamp(1.0, 7.0) / 7.0;
    let paid_component = if m.is_free { QUALITY_FREE_PAID_COMPONENT } else { QUALITY_PAID_COMPONENT };
    (((token_component * QUALITY_TOKEN_WEIGHT) + (paid_component * QUALITY_PAID_WEIGHT)) * UNBENCHMARKED_QUALITY_SCALE)
        .clamp(0.0, 1.0)
}
```

- [ ] **Step 6: Derive the tier at registration.** In `registry.rs`, change `pub fn register(&mut self, spec: ModelSpec)` to `pub fn register(&mut self, mut spec: ModelSpec)` and make its first statement:

```rust
        if spec.capabilities.tier == super::ModelTier::Unknown
            && !matches!(
                spec.provider_type,
                super::ProviderType::Ollama | super::ProviderType::PopuliMesh | super::ProviderType::VoxLocal
            )
        {
            spec.capabilities.tier = super::tiering::derive_tier(spec.is_free, spec.cost_per_1k_output);
        }
```

- [ ] **Step 7: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models:: catalog 2>&1 | tail -25`
Expected: all pass. If a pre-existing `scoring` or `models::tests` test now fails (e.g. it asserted the old uncapped paid/free quality or a specific registered tier), STOP and name it with the failing assertion — do not edit it.

- [ ] **Step 8: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/tiering.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/models/scoring.rs crates/vox-orchestrator/src/models/registry.rs crates/vox-orchestrator/src/models/tests.rs
git commit -m "feat(models): score quality from the live benchmark index and derive flagship tier from price"
```

---

### Task 5: Mode guarantees and the provider-key gate on the dispatch path

<!-- AMENDED: R6 — the key gate lives in the dispatch selector, not in best_for_internal, so existing key-gate tests (select.rs key_gate_*), cloud-spec registry tests and tests/economy_test.rs are untouched. -->

**Files:**
- Create: `crates/vox-orchestrator/src/models/mode_select.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs` (add `pub mod mode_select;`)
- Modify: `crates/vox-orchestrator/src/models/key_guard.rs` (append `selection_key_available`, its override and a test)
- Modify: `crates/vox-orchestrator/src/runtime.rs` (`resolve_task_cost_policy`; the `let routed = { … };` block in `AiTaskProcessor::process`; the three `resolve_task_cost_policy` tests)

**Interfaces:**
- Consumes: `ModelRegistry::best_for_task_with_filter` (existing); `crate::mode::ClutchProfile` (existing); tiers (Task 4).
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
    /// guard it wins under Economy; the first fixture let the Fast model win regardless. -->
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

Add `pub mod mode_select;` to `models/mod.rs`. If `AgentTask::task_category` is an `Option<TaskCategory>` or `estimated_complexity` is not a plain `u8` field on `AgentTask`, adapt only `hard_task()` and say so. Use `AgentTask::new`, never an `AgentTask { … }` literal (Phase 5 adds a field to it).

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

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::mode_select models::key_guard > target/models-t5-red.txt 2>&1; tail -20 target/models-t5-red.txt`
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

so `routed` stays `Option<ModelSpec>` for all later uses. The block to edit is the one that ends just before the comment starting "Code-review fix: `routed == None`".

Update the three existing `resolve_task_cost_policy` tests in `runtime.rs` (the only callers besides dispatch; `rg -n "resolve_task_cost_policy" crates/vox-orchestrator/src/runtime.rs`) to destructure four values — this is the one sanctioned edit to existing tests — and add one assertion to the first of them: an unconfigured task returns the same clutch `crate::mode::resolve_task_policy(None, None, None, None, None, None).0` returns.

- [ ] **Step 8: Run the dispatch and model tests**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- runtime models:: 2>&1 | tail -20 && timeout 1500s cargo test -p vox-orchestrator --tests 2>&1 | grep -E "test result|FAILED" | tail -10`
Expected: all pass, including integration tests such as `tests/economy_test.rs`.

- [ ] **Step 9: Mutation proofs** — (a) change `matches!(clutch, ClutchProfile::Efficiency | ClutchProfile::Balanced)` to `false`, run `timeout 1500s cargo test -p vox-orchestrator --lib models::mode_select > target/models-mutant-elite.txt 2>&1`, confirm `efficiency_on_a_hard_task_never_picks_elite ... FAILED`, restore; (b) change `selection_key_available(&m.provider_type) && pred(m)` to `pred(m)`, run the same command into `target/models-mutant-keygate.txt`, confirm `a_provider_without_a_key_is_never_picked ... FAILED`, restore; confirm `git diff` shows only the intended edits.

- [ ] **Step 10: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/mode_select.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/models/key_guard.rs crates/vox-orchestrator/src/runtime.rs
git commit -m "fix(models): dispatch keeps Efficiency/Balanced off flagships and never picks a keyless provider"
```

---

### Task 6: A council-pinned alias that a newer family member has superseded is skipped

**Decision (2026-09-28, recorded in the ledger):** the council's pins file is not edited. Instead `select_via_premium_alias` skips a pin when the registry holds a newer member of the pin's family, falling through to the scorer. The council still chooses the family; the version tracks the live catalog.

**Files:**
- Modify: `crates/vox-orchestrator/src/models/family.rs` (add `impl ModelRegistry { pub fn is_superseded_in_registry(&self, m: &ModelSpec) -> bool }` and its test)
- Modify: `crates/vox-orchestrator/src/models/select.rs` (`select_via_premium_alias`, and a test in its test module)

**Interfaces:**
- Consumes: `family::{newest_per_family, is_superseded}` (Task 2).
- Produces: `ModelRegistry::is_superseded_in_registry(&self, m: &ModelSpec) -> bool`.

- [ ] **Step 1: Write the failing tests.** Append to `family.rs`'s test module:

```rust
    #[test]
    fn registry_reports_a_superseded_member() {
        use crate::models::ModelRegistry;
        let mut r = ModelRegistry::default();
        r.register(dated("acme/widget-4.8", Some(1_700_000_000)));
        r.register(dated("acme/widget-5.5", Some(1_760_000_000)));
        assert!(r.is_superseded_in_registry(&dated("acme/widget-4.8", Some(1_700_000_000))));
        assert!(!r.is_superseded_in_registry(&dated("acme/widget-5.5", Some(1_760_000_000))));
    }
```

Append to `select.rs`'s test module (it already imports `ModelRegistry`, `SelectionAxes`, `SelectionIntent`, `TaskCategory`, `select`, `SelectionReason`, `file_serial`):

```rust
    #[test]
    #[file_serial]
    fn select_skips_a_premium_alias_pin_that_a_newer_family_member_supersedes() {
        // ModelRegistry::new() carries the codegen pin `anthropic/claude-3-7-sonnet`. Re-register it as a
        // dated local-provider spec (no key needed) and add a newer member of the same family.
        let mut registry = ModelRegistry::new();
        let mut pin = registry.get("anthropic/claude-3-7-sonnet").expect("bootstrap pin").clone();
        pin.provider_type = crate::models::ProviderType::Ollama;
        pin.capabilities.released_at = Some(1_700_000_000);
        let mut newer = pin.clone();
        newer.id = "anthropic/claude-sonnet-5.5".into();
        newer.canonical_slug = newer.id.clone();
        newer.capabilities.released_at = Some(1_790_000_000);
        registry.register(pin);
        registry.register(newer);
        let intent = SelectionIntent {
            axes: SelectionAxes::QUALITY_FIRST,
            ..SelectionIntent::for_task(TaskCategory::CodeGen)
        };
        let outcome = select(&intent, &registry).expect("a model exists");
        assert!(
            !matches!(outcome.reason, SelectionReason::PremiumAlias { .. }),
            "the superseded pin must not be honoured, got {:?}",
            outcome.reason
        );
    }
```

If `registry.get(..)` returns an owned spec rather than a reference, drop the `.clone()`; adapt only that accessor.

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::family models::select::tests::select_skips_a_premium_alias_pin > target/models-t6-red.txt 2>&1; tail -20 target/models-t6-red.txt`
Expected: FAIL — `no method named is_superseded_in_registry`; after adding only that method, the select test would still FAIL (the pin is honoured). Save the compile-failure output now.

- [ ] **Step 3: Implement.** In `family.rs` (above `#[cfg(test)]`):

```rust
impl crate::models::ModelRegistry {
    /// True when the registry holds a newer dated member of `m`'s family.
    #[must_use]
    pub fn is_superseded_in_registry(&self, m: &ModelSpec) -> bool {
        // ponytail: O(models) per call; only used on the premium-alias path.
        is_superseded(m, &newest_per_family(self.models.values()))
    }
}
```

`self.models` is private to `registry.rs`; if it does not compile from `family.rs`, expose it with a `pub(crate) fn models_iter(&self) -> impl Iterator<Item = &ModelSpec>` in `registry.rs` next to `register`, use that here, and say so. In `select.rs`, in `select_via_premium_alias`, directly after `let model = registry.get(&alias)?;` add:

```rust
    if registry.is_superseded_in_registry(&model) {
        return None;
    }
```

- [ ] **Step 3: Run to verify pass, plus the select tests**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::family models::select 2>&1 | tail -20`
Expected: all pass, including the existing `select_with_premium_alias_honors_alias_when_intelligence_high` (its bootstrap pin is undated, so never superseded).

- [ ] **Step 4: Mutation proof** — delete the two inserted `select.rs` lines, run the Step 2 command into `target/models-mutant-pin.txt`, confirm `select_skips_a_premium_alias_pin_that_a_newer_family_member_supersedes ... FAILED`, restore, confirm `git diff`.

- [ ] **Step 5: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/family.rs crates/vox-orchestrator/src/models/select.rs crates/vox-orchestrator/src/models/registry.rs
git commit -m "fix(models): a premium-alias pin superseded by a newer family member is skipped"
```

---

### Task 7: Verification sweep — Owner: Claude

- [ ] `timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5` and `timeout 1500s cargo test -p vox-orchestrator --tests 2>&1 | grep -E "test result|FAILED"` — all pass.
- [ ] `timeout 1500s cargo test -p vox-orchestrator-mcp --lib model_route_policy 2>&1 | tail -5` — the MCP key-gate tests still pass.
- [ ] `timeout 1500s cargo clippy -p vox-orchestrator -p vox-research-shim --all-targets -- -D warnings` — clean for touched files.
- [ ] `timeout 1500s cargo run -q -p vox-cli -- ci pre-push` (fast tier); regenerate and commit any inventory it names.
- [ ] Record a live check: fetch the catalog and print what `derive_tier` + `quality_score` + `family` rank for Efficient at complexity 10 (the expected leaders by intelligence-per-dollar on 2026-09-28 are `xiaomi/mimo-v2.6-pro`, `z-ai/glm-5.3-flash`, `deepseek/deepseek-v4.1-flash`, `openai/gpt-6-luna`, `google/gemini-3.8-flash`; no `anthropic/claude-opus-*`).

---

## Decisions (resolved 2026-09-28; the user delegated open decisions to Claude)

1. **Council pins (was Task 8):** keep exact pins in the contract; skip a pin a newer family member supersedes (Task 6). No council sign-off is needed because no ratified file changes.
2. **Low-risk with the default clutch:** the Efficiency/Balanced Elite exclusion stays even when a Low-risk posture forces `Performance` preference. Efficient must mean efficient; a user who wants flagships selects Genius.
3. **Chat lane first in the next plan:** yes — `vox-orchestrator-mcp` `llm_bridge/model_route_policy/resolve.rs` gets the mode-aware selector before the trace work, because it is what the chat GUI shows (plan `2026-09-28-model-routing-chat-lane.md`, written after reading `resolve.rs`).
4. **Research-shim tier shift:** accepted. Tier stamped from price at registration now feeds `vox-research-shim`'s tier scoring (Free −0.8, Pro +1.0); that is the intended behaviour, since those scores were built for populated tiers.
5. **Static seed contract:** dropped in favour of OpenRouter's live `benchmarks` + `created` + `expiration_date`; the offline fallback is the bootstrap catalog (refreshed by a follow-up `.vox` script), where unbenchmarked models score via the scaled-down proxy.

## Execution record (2026-09-29)

Tasks 1–6 committed: `a7004ce11`, `7577241f6`, `b6976a1d0`, `1eedbd7bc`, `fefa01d7a`, `5ed86c47e`. Every guard was mutation-proven (see the R4 note for the one execution-time amendment). Two flaky tests found in passing and fixed separately (`vram` hint tests now `file_serial`).

## Deferred (next plans)

- **Chat lane, `explain_selection`, `best_free_for*` / `cheapest*`, the Thompson fallback in `registry_model_resolve.rs`, GUI `suggest_model_for_task`** do not yet honour recency or the key gate; the chat-lane plan covers the first, the rest follow it.
- **`benchmarks.artificial_analysis.coding_index` / `agentic_index`** (null for most models today) should weight quality for CodeGen / tool-heavy tasks once populated.
- **OpenRouter `/models` reports no latency**, so `latency_p50_ms` is never populated from the catalog; responsiveness comes from the scoreboard. Wire measured latency into the mode objectives with the family-keyed scoreboard plan.
- **`~vendor/…-latest` aliases** (18 families, with `alias_target`) could replace the heuristic `family_key` where present.
- **Offline bootstrap refresh:** `scripts/refresh-model-catalog.vox` regenerates `model-catalog.bootstrap.v1.json` from the live catalog (including `created` and the index) so offline runs are not stale.
- **`select.rs#select_via_scorer`'s own key predicate** becomes redundant once the chat lane uses the mode selector.

## Execution Order

- **Sequential constraints:** Task 1 → 2 (`family.rs` reads `released_at`); 2 → 3; 3 → 4 (both modify `registry.rs` and `models/tests.rs`); 4 → 5 (dispatch relies on tiers and quality); 5 → 6; `models/mod.rs` is touched by Tasks 2, 4, 5 — sequential; `family.rs` by 2 and 6 — sequential.
- **Phase 5 interplay:** plans 05-04…05-07 change `types/tasks.rs`, `orchestrator/**`, `hopper/**`, GUI chat files; this plan touches `models/**`, `catalog.rs`, `runtime.rs`. Only `runtime.rs` could overlap (05-05 may edit dispatch wiring near it — check `git diff` before driving Task 5). Drive this plan after Phase 5 completes, or between Phase 5 plans, never concurrently (shared working tree, index and build).
- **Pre-flight:** working tree clean for the task's files; HEAD on `main`; no schema change.
- **SDD ledger:**
  - R1 supersession judged among eligible candidates — ruling: settled
  - R2 `:free` is its own family — ruling: settled
  - R3 parameter sizes are part of the family — ruling: settled
  - R4 unbenchmarked quality scaled by 0.6 — ruling: settled, **amended at execution (2026-09-29): the scale applies to `ProviderType::OpenRouter` models only.** Only OpenRouter's catalog carries the index, so for direct-provider and local models a missing index means "no benchmark exists here", not "unbenchmarked long tail". Scaling them demoted every Anthropic-direct flagship below unbenchmarked local models and broke the pre-existing guard `models::scoring::tests::free_local_model_does_not_win_high_complexity`. Proven by mutation (scale applied to all providers fails that guard). The three Step 2 quality tests were also moved out of the `populi-transport`-gated `donations_vox_wiring_tests` module (where the driven agent appended them, so they never ran) into `mod tests`.
  - R5 tier derived from price at registration when `Unknown`, cloud providers only — ruling: settled
  - R6 key gate in the dispatch selector with a thread-local override — ruling: settled
  - R7 Task 3 fixture cost within the safety cap — ruling: settled
  - `:batch`, `~alias` and expired catalog entries skipped at parse — ruling: settled
  - Decisions 1–5 above — ruling: settled
  - `registry.rs` Tasks 3 → 4 (→ 6 if `models_iter` is added) — sequential — settled; `models/tests.rs` Tasks 3 → 4 — sequential — settled; `models/mod.rs` Tasks 2 → 4 → 5 — sequential — settled
