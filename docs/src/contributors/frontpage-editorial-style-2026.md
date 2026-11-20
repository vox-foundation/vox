---
title: "Frontpage and README Editorial Style (2026)"
description: "The writing register, section structure, banned wording and README-versus-frontpage split for voxlang.org and the repository README."
category: "Contributors"
status: "current"
topic: "doc-governance"
domain: "documentation"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Frontpage and README Editorial Style (2026)

<!-- kb:summary -->
Rules for the two pages most readers see first: the voxlang.org frontpage and the repository README. It sets a plain news-report register, a what/why/benefit structure for every section, the wording to avoid, and which page owns which content. A Playwright check enforces the banned wording on the frontpage. The goal is that a newcomer learns what Vox is and why it works the way it does without being sold to.
<!-- /kb:summary -->

## Register

Write like a careful news report about the project.

- State what something is, then why it was built that way, then who gains. Keep the three separate.
- Prefer a number or a link to an adjective. If a claim has a source, link it. If it comes from the project's own benchmark, say so.
- Third person, present tense, short sentences. Explain a term the first time it appears.
- Name limits next to the feature they limit: "Prototype", "needs `VOX_REPO_ROOT`", "one benchmark, 100 tasks".
- Do not repeat a claim. The idea that one declaration produces a database table, a server, a client and an agent tool is made in one place on the frontpage (the playground and "Why Vox") and nowhere else.

## Structure of a feature section or slide

1. **What it is.** One or two sentences.
2. **Why it is built this way.** The problem it avoids.
3. **For AI-written code.** What an agent gets that it would not otherwise have.
4. **For the person using it.** The practical difference.
5. **Try it.** A real command and a link.

Terminal output shown on the page must be real output of the command shown. If it was shortened, say so.

## Wording to avoid

Do not use: revolutionary, seamless, seamlessly, blazing, blazingly, cutting-edge, game-changing, supercharge, unlock, effortless, effortlessly, next-generation, world-class, best-in-class, magical, 10x, turnkey, "zero drift", "100%" as a bare guarantee. The frontpage check in `docs-astro/tests/suite-portal-showcase.spec.ts` fails on these.

## Which page owns what

| Content | Frontpage (voxlang.org) | README (repository) |
| :--- | :--- | :--- |
| Audience | Someone deciding whether to look further | Someone building, changing or auditing the repository |
| Owns | What Vox is, the playground, the two tours, the "Why Vox" explanation, a short status summary | Build and install steps, the full status tables and log, repository layout, contributing, citations, licence |
| Does not repeat | Repository layout, the full status matrix, the change log | Interactive demos and the slide-by-slide tours |
| Shared text | The `why_vox` block, kept identical in both files | The same block |

Counts that appear on both pages (tool count, view count, benchmark figures) must come from a generated source or be checked by the docs drift audit. Do not retype a number that a generator already publishes.

## Graphics

- Prefer a real capture or a real command output to an illustration.
- Any text inside an image must be exact and must match what the product does. Re-generate the image if it does not.
- Provide light and dark versions, or use a vector (SVG) that works on both.
- Captures are taken at 2× and shipped at 1280 and 2560 pixels wide.
- Say when a capture uses sample data.
