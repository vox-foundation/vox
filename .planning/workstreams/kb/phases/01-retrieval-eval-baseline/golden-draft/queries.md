# Golden-set seed questions (human-authored)

Scaffold prepared by the agent. The wording of every question below the `## Queries` heading is yours alone
(EVAL-02, AP-02). This file lives outside every index root and must stay free of the canary GUID
and of anything copied from `contracts/kb/eval/README.md`. It is deleted once the BEIR files are materialized.

author:

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

