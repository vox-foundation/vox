---
title: "Library"
description: "Home for general, non-Vox curated knowledge that Vox agents rely on, indexed by the KB alongside the Vox docs."
category: "Library"
status: "current"
sort_order: 1
---

# Library

The Library holds general curated knowledge that Vox agents and contributors rely on but
that is not about Vox internals: reference notes on external tools, protocols, and
techniques.

- **Scope:** general knowledge only. Vox architecture, language, and CLI docs stay in
  their existing sections.
- **How pages get in:** through a reviewed, human-merged pull request, like any other doc.
  There is no other path.
- **Frontmatter:** every page needs the standard frontmatter (`title`, `description`,
  `category: "Library"`); see [documentation governance](../contributors/documentation-governance.md).
- **Indexing:** `vox kb reindex` indexes library pages together with the Vox docs, so no
  extra configuration is needed when a page is added.
