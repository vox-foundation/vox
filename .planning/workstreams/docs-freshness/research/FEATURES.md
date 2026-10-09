# Feature Landscape — Self-Maintaining Docs (2024–2026 SOTA)

**Workstream:** docs-freshness (v1.3) · **Dimension:** FEATURES · **Researched:** 2026-10-08
**Confidence:** MEDIUM-HIGH (vendor docs + peer-reviewed/arXiv papers read directly; repo claims verified by grep)

Outcome key (from CONTEXT.md): **O1** drift detection → CI/PR/llms.txt · **O2** generated reference ·
**O3** agent-drafted fix PRs · **O4** case-by-case pruning · **O5** reader freshness badges.
Vox status: ✅ have · 🟡 partial · ❌ lack.

## 1. The industry pattern in one paragraph

Mature docs teams stopped treating accuracy as a writing problem and made it a **verification problem**:
(a) anything derivable from code is *generated* (Stripe: OpenAPI → API ref + SDK snippets, validated in the
same PR as the API change); (b) anything executable is *tested* (rustdoc doctests, Python doctest, Doc
Detective "docs as tests"); (c) prose that mentions code is *anchored* to symbols so a code change flags the
doc (Swimm Auto-sync, DOCER); (d) the remainder carries *ownership + review-by dates* (Google `freshness:`
comments, GitLab triage boards); and since 2025, (e) an **LLM agent watches merged code PRs and opens doc PRs
for human review** (Mintlify Automations, DeepDocs, Dependadocs), while (f) docs are *served to agents* via
llms.txt + per-page markdown + a hosted MCP endpoint (Mintlify, GitBook). Vox has strong (a)/(b) for its
language surface and a solid llms.txt, but lacks (c), (d)-as-enforced, (e), and the public half of (f).

## 2. Table Stakes (missing = docs rot silently)

| Feature | Who does it | Outcome | Vox | Notes |
|---|---|---|---|---|
| Executable code samples tested in CI (doctests) | Rust rustdoc, Python doctest/Sphinx | O1 | ✅ | `vox ci doctest-md --strict`, `vox-doc-pipeline` doctest for ```vox fences; `// vox:skip` escape hatch. |
| Snippets **included from tested source**, not pasted | mdBook `{{#include file:anchor}}`, Sphinx `literalinclude`, Docusaurus raw-loader | O1/O2 | ✅ | `remark-vox-include.mjs` + pipeline errors on missing file/anchor (`pipeline/mod.rs:393-408`). Policy says prefer inline fences for new code — reconsider (see §6). |
| Internal + external link checking | mdBook-linkcheck, lychee, Stripe Markdoc validator | O1 | ✅ | `vox ci check-links`, `anchors.rs`. |
| Structured frontmatter schema lint | Markdoc schemas, Starlight content collections | O1/O5 | ✅ | `VALID_CATEGORIES`, required keys; `last_updated` hand-authoring is an error (git-derived). |
| Reference generated from code/contracts | Stripe OpenAPI→ref; K8s `gen-apidocs`; Sphinx autodoc | O2 | 🟡 | 4 generated refs (CLI surface, plugin catalog, bundles, mens defaults) + `ssot-drift`/`ssot-autoregen`. Gaps: config keys/env vars, contracts/*.yaml schemas, MCP tool catalog, stdlib/builtins, error codes. |
| Docs change in the same PR as code ("docs DoD") | GitLab (tech-writer review in code MR), Stripe | O1 | 🟡 | CODEOWNERS covers only `/docs/src/architecture/`; no rule "crate X changed → doc Y must be touched or acknowledged". |
| Ownership per page | Google (SWE book ch.10: "documents without owners become stale"), Fuchsia `last_reviewed_date` | O4/O5 | ❌ | No `owner:` frontmatter; 668 live pages, 58% architecture, effectively unowned. |
| "Last updated" shown to readers | Starlight, GitLab handbook proposal #367 | O5 | 🟡 | `lastUpdated: true` in `astro.config.mjs` (git commit date). Measures *touched*, not *verified* — misleading for honest-looking stale pages. |
| llms.txt + llms-full.txt | llmstxt.org v2 (thousands of sites; Lighthouse audits it; OpenAI/Anthropic/Gemini publish) | O1 | ✅ | Curated `docs/src/.well-known/llms.txt` + `starlight-llms-txt` plugin with `llmsFullTxt: true`. Risk: two sources (curated vs plugin) can disagree. |
| Archived/deprecated content de-indexed | Starlight `pagefind:false`, robots noindex | O4 | ✅ | `routeData.ts` noindexes `archive/`. `status: deprecated/legacy` pages are NOT noindexed or bannered. |

## 3. Differentiators (high leverage; mostly what Vox lacks)

| # | Feature | Evidence / who | Outcome | Vox | Complexity |
|---|---|---|---|---|---|
| D1 | **Code-symbol anchoring with deterministic CI check** — doc declares the symbols/paths it describes; PR changing/removing them flags the doc (auto-fix trivial renames, fail/annotate on substantive change) | Swimm Auto-sync (3 states: up-to-date / auto-syncable / outdated; GitHub check per PR); DOCER GitHub Action (28.9% of top-1000 GitHub repos have ≥1 outdated code-element reference — arxiv 2307.04291) | O1, O3 | ❌ | Med — extract inline-code identifiers + paths from md, resolve via `vox graph query` / rustdoc JSON / CLI registry; emit `vox audit docs` JSONL. |
| D2 | **Agent-drafted doc PRs triggered by merged code PRs** (batched, cites source PRs, assigns code author as reviewer, never auto-merge) | Mintlify Automations (beta 2026-03; "Update from code changes" twice-weekly batch or per-PR; reviewers = source PR authors); DeepDocs (per-commit, separate branch, report); Dependadocs (OSS, Actions-only, BYOK, "only flag factually incorrect, ignore style") | O3 | ❌ | Med — scheduled workflow: merged-PR diffs since last run → D1/D3 findings → LLM via `vox_actor_runtime::llm` → draft PR. Batching avoids PR spam. |
| D3 | **LLM claim verification with false-positive control** — extract factual claims from prose, verify against code; precision over recall | DocPrism (arxiv 2511.00215): naive prompting flags 90–98% of functions; structured "local categorization" cut flag rate to 14–19%, F1 0.22→0.77, ≥11% of documented methods truly inconsistent. CASCADE (arxiv 2604.19400): LLM generates tests from docs, reports only when tests fail on code — execution-gated precision. Metamon (arxiv 2502.02794): metamorphic queries. | O1, O3 | ❌ (Docs Reality Audit taxonomy exists but dormant: 0 findings since 2026-05-12) | High — must be gated: deterministic D1 first, LLM only on pages D1 can't cover; route results into existing `docs-reality-audit` findings JSON. |
| D4 | **Review-by / freshness metadata with owner + expiry nudges** | Google `<!--* freshness: {owner, reviewed} *-->` + email after ~3 months, "Last reviewed by…" byline increased adoption (abseil.io SWE book ch.10); Fuchsia `last_reviewed_date`; freshness-spec (2026, "freshness is not correctness") | O4, O5 | ❌ | Low — add `owner`, `last_verified`, `review_by` (or `review_cadence`) to frontmatter schema; scheduled job opens issue/PR listing expired pages. |
| D5 | **Reader-visible trust badge** derived from verification, not mtime ("Verified against v0.6.0 · 12 days ago" / "Research — may not reflect shipped code" / "Stale: 3 referenced symbols removed") | GitLab handbook #367 (Wikipedia-style maintenance templates); Starlight banners | O5 | ❌ | Low-Med — Starlight route middleware (pattern already in `routeData.ts`) reads frontmatter + a generated `doc-health.json` from D1/D4. |
| D6 | **Docs-as-tests for procedures** (tutorial/how-to steps executed: shell commands, CLI output, HTTP) | Doc Detective (OSS, GitHub Action, JSON results; docsastests.com) | O1 | 🟡 | Vox doctests compile code but don't run tutorial *shell steps* (`vox new`, `vox run …`). Med; scope to tutorials (7) + how-to (38). |
| D7 | **Hosted docs MCP endpoint + per-page `.md` + agent feedback tool** ("report outdated page") | Mintlify `/mcp` (search, read page, submit feedback → analytics), `/.well-known/mcp`; GitBook `/~gitbook/mcp` on every site; llmstxt v2 `rel="alternate" type="text/markdown"` | O1 | 🟡 | `vox-search` ingests `docs/src` internally; no public endpoint, no per-page markdown links, no agent-feedback channel. A feedback tool turns every agent consumer into a drift sensor. |
| D8 | **Freshness-aware llms.txt** — exclude/flag `research`/`roadmap`/stale pages from agent-facing indexes | freshness-spec: stale retrieval gives old docs "synthetic authority" | O1, O5 | ❌ | Low — llms-full currently concatenates everything the plugin sees; filter by `status` + D4 expiry. |
| D9 | **Same-PR doc-impact check** ("this PR touches `crates/vox-cli/src/commands/run*` → these 6 pages reference it; touch one or add `docs-impact: none`") | GitLab docs DoD; Swimm PR check | O1 | ❌ | Low once D1's reverse index exists; PR comment, non-blocking initially. |
| D10 | **Diátaxis-typed corpus with per-type rules** (tutorial/how-to/reference/explanation; reference must be generated; explanation may be human) | diataxis.fr (adopted by Canonical, Django, Cloudflare-style IA) | O2, O4 | 🟡 | Sections already mirror Diátaxis (tutorials/how-to/reference/explanation) but 388 architecture pages sit outside it; a type→policy map drives what must be generated vs. verified vs. owned. |
| D11 | **Pruning signals report** (no inbound links, no traffic, references removed symbols, superseded-by, duplicate topic) → human decision per page | Google SWE book "designate canonical docs, deprecate duplicates"; GitLab monthly triage boards | O4 | 🟡 | `vox-doc-inventory` relevance scoring exists → extend to emit a prune/merge/keep candidate report; pairs with D1 (dead-symbol pages are top candidates). |

## 4. Anti-Patterns (explicitly do NOT build)

| Anti-pattern | Why it fails | Instead |
|---|---|---|
| **Auto-merging LLM prose** (Mintlify `automerge: true`, DeepDocs-style direct commits) | Hallucinated "fixes" become authoritative; violates scoping decision | Draft PRs only, code author auto-requested as reviewer. |
| **Naive "is this doc consistent with this code?" prompting** | DocPrism: 90–98% flag rate → alert fatigue, gate gets disabled | Deterministic symbol checks first; LLM with structured categorization, precision-first; execution-gated where possible (CASCADE). |
| **Blocking CI on LLM verdicts** | Non-deterministic gate on a 30-min PR budget; flaky = ignored | LLM runs scheduled/nightly (≤180 min) → findings + draft PRs; PR-time gate stays deterministic. |
| **"Last updated" = freshness** | A typo fix resets the clock on a wrong page; Vox already shows git dates | Separate `last_verified` set only by a verification event (human review or passing D1/D6). |
| **Per-commit doc agent runs** | PR spam, cost; Mintlify itself defaults to twice-weekly batching | Batch merged PRs since last run; append to open doc PR. |
| **Blanket archive / time-based auto-delete** | Scoping decision: case by case; "stale ≠ wrong" | Expiry → banner + triage report, human decides. |
| **Pasting example code into prose** | Silent drift, no compiler sees it | Include from golden/tested files or doctest every fence. |
| **Migrating to a hosted platform to get these features** | Mintlify/GitBook/Swimm features are mostly reproducible on Starlight + `vox ci`; migration costs Vox-specific doctests, grammar, include plugin | Borrow designs; compare cost honestly in STACK/COMPARISON research. |
| **Two hand-curated agent indexes** | Curated `llms.txt` vs plugin output drift | Generate llms.txt from frontmatter (curated *ordering* via `sort_order`/flag). |

## 5. Mapping to the 5 outcomes

| Outcome | Have | Highest-leverage additions |
|---|---|---|
| O1 drift → CI/PR/LLMs | doctests, includes, link check, ssot-drift, retired-symbol-check | D1 symbol anchoring, D9 PR impact comment, D8 freshness-aware llms.txt, D7 agent feedback |
| O2 generated reference | 4 generators + autoregen bot | Env vars (`contracts/config/env-vars.v1.yaml`), contracts schemas, MCP tools, builtins, error codes (Stripe model) |
| O3 agent-drafted PRs | LLM facade, orchestrator model selection, `vox audit` JSONL | D2 batched scheduled agent fed by D1/D3 findings |
| O4 case-by-case pruning | doc-inventory relevance, Docs Reality Audit taxonomy (dormant) | D11 prune report + D4 owner/expiry; revive audit as the findings sink |
| O5 reader badges | Starlight git `lastUpdated` | D4 `last_verified` + D5 status/verification banner middleware |

## 6. Feature Dependencies

```
D4 frontmatter (owner, last_verified, review_by) ──► D5 badges ──► D8 freshness-aware llms.txt
D1 symbol extraction + resolution (vox graph) ──► D9 PR impact comment
                                              ──► D11 prune report (dead-symbol pages)
                                              ──► D3 LLM verification (only on residue D1 can't decide)
D1 + D3 findings (docs-reality-audit JSON) ──► D2 agent-drafted PRs ──► (human merge sets last_verified)
D10 type→policy map ──► decides which pages need D1/D6 vs owner-review only
```

Policy note: AGENTS.md says "do NOT use `{{#include}}` for new code" (prefers inline fences that are
doctested). Both are table stakes; inline+doctest is fine for Vox snippets, but Rust/TS/CLI-output snippets
have no doctest path — includes (or generated blocks) are the only drift-proof option there.

## 7. MVP Recommendation (feature-level; phase order is for SUMMARY/roadmap)

1. **D4 + D5** (cheap, immediately honest UI): frontmatter `owner`/`last_verified`/`review_by`, banner for
   `research`/`roadmap`/`deprecated`/expired. Seeds O4/O5.
2. **D1 + D9** deterministic drift: code-element reference checker over inline code + paths, resolved via
   `vox graph` / CLI registry / contracts; `vox audit docs` JSONL; PR comment, warn-only first.
3. **D11** prune report from D1 + doc-inventory signals → human case-by-case decisions.
4. **D2** scheduled, batched agent opening draft PRs from D1 findings (+ merged-PR diffs).
5. **D3** LLM claim verification on high-traffic `current` pages only, precision-first, nightly.

Defer: D6 Doc Detective-style procedure execution (only 45 tutorial/how-to pages; doctests cover most
risk), D7 public MCP endpoint (llms.txt + per-page .md links get 80% of value first).

## Sources

- Doc Detective / Docs as Tests — https://github.com/doc-detective/doc-detective , https://docs.doc-detective.com/docs/introduction.mdx , https://www.docsastests.com/ (HIGH, primary)
- Swimm Auto-sync & CI — https://swimm.io/blog/how-does-swimm-s-auto-sync-feature-work , https://swimm.io/blog/swimm-native-integrations (MEDIUM, vendor)
- DOCER outdated code-element refs — https://arxiv.org/html/2307.04291 (HIGH)
- DocPrism — https://arxiv.org/html/2511.00215v2 ; CASCADE — https://arxiv.org/abs/2604.19400 ; Metamon — https://arxiv.org/html/2502.02794v1 ; LLM doc→code traceability — https://dl.acm.org/doi/pdf/10.1145/3819820 (HIGH, academic)
- Mintlify agent/automations/MCP — https://www.mintlify.com/docs/agent/index.md , https://www.mintlify.com/docs/automations , https://www.mintlify.com/blog/automations , https://www.mintlify.com/docs/ai/model-context-protocol (MEDIUM-HIGH, vendor docs)
- GitBook MCP / LLM-ready docs — https://gitbook.com/docs/ai-for-your-readers/mcp-servers-for-published-docs , https://gitbook.com/docs/getting-started/llm-ready-docs (MEDIUM-HIGH)
- DeepDocs — https://docs.deepdocs.dev/get-started/introduction/ ; Dependadocs — https://github.com/usr-wwelsh/Dependadocs (MEDIUM)
- llms.txt v2 — https://llmstxt.org/ (HIGH)
- Google freshness practice — https://abseil.io/resources/swe-book/html/ch10.html , https://webrtc.googlesource.com/src/+/HEAD/g3doc/how_to_write_documentation.md ; Fuchsia metadata — https://fuchsia.googlesource.com/fuchsia/+show/7dedc3f2bbbba618ff0f1cda6d9e67cbf3e6f98a/docs/development/source_code/metadata.md (HIGH)
- freshness-spec (experimental) — https://github.com/Hello-GregKulp/freshness-spec (LOW, single author)
- GitLab docs workflow / freshness proposal — https://handbook.gitlab.com/handbook/product/ux/technical-writing/ , https://gitlab.com/gitlab-com/content-sites/handbook/-/work_items/367 (HIGH / MEDIUM)
- Stripe Markdoc & API→docs flow — https://stripe.dev/blog/markdoc , https://stripe.dev/blog/how-api-changes-flow-into-stripes-developer-products.md (HIGH)
- Diátaxis — https://diataxis.fr/ (HIGH; not re-fetched this session, well-established)
- Repo verification: `docs-astro/astro.config.mjs` (starlight-llms-txt, lastUpdated), `docs-astro/src/routeData.ts`, `docs-astro/src/utils/git-dates.mjs`, `docs-astro/src/plugins/remark-vox-include.mjs`, `crates/vox-doc-pipeline/src/pipeline/*`, `.github/CODEOWNERS`, `contracts/reports/docs-reality-audit/metrics.v1.json` (0 findings), `crates/vox-search/src/ingest.rs`.
