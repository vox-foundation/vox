# Feature Research

**Domain:** Curated knowledge base as first-class agent context (KB browse + per-turn injection controls + reviewed auto-growth inbox + staleness/trust governance), surfaced in the Vox Tauri/React GUI
**Researched:** 2026-10-03
**Confidence:** MEDIUM overall. Product behavior claims come from vendor docs/help pages cross-checked against at least one independent search result (MEDIUM, per `classify-confidence --verified`). Single-blog or forum claims are marked LOW. Local-code claims (what Vox already has) were verified by reading the source in this worktree and are HIGH.

---

## 1. How the best products do it (survey)

Each product was assessed on four axes: **(C)** controls over what context is injected, **(P)** provenance/trust display, **(S)** staleness handling, **(I)** ingestion/review.

### (a) AI coding tools

| Product | (C) Context controls | (P) Provenance / trust | (S) Staleness | (I) Ingestion / review | Conf. |
|---|---|---|---|---|---|
| **Cursor** | Rules with 4 activation modes: Always Apply, Apply Intelligently (agent decides from the `description`), Apply to Specific Files (globs), Apply Manually (`@rule`). `@Docs` is a manual @-mention over crawled, embedded doc sites. Rule size guidance is ~500 lines max. | Little. `@Docs` hits aren't shown with dates. | **Its weak point.** Users report `@Docs` serving pages that are a year old, re-index runs that silently fail, a status showing "indexed" while the index is actually failing, and deleted indexes that still persist. | Rules are files you write and keep in version control. Auto-generated **Memories were removed in 2.1.x (Nov 2025)**. Users were told to export them to `.mdc` rules. | MEDIUM ([rules](https://cursor.com/docs/context/rules), [@docs](https://docs.cursor.com/context/@-symbols/@-docs), [stale docs](https://forum.cursor.com/t/docs-mention-provides-ai-with-outdated-docs-1-year-old-pages/58871), [wrong status](https://forum.cursor.com/t/docs-indexing-status-shows-incorrect-states-working-doc-shows-indexing-while-failing-doc-shows-indexed/38940), [can't delete](https://forum.cursor.com/t/cant-truly-delete-docs-index/98728), [memories removed](https://forum.cursor.com/t/are-my-memories-gone/144057)) |
| **Claude Code** | CLAUDE.md/AGENTS.md are always loaded. Path-scoped `.claude/rules/` load only when matching files are touched. Skills load on demand (progressive disclosure). `@path` imports are capped at 4 hops, and external imports need an approval dialog. Guidance is <200 lines per CLAUDE.md. | `/context` breaks token usage down by category (system prompt, tools, MCP, memory files, skills, messages, free space). `/memory` lists the instruction files that were loaded. | `/doctor prompt-audit` finds outdated or conflicting instructions and references to files that no longer exist, then proposes edits without applying them. | Auto memory is written by Claude, but it is plain files under user control. Only the first 200 lines / 25 KB of the index are loaded. | MEDIUM ([memory docs](https://code.claude.com/docs/en/memory), [/context](https://blog.vincentqiao.com/en/posts/claude-code-context/)) |
| **Windsurf / Devin Cascade** | Rules can be global or workspace-level and are kept in version control. Auto-generated Memories are retrieved "when relevant". | Memories are local only, aren't committed, and don't carry across workspaces. | None documented. | Memories are written automatically, or created when the user says "create a memory of…". | MEDIUM ([memories](https://docs.windsurf.com/windsurf/cascade/memories)) |
| **Continue.dev** | `@docs` crawled sites into local embeddings. **Deprecated** in favour of agent-mode awareness (tool/MCP retrieval). | n/a | n/a | Docs could be added through a dialog or in bulk via config. | MEDIUM ([docs](https://docs.continue.dev/customize/deep-dives/docs), [deprecation](https://docs.continue.dev/reference/deprecated-docs)) |
| **Amp (Sourcegraph)** | AGENTS.md with glob-scoped frontmatter and `@file` mentions. Threads can be compacted or restarted with a summary. | No context inspector. One user asked for a "Context Inspector" showing what is supplied to the model. That request was made on the Codex forum, which signals an unmet need across tools. | n/a | n/a | LOW ([amp](https://codex.danielvaughan.com/2026/04/07/sourcegraph-amp-multi-repo-orchestrator/), [inspector request](https://community.openai.com/t/developer-context-inspector-for-codex-sessions/1393972)) |
| **GitHub Copilot Spaces** | Curated per-task scope: repos, files, PRs, issues, free text, and uploads. Narrower scope means better retrieval. Spaces replaced Knowledge Bases (sunset Nov 2025). | Viewers see only the sources they have access to. | **GitHub sources update automatically** as the repo changes. | Sources are added manually. Spaces can be shared with admin/editor/viewer roles. | MEDIUM ([docs](https://docs.github.com/copilot/using-github-copilot/copilot-spaces/about-organizing-and-sharing-context-with-copilot-spaces), [KB sunset](https://www.agentpatterns.ai/tools/copilot/copilot-spaces/)) |
| **Devin DeepWiki** | `.devin/wiki.json` steers generation through `repo_notes` and an explicit page list (30 pages, or 80 on enterprise). | Pages are generated from the code. | Regenerated on demand. | Auto-generated wiki pages that large monorepos can't fully fit. | MEDIUM ([docs](https://docs.devin.ai/work-with-devin/deepwiki)) |

### (b) PKM / enterprise KB tools

| Product | (C) | (P) | (S) | (I) | Conf. |
|---|---|---|---|---|---|
| **Guru** (the reference model for verification) | AI answers are drawn from Cards. | Every Card shows a **verified/unverified badge, the verification date and the assigned verifier**. | Each Card has a verification interval (weekly up to annually, or a specific date; the default is 1 month). Overdue Cards turn Unverified and go into the verifier's task queue **but stay searchable**. A "My Tasks" view prioritises Cards by a relevancy score combining popularity, trust score and how overdue they are. Cards that are **unverified and unused** go to an auto-archive queue. Admins get bulk actions. | Verifiers are assigned, or suggested by AI ("Suggest Expert"). Only Author/Owner/Admin roles can verify. | MEDIUM ([features](https://www.getguru.com/features/verification), [intro](https://www.getguru.com/blog/introducing-verification)) |
| **Notion** (wikis) | Q&A over the workspace. | Pages can be marked "Verified", either indefinitely or with an expiry. **Verified pages are prioritised in search, @-mentions and Notion AI answers.** | When verification expires, the owner is notified (inbox and email). | Owner defaults to the page creator. | MEDIUM ([help](https://notion.com/help/wikis-and-verified-pages)) |
| **Confluence / Rovo** | Search returns candidate docs, and the calling agent has to read them. | Citations are given. | **Rovo doesn't notice when docs are old.** Stale or poorly structured sources produce stale answers. | n/a | LOW ([review](https://slite.com/learn/atlassian-rovo-ai-review)) |
| **Glean** | Permission-aware retrieval. | Answers cite their sources. | Permission freshness propagates in near real time. | Connector crawl. | LOW ([overview](https://atlan.com/know/ai-agent/data-for-ai/enterprise-search-with-ai.md)) |
| **Obsidian** (Copilot, Smart Connections) | Chat over the vault. Folders, files and tags can be excluded from AI operations, and a "Refresh preview" shows what is exposed. | Links to related notes. | n/a | Manual notes. | LOW ([setup guide](https://www.asianefficiency.com/technology/obsidian-ai-plugins-setup/)) |
| **Mintlify / GitBook** | Both auto-generate `/llms.txt` (a curated index) and `/llms-full.txt` (everything concatenated) and expose docs to agents through MCP. | n/a | Regenerated on every publish. | n/a | MEDIUM ([Mintlify](https://www.mintlify.com/library/best-llms-txt-platforms), [spec guide](https://codersera.com/blog/llms-txt-complete-guide-2026/amp/)) |

### (c) Research agents

| Product | (C) | (P) | (S) | (I) | Conf. |
|---|---|---|---|---|---|
| **NotebookLM** | **A checkbox per source** decides whether it is used for the next answer. Answers are grounded only in the selected sources. | Inline citations are **direct quotes** linked to the source passage. | n/a | **"Discover sources"**: describe a topic and it returns up to 10 web sources, each **annotated with why it's relevant**. The user ticks the ones to import. Nothing is added automatically. | MEDIUM ([help](https://support.google.com/notebooklm/answer/14276569?hl=en), [discover](https://blog.google/technology/google-labs/notebooklm-discover-sources/)) |
| **Perplexity Spaces** | Per-Space toggles for **Web vs My Files**, a recency filter, and custom instructions applied to every thread in the Space. | Source cards. | Recency filter. | Files, links and connectors. Pro allows 50 files per Space. | LOW ([Spaces guide](https://www.educatorstechnology.com/2025/04/perplexity-ai-spaces.html)) |
| **ChatGPT deep research** | **The plan is shown for review and editing before the run starts.** Sites → Manage sites offers **"restrict to these sites"** or **"prioritise these sites, but allow full web"**. The user can interrupt mid-run to adjust sources. Connectors are read-only. | Report citations. | n/a | n/a | MEDIUM ([help](https://help.openai.com/en/articles/10500283-deep-research)) |
| **Gemini deep research** | A plan card with Edit plan / Start research. Sub-questions can be added or removed before any browsing. | Report citations. | n/a | n/a | LOW ([guide](https://www.ai-toolbox.co/gemini-management-and-productivity/how-to-use-gemini-deep-research-2026)) |
| **Elicit / Consensus** | Filters for journal tier, methods and citation thresholds. | Elicit gives **sentence-level citations per extracted cell**. Consensus Meter shows the yes/no/mixed distribution. Commentators warn these quality signals are *weak* proxies. | Recency is shown. | n/a | LOW ([Consensus deep dive](https://aarontay.substack.com/p/a-2025-deep-dive-of-consensus-promises), [comparison](https://casrai.org/compare/elicit-vs-consensus)) |
| **STORM / Co-STORM** (Stanford OVAL) | Perspective-guided question asking leads to an outline, then a Wikipedia-style article with references. Co-STORM adds human-in-the-loop turn management and a mind map. | References per section. | n/a | This is the template for drafting a *Library article* from web research. | MEDIUM ([repo summary](https://github.com/sugatoray/stanford-storm), [decoder](https://the-decoder.com/stanford-ai-experiment-storm-generates-wikipedia-style-articles/)) |

### (d) Knowledge-ops practice

- **KCS v6 article states:** Work in Progress → Not Validated → Validated → Archived. Progression is non-linear. Archived articles are *logically deleted* (removed from search but kept so links don't break). The guiding principles are "**reuse is review**" (use is validation) and "**flag it or fix it**". Licensing tiers decide who can set Validated. MEDIUM ([KCS article state](https://library.serviceinnovation.org/KCS/KCS_v6/KCS_v6_Practices_Guide/030/040/010/030))
- **Wikipedia:** verifiability means challengeable claims need inline citations (`{{citation needed}}` marks a *specific* passage). The **Perennial Sources** list rates sources as Generally reliable / No consensus / Generally unreliable / Deprecated / Blacklisted. Each entry has a last-updated date and links to the discussions behind it, and "context matters". This is a categorical tier list, not a numeric score. MEDIUM ([RSP](https://en.wikipedia.org/wiki/Wikipedia:Reliable_sources/Perennial_sources), [CN template](https://www.wikipedia.org/wiki/T:CN))
- **Context-budget evidence:** Chroma's "Context Rot" study found all 18 frontier models degrade as input grows, and semantically similar distractors make it worse. Anthropic recommends **just-in-time retrieval** (keep lightweight identifiers and load on demand) over pre-loading. Both argue for a *small, inspected* auto-injection budget plus tool-based retrieval. MEDIUM ([Chroma](https://research.trychroma.com/context-rot), [Anthropic summary](https://agentic-ai.readthedocs.io/en/latest/ContextEngineering/anthropic/))
- **Security:** indirect prompt injection through web content can *persistently poison* long-term memory or a RAG store, and every future session that retrieves it is affected. Unit 42 published a proof of concept against agent memory. MEDIUM ([Unit 42](https://unit42.paloaltonetworks.com/indirect-prompt-injection-poisons-ai-longterm-memory/), [overview](https://atlan.com/know/prompt-injection-attacks-ai-agents/))
- **Citation hallucination:** a citation that doesn't support its claim *raises* reader trust while breaking verifiability. Verification has to be a separate step checked against the source, not the generator checking itself. MEDIUM ([FACTUM](https://arxiv.org/pdf/2601.05866), [VeriCite](https://arxiv.org/pdf/2510.11394))
- **Paid web cost reference:** OpenRouter's web plugin through Exa costs $4 per 1000 results, about $0.02 per request at the default `max_results=5`, plus prompt tokens. Native engines pass the provider's price through. MEDIUM ([OpenRouter](https://openrouter.ai/docs/web-search))

### Cross-cutting patterns (what the survey converges on)

1. **Three activation tiers are universal.** Always-on (CLAUDE.md, Cursor Always), relevance-selected (Cursor "Apply Intelligently", skills chosen by description, Notion verified-priority RAG), and manual `@`-mention (`@Docs`, `@rule`, `@file`). Users expect all three and expect to see which one fired.
2. **Scope is set per conversation or per space, not globally.** Examples are NotebookLM checkboxes, Copilot Spaces, Perplexity Web/Files toggles, and ChatGPT's restrict-vs-prioritise sites.
3. **Unreviewed auto-capture is in retreat.** Cursor removed Memories in favour of version-controlled rules. ChatGPT memory draws persistent complaints (confusing toggles, confabulated memories). NotebookLM *suggests* web sources but makes the user import them. Vox's locked "agent drafts PR, human merges" decision is on the right side of this trend.
4. **Verification is a badge plus an owner plus an interval, not a gate.** Guru and Notion keep unverified content visible but flagged, prioritise verified content in AI answers, and route overdue items to a queue. KCS treats use as a review signal.
5. **Staleness is the top complaint against everyone who doesn't model it**: Cursor `@Docs`, Rovo, and the 9-month-old index reports.
6. **Context inspection is shallow everywhere.** Claude Code `/context` breaks usage down *by category*. Nobody shows a per-turn, per-chunk view of *which KB passages were injected, why, and what was dropped*. That is the clearest gap.

---

## 2. What Vox already has (dependency map)

HIGH confidence. Verified by reading source in this worktree.

| Existing asset | Location | Reuse for |
|---|---|---|
| `parse_kb_mentions` (`@name` → kb scopes) | `crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs:236` | `@kb` per-chat scope. The backend parser already exists, but the composer has no autocomplete. |
| `run_retrieval_bundle`, plus `kb_lines` / `rrf_fused_lines` / `RetrievalEvidenceEnvelope` / `format_backend_mix` | same file, `:293` | Inspector payload: lines, backends, retrieval tier |
| Autonomous retrieval preamble (skips the web block when research mode supplies numbered sources) | `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:721-821` | Injection on/off, budget, and the payload the Inspector must report |
| `ContextTrustTier {Untrusted, Advisory, Trusted, SystemVerified}`, `ContextBudget` | `crates/vox-orchestrator/src/context_envelope.rs:90` | `trust_tier` vocabulary and budget. **Map doc `trust_tier` to this enum; don't invent a second vocabulary.** |
| Frontmatter `status` vocab (`current`/`research`/`roadmap`/`legacy`/`deprecated`/…) | `docs/src/contributors/documentation-governance.md` | Library facets. `deprecated`/`legacy` should be down-ranked by default. |
| `MemoryView` (corpus chips, `vox_search_query`, `KbHealthDto`) | `surfaces/Memory/MemoryView.tsx` (973 LOC) | Library search pane. It is absorbed into Library rather than rewritten. |
| `DocReader` | `surfaces/DocReader/DocReader.tsx` | Library reader. **Gap: it renders raw markdown inside `<pre>` with no frontmatter header.** Reuse `ResearchReportMarkdown` for rendering. |
| `EmpiricalVerificationBadge`, `TrustChip` (formal / corroborated / uncorroborated) | `DocReader/`, `Research/TrustChip.tsx` | Provenance badges on docs and web citations |
| `DiscoveryInbox` (list, ack, live events, strong-candidate toast) | `surfaces/Scientia/DiscoveryInbox.tsx` | Template for the KB Inbox list. The pattern can be copied without coupling to Scientia's tables. |
| `ResearchModeDropdown` (`auto`/`fast`/`deep`/`off`) | `surfaces/Chat/ResearchModeDropdown.tsx` | **Overlaps the "web-fallback toggle".** `off` already means no external search. Reconcile into one control rather than adding a second toggle that contradicts it. |
| `TurnTrace` + `ChatTurnEventRow` with an `onExcludeSkill` "not this one" chip | `surfaces/Chat/` | The exact interaction pattern for "exclude this source" from the Inspector |
| `ContextWindowMeter` (aggregate token bar) | `surfaces/Chat/ContextWindowMeter.tsx` | The Inspector adds a per-source breakdown under it |
| `GroundingCheckToggle` (post-reply unsupported-claim check) | `surfaces/Chat/GroundingCheckToggle.tsx` | Citation verification of KB-grounded answers |
| `docs/src/.well-known/llms.txt` | docs tree | External-agent exposure already exists |
| `kb_tools` MCP handlers (`signal_chat`/`signal_research`/`signal_scientia`) | `crates/vox-orchestrator-mcp/src/kb_tools/` | Tool-based (just-in-time) KB retrieval for agents |

---

## Feature Landscape

### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes / Vox dependency |
|---|---|---|---|
| **Library: browse + search the whole corpus** (`docs/src/` + `docs/src/library/`) with facets for category, status, trust_tier and staleness | Every KB tool has this. It replaces two half-surfaces (Memory and DocReader). | MEDIUM | Absorb `MemoryView` search and corpus chips. Facets come from the frontmatter index. Must go through `vox-search` (AGENTS.md forbids a bespoke index). |
| **Rendered doc reader with a provenance header**: title, description, status, trust_tier chip, `review_by` badge, last-updated (from git), source links, "Open in editor" | Guru and Notion show verified status, date and owner on every page. The current DocReader shows raw `<pre>`. | LOW–MED | Reuse `ResearchReportMarkdown`, `TrustChip`, `EmpiricalVerificationBadge`. Read-only. Editing happens in the editor and goes through PRs. |
| **Staleness badge** (`review_by` passed shows "Needs review", amber) shown on reader, search hits, and Inspector rows | Guru unverified badge, Notion expiry. Staleness is the #1 complaint against Cursor and Rovo. | LOW | Pure frontmatter date compare. Stale docs stay visible and searchable (Guru, KCS). Down-rank them, don't hide them. |
| **Per-chat KB scope via `@kb` mentions with autocomplete** | Cursor `@Docs`/`@rule`, Amp `@file`, Claude Code `@path`. Manual mention is the universal escape hatch. | MEDIUM | The backend `parse_kb_mentions` exists. Needs a composer typeahead over library sections/tags, plus a scope chip row showing the active scope. |
| **Injection on/off + token budget** (per chat, with a Settings default) | Context-rot evidence. Claude Code's <200-line and Cursor's ~500-line guidance show vendors cap always-on context. | LOW | `ContextBudget` exists. Ship a conservative default. |
| **Web-fallback toggle** | Perplexity Web/My Files, ChatGPT restrict-sites, NotebookLM sources-only | LOW | **Unify with `ResearchModeDropdown`.** A second web switch would contradict `off`. |
| **Trust-tier filter** (minimum tier eligible for injection) | Notion prioritises verified pages. Wikipedia RSP-style categorical reliability. | LOW | Filter on the `ContextTrustTier` mapping. |
| **Source pin / exclude per chat** | NotebookLM per-source checkboxes, ChatGPT restrict vs prioritise, Obsidian exclusions | MEDIUM | Offer two pin modes, following ChatGPT: **restrict** (only these) and **prioritise** (boost these, allow others). Exclude reuses the `onExcludeSkill` chip pattern. |
| **Per-turn "injected context" Inspector**: each injected chunk with source path/section, tokens, score/backend, trust tier, staleness, and why it was selected (always / relevance / @mention / pinned), plus total against budget | Claude Code `/context` sets the floor (per-category tokens). There was an explicit user request for a context inspector in Codex. Users can't debug answers without it. | MEDIUM | Payload from `RetrievalEvidenceEnvelope`, `kb_lines`, `format_backend_mix`. Lives in or next to `TurnTrace`/`ResearchTracePanel`. Row actions are Open, Exclude from this chat, and Flag. |
| **Inline citations from KB-grounded answers back to doc + section** | NotebookLM quote citations, Elicit sentence-level citations, Copilot Spaces | MEDIUM | Research turns already number their sources. Extend this to KB preamble chunks. |
| **Inbox queue**: agent-drafted additions/updates, each with diff, source URLs + `TrustChip`, proposed frontmatter (trust_tier, review_by), and Accept (opens a PR) / Reject (with reason) / Open draft | NotebookLM Discover (suggest, the human imports), KCS Not Validated → Validated, Guru verification queue | MEDIUM–HIGH | Copy the `DiscoveryInbox` list/live-event pattern. Accept creates a branch and PR. **A human merges** (locked decision). The PR is the review artifact. |
| **Duplicate check before draft** ("similar existing docs" shown on each Inbox item) | KCS search-before-create, Guru duplicate detection | MEDIUM | Run a `vox-search` similarity query over the draft title/abstract. |
| **Honest index/health status** (doc count, last indexed commit, failures, stale count) | The Cursor "shows indexed while failing" complaint | LOW | `KbHealthDto` exists. Add last-indexed SHA and an error list. Never report success on failure. |
| **Paid web spend display + cap** (free providers first, then paid until the cap) | Locked decision. OpenRouter Exa costs about $0.02 per request at defaults. | MEDIUM | Show month-to-date spend and the cap in Settings and in the Inspector footer for turns that used paid web. A turn that hits the cap is shown as "capped", not a silent failure. |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---|---|---|---|
| **Per-chunk "why selected" + dropped-candidates view** (what retrieval found but cut for budget, trust or staleness) | No surveyed product shows *what was left out*. This makes injection tunable instead of a black box. | MEDIUM | The bundle already computes candidates before truncation. Record the cut reason. |
| **Usage-weighted verification queue**: overdue docs sorted by injection count × days overdue × trust tier | Guru's "relevancy score" prioritisation plus KCS "reuse is review". The injection telemetry provides usage for free. | MEDIUM | Count injections per doc in the turn trace. Docs that are unused and stale become archive candidates (Guru auto-archive), handled by setting `status: legacy`, never by deletion (KCS logical delete). |
| **Scheduled agent re-verification**: a durable `@scheduled` job re-checks the claims of overdue docs against their cited sources and drafts an update or "still valid, extend review_by" item into the Inbox | Guru and Notion only nag humans. Here an agent does the checking and the human only approves. | HIGH | Locked for v1. Keep the v1 slice minimal (flag + draft), with the human merging. Runs on the existing durable runtime. |
| **"Flag it or fix it" from anywhere**: Flag in Inspector or Reader opens an Inbox item, and "Save answer to Library" from chat drafts an article | KCS's most effective capture practice | LOW–MED | Both produce Inbox items, so there is one review path. |
| **Counterfactual replay**: re-run a turn without source X or with injection off, and diff the answers | Directly answers "did the KB help?". It also feeds retrieval evals. | MEDIUM | Ship after the Inspector. Reuses the turn re-run machinery. |
| **Retrieval-eval health in Library** (golden-question pass rate, last run, regressions) | Quality is visible rather than assumed. Ties into the milestone's eval/lint goal. | MEDIUM | Read-only panel over eval results. The eval harness itself is a separate phase. |
| **Domain reliability list for web sources** (RSP-style categories with last-reviewed date and rationale), feeding Inbox trust_tier proposals | Categorical and auditable, better than a fake-precise score | MEDIUM | Store as a contract file in the repo so changes are reviewed via PR. |
| **STORM-style article drafting** for `docs/src/library/` (outline → sections with per-section references) | Better Inbox drafts than single-shot summaries | HIGH | Defer. Make the draft format compatible with it now. |
| **Library exposed to external agents** (llms.txt plus KB MCP tools) | The same curated KB serves Claude Code, Cursor and others, not just Vox chat | LOW | Mostly exists already. Keep llms.txt generated from frontmatter. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---|---|---|---|
| **Auto-promote web research or agent "memories" straight into injectable KB** | Feels like free growth | Indirect prompt injection persistently poisons every later session (Unit 42). Cursor removed auto-Memories. ChatGPT memory confabulation complaints. | Everything goes through Inbox, then PR, then a human merge. Drafts are **never** injected before merge. At most, a draft can be shown as an unmerged suggestion in the Inbox. |
| **Inject-everything / large default budget** | "More context = smarter" | Context rot: every model degrades and distractors amplify errors. Anthropic measured a 30% retrieval drop from unpruned history. | Small default budget, relevance plus mention selection, and tool-based just-in-time retrieval for depth |
| **Hiding or blocking unverified/stale docs** | Seems safer | Removes useful knowledge. KCS and Guru deliberately keep it visible. Hidden staleness is worse than labelled staleness. | Badge plus down-rank plus a trust-tier filter the user controls |
| **Hard-deleting stale docs** | Tidy | Breaks links and history | `status: legacy`/`deprecated` (KCS logical archive) |
| **Single numeric "trust score" per doc** | Looks precise | Commentators warn Consensus-style quality signals are weak proxies, and a number invites false confidence | Categorical `trust_tier` plus provenance links, as in Wikipedia RSP |
| **In-GUI rich editor for library docs** | Convenience | Creates a second write path around git, PR review, frontmatter lint and the doc pipeline | Read-only reader with "Open in editor". All writes go through Inbox → PR. |
| **Crawling arbitrary third-party doc sites into the KB** (Cursor `@Docs`-style) | Popular feature | Cursor's forum shows chronic staleness, silent failures and undeletable indexes. Continue deprecated `@docs`. | Web fallback at query time, and the Inbox for anything worth keeping |
| **Auto-generated wiki of the codebase** (DeepWiki-style) | Impressive demo | Duplicates and drifts from the curated corpus, and the pages aren't reviewed | `vox graph` covers structural questions, and the curated docs cover the rest |
| **A second skill/KB index or loader** | Speed | Violates the AGENTS.md SSOT ("search goes through vox-search") | Facets and filters on `vox-search` |
| **A separate web on/off toggle next to Research mode** | Literal reading of a locked control | Two switches contradicting each other (`off` vs fallback-on) | One control. Fold the fallback into `ResearchModeDropdown` semantics, or make the toggle drive it. |

---

## 3. Concrete GUI feature list and UX patterns

### Library surface (absorbs DocReader + Memory browsing)
- **Layout:** left pane is a tree or facet list (Vox docs vs `library/`, category, status). Centre is a search bar plus result list. Right pane is the reader.
- **Search hits:** title, path, section anchor, highlight (`renderHighlights` exists), and chips for trust_tier, status and staleness. Keyboard navigation (the virtual list hook exists).
- **Reader header:** title, description, `status`, `TrustChip`, a `review_by` badge (green = current, amber = due in 14 days or less, red = overdue), last-updated from git, "Used in N turns (30d)", and actions: Open in editor, Flag, Pin to current chat, Copy `@kb` mention.
- **Health strip:** doc count, last-indexed SHA, index errors, stale count, eval pass rate. Clicking stale count goes to the verification queue.
- **Verification queue tab:** overdue docs sorted by usage × overdue, with bulk actions "Draft re-verification" and "Mark legacy (PR)".

### Inbox (inside Library; it has its own badge count)
- Item card: kind (new / update / re-verify / flag), title, origin (research turn, scheduled job or user flag, with a link), proposed path, **unified diff**, cited sources with `TrustChip`, proposed `trust_tier`/`review_by`, and similar existing docs.
- Actions: **Open PR** (creates a branch and commit, and the link appears on the card), **Reject** (a reason is required, and it feeds future drafting), Snooze, Open draft in editor.
- States follow KCS: Draft → PR open → Merged / Rejected. Live updates and a toast for high-confidence items reuse the `DiscoveryInbox` mechanics.
- Never offer "accept and inject now". Merge is the only path to injectable.

### Inspector (per assistant turn; expandable in `TurnTrace`, with a dockable rail)
- Header: total tokens injected vs budget and a breakdown by activation (always / relevance / @mention / pinned / web). This extends `ContextWindowMeter`.
- One row per chunk: source path#section, tokens, rank/score + backend, trust tier, staleness badge, and actions **Open**, **Exclude from chat** (the `onExcludeSkill` pattern), and **Flag**.
- A "Dropped" disclosure listing candidates cut, with the reason (budget, tier, stale, excluded).
- Web block: provider, free/paid, cost of this turn, and the cap state.
- Later (v1.x): "Re-run without…" for counterfactual replay.

### Chat composer controls (per chat)
- An `@kb` typeahead, and an active-scope chip row under the composer (removable chips).
- A compact "Context" popover: injection on/off, budget slider, minimum trust tier, a pin mode (restrict/prioritise), and a link to the Inspector. The research/web mode stays in the existing dropdown.

### Settings (defaults)
- Default injection on/off and budget. Default minimum trust tier. Down-rank stale docs (on). Default `review_by` interval per trust tier. Re-verification schedule. Web provider order (free first) and the monthly paid cap with month-to-date spend. A domain reliability list (read-only view of the contract file plus "propose change" which goes to the Inbox).

---

## Feature Dependencies

```
Frontmatter schema (trust_tier, review_by) + lint
    └──requires──> Library facets / staleness badges / trust filter
                       └──requires──> Verification queue ──> Scheduled re-verification

Retrieval bundle records per-chunk evidence (+ dropped candidates, activation reason)
    └──requires──> Inspector
                       ├──enables──> Exclude/Flag from Inspector
                       └──enables──> Counterfactual replay (v1.x)
    └──requires──> Inline KB citations

Injection telemetry (doc used in turn) ──enhances──> Usage-weighted verification queue

Inbox item model + PR creation
    └──required by──> Web-research auto-growth, Flag-it, Save-answer-to-Library,
                       Scheduled re-verification drafts

Per-chat scope state (@kb, pin restrict/prioritise, exclude, budget, tier)
    └──requires──> Composer typeahead + Context popover
    ──conflicts──> a separate web toggle (must unify with ResearchModeDropdown)

Paid-web spend ledger ──requires──> cap enforcement ──> Settings + Inspector cost display

Library surface ──absorbs──> MemoryView search + DocReader (retire their nav entries)
```

### Dependency Notes
- **The frontmatter schema comes first.** Badges, filters, queue and re-verification all read `trust_tier`/`review_by`. Lint them before building UI on them. Map `trust_tier` onto the existing `ContextTrustTier` enum.
- **The Inspector depends on backend evidence capture.** `RetrievalEvidenceEnvelope` currently reports lines and backends. Per-chunk tokens, activation reason and dropped candidates have to be emitted into the turn events first. Without that, the Inspector shows a guess.
- **Every growth path converges on the Inbox.** Build the Inbox and PR plumbing once, before web auto-growth or re-verification, so neither invents its own review path.
- **The web-fallback toggle conflicts with `ResearchModeDropdown` `off`.** This needs to be resolved in requirements.

---

## MVP Definition

### Launch With (v1), matching the locked decisions
- [ ] Frontmatter `trust_tier` + `review_by` schema, lint, and mapping to `ContextTrustTier`. Everything else keys off it.
- [ ] Library surface: facet search (absorbing Memory), rendered reader with provenance header (absorbing DocReader), staleness badges, health strip.
- [ ] Per-chat controls: `@kb` typeahead + scope chips, injection on/off + budget, trust-tier filter, pin (restrict/prioritise) / exclude, and a web fallback unified with research mode.
- [ ] Inspector: per-chunk rows, activation reason, tokens vs budget, Open/Exclude/Flag, and web cost line.
- [ ] Inbox: draft → PR → human merge, with diff, sources and TrustChips, a similar-docs check, and reject with reason. Nothing is injectable before merge.
- [ ] Scheduled re-verification (minimal): flag overdue docs and draft re-verify items into the Inbox.
- [ ] Free-first web with a paid cap, and spend shown in Settings and the Inspector.

### Add After Validation (v1.x)
- [ ] Dropped-candidates view: add once the Inspector shows users asking "why wasn't X used?".
- [ ] Usage-weighted verification queue: add once the injection telemetry has about 30 days of data.
- [ ] Counterfactual replay: add when the retrieval evals need per-source attribution.
- [ ] Retrieval-eval health panel in Library: add once the eval harness phase lands.
- [ ] Domain reliability list contract plus editor-via-Inbox.

### Future Consideration (v2+)
- [ ] STORM-style outline-first article drafting: defer until Inbox draft quality is measured.
- [ ] Shared/team scopes à la Copilot Spaces: defer until there is a multi-user need.
- [ ] Any third-party doc-site crawling: likely never (see anti-features).

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---|---|---|---|
| Frontmatter trust_tier/review_by + lint | HIGH | LOW | P1 |
| Library browse/search/reader (absorb Memory+DocReader) | HIGH | MEDIUM | P1 |
| Staleness badges + down-rank | HIGH | LOW | P1 |
| Inspector (per-chunk) | HIGH | MEDIUM | P1 |
| `@kb` typeahead + scope chips | HIGH | MEDIUM | P1 |
| Injection toggle + budget + tier filter | HIGH | LOW | P1 |
| Pin restrict/prioritise + exclude | MEDIUM | MEDIUM | P1 |
| Inbox → PR → merge | HIGH | HIGH | P1 |
| Web fallback unified with research mode | MEDIUM | LOW | P1 |
| Paid spend cap + display | MEDIUM | MEDIUM | P1 |
| Scheduled re-verification (minimal) | MEDIUM | HIGH | P1 (locked) |
| Inline KB citations | HIGH | MEDIUM | P1 |
| Dropped-candidates view | MEDIUM | LOW | P2 |
| Usage-weighted verification queue | MEDIUM | MEDIUM | P2 |
| Counterfactual replay | MEDIUM | MEDIUM | P2 |
| Eval health panel | MEDIUM | LOW | P2 |
| Domain reliability list | MEDIUM | MEDIUM | P2 |
| STORM drafting | LOW | HIGH | P3 |

## Competitor Feature Analysis

| Feature | Guru / Notion | NotebookLM / ChatGPT DR | Cursor / Claude Code | Our Approach |
|---|---|---|---|---|
| Scope control | Verified-first ranking | Per-source checkboxes; restrict vs prioritise sites | Rule activation modes; `@` mentions; path-scoped rules | `@kb` + pin (restrict/prioritise) + exclude + trust-tier floor |
| Provenance | Verifier + date badge | Quote-level inline citations | Minimal | TrustChip + git date + section-level citations + Inspector |
| Staleness | Interval → unverified → owner queue → auto-archive | n/a | Stale `@Docs` (complaint); prompt-audit | `review_by` badge + down-rank + agent re-verification drafts + `legacy` archival |
| Ingestion | Manual cards, assigned verifier | Discover sources: suggest, user imports | Rules hand-written; auto-memories removed | Agent drafts → Inbox → PR → human merge; never injected pre-merge |
| Context visibility | none | Sources panel | `/context` per-category tokens | Per-turn per-chunk Inspector with reasons + dropped list |

## Sources

- Cursor: https://cursor.com/docs/context/rules ; https://docs.cursor.com/context/@-symbols/@-docs ; forum threads 58871, 38940, 98728, 144057 (linked above)
- Claude Code memory: https://code.claude.com/docs/en/memory ; /context: https://blog.vincentqiao.com/en/posts/claude-code-context/
- Windsurf memories: https://docs.windsurf.com/windsurf/cascade/memories
- Continue @docs: https://docs.continue.dev/customize/deep-dives/docs ; https://docs.continue.dev/reference/deprecated-docs
- Amp: https://codex.danielvaughan.com/2026/04/07/sourcegraph-amp-multi-repo-orchestrator/ ; inspector request: https://community.openai.com/t/developer-context-inspector-for-codex-sessions/1393972
- Copilot Spaces: https://docs.github.com/copilot/using-github-copilot/copilot-spaces/about-organizing-and-sharing-context-with-copilot-spaces
- DeepWiki: https://docs.devin.ai/work-with-devin/deepwiki
- Guru: https://www.getguru.com/features/verification
- Notion: https://notion.com/help/wikis-and-verified-pages
- Rovo: https://slite.com/learn/atlassian-rovo-ai-review
- llms.txt: https://www.mintlify.com/library/best-llms-txt-platforms ; https://codersera.com/blog/llms-txt-complete-guide-2026/amp/
- NotebookLM: https://support.google.com/notebooklm/answer/14276569?hl=en ; https://blog.google/technology/google-labs/notebooklm-discover-sources/
- ChatGPT deep research: https://help.openai.com/en/articles/10500283-deep-research
- Consensus/Elicit: https://aarontay.substack.com/p/a-2025-deep-dive-of-consensus-promises ; https://casrai.org/compare/elicit-vs-consensus
- STORM: https://github.com/sugatoray/stanford-storm
- KCS v6: https://library.serviceinnovation.org/KCS/KCS_v6/KCS_v6_Practices_Guide/030/040/010/030
- Wikipedia RSP: https://en.wikipedia.org/wiki/Wikipedia:Reliable_sources/Perennial_sources
- Context rot: https://research.trychroma.com/context-rot ; Anthropic context engineering summary: https://agentic-ai.readthedocs.io/en/latest/ContextEngineering/anthropic/
- Memory poisoning: https://unit42.paloaltonetworks.com/indirect-prompt-injection-poisons-ai-longterm-memory/
- Citation hallucination: https://arxiv.org/pdf/2601.05866 ; https://arxiv.org/pdf/2510.11394
- OpenRouter web pricing: https://openrouter.ai/docs/web-search
- Local code: `crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs`, `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`, `crates/vox-orchestrator/src/context_envelope.rs`, `crates/vox-gui/ui/src/components/surfaces/{Memory,DocReader,Research,Scientia,Chat}/`

---
*Feature research for: Vox Knowledge Base (workstream kb, milestone v1.1)*
*Researched: 2026-10-03*
