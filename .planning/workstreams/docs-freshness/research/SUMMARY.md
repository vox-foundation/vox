# Research Summary — docs-freshness (v1.3 Self-Maintaining Public Docs)

**Synthesized:** 2026-10-08 from CONTEXT.md + STACK / FEATURES / ARCHITECTURE / PITFALLS
**Overall confidence:** MEDIUM-HIGH. The internal ground truth is HIGH: it was read from code, measured, or taken from `gh` run data. Vendor pricing and the 2026 studies are MEDIUM.

## Executive summary

Teams that keep docs accurate treat accuracy as a **verification problem**, not a writing problem. Anything derivable from code is generated. The rest is anchored to the code symbols it describes and checked deterministically in CI. An LLM is only used to draft fixes for findings that a deterministic check has already selected, and humans review them as draft PRs. Vox already has most of the substrate: doctests, includes, four generators plus `ssot-autoregen`, the LLM facade, the `vox audit` umbrella, graphify, and the Docs Reality Audit taxonomy. What's missing is the step that compares **doc content against code**. As a result, 383 pages say `status: current` and none of those claims is verified.

**The first job is not detection. It is getting the site to deploy.** voxlang.org has not deployed since 2026-05-12. Every run since 2026-09-20 has failed with Cloudflare `Authentication error [code: 10000]`, and issue #462 has collected 57 bot comments. Until a human rotates the token, every docs fix is invisible to readers and scrapers. Any freshness metric measures the repo, not the site. The second job is shrinking the corpus. 58% of pages are architecture, and roughly 270 are `research` or `roadmap`, so drift detection run across all 668 pages would drown in findings on dead notes. That is how the Docs Reality Audit died: it produced zero findings, sat behind a gate that could only pass, and nobody consumed its output.

The main risks all come from humans tuning out the signal: false positives, a flood of comments, LLM prose that is confidently wrong, prompt injection into a bot holding write tokens, and badges that show edit dates as if they were accuracy. The mitigations are consistent across all four research files. Pick findings deterministically. Gate pull requests only on reader-facing `current` pages, with a baseline ratchet. Run the LLM only on a schedule, inside a budget, through `vox_actor_runtime::llm`. Every drafted claim must cite an anchor that resolves and gets re-checked. Open draft PRs only, with a cap on how many are open at once. Split the work into an unprivileged LLM job and a privileged PR job.

## Build vs. buy verdict: hybrid, mostly build

- **Build:** drift detection, generated reference, pruning report, and badges. No vendor can resolve claims against the Vox CLI registry, its contracts, graphify, or `vox` doctests. Starlight stays; migrating to Mintlify or GitBook would mean rewriting the Vox Shiki grammar, the include plugin, the sidebar logic, and pagefind.
- **Buy (optional, as a benchmark only):** run Promptless (free if Vox qualifies as OSS) or Dosu against a **fork** for 30 days. Its suggestions serve as an independent precision/recall yardstick for our own drafts. It never gets write access to the real repo. Reconsider buying only if our draft acceptance rate stays clearly below the benchmark's.
- **Reconciled cost.** STACK's $15–40/mo cheap-tier figure includes about $2–7/mo of *per-PR LLM checks*. Those are dropped here, because no LLM runs on pull requests (see contradiction 2 below). What's left:
  - Incremental weekly verification sweep: about $8/mo on a cheap model, about $20 on a mid-tier one.
  - Draft PRs: about $5/mo cheap, $15–25 mid-tier.
  - One-off LLM pruning classification, if we use one at all: $12–35.
  - **Steady state is about $13–45/mo.** Gemini's promotional pricing ends 2026-12-31, which roughly doubles the cheap tier but keeps it under about $30. A full weekly sweep on Opus-class models has a ceiling of about $200/mo.
  - For comparison: Mintlify Pro is $450/mo and Promptless Growth is $1,000–1,500/mo. CI compute costs about $0 because hosted runners are free for public repos. The real cost is engineering time, and most of it goes into the deterministic layer, which delivers value with no LLM at all.

## Contradictions resolved

1. **Where freshness lives: frontmatter (FEATURES D4) or a generated ledger (ARCHITECTURE).** Both, split by who writes it.
   - Frontmatter holds only facts that people author and that rarely change: `owner`, `review_cadence`, and an optional `code_anchors`/`related_files` list. Each key is defined once in `vox-doc-pipeline`, with a parity test against the governance doc.
   - Machine verification state goes in the generated `contracts/reports/docs-freshness/ledger.v1.json`: anchor shas, `last_verified_commit`, and `state ∈ {verified, drifting, broken, unanchored, historical}`. That file is marked `linguist-generated`.
   - There is **no hand-written `last_verified` in frontmatter**. It would contradict the existing ban on hand-authored `last_updated` (`HandAuthoredLastUpdated`), and bot edits across 668 files would swamp diffs.
2. **LLM on pull requests.** ARCHITECTURE says "advisory on PR"; STACK prices per-PR LLM checks; PITFALLS and FEATURES forbid it. **Decision: no LLM on pull requests in v1.** At PR time, checks are deterministic only, plus an impact comment that is edited in place. All LLM work is nightly or weekly over a bounded top-K queue.
3. **Build order: ARCHITECTURE's "extractor first" vs. PITFALLS' "prune before drift detection."** These are compatible. The extractor runs **report-only**, with no gate, which is exactly the measurement the prune report needs. The human prune pass happens *before* any blocking gate or LLM layer. FEATURES' "D4+D5 first" is honoured partially: banners driven only by frontmatter (`research`, `roadmap`, `deprecated`) ship in Phase 1. Banners based on the ledger have to wait for it.
4. **Phase names.** PITFALLS' P0–P5 and ARCHITECTURE's steps 1–9 map to the phases below as follows:
   - P0 → Phase 1.
   - P1 → step 1 in Phase 2, steps 2–3 in Phase 3.
   - P3 → step 5 in Phase 2.
   - P2 → step 8 in Phase 4.
   - P5 → step 4 in Phase 5.
   - P4 → step 6 in Phase 6, step 7 in Phase 7.
   - Step 9, the graph corpus, is deferred.
5. **Prose-lint single source of truth.** STACK suggests Vale *or* `retired-symbol-check`. **Keep `retired-symbol-check` as the single source of truth for retired names.** Defer Vale.
6. **Docs Reality Audit and the overlapping specs.** Don't build a fourth design. Reactivate the audit's taxonomy and schemas by **generating** its findings. Formally merge or retire `2026-08-22-docs-corpus-repair*` and `2026-09-14-deep-research-documentation-engine*` in Phase 1.

## Suggested phases

| # | Phase | Goal | Depends on |
|---|---|---|---|
| 1 | **Deploy unblock & public-surface honesty** | Get voxlang.org deploying again and keep it honest. Rotate the Cloudflare token (user), add a liveness alert that escalates instead of re-commenting, use `fetch-depth: 0` and verify git dates through the symlink, add post-deploy smoke checks (llms.txt links resolve, `/voxup` serves the install script, no retired syntax), show banners and noindex for `research`/`roadmap`/`deprecated`, close #462, and merge or retire the overlapping specs. | none |
| 2 | **Measure & prune** | Build the deterministic mention extractor and resolvers in `vox-doc-pipeline::claims`, report-only. Generate auto-findings into the Docs Reality Audit, with a schema bump for `source`, `fingerprint`, and `detector`. Produce the prune/merge/keep report. Run the **human prune pass, case by case**. By default, `research` and `roadmap` pages are out of drift scope and out of the public sidebar. | 1 |
| 3 | **Deterministic drift gate & PR impact** | Build the anchor reverse index and a PR impact comment that is edited in place. Add the freshness ledger, hashing symbol spans rather than whole-file blob shas. Add a baseline ratchet that can only tighten. The gate blocks only on `status: current` pages under reference, tutorials, and how-to. Suppressions carry owner, reason, and expiry. Add a control-character lint. | 2 |
| 4 | **Generated reference expansion** | Turn hand-maintained pages into `*.generated.md` built from `env-vars.v1.yaml`, config keys, error codes, and the MCP tool catalog. Each generated page must name its reader. Every generated page must have a registered generator, and generators get tests for dead paths. | 2 (can run in parallel with 3, 5) |
| 5 | **Reader trust signals & agent surfaces** | Add `freshness.mjs` with ledger-driven banners, sidebar badges, and "Verified against `<sha>`". Add a `current` custom set and demote/exclude rules for llms.txt. Retire the hand-written `llms-full.txt`. CI checks that every curated llms.txt URL points to a `verified` page. | 3 |
| 6 | **LLM claim verification (nightly)** | Build `vox-doc-verify`, modelled on `vox-effort-route`, using the facade and the `DocClaimJudge` category. It only examines what the deterministic layer cannot decide. Verdicts use the audit taxonomy and must cite evidence lines that exist. Add a content-hash cache and a per-run budget cap. Build a golden eval set from past drift commits (`e2bb133a1`, `49a08f34e`, `f48dbc810`, …). | 2, 3; user-authorized edges |
| 7 | **Draft-PR bot** | Write `scripts/docs-fix-drafts.vox` and `docs-freshness.yml`. Use two jobs: an LLM job with read-only access that produces a patch artifact, and a PR job with no LLM. Copy the shape of the `ssot-autoregen` job. Restrict changes to `docs/src/**` excluding generated files. Re-lint, re-doctest, and re-run `retired-symbol-check` on every patch. Allow at most 3 open draft PRs, track precision, and optionally run the 30-day fork benchmark. Requires a security review before merge. | 6; PAT |

**Research flags:**
- **Needs `--research-phase`:**
  - Phase 3: the symbol-span hashing design, and measuring the false-positive rate before turning on blocking.
  - Phase 6: prompt design and categorization in the style of DocPrism, plus building the golden set.
  - Phase 7: workflow security, especially the `workflow_run`/artifact split and PAT scope.
- **Standard patterns, skip research:**
  - Phase 1: ops work.
  - Phase 4: the existing generator plus `ssot-autoregen` pattern.
  - Phase 5: Starlight `routeData` and the existing `git-dates.mjs` pattern.
- **Phase 2:** needs light research on the mention-classification rules, i.e. telling illustrative code apart from code that prescribes behaviour.

## Features

- **Table stakes, already present:** doctests, includes from tested source, link checks, frontmatter lint, llms.txt, archive noindex.
- **Table stakes, partial or missing:**
  - Generated reference for env, config, and errors.
  - A per-page owner.
  - Deprecated pages are not yet bannered or noindexed.
  - A rule that docs change in the same PR as the code they describe (a "docs definition of done").
- **Differentiators, in priority order:**
  - D1 code-symbol anchoring
  - D9 PR impact comment
  - D11 pruning report
  - D5 trust badges, driven by verification rather than edit time
  - D8 freshness-aware llms.txt
  - D3 precision-first LLM verification
  - D2 batched agent-drafted PRs
  - Deferred: D6 docs-as-tests for tutorials and how-tos, and D7 a hosted docs MCP endpoint with a feedback tool. Measure Cloudflare logs before building D7.
- **Anti-patterns, do not build:**
  - Auto-merging LLM prose.
  - Naive "is this consistent?" prompting, which flags 90–98% of items.
  - Blocking CI on LLM verdicts.
  - Treating "last updated" as freshness.
  - Running the agent on every commit.
  - Blanket or time-based archiving.
  - Migrating to a hosted platform.
  - Two hand-curated agent indexes.
  - Vendor actions that call LLM hosts directly (claude-code-action and similar), which bypass the facade.

## Top pitfalls → phases

| Pitfall | Prevention | Phase |
|---|---|---|
| Fixing the repo while the site is undeployed or dishonest | Restore deploy, run smoke checks against the *deployed* site, add liveness escalation | 1 |
| A program nobody runs (how the Docs Reality Audit died) | Findings are machine-generated only; metrics distinguish "empty" from "healthy"; liveness check on scheduled artifacts | 2, 3 |
| Corpus bloat drowning detection | Prune first; scope out `research`/`roadmap`; new `current` architecture pages need anchors | 2 |
| Checker false positives leading to a sprawl of ignores | Gate only reader-facing `current` pages; honour `vox:skip`/`text` fences; suppressions expire; tell "mentions" apart from "prescribes" structurally | 3 |
| Edit dates shown as accuracy | `verified-against` sha comes only from CI or human attestation; ignore mechanical commits | 3, 5 |
| Confidently wrong LLM prose | The deterministic layer picks what to fix; every claim cites an anchor that is re-verified; unverifiable claims are dropped; golden eval set | 6, 7 |
| Prompt injection into an agent holding write tokens | No `pull_request_target`; two-job split; `persist-credentials: false`; SHA pins; path allowlist; security review | 7 |
| Alert fatigue (#462 is the local example) | One report edited in place; cap of 3 open bot PRs; demote signals that fall below a precision floor | 1, 3, 7 |
| Cost blow-ups or denial-of-wallet | No LLM on pull requests; bounded queue; facade budget cap; hash cache | 6 |
| Flaky docs toolchain (starlight-llms-txt 0.11 broke the build) | Frozen lockfile; treat peer-dependency warnings as errors; don't upgrade the plugin just to get llms-full exclude | 1, 5 |

## Requires user authorization

1. **Cloudflare API token rotation** with `Pages:Edit` on `vox-docs`. This blocks everything else.
2. **Crate-edge exceptions:**
   - New L3 crate `vox-doc-verify`, plus its `layers.toml` and `where-things-live.md` rows.
   - Edges `vox-doc-verify → vox-actor-runtime, vox-doc-pipeline, vox-config`.
   - Edge `vox-cli → vox-doc-verify`.
   - Edge `vox-audit → vox-doc-pipeline`, or host `vox audit docs` in `vox-cli` instead.
   - Phases 1–5 need **no** new edges.
3. **A PAT secret for bot PRs**, either reusing `SSOT_AUTOREGEN_TOKEN` or adding a new scoped token. Pull requests opened with `GITHUB_TOKEN` don't trigger `docs-quality.yml`.
4. **Model-routing contract change:** a new `TaskCategory::DocClaimJudge` in `model-routing.v1.yaml`, plus a `vox.toml [audit.docs]` budget. The LLM spend envelope is about $13–45/mo.
5. **Gate policy:** make the drift gate blocking on reader-facing `current` pages, after the baseline is set.
6. **Per-page prune decisions** in Phase 2, and the overall policy on whether `research`/`roadmap` pages leave the public sidebar.
7. **Optional:** apply to the Promptless/Mintlify OSS programs, and run the 30-day fork benchmark.

## Open questions

- Does the content symlink in `setup-content.mjs` break git-date lookup even with `fetch-depth: 0`? This needs a local build to check.
- llms-full has no exclude hook in starlight-llms-txt 0.10. Options: upstream a patch, point agents at the `current` custom set, or generate the file ourselves.
- How does a human attest a page so the ledger re-stamps it? Candidates: a merged PR that touches the doc and passes the drift check, or an explicit attest command.
- Who are the owners for about 668 pages? CODEOWNERS currently covers only `docs/src/architecture/`.
- Should most of the 388 architecture pages be public at all, or moved to contributor-only or out of the sidebar?
- Is Vox eligible for the OSS programs? (Is it VC-backed or company-maintained?)
- Does the facade support batch-API calls (about 50% cheaper sweeps)?
- Do Cloudflare logs show real llms.txt traffic? Answer this before investing in D7 or D8.
- Can the anchor hash use graphify item spans reliably, or does it fall back to file sha, which churns on rustfmt?

## Sources

Aggregated from the four research files. Internal (HIGH):
- Commits `015f430ca`, `4e98a0f87`, `59e30f6b6`, `94822d35d`, `e2bb133a1`, `e32f81651`, `6d7c4c376`.
- Issue #462 and `gh run list docs-deploy.yml`.
- Crates `vox-doc-pipeline`, `vox-cli-ci/docs_reality_audit.rs`, `vox-effort-route`, `vox-audit`.
- `docs-astro/*` and `.github/workflows/*`.

External:
- Vendor pricing (HIGH): Mintlify, Promptless, GitBook, Dosu.
- arXiv (MEDIUM–HIGH): DocPrism 2511.00215, CASCADE 2604.19400, DOCER 2307.04291, 2606.09935, 2604.03196.
- CSA notes, the Ahrefs llms.txt study, and the Google SWE book ch.10.
