# Tab 2: Free Tier Metadata Catalog & Chat Research Killswitch Registration

> **Harness & Execution Directives (Gemini Flash 3.8 under Antigravity Desktop Harness):**
> - **Atomic Green Commits:** Every task must compile, pass tests, and end **GREEN** before committing.
> - **Verify-Before-Use:** Before modifying or importing any symbol, execute a pre-flight `rg` command to verify the actual in-repo signature.
> - **Single Command Discipline:** Emit exactly **one terminal command per step**. Never chain with `&&`, `|`, `;`, or wrap in `bash -lc` (causes allowlist parsing failures and orphaned processes on Windows/PowerShell).
> - **Scoped Rustfmt:** Run `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`. **NEVER** run `cargo fmt --all` (causes Windows command-line overflow error 206).
> - **Strict File Disjointness:** Files modified in Tab 2 (`vox-secrets`) are strictly isolated from Tab 1 (`vox-search` & `vox-db`).
> - **Two-Strike Circuit Breaker:** If a build or test fails twice consecutively on the same step, STOP and report immediately. Never weaken CI flags or alter `layers.toml`.

---

## 1. Handoff Contract

- **Upstream Dependencies:** None (Foundational backend wave; runs parallel to Tab 1).
- **Downstream Deliverables:**
  1. `crates/vox-secrets/src/spec/free_tier.rs`: Data model and catalog of validated free tier API key offers (`FreeTierOffer`).
  2. `crates/vox-secrets/src/spec/ids.rs`: Registered `SecretId::VoxChatResearchEnabled`.
  3. `crates/vox-secrets/src/spec/registry/config.rs`: Registration of `VOX_CHAT_RESEARCH_ENABLED` (boolean, default: `true`).
  4. `crates/vox-secrets/src/spec/registry/platform.rs`: Updated remediation text for Tavily with direct acquisition link.
- **Handoff Consumers:**
  - Tab 4 (Orchestrator) consumes `SecretId::VoxChatResearchEnabled` to bypass research in chat turns.
  - Tab 5A (Tauri IPC) and Tab 5C (Drawer) consume `FreeTierOffer` metadata to render the free key acquisition cards in the UI.

---

## 2. Context & Technical Specification

### 2.1 Free Tier Metadata Catalog
The Axis GUI surfaces direct, validated signup links for external search and inference services that offer free tiers without requiring a credit card.
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeTierOffer {
    pub provider_id: &'static str,
    pub name: &'static str,
    pub signup_url: &'static str,
    pub free_tier_description: &'static str,
    pub quota_summary: &'static str,
    pub requires_credit_card: bool,
    pub secret_id: SecretId,
}
```

Canonical Catalog Entries:
- **Tavily Web Search**: 1,000 queries/month free at `https://app.tavily.com/sign-up` (no credit card).
- **Google Gemini**: Free API key with rate-limited quota at `https://aistudio.google.com/app/apikey` (no credit card).
- **OpenRouter**: Free tier model access at `https://openrouter.ai/keys` (no credit card).
- **Semantic Scholar**: Free academic graph API at `https://www.semanticscholar.org/product/api` (no credit card).

### 2.2 Chat Research Killswitch
For automated CI and offline development, chat interactions must have an isolated killswitch that bypasses web retrieval completely:
- Canonical Environment Variable: `VOX_CHAT_RESEARCH_ENABLED`.
- Default: `true`.
- Behavior: When set to `false`, chat turns bypass the research router and answer immediately from internal knowledge, while the `/research` slash command and Knowledge menu remain fully operational.

---

## 3. Step-by-Step Implementation

### Step 1: Pre-flight Verification
Run `rg` to verify existing secret registration patterns:
```bash
rg "SecretId::VoxSearchTavilyEnabled" crates/vox-secrets/src/spec/ids.rs
```

### Step 2: Write Failing Unit Test for Free Tier Catalog
In `crates/vox-secrets/src/lib.rs` (or test module):
```rust
#[cfg(test)]
mod free_tier_tests {
    use super::*;

    #[test]
    fn test_free_tier_catalog_contains_verified_providers() {
        let offers = spec::free_tier::list_free_tier_offers();
        assert!(offers.iter().any(|o| o.provider_id == "tavily" && o.signup_url == "https://app.tavily.com/sign-up" && !o.requires_credit_card));
        assert!(offers.iter().any(|o| o.provider_id == "gemini" && o.signup_url == "https://aistudio.google.com/app/apikey"));
        assert!(offers.iter().any(|o| o.provider_id == "openrouter" && o.signup_url == "https://openrouter.ai/keys"));
        assert!(offers.iter().any(|o| o.provider_id == "semantic_scholar" && o.signup_url.contains("semanticscholar.org")));
    }
}
```

### Step 3: Run Failing Free Tier Test
```bash
cargo test -p vox-secrets test_free_tier_catalog_contains_verified_providers
```
Expected: FAIL (unresolved module `free_tier`).

### Step 4: Implement `free_tier.rs`
Create `crates/vox-secrets/src/spec/free_tier.rs` defining `FreeTierOffer` and `list_free_tier_offers()`.

### Step 5: Register `VoxChatResearchEnabled`
1. In `crates/vox-secrets/src/spec/ids.rs`, add `VoxChatResearchEnabled` to `enum SecretId`.
2. In `crates/vox-secrets/src/spec/registry/config.rs`, register the secret spec with environment variable `"VOX_CHAT_RESEARCH_ENABLED"` and default `true`.
3. In `crates/vox-secrets/src/spec/registry/platform.rs`, update Tavily key remediation with `https://app.tavily.com/sign-up`.

### Step 6: Verify Tests Pass
```bash
cargo test -p vox-secrets test_free_tier_catalog_contains_verified_providers
```
Expected: PASS.

### Step 7: Format Code
```bash
cargo fmt -p vox-secrets
```

### Step 8: Atomic Commit
```bash
git add crates/vox-secrets/src/spec/free_tier.rs crates/vox-secrets/src/spec/ids.rs crates/vox-secrets/src/spec/registry/config.rs crates/vox-secrets/src/spec/registry/platform.rs
git commit -m "feat(secrets): register FreeTierOffer catalog and VoxChatResearchEnabled killswitch"
```
