# Golden-set seed questions (human-authored)

Scaffold prepared by the agent. The wording of every question below the `## Queries` heading is yours alone
(EVAL-02, AP-02). This file lives outside every index root and must stay free of the canary GUID
and of anything copied from `contracts/kb/eval/README.md`. It is deleted once the BEIR files are materialized.

author: agent-seed (Claude Opus, delegated by brbrainerd 2026-10-04; see 01-CONTEXT D-02)

## Intents

Aim for at least 4 questions per intent.

| Intent | Meaning |
|---|---|
| `where` | Where does something live: a file, crate, doc or config key |
| `how-to` | How do I accomplish a concrete task |
| `concept` | What is X, or how does X work |
| `policy` | What is the rule, allowed or forbidden, required process |
| `why` | Why was a decision made, why does it work this way |
| `troubleshoot` | Something fails or behaves unexpectedly; what is the cause or fix |
| `multi-hop` | The answer needs two or more docs combined |

## Corpus areas

Tracked, non-archive `.md` files under `docs/src`, grouped by first path segment. Total: 670 docs.
Spread your questions across these areas, weighted by how often you really ask about them.

| Area (`docs/src/<area>/`) | Docs |
|---|---|
| architecture | 390 |
| reference | 108 |
| adr | 49 |
| how-to | 38 |
| contributors | 21 |
| ci | 19 |
| explanation | 17 |
| tutorials | 7 |
| operations | 6 |
| journeys | 4 |
| learning | 3 |
| (top-level files) | 2 |
| api | 2 |
| ref | 2 |
| case-studies | 1 |
| examples | 1 |
| **Total** | **670** |

Topics that cut across areas: CI and gates, secrets (Clavis), CLI surface, GUI (Tauri), MENS training,
orchestrator and agents, search and retrieval, crypto policy, versioning, build broker, language syntax.

## Sources of real questions

- The primary-navigation list in `AGENTS.md`: what would you ask an agent that points at those entries.
- Issue titles: `gh issue list --state all --limit 100 --json title`.
- Things you or your agents recurrently ask or get wrong during sessions.
- Moments you did not know where something lived, or why a rule exists.

## Authoring rules

- Write each question the way you would ask an agent, in your own words.
- Do not copy or lightly rephrase sentences from the docs. Questions written from a doc reward lexical echo.
- Spread across intents (at least 4 each) and across areas.
- Target at least 40, aim for 48.
- One question per line, in exactly this format, under `## Queries`:

  `- [intent] question text`

  where `intent` is one of `where`, `how-to`, `concept`, `policy`, `why`, `troubleshoot`, `multi-hop`.
- Fill in the `author:` line above.
- Do not paste anything from `contracts/kb/eval/README.md` into this file.

## Queries

- [where] I'm adding a new subsystem - is there a lookup that tells me which crate it should go in, by layer?
- [where] Which page tracks the current names of the daemon binary and env var families versus old aliases like vox-dei-d?
- [where] Where can I get a nightly binary without building from source, is there a tag I can pull?
- [where] what env var tells vox-db which remote libSQL database to connect to?
- [where] which DB table holds the per-activity replay log for durable workflows, and what is it keyed on?
- [where] where's the measured breakdown of why local commits and pushes were taking minutes to hours?
- [where] In the component taxonomy, which crate owns agent capabilities like file or network access?
- [how-to] how do I fine-tune MENS on an Apple Silicon laptop using Metal?
- [how-to] how do I limit how many cargo builds the broker lets run at once on this box?
- [how-to] how do I get a plain SKILL.md folder picked up by vox and by the other agent harnesses?
- [how-to] steps to run a populi cluster across my own machines linked by Tailscale or WireGuard
- [how-to] how do I write property tests or a fuzz target for a function in a .vox file?
- [how-to] make vox chat use a model I pulled into Ollama instead of a cloud provider
- [how-to] I just built vox from source, walk me through making a new project, type-checking it and running it
- [concept] actor vs activity vs workflow in Vox, when would I reach for each?
- [concept] what kinds of data go in libSQL vs append-only logs vs content-addressed blobs?
- [concept] Socrates protocol: what is it and what states does a question move through?
- [concept] how are MCP tool calls labeled by side-effect level, e.g. read-only vs touching the network?
- [concept] what does the Vox IR represent and how stable is it meant to be?
- [concept] after a crash, what happens to activities that already finished when the workflow resumes?
- [concept] how do the CLI, orchestrator and MCP tools avoid each carrying their own retrieval implementation?
- [policy] can I depend on blake3 or sha2 directly in my crate, or does hashing have to go through something else?
- [policy] are we allowed to vendor code from an AGPL project, and what provenance must we record for MIT/Apache copies?
- [policy] what frontmatter does a new page under docs/src need and which category values are accepted?
- [policy] is pushing with --no-verify ever acceptable, and what am I expected to do right after?
- [policy] may a crate read TURSO_* environment variables directly to get database credentials?
- [policy] does vox send any telemetry off the machine without me opting in, and can payloads include source code?
- [policy] how big can a single .rs file get before the toestub audit fails the build?
- [why] what drove moving model fine-tuning off the Python toolchain?
- [why] why was the browser-served dashboard dropped in favor of a native desktop shell?
- [why] why adopt iroh for mesh networking instead of finishing the home-grown HTTP control plane?
- [why] why do VoxScripts sent to a peer run in the interpreter rather than wasm or containers?
- [why] why are some release workflows exempt from canceling older runs?
- [why] if LLMs aren't actually better at PowerShell, why prefer pwsh for agent shells on Windows?
- [why] why does the lockfile end up with both ring and aws-lc-rs?
- [troubleshoot] invoking vox ci ssot-drift by hand flags crates that are already allowlisted, but the git hook passes
- [troubleshoot] cargo errors that a dependency I added to a member crate isn't defined at the root
- [troubleshoot] cargo test just sits there warning that a test is still going after a minute, but it never times out
- [troubleshoot] vox run exits with status 77 on my script
- [troubleshoot] worker registration against the populi server keeps coming back 401
- [troubleshoot] budget model selection keeps choosing an expensive Anthropic model as if it were free
- [troubleshoot] secrets doctor warns that my API key uses a deprecated alias name - how long will it keep working?
- [multi-hop] adding a brand-new provider API key: where do I register it, and what's the emergency procedure if it leaks?
- [multi-hop] my workflow branches on a random number - will replay reproduce it, and will anything catch that before runtime?
- [multi-hop] which bundle should a mesh node install, and what limits apply to scripts it receives from peers?
- [multi-hop] when I pick the Docs chip in the GUI search palette, which crate actually runs the query and over which corpora?
- [multi-hop] I want to ship local telemetry to our own ingest server - what's the opt-in flow and which settings hold the URL and token?
- [multi-hop] my PR only bumps Cargo.lock - does CI test the whole workspace, and how do I reproduce that locally first?
