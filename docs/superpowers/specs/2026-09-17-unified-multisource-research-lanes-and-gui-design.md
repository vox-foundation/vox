---
title: "Unified Multi-Source Research Retrieval, Dual Lanes, Drive Profiles, and GUI Visual Debugger"
description: "Master architectural specification unifying keyless retrieval (OpenAlex, arXiv, Wikipedia), dual-lane execution (Fast vs Deep), quota visualization, Drive Profiles (Clutch + Risk Posture), progressive disclosure, and the Axis GUI Visual Debugger."
category: "architecture"
status: "current"
date: 2026-09-17
---

# Unified Multi-Source Research Retrieval, Dual Lanes, Drive Profiles, and GUI Visual Debugger

## 1. Executive Summary & Problem Statement

### 1.1 The Legacy Retrieval & Observability Crisis
The previous research pipeline in Vox suffered from four compounding failures:
1. **Fragile, Key-Dependent Cascade**: Retrieval relied on a sequential fallback cascade:
   $$\text{SearXNG} \longrightarrow \text{Tavily} \longrightarrow \text{DuckDuckGo} \longrightarrow \text{Wikipedia}$$
   In standard environments without self-hosted SearXNG or a paid Tavily key, the cascade hit the dead-weight `api.duckduckgo.com` Instant Answer endpoint. That endpoint returns HTTP 200 with empty topic lists for technical queries, generating **zero evidence** while never tripping circuit breakers.
2. **"Fake Success" & Silent Degradation**: When retrieval returned 0 hits, the pipeline silently fell back to user-prompt claim extraction, outputting *"No external sources were found... Answering from internal knowledge only"*, marking `ResearchStage::Completed`, and returning success without external grounding.
3. **GUI Bloat & Uncalibrated Jargon**: The research surface stacked live probers, search inputs, session lists, raw `<pre>` dumps, and mathematical tensors vertically. User-facing cards displayed opaque academic jargon (*"Dirichlet prior $\alpha=(3,2,1)$"*, *"Contradiction Entropy 1.46 bits"*) without plain-language explanations of *how* or *why*.
4. **Fragmented Execution Knobs**: Users faced disconnected, competing controls across the app: Clutch (`Free`, `Efficiency`, `Balanced`, `Genius`), Risk Posture (`High`, `Moderate`, `Low`), and ad-hoc research toggles, causing button fatigue and confusion over cost vs. accuracy tradeoffs.

### 1.2 Core Architectural Axioms
1. **Zero-Key Baseline**: Research retrieval MUST function out of the box with zero API keys on any domain (code, science, math, history, general facts) using authoritative open repositories: **Wikipedia**, **OpenAlex** (250M+ scholarly works), and **arXiv** (2M+ preprints).
2. **Dual-Lane Execution Model**:
   - **Fast Lane**: Single-hop parallel dispatch bounded by a strict 1,500 ms deadline for conversational sub-second lookups.
   - **Deep Research Lane**: Multi-hop iterative CRAG loop with subquery fan-out, domain-stratified scraping, multi-perspective verification, and long-form synthesis.
3. **Epistemic Zero-Hit Hard Halt**: If $\text{all\_hits} = \emptyset$, the pipeline MUST halt immediately with `ResearchStage::Failed`. Never synthesize from internal knowledge when external research was requested.
4. **Honesty Sentry Invariant**: Low-evidence results ($< 0.35$ confidence) must carry `data-testid="empty-results-notice"` to satisfy `sentryHonesty.ts` without triggering a `fake_success` defect.
5. **Unified Drive Profile**: Replaces disconnected Clutch and Risk knobs with a single slider/selector (`⚡ Fast`, `⚖️ Balanced` [Default], `🧠 Deep Genius`, `⚙️ Custom`), balancing Inaccuracy Risk against Cost/Latency Explosion Risk.
6. **Progressive Disclosure & Zero Dock Bloat**: Chat remains conversational and unbloated. Research turns render a compact 1-line `ResearchMicroCard`; clicking `[ View Details ]` slides out the right-hand `InspectorDrawer` (`z-45`) for ephemeral forensics.
7. **Multi-Layer Modal Hierarchy**: Base viewports live at `z-0`, `InspectorDrawer` at `z-45`, and the `ResearchEngineDrawer` (free key acquisition & source config) at `z-50` with circular focus trapping and isolated `Escape` handling.
8. **SSOT Explainer & Action Registry**: A typed dictionary (`researchExplainerRegistry.ts`) binds plain-English *why/how* descriptions to "Under the Hood" mathematical formulas, ensuring labels and formulas never drift.

---

## 2. System Architecture & Information Scenes

```mermaid
flowchart TD
    subgraph ClientSurfaces["Client Interaction Scenes (Axis UI)"]
        SceneA["Scene A: Chat View<br/>• Unified Drive Profile (Fast/Balanced/Genius)<br/>• Compact ResearchMicroCard (1-line)<br/>• Killswitch Guard (VOX_CHAT_RESEARCH_ENABLED)"]
        SceneB["Scene B: Knowledge Hub<br/>• Tab 1: Research & Discovery Lab (Lane Switcher, Quota Strip, DAG Canvas)<br/>• Tab 2: Project Knowledge & Docs Search (SSOT Publisher)<br/>• Tab 3: Epistemic & Pipeline Debugger (8-Stage Stepper, Sentries)"]
        SceneC["Scene C: Global Status Bar<br/>• Cluster Pill: Systems 4/4 Ready<br/>• Web, Models, Indices, Workspace Health"]
    end

    subgraph Drawers["Multi-Layer Modal Hierarchy"]
        DrawerInspector["InspectorDrawer (z-45)<br/>• Live Findings Timeline<br/>• Stage Performance (Latency & Tokens)<br/>• Under the Hood (Formulas & Payloads)"]
        DrawerEngine["ResearchEngineDrawer (z-50)<br/>• Source Toggles (Wiki, OpenAlex, arXiv, Tavily)<br/>• Free Key Acquisition Hub (Direct URLs)<br/>• Focus Trap & Isolated Escape"]
    end

    subgraph Orchestration["vox-research-shim & vox-orchestrator"]
        Router["Drive Profile Router & Intent Analyzer"]
        Escalator["Dynamic Auto-Escalation Engine<br/>(Query Entropy & Conflict Detector)"]
        Pipeline["8-Stage Pipeline Stepper<br/>(Zero-Hit Hard Halt & Wiremock CI)"]
    end

    subgraph SearchMesh["vox-search: Integrated Provider Mesh"]
        Dispatcher["WebSearchDispatcher (Lane-Aware)"]
        subgraph Keyless["Zero-Key Baseline"]
            Wiki["Wikipedia API"]
            OpenAlex["OpenAlex Works API"]
            ArXiv["arXiv Atom API"]
        end
        subgraph Keyed["Key-Enhanced Multipliers"]
            Tavily["Tavily Search & /extract"]
            SearXNG["SearXNG Meta-Search"]
            SemScholar["Semantic Scholar API"]
        end
        RRF["True Reciprocal Rank Fusion (k=60)"]
    end

    SceneA -->|Click [Inspect]| DrawerInspector
    SceneB -->|Click [Configure Sources]| DrawerEngine
    SceneA --> Router
    SceneB --> Pipeline
    Router --> Escalator
    Escalator --> Pipeline
    Pipeline --> Dispatcher
    Dispatcher --> Keyless
    Dispatcher --> Keyed
    Keyless --> RRF
    Keyed --> RRF
    RRF --> Pipeline
```

---

## 3. Provider Specification & Free Key Governance

### 3.1 Integrated Provider Catalog

| Provider ID | Auth Model | Free Quota / Limits | Primary Utility | Failure Behavior |
| :--- | :--- | :--- | :--- | :--- |
| `wikipedia` | **Keyless** | Unlimited | High-level conceptual definitions, history, overviews. | Fail-open (empty list on 4xx/5xx). |
| `openalex` | **Keyless** (opt key) | 100k requests/day free | 250M+ academic papers, inverted abstract reconstruction. | Fail-open (returns empty list). |
| `arxiv` | **Keyless** | 3 req/sec rate limit | Preprints across CS, AI, mathematics, physics. | Fail-open (returns empty list). |
| `tavily` | **Keyed** (Clavis) | 1,000 requests/month free | Live web search, clean markdown extraction. | Fail-open if unconfigured or quota spent. |
| `searxng` | **Keyless / URL** | Unlimited (self-hosted) | General multi-engine web search meta-retrieval. | Fail-open if URL unreachable. |
| `duckduckgo`| **PRUNED** | Deprecated endpoint | *Pruned from fan-out; legacy stubs retained for tests*.| N/A |

### 3.2 Free Tier Metadata Catalog (`vox-secrets`)
All API keys and direct acquisition URLs are ledgered in `vox-secrets` (Clavis vault). Raw keys are write-only and never leaked into DOM states or client DTOs.

```rust
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

Validated acquisition endpoints:
- **Tavily**: `https://app.tavily.com/sign-up` (1,000 searches/mo, no credit card required)
- **Google Gemini**: `https://aistudio.google.com/app/apikey` (free tier rate limits, no credit card required)
- **OpenRouter**: `https://openrouter.ai/keys` (free tier models available)
- **Semantic Scholar**: `https://www.semanticscholar.org/product/api` (free academic rate limits)

### 3.3 SQLite Quota Persistence (`vox_db`)
Table `provider_quota_usage`:
```sql
CREATE TABLE IF NOT EXISTS provider_quota_usage (
    provider TEXT NOT NULL,
    period_key TEXT NOT NULL,       -- 'YYYY-MM'
    units_spent INTEGER NOT NULL DEFAULT 0,
    units_limit INTEGER NOT NULL DEFAULT 1000,
    last_synced_at TEXT NOT NULL,
    PRIMARY KEY (provider, period_key)
);
```

---

## 4. Dual Lanes, Dynamic Escalation & Mathematical Formulations

### 4.1 Dual Lane Specifications

| Parameter | Fast Lane (`⚡ Fast`) | Deep Research Lane (`🔬 Deep`) |
| :--- | :--- | :--- |
| **Max Timeout** | 1,500 ms hard cutoff | 4,000 ms per provider (cumulative ~30–60s) |
| **Search Hops** | Single hop (parallel fan-out) | Multi-hop iterative CRAG (1–3 waves) |
| **Query Expansion** | None (direct query dispatch) | BM25 IDF subquery fan-out ($n = 3–8$) |
| **Target Sources** | 3–5 sources | 15–30+ sources across $\ge 2$ domains |
| **Claim Verification** | Fast heuristic / keyless cross-check | Full NLI log-odds & resample stability |
| **Cost & Latency** | $< 1.5\text{s}$, 0 token cost | $15–60\text{s}$, multi-round reasoning budget |

### 4.2 Dynamic Auto-Escalation Engine
When in **Balanced (Default)** mode, the pipeline dynamically determines whether to escalate from Fast to Deep Research based on two signals:

1. **Query Information Entropy $\mathcal{H}(Q)$**:
   $$\mathcal{H}(Q) = -\sum_{i=1}^{n} P(w_i) \log_2 P(w_i)$$
   If $\mathcal{H}(Q) \ge 2.50$, the topic is multi-faceted, triggering subquery fan-out.
2. **Epistemic Contradiction Entropy $\mathcal{H}_{\text{verdict}}$**:
   $$\mathcal{H}_{\text{verdict}} = -\sum_{k \in \{\text{sup}, \text{cont}, \text{ref}\}} \hat{\theta}_k \log_2 \hat{\theta}_k$$
   Where $\hat{\theta}$ is the posterior veracity distribution under the Dirichlet conjugate prior:
   $$P(\boldsymbol{\theta} \mid \boldsymbol{\alpha}) = \frac{1}{\mathrm{B}(\boldsymbol{\alpha})} \prod_{k=1}^{3} \theta_k^{\alpha_k - 1}$$
   If $\mathcal{H}_{\text{verdict}} > 0.90$ (sources conflict on benchmarks or claims), the engine auto-escalates to Deep Investigation, dispatches targeted verification subqueries, and upgrades the verifier model.

### 4.3 True Reciprocal Rank Fusion (RRF)
$$\text{RRF}(d \in \mathcal{D}) = \sum_{m \in M} \frac{1}{k + r_m(d)} \cdot \omega_m$$
Where $k = 60$, $r_m(d)$ is the 1-based rank under ranker $m$, and $\omega_m$ is the source authority multiplier ($\omega_{\text{arxiv}} = 1.2$, $\omega_{\text{openalex}} = 1.1$, $\omega_{\text{wiki}} = 1.0$, $\omega_{\text{web}} = 1.0$).

---

## 5. Information Architecture & User Experience

### 5.1 Scene A: Chat View
- **Composer**: Unified Drive Profile button (`⚡ Fast` · `⚖️ Balanced` [Default] · `🧠 Deep Genius` · `⚙️ Custom`).
- **ResearchMicroCard**: Inlined only when research occurs during a turn:
  ```
  🔬 Fast Lane: 5 keyless sources (arXiv, Wikipedia) · 2 claims verified · 420ms  [ View Details ▾ ]
  ```
- **Killswitch**: Respects `VOX_CHAT_RESEARCH_ENABLED`. When disabled via Settings or env, chat turns execute without web retrieval for offline development.

### 5.2 Scene B: Knowledge Hub
- **Tab 1: Research Lab**:
  - Segmented Lane Switcher (`⚡ Fast` vs `🔬 Deep Research`).
  - Source Quota Badges (`Wikipedia: Free`, `arXiv: Free`, `OpenAlex: Free`, `Tavily: 842/1k`).
  - `Configure Sources & Free Keys` button triggering `ResearchEngineDrawer` (`z-50`).
  - Headline banner displaying plain-language evidence consensus.
  - Comprehensive 5-section synthesis report (Executive Summary, Architectural Tradeoffs, Grounded Claims, Contested Findings, Practical Implications).
  - Epistemic DAG Canvas (`c1: Supported`, `c3: Contested`).
  - Low-Evidence Guard (`[data-testid="empty-results-notice"]`) when evidence confidence $< 0.35$.
- **Tab 2: Project Knowledge & Docs Explorer**:
  - Full-text search over `docs/`, ADRs, and historical research sessions.
  - One-click "Publish to Architecture SSOT".
- **Tab 3: Epistemic & Pipeline Debugger**:
  - Interactive stepper across all 8 stages (`queued` $\rightarrow$ `completed`).
  - Live Source Prober with live latency badges.
  - Invariant Sentries (Honesty, ErrorLeak, Occlusion, Watchdog) with emerald compliance shield.

### 5.3 Scene C: Global Status Bar
- Global pill: `● Systems 4/4 Ready` + Active Profile `[ ⚖️ Balanced ]`.
- Clustered Popover:
  1. **Web & Retrieval**: Status and latency of OpenAlex, arXiv, Wikipedia, Tavily.
  2. **Inference & Models**: Active model, fallback readiness, token velocity.
  3. **Indices & Storage**: Tantivy index, Qdrant vector store, SQLite quota DB.
  4. **Workspace & Runtime**: Daemon status, Actor runtime, VCS status.

### 5.4 Multi-Layer Modal Safety
- `z-0`: Base Viewport (Chat, Knowledge, Settings).
- `z-45`: `InspectorDrawer` (Ephemeral stage payloads and sentry logs).
- `z-50`: `ResearchEngineDrawer` (Free key acquisition, source toggles, quota sync) with circular focus trap and isolated `Escape` handling.

---

## 6. SSOT Plain-Language & Action Registry

All stage titles, plain-English summaries, rationale descriptions, action tooltips, and mathematical drawer metadata are declared in a single source of truth at `crates/vox-gui/ui/src/debugger/researchExplainerRegistry.ts`:

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
```

This guarantees that whenever search algorithms or thresholds are updated, the user-facing plain English and the developer mathematical drawer update in lockstep without string duplication.
