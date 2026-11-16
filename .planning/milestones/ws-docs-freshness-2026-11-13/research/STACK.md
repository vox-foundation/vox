# Stack / Tooling Research — docs-freshness (cost vs benefit)

**Researched:** 2026-10-08 · **Mode:** comparison · **Overall confidence:** MEDIUM-HIGH
(prices fetched from vendor pages on 2026-10-08; items marked *[unverified]* were not confirmed from a primary source)

**Recommendation: Hybrid that leans heavily on building — extend the in-house pipeline (vox-doc-pipeline +
graphify + the dormant Docs Reality Audit) for drift detection, generated references, pruning, and badges;
use a GitHub Actions LLM agent, called through the Vox facade, to draft PRs. Do not migrate off Starlight.
Optionally trial one hosted drift bot (Dosu free tier, or Promptless via its OSS program) for 30 days as a
benchmark — not as the system of record.**

Outcome key used below: **D** drift detection · **G** generated reference · **A** agent-drafted fix PRs ·
**P** corpus pruning · **B** reader freshness badges.

---

## 1. Paid / hosted platforms

| Product | Outcomes | Price at our scale (~670 pp, small team, public Apache-2.0 repo) | Lock-in / migration from Starlight | Fit with Rust + custom Vox language |
|---|---|---|---|---|
| **Mintlify** | A (agent + "Automations": *Update from code changes*, run on schedule or per merged PR, opens PRs), partial D, B none | Starter $0 (no agent/automations). **Pro $450/mo** includes the agent, the assistant, and automations; 10k credits/mo; automations cost **250 credits per update (free when nothing changes)**; overage $0.01/credit. OSS program: Pro free *only for non-commercial projects that are not VC-backed and not maintained by a company* — eligibility for Vox is unknown | **High.** It's a hosted renderer: MDX + `docs.json`, its own components. Means rewriting the frontmatter sidebar, the `vox` Shiki grammar, the remark include plugin, and the pagefind setup. Doctests and `vox ci` gates stay in our CI either way | Agent reads repos generically, so it works on Rust. Does not understand Vox semantics, `vox` doctests, or our generated-file rule ("fix at the generator") |
| **GitBook** | A (GitBook Agent, Ultimate only), Git Sync | Free (1 user, Git sync). Essential $65/site/mo; **Ultimate $249/site/mo** + $12/user, includes $100 of AI credits (annual pricing) | **High.** Block editor round-trips through Git Sync and rewrites markdown; fights `vox:skip` fences and generated files | Agent is generic. No code-coupled drift detection |
| **ReadMe** | API-reference oriented; "Docs Audit" (preview), "GitHub AI Writer", AI linter | Ask AI is +$150/mo; base tiers exist but the page gave no dollar amounts *[unverified]* | **High.** Built around OpenAPI and a hosted editor | **Poor.** Value comes from OpenAPI, and Vox has no OpenAPI surface |
| **Fern** | G (OpenAPI/gRPC/GraphQL to SDK and docs), Fern Agent | **Free: 10 members, 1,000 pages, 250 AI credits/mo.** Enterprise is custom | **High** (its own docs config and renderer) | **Poor.** The generator needs an API spec. The free hosting tier is the only draw |
| **Swimm** | D (code-coupled docs; historically auto-synced snippets) | **Enterprise-only, priced by lines of code**; now positioned for mainframe/COBOL modernization. No self-serve tier | High (proprietary doc format/IDE plugin) | Generic; no Vox parser. **Not a fit** |
| **DeepWiki / Devin Wiki** | Generates a parallel wiki (not *our* docs); `.devin/wiki.json` steers it | **Free for public repos** (low effort). Medium/high effort costs 5–40 ACUs per wiki on a paid Devin plan | None, because it sits alongside our docs. It doesn't fix our docs | Generic code reading. Useful as a **free cross-check oracle** ("what does the code say X does?"), not as a fixer |
| **Promptless** | A + D (PR/commit triggers, opens doc PRs, "replay recent PRs" calibration) | Startup $500/mo (≤200 pages). **Growth $1,000–1,500/mo for 500–1,000 pages**. **Free for eligible non-commercial OSS projects** (CNCF, LF, and others; apply by email) | **Low.** Works docs-as-code on our repo and opens PRs against `docs/src/` | Generic LLM reading. Needs our style/generated-file rules taught through config |
| **Dosu** | A + D ("self-documenting PRs", syncs existing docs in place, MCP) | **Free: 200 credits/mo, public repos only.** Pro $16/mo (4,000 credits). An OSS maintainer program exists | **Low** (in-repo PRs) | Generic. Free tier is the cheapest way to benchmark a hosted bot |
| **Theneo** | G for REST APIs (OpenAPI-driven) | Not fetched *[unverified]* | High | **Not a fit** (no API spec) |
| **DeepDocs / DocuGardener / Pushpen** (2025–26 GitHub-app drift bots) | D + A on every PR | DocuGardener: open source and self-hostable, hosted tier ~5 repos / 500 analyses *[price unverified]*. DeepDocs/Pushpen: SaaS *[price unverified]* | Low | Generic; young products with small user bases (single-maintainer risk) |

**Takeaway.** No paid product resolves claims against *Vox semantics*, meaning the compiler, the `vox` CLI
surface, contracts, or graphify. They all do the same generic thing: an LLM reads the diff and the docs. That
is exactly the part we can rebuild cheapest. The products that host the site (Mintlify, GitBook, ReadMe, Fern)
add a migration tax and give us nothing for outcomes G, P, or B that we don't already have.

## 2. Open-source / self-hostable

| Tool | Outcomes | Cost | Status in repo / recommendation |
|---|---|---|---|
| **Astro Starlight** (current) | Host, B (built-in `lastUpdated`), sidebar from frontmatter, pagefind | $0 | **Keep.** Component overrides (`PageTitle`, `MarkdownContent`) can render a frontmatter-driven "verified against <sha> on <date>" badge |
| **starlight-llms-txt** 0.10.0 | `llms.txt`, `llms-full.txt` | $0 | **Already installed.** Extend it by excluding `status: research/roadmap/deprecated` and prepending drift warnings to `llms-full.txt` |
| **starlight-links-validator** | Internal links checked at build time | $0 | Overlaps with `vox ci check-links`. Skip unless the gate misses anchors |
| **lychee** (+ lychee-action) | External link rot | $0, Rust, Apache-2.0 | **Add to the weekly scheduled job**, not to PRs, because external links are flaky. Wrap it in a `.vox` script |
| **Vale** | Prose style/terminology lint (retired names, banned hedges) | $0 | **Add**, scoped to non-archive files. Encode retired surfaces (`vox-dei`, `@endpoint`, `TURSO_URL`…) as a Vale vocabulary *or* keep them in `retired-symbol-check`. Pick one SSOT |
| **markdownlint** | Formatting | $0 | Already runs on contributors only; widen it gradually |
| **Doc Detective** | Docs-as-tests (runs procedures from docs) | $0 but **AGPL-3.0**, browser/UI-centric | **Skip.** `vox` doctests and `doctest-md --strict` already cover executable docs. Extend those to CLI transcripts instead |
| **Docusaurus / MkDocs Material** | Alternative hosts | $0 | **No benefit** over Starlight. A migration only costs us |
| **DocPulse** (tree-sitter code↔doc link graph + LLM verifier/repairer, MIT-style GH Action) | D + A | $0 + LLM | **Pattern to copy, not adopt.** It supports only Python/TS/C#. Our equivalent of its link graph is graphify plus frontmatter `related_files` |
| **DriftDoc** (`related_files` frontmatter, 15 languages incl. Rust, BYO model) | D + A | $0 + LLM | **Pattern to copy:** a `related_files:` / `verified_against:` frontmatter contract gives near-zero-false-positive matching |
| **msiric/autodocs** (deterministic sync, LLM detect, FIND/REPLACE, deterministic verify) | D + A | $0 + LLM | **Best design reference.** Each REPLACE is checked against source and labeled EVIDENCED / MISMATCH / UNVERIFIED; only EVIDENCED edits are auto-applied to a draft PR |
| **claude-code-action** (MIT, 8.9k★), OpenHands, Copilot coding agent | A (agentic PR drafting on runners) | $0 action + model spend | **Policy conflict:** each calls a vendor endpoint directly, bypassing `vox_actor_runtime::llm`. Use one only as the *harness* if it can point at a facade-compatible endpoint. Otherwise drive drafting from a `vox` subcommand |

GitHub Actions minutes on standard hosted runners are **free for public repos**, so CI compute is ≈ $0. The
only real marginal cost of building in-house is LLM tokens.

## 3. In-house LLM spend estimate (through the model-agnostic facade)

Token prices (USD per 1M tokens, input/output, list prices as of 2026-09-30 / 10-03; sources below): Gemini 3.8
Flash $0.75/$3.75 (promotional through 2026-12-31, then $1.50/$7.50); DeepSeek V4.1 Flash $0.30/$1.20 (peak; off-peak is half); Claude
Sonnet 5.5 $2/$10; Claude Opus 5.5 $4/$20; Claude Haiku 4.5 $1/$5. Cache reads are about 10% of the input
price.

Assumptions: ~200 merged PRs/month (1,113 crate-touching commits in 60 days). A deterministic pre-filter
(changed paths → `related_files`/graphify → candidate doc sections) caps each check at ≤6 sections.

| Workload | Tokens per unit | Cheap tier (Flash-class) | Mid tier (Sonnet-class) |
|---|---|---|---|
| Per-PR drift check (200/mo; ~60% skipped by the deterministic filter, so ~80 LLM calls) | 30k in / 2k out | ~$0.03 → **$2–3/mo** | ~$0.08 → **$6–7/mo** |
| Draft fix PRs (agentic, ~30/mo) | ~300k in (mostly cached) / 20k out | ~$0.15 → **$5/mo** | ~$0.50–0.80 → **$15–25/mo** |
| Weekly full sweep, 668 live pages × 4/mo | ~10k in / 1k out per page | ~$7.5/sweep → **$30/mo** | ~$20/sweep → **$80/mo** |
| Weekly sweep, incremental (only pages whose linked code changed, ~25%) | same | **~$8/mo** | **~$20/mo** |
| One-off pruning audit (all 668 pages, classify keep/merge/archive) | ~15k in / 1.5k out | **~$12 once** | **~$35 once** |

**Monthly range: about $15–40 with cheap models plus an incremental sweep; about $50–110 with a mid-tier model and
a full weekly sweep; at most about $200 if Opus-class drafts everything.** Batch-API discounts (~50%) would cut the
sweep further if the facade gains a batch path *[facade batch support unverified]*. By comparison, Mintlify Pro
is $450/mo and Promptless Growth is $1,000–1,500/mo.

## 4. Mapping the five outcomes to the cheapest adequate tool

| Outcome | Build on existing | Buy | Verdict |
|---|---|---|---|
| **D** drift detection | `vox audit docs`: extract claims (CLI flags, crate/symbol names, env vars, contract paths, `status: current`) and resolve them against `command-sync` output, graphify, contracts, and `retired-symbol-check`. Write findings JSONL into the Docs Reality Audit taxonomy | Mintlify/Promptless/Dosu do diff→LLM only, with no deterministic claim resolution | **Build.** Deterministic first; LLM only for prose claims |
| **G** generated reference | Already have 4 generators plus `ssot-autoregen`. Add generators for config/env vars (`contracts/config/env-vars.v1.yaml`), the error catalog, crate map (graphify), and stdlib builtins | Fern/ReadMe need OpenAPI, which we don't have | **Build** (no vendor can read Vox contracts) |
| **A** agent-drafted PRs | A scheduled `.vox` job: findings → facade LLM → FIND/REPLACE → deterministic verify → draft PR labelled `docs-drift` | Promptless (free if OSS-eligible), Dosu ($0–16), Mintlify ($450) | **Hybrid.** Build the path through the facade. Optionally benchmark Dosu/Promptless on a fork for 30 days |
| **P** pruning | `vox-doc-inventory` relevance + git staleness + inbound links + an LLM classification pass → audit report → case-by-case PRs | None does this well | **Build** (~$12–35 one-off) |
| **B** freshness badges | Frontmatter `verified_against`/`last_verified` set by the audit, rendered by a Starlight override; fix `lastUpdated` (see pitfall) | Mintlify has none comparable | **Build** (pure Starlight) |

## 5. Recommendation matrix

| Criterion | Build-on-existing | Buy (Mintlify Pro / Promptless) | **Hybrid (recommended)** |
|---|---|---|---|
| Cash cost / month | $15–110 LLM | $450 (Mintlify) to $1,000–1,500 (Promptless), or $0 if OSS-eligible | $15–110 LLM + $0–16 benchmark |
| Migration cost | None | Mintlify/GitBook: rewrite site, grammar, sidebar. Promptless/Dosu: none | None |
| Understands Vox semantics | **Yes** (compiler, CLI registry, contracts, graphify) | No | Yes |
| Respects repo policy (facade, `.vox`, generated-file rule) | Yes | No (vendor LLM, vendor bot edits generated files unless taught) | Yes (bot only as a benchmark) |
| Time to first value | 2–4 phases | Days | Days for the benchmark, phases for the core |
| Vendor risk | None | Pricing churn; OSS eligibility uncertain | Contained |

**Why hybrid wins.** Most of the value is in *deterministic* claim resolution against Vox-specific truth
sources (CLI registry, contracts, graphify, compiler doctests), and no vendor can buy us that. The generic
"LLM reads diff, proposes doc edit" part is cheap to run ourselves (< $40/mo at cheap tiers). It also stays
inside the mandatory LLM facade, and model selection can upgrade it automatically. A 30-day hosted-bot
benchmark (Dosu free tier, which works on public repos, or Promptless if its OSS program accepts Vox) gives us an
independent precision/recall yardstick for our own drafts at near-zero cost. Revisit "Buy" only if, after the
core phases, the drafted-PR acceptance rate stays under ~30% while a vendor's is well above it.

## 6. Pitfalls found during stack review

- **"Last updated" is probably wrong on voxlang.org today.** `docs-deploy.yml` uses `actions/checkout@v7`
  without `fetch-depth: 0` (shallow clone). Starlight's `lastUpdated: true` reads git history, so every page can
  show the deploy commit's date or none at all. Content is also reached through a symlink
  (`docs-astro/src/content/docs → docs/src`), which may break git-log path resolution *[verify with a local
  build]*. Fix this before building any badge, or the badge misleads.
- `lastUpdated` measures *edited*, not *verified*. A badge needs an audit-written `last_verified` field.
  Otherwise trivial edits make stale pages look fresh.
- Vendor GH Actions (claude-code-action, DocPulse, knowledge-diff) take `ANTHROPIC_API_KEY`/`OPENAI_API_KEY`
  directly, which violates the LLM-boundary and secrets policy. Use them as design references only.
- Hosted bots will "fix" `*.generated.md` and archive files unless they're explicitly excluded. Include those
  exclusions in any benchmark config.
- Mintlify's OSS program excludes company-maintained or VC-backed projects. Confirm Vox's status before
  assuming $0.

## Sources (fetched 2026-10-08)

| Source | Confidence |
|---|---|
| https://www.mintlify.com/pricing ; https://www.mintlify.com/docs/automations ; https://github.com/mintlify/docs/blob/28fdfba3/automations/reference.mdx ; https://www.mintlify.com/oss-program | HIGH (vendor primary) |
| https://www.gitbook.com/pricing | HIGH |
| https://readme.com/pricing (base tier $ amounts not rendered) | MEDIUM |
| https://buildwithfern.com/pricing | HIGH |
| https://swimm.io/pricing | HIGH (no public price) |
| https://docs.devin.ai/work-with-devin/deepwiki | HIGH |
| https://promptless.ai/pricing ; https://promptless.ai/oss | HIGH |
| https://dosu.dev/pricing | HIGH |
| https://starlight.astro.build/resources/plugins/ | HIGH |
| https://github.com/lycheeverse/lychee-action ; https://github.com/doc-detective/doc-detective ; https://github.com/anthropics/claude-code-action | HIGH |
| https://github.com/msiric/autodocs ; https://github.com/YoniRaviv/DocPulse ; https://github.com/prabhakarnit/DriftDoc ; https://docugardener.dev/ ; https://deepdocs.dev/ | MEDIUM (project READMEs, small projects) |
| https://tiktokenizer.org/llm-api-pricing (verified against provider pages 2026-09-30) ; https://llmprice.gitlab.io/ (2026-10-03) | MEDIUM (aggregators) |
| Repo: `docs-astro/package.json`, `astro.config.mjs`, `scripts/setup-content.mjs`, `.github/workflows/docs-deploy.yml` | HIGH (inspected) |
