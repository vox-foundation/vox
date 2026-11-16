# Domain Pitfalls — docs-freshness

**Domain:** automated doc↔code freshness (drift detection, generated reference, LLM draft-PR agent, pruning, reader freshness signals)
**Researched:** 2026-10-08
**Overall confidence:** HIGH for internal evidence (git history, `gh` run data, measured in-repo); MEDIUM for external (2026 studies/vendor docs, cross-checked where possible).

Phase owners below use this suggested split (roadmapper may rename):
**P0 Baseline & deploy** · **P1 Deterministic drift detection** · **P2 Generated reference** · **P3 Audit & prune** · **P4 LLM draft-PR agent** · **P5 Reader signals & llms.txt**.

---

## 0. The headline internal finding (read first)

**The public site is not being deployed.** `docs-deploy.yml` on `main`: last 400 runs = 230 failure / 93 cancelled / 77 success; the most recent success in that window is **2026-05-12**. Every run since 2026-09-20 has failed (latest error: Cloudflare Pages API request for project `vox-docs` failed). The failure-visibility bot (added `6d7c4c376`, 2026-07-23) opened issue **#462 "docs-deploy is failing on main" on 2026-07-24; it has 57 comments and is still open.** `front-facing-honesty-audit-2026.md` already noted "Live voxlang.org stays stale until docs-deploy…".

Implication: every in-repo docs fix of the last ~5 months is invisible to readers and scrapers. Any freshness work that ships before deploy is green measures the repo, not the site. This is also the single best local example of alert fatigue (Pitfall 3).

---

## Critical Pitfalls

### 1. Building a program that nobody runs (the Docs Reality Audit failure mode)
**What went wrong here:** The Docs Reality Audit Program (`contracts/documentation/docs-reality-audit.program.v1.yaml`) shipped a taxonomy, priority formula, three JSON schemas, a `vox ci docs-reality-audit verify|metrics` CLI, and a weekly/monthly/release cadence — then got **zero findings ever** (`findings.v1.json` = `[]`, one commit `3295a3bee`, 2026-05-12). Why it went dormant, from the commits:
- **Manual input, automated validation.** `verify` checks schemas and that *path hints resolve*; nothing *produces* findings. The cadence depended on a human hand-writing JSON rows. No human did.
- **Wired into a gate that can only pass.** It ran inside `ssot-drift` on every push, so it was "green" for months while measuring nothing. `015f430ca` found `rollout_milestone_pct` returned 25 for both an empty backlog and 80 open findings, and that `metrics.v1.json` sat stale beside a 10→150-claim inventory with a green build ("a number nobody reads going unquestioned for three months").
- **No consumer.** Zero readers of the metrics; no PR comment, no issue, no llms.txt surface. `d48683ca7` finally re-labelled the doc "dormant" because it described an unperformed ritual as current practice.
- **Overlapping programs.** `docs/superpowers/specs/2026-08-22-docs-corpus-repair-design.md` and `2026-09-14-deep-research-documentation-engine-design.md` (marked `status: current`, no CLI implemented) are parallel efforts in the same space.
**Warning signs:** a schema/taxonomy lands before the first real finding; a gate that cannot fail on an empty backlog; metrics with no named consumer; cadence words ("weekly") with no scheduler.
**Prevention:** findings must be **machine-generated** (graph/symbol/CLI-registry resolution), never hand-entered. Reuse the existing taxonomy/schemas rather than a fourth design. Every metric needs a consumer and a "not started vs healthy" distinction. Add a liveness check (like `ci-liveness.yml`): if the scheduled job produced no artifact in 2× cadence, open a `Nightly stale:` issue. Explicitly retire or merge the overlapping specs in P0.
**Owner:** P0 (decide fate of dormant program + overlapping specs), P1 (generator-first findings).

### 2. LLM drafts confident-but-wrong prose (and agents fix falsehoods with new falsehoods)
**What went wrong here:** this repo's own agent-authored fixes repeatedly replaced one false claim with another:
- `e2bb133a1` "correct my own earlier wrong claim": an agent closed a doc gap claiming tables get zero auto-injected fields; two crates both define `table_to_ddl`, it cited one path while describing the other.
- `d48683ca7`: a draft fix of a false `chat.rs` comment "wrongly said 'nothing sleeps' — replacing one falsehood with another."
- `c4eb26607`: merge resolution reintroduced retired `vox-ludus` into SSOT docs.
- `f48dbc810`: agent-built Python heredocs turned `\a`/`\v` into BEL/VT bytes, silently corrupting documented paths.
- `49a08f34e`: a `status: current`, `training_eligible: true` SSOT taught removed `@endpoint` syntax; its fences carried `// vox:skip`, so it "passed every gate while teaching syntax that does not parse."
**External:** CodeRabbit study — 56.3% of agentic review comments rejected, 43% of those false positives, mostly from missing system-wide context ([arXiv 2607.03316](https://arxiv.org/html/2607.03316v2)). Vendors converge on a generator + independent verifier and never auto-merge ungrounded drafts (DocuGardener's temp-0 verifier, [github.com/docugardener](https://github.com/docugardener/docugardener); DocPulse deterministic suspect selection + bounded agent, exit 1 even after fixing, [github.com/YoniRaviv/DocPulse](https://github.com/YoniRaviv/DocPulse)).
**Consequences:** wrong prose under a "current" badge is worse than stale prose; here it also poisons MENS training corpus (`training_eligible`).
**Prevention:** deterministic layer picks *what* is stale (symbol/CLI/env-var/contract resolution via `vox graph`, command registry, `contracts/config/env-vars.v1.yaml`); LLM only drafts *how to fix*. Every drafted claim must cite a resolvable code anchor (path + symbol) that a deterministic checker re-verifies post-draft; unverifiable claims are dropped, not softened. Drafted fences must compile (no new `vox:skip` from the bot). Hold out a golden set of real past drift cases (the commits above) as a regression eval before enabling the agent.
**Owner:** P4 (with eval harness gating enablement); P1 supplies the anchors.

### 3. Alert fatigue: drift signals nobody acts on
**What went wrong here:** issue #462 — 57 bot comments over 2.5 months, deploy still broken. Same shape as Pitfall 1: visibility without ownership.
**External:** human comments addressed ~60% vs automated-review comments 0.9–19.2%; 12/13 review bots had signal ratio <60% ([arXiv 2604.03196](https://arxiv.org/pdf/2604.03196v1)). Reviewers habituate to agent PRs: approval rate rises while inline scrutiny falls 22% ([arXiv 2606.22721](https://arxiv.org/html/2606.22721v1)).
**Warning signs:** one long-lived issue accumulating bot comments; PR comment on every PR; drafts sitting unreviewed >7 days; approval of bot PRs with zero inline comments.
**Prevention:** one de-duplicated, *edited-in-place* report (not N comments); hard WIP cap on open bot doc PRs (e.g. ≤3) — the bot stops drafting when the queue is full; severity tiers where only P0 public-surface drift blocks; per-signal precision tracking (accepted/rejected), auto-demote signals below a precision floor; escalate a repeat failure to a different channel/owner instead of commenting again.
**Owner:** P1 (report shape, severity), P4 (WIP cap, precision tracking), P0 (fix #462 and add escalation).

### 4. Prompt injection into a CI agent holding write tokens
**External:** "Comment and Control"/PromptPwnd exfiltrated `GITHUB_TOKEN` and provider keys from Claude Code, Gemini CLI and Copilot actions via PR/issue text ([CSA note](https://labs.cloudsecurityalliance.org/research/csa-research-note-ai-github-actions-security-20260503-csa-st/)). Config-file injection: a fork PR edits `AGENTS.md`/`CLAUDE.md`, the agent loads it as operator instructions; `actions/checkout` persists the token in `.git/config` and a PR body got it exfiltrated ([arXiv 2606.09935](https://arxiv.org/html/2606.09935v1)). 496 confirmed agentic-workflow-injection vulns across 10,792 repos ([arXiv 2605.07135](https://arxiv.org/html/2605.07135v1)). prt-scan auto-applied "safe to test" labels with stolen tokens ([CSA prt-scan](https://labs.cloudsecurityalliance.org/research/csa-research-note-github-actions-prt-scan-supply-chain-2026/)).
**Repo-specific exposure:** the doc agent's *input is the repo itself* — docs, AGENTS.md, code comments, llms.txt — all writable by any PR. A doc page is an injection vector by design.
**Prevention (non-negotiable for P4):**
- Never `pull_request_target`; never run the agent on fork content. Trigger on `schedule`/`workflow_dispatch`/`push: main` only, reading `main`.
- Two-job split: an **unprivileged** job (`contents: read`, no secrets except the model key) produces a patch artifact; a separate **privileged** job with no LLM opens the draft PR from that artifact (`workflow_run`/artifact pattern). Model output is data, never shell.
- Copy the hardened `ssot-autoregen` job shape (`.github/workflows/ci.yml:320`): same-repo only, SHA-pinned actions, `persist-credentials: false`, job-scoped `permissions`.
- Agent tool allowlist = read/grep/graph-query; no bash, no network beyond the LLM facade.
- Bot PRs restricted to `docs/src/**` paths; a post-check rejects any diff touching `.github/`, `AGENTS.md`, `CLAUDE.md`, `contracts/`, `crates/` or `llms.txt` (those need human authorship). CODEOWNERS (`.github/CODEOWNERS`) already routes these to maintainers — require review, never auto-approve.
**Owner:** P4 (design gate before any code), reviewed by `security-review`.

### 5. Fixing the repo while the public surface stays broken / dishonest
**What went wrong here:** see §0; plus `59e30f6b6` — llms.txt sent agents to a 404 whose only copy is in the tombstoned archive (plus a `_redirects` 301 into the 404); `94822d35d` — `vox-doc-inventory` *generator* hardcoded a dead path into `first_read_for_agents`, so regenerating "faithfully reproduced the dead entry"; honesty audit: live `voxlang.org/voxup` 404 means `curl … | sh` pipes HTML into a shell.
**Prevention:** freshness is measured against the **deployed** site too: post-deploy smoke (llms.txt links resolve, install URLs return the script, no retired syntax in rendered HTML). Generators get dead-path tests, not just their outputs.
**Owner:** P0 (deploy green + post-deploy smoke), P5.

---

## Moderate Pitfalls

### 6. "Last updated" measures edits, not accuracy
Repo already removed hand `last_updated` (`4e98a0f87`, 337 files) and derives it from git. But git mtime is falsified by typo fixes, `sed` sweeps, frontmatter migrations and formatters — and the 337-file strip itself bumped every page's date. External: last-edited "lies in both directions"; use a falsifiable *last-verified* stamp ([datadef.io](https://datadef.io/guides/en/what-makes-documentation-trustworthy)); thin-wrapper pages show misleading dates ([medic/cht-docs#2215](https://github.com/medic/cht-docs/issues/2215)).
**Prevention:** badge shows `verified-against: <sha>` set only when the page's anchors resolved in CI (machine-verified) or a human attests; ignore mechanical commits (bulk/regen/fmt) when computing dates; generated pages show their generator's input date. Never render "current" from `status:` alone (383 `current` claims, none verified).
**Owner:** P5, depends on P1 anchors.

### 7. False positives in link/symbol checkers → people add ignores until the gate is meaningless
Repo history: dozens of `fix(docs): repair N broken links` commits, `.lycheeignore` route patterns added, external links pushed to nightly lychee. External: lychee users disable it after GitHub 429/secondary rate limits; fixes are caching, low concurrency, accepting 429 ([lychee#1873](https://github.com/lycheeverse/lychee/issues/1873), [lychee-action#289](https://github.com/lycheeverse/lychee-action/issues/289), [rate-limit docs](https://lychee.cli.rs/troubleshooting/rate-limits/)). Symbol checkers false-positive on prose that names retired symbols in historical sections — see the carve-out churn ("scope the historical-doc carve-out to a Retired/Historical section", `is_historical_or_audit_doc()`), and `d48683ca7`'s NUL byte that made grep skip a file silently.
**Prevention:** internal links/anchors/symbols block on PR (offline, deterministic); external URLs nightly only, cached. Every suppression carries owner + reason + expiry (existing `contracts/toestub/suppressions.v1.json` pattern), and the count of suppressions is itself a tracked metric. Distinguish "mentions" from "prescribes" (code fences and imperative text vs. history sections) structurally, not with path allowlists.
**Owner:** P1.

### 8. Flaky docs build blocks merges (or a broken build stays red on main)
`e32f81651`: `starlight-llms-txt` 0.11 pulled `@astrojs/mdx@7` needing astro 7; main's `pnpm build` was red — "the third instance of one pattern." Also `fix(docs-astro): unbreak the doc build — missing pnpm config`, quoted-YAML-date schema breaks, Windows path bug in RSS util, empty RSS feed. External: same class across ecosystems (peer-dep warnings not failures).
**Prevention:** lockfile-frozen installs; treat peer-dependency warnings as errors in docs CI; Dependabot groups for `docs-astro` must run the full build; keep the Starlight build in `docs-quality` path-filtered so non-docs PRs aren't blocked by a docs-toolchain break, but make deploy failure page a human (see #3).
**Owner:** P0.

### 9. LLM-in-CI cost blow-ups and denial-of-wallet
Repo constraints: PR jobs ≤30 min, scheduled ≤180 min; calls must go through `vox_actor_runtime::llm`. External: denial-of-wallet is a named attack class for CI agents (arXiv 2606.09935). Running an LLM on every PR × 668 pages is the obvious overrun.
**Prevention:** LLM never runs on PR; deterministic detection on PR, LLM drafting on a schedule over a **bounded queue** (top-K findings by priority). Hard per-run token/cost budget enforced in the facade call site; content-hash cache so unchanged (page, anchor) pairs are never re-sent. Model selected via `vox-orchestrator::models` registry (cheap model for triage, stronger for drafting) — no hardcoded vendor hostnames (`llm_provider_call` detector will fail CI).
**Owner:** P4.

### 10. Generated docs nobody reads; corpus bloat outpaces pruning
Baseline: 668 live pages, 388 (58%) architecture, ~200 `research`, ~72 `roadmap`; 49 new architecture pages added in the last 60 days alone. Repo already hand-maintains generated refs and regenerated after merges ≥6 times ("regenerate … after merge" — now blocked by AGENTS.md policy). External: "cull before you fix" — archive what nobody will re-verify, then stamp the remainder (datadef.io).
**Prevention:** every new generated page must name a reader (sidebar entry, llms.txt link, or `vox` CLI help link) or not ship. Gate new `status: current` architecture docs on having verifiable anchors. Prune *before* building drift detection over 668 pages, else P1 drowns in findings on dead research notes. Prune case-by-case per brief, but default `research`/`roadmap` pages out of drift scope and out of the public sidebar.
**Owner:** P3 (before P1 scales), P2.

### 11. llms.txt bloat and unread agent surfaces
Current file is 34 lines / 2.8 KB — good. External: 97% of llms.txt files get zero requests; agents fetch it when *linked*, not speculatively; coding agents (Claude-Code) are the main real reader; stale/compromised llms.txt misleads every agent that reads it ([Ahrefs 137K-site study](https://ahrefs.com/blog/llmstxt-study/), [EZY 83-site logs](https://www.ezy.ai/research/do-ai-bots-read-llms-txt)). Repo has **two** llms.txt authorities (hand file + Starlight plugin) that must "update together" — a split-brain by decision.
**Prevention:** keep the hand file small and curated; don't dump drift reports into it — link one stable drift-report URL instead. Every llms.txt link resolves on the deployed site (P0 smoke). Generate one from the other, or test parity, to end the two-authority drift. Measure Cloudflare logs before investing more.
**Owner:** P5.

---

## Minor Pitfalls

- **Frontmatter vocabulary drift:** governance doc advertised slugs the lint rejects (`0bae3b6a2`); ~15 `fix(docs): correct frontmatter category/status` commits. Any new freshness frontmatter key (`verified-against`, `owner`) must be defined once in `vox-doc-pipeline` with a parity test against the governance doc. *Owner: P1/P5.*
- **Self-referential count drift:** the program's own spec hardcoded "275 plans" which drifted to 278 within the program (`015f430ca`). Never hardcode corpus counts in docs; cite a generator. *Owner: all.*
- **Gates conditional on file existence:** `run_body_helpers/docs.rs` guard re-arms only if `architecture-index.md` reappears (196-file hard fail). Avoid gates whose activation depends on an absent file. *Owner: P1.*
- **Merge resolution resurrecting retired names** (`c4eb26607`): bot PRs must rebase-and-revalidate before review, and `retired-symbol-check` runs on the bot's diff. *Owner: P4.*
- **Cross-platform byte corruption** from agent tooling (BEL/VT, BOMs, NUL): add a control-char scan to doc lint. *Owner: P1.*

---

## Security constraints for the PR-opening agent (checklist)

| Constraint | Source | Concretely |
|---|---|---|
| No `pull_request_target`; no fork content to the agent | CSA, arXiv 2606.09935 | schedule/dispatch/push-main triggers only |
| Least-privilege tokens, job-scoped | AGENTS.md CI contract, `workflow-permissions (strict)` gate | LLM job `contents: read`; PR job `contents: write, pull-requests: write`, no LLM |
| `persist-credentials: false`, SHA-pinned actions | `ssot-autoregen` precedent | copy that job verbatim |
| Model key via secrets SSOT | `vox_secrets::resolve_secret`; `vox ci secret-env-guard`, `secrets-parity` | add `SecretId` + spec; no `env::var` |
| Model-agnostic | `vox_actor_runtime::llm`, `llm_provider_call` detector | no vendor hostnames/SDKs; registry-selected model |
| Draft PRs only, never auto-merge | brief decision | branch protection + CODEOWNERS review; bot cannot approve |
| Path allowlist on bot diffs | this research | `docs/src/**` (non-generated) only |
| Automation as `.vox` | VoxScript-first policy | orchestration in `scripts/*.vox`, not shell |
| Token budget + timeout | CI caps, denial-of-wallet | ≤180 min scheduled; per-run token cap |

---

## Phase-Specific Warnings

| Phase | Likely pitfall | Mitigation |
|---|---|---|
| P0 Baseline & deploy | Building on an undeployed site; #462 noise | Fix Cloudflare deploy first; post-deploy smoke; escalate not re-comment; decide fate of dormant audit + overlapping specs |
| P1 Drift detection | Hand-entered findings; gate that can't fail; checker FPs → ignore sprawl | Generator-produced findings via `vox graph`/registries; empty-vs-healthy metrics; suppressions with expiry |
| P2 Generated reference | Unread generated pages; regen-after-merge churn | Named reader per page; rely on `ssot-autoregen`; fix at generator |
| P3 Audit & prune | Drift tooling drowning in 200 research pages | Prune/out-of-scope before P1 scales |
| P4 LLM draft-PR agent | Wrong prose; injection; cost; reviewer habituation | Anchor-verified drafts, golden eval, two-job split, WIP cap, precision tracking |
| P5 Reader signals | Edit dates masquerading as accuracy; two llms.txt authorities | `verified-against` sha from CI; single llms.txt source |

## Sources

Internal (HIGH): commits `015f430ca`, `d48683ca7`, `4e98a0f87`, `59e30f6b6`, `94822d35d`, `49a08f34e`, `e2bb133a1`, `f48dbc810`, `c4eb26607`, `0bae3b6a2`, `e32f81651`, `6d7c4c376`; `gh run list --workflow docs-deploy.yml`; issue #462; `docs/src/contributors/docs-reality-audit-program.md`; `docs/src/architecture/front-facing-honesty-audit-2026.md`; `.github/workflows/ci.yml` (`ssot-autoregen`).
External (MEDIUM): URLs inline above (arXiv 2607.03316, 2604.03196, 2606.22721, 2606.09935, 2605.07135; CSA research notes; lychee docs/issues; Ahrefs, EZY llms.txt log studies; datadef.io; DocuGardener, DocPulse READMEs).
