# Plan 20-08 Summary: Relative Markdown Link Rewriting

## What was built
- **Remark Plugin (`docs-astro/src/plugins/remark-doc-links.mjs`)**:
  - Exported pure `resolveDocLink(url, fromFile, ctx)` and plugin `remarkDocLinks(options)`.
  - In-docs relative links (`foo.md`, `../reference/installation.md#anchor`) are rewritten to clean Starlight URLs (`/reference/installation/#anchor`), computing routes via `docSlug()`.
  - Out-of-tree links leaving `docs/src` to repository code or contracts are rewritten to GitHub URLs (`https://github.com/vox-foundation/vox/blob/main/<path>` or `/tree/main/<path>` for directories).
  - Handles line number and range suffixes (`:42` -> `#L42`, `:10-20` -> `#L10-L20`).
  - Out-of-tree documentation (`docs/superpowers/**`) and retired research (`docs/src/archive/**`) are sent to GitHub blob.
  - Path traversal guard: links attempting to escape the repository root throw an error in strict mode.
  - Robust AST walker: rewrites both markdown `link`/`definition` nodes and embedded raw HTML `<a href="...">` nodes.
- **Astro Config Registration (`docs-astro/astro.config.mjs`)**:
  - Registered `remarkDocLinks` in `markdown.remarkPlugins` right after `remarkVoxInclude` with `{ repoUrl: 'https://github.com/vox-foundation/vox' }`.
- **Unit Tests (`docs-astro/tests/unit/remark-doc-links.test.mjs`)**:
  - 12/12 unit tests passing covering in-docs links, anchors, index routes, README routes, missing targets, out-of-tree files, line ranges, directories, AGENTS.md, archive, and repository escape attempts.
- **Build Output Verification Spec (`docs-astro/tests/build-output/links.spec.ts`)**:
  - `dist/tutorials/tut-getting-started/index.html` verified to contain `/reference/installation/` and `/reference/installation/#quick-install-from-source` with 0 relative `.md` links.
  - Whole-site proof: 0 relative `.md` hrefs across all 691 built HTML files.
  - Whole-site proof: 0 relative links climb above the site root.
  - Whole-site proof: 2,300 GitHub blob/tree links checked against the repo working tree; 0 broken links.

## Verification & Metrics
- In-docs relative links rewritten across site: all in-docs markdown cross-links now render as clean site URLs.
- Relative `.md` href count in built HTML: **0**
- Escaping relative href count: **0**
- GitHub repo links checked and verified: **2,300**
- Tests passing:
  - `node --test tests/unit/remark-doc-links.test.mjs`: 12/12 pass
  - `npx playwright test tests/build-output/links.spec.ts`: 4/4 pass
  - Full playwright suite: 34/34 pass (1 skipped)
- Mutation test:
  - Injected `[Mutation Test](nonexistent-mutation-target-999.md)` into `docs/src/tutorials/tut-getting-started.md`.
  - Build failed during parsing with `remark-doc-links: Target not found: 'nonexistent-mutation-target-999.md' in '.../tut-getting-started.md'`.
  - Restored tutorial; build and tests returned to green.
