/**
 * Starlight route middleware.
 *
 * 1. "Last updated" dates. Starlight's own lookup (`lastUpdated: true`) runs
 *    `git log` on `src/content/docs`, a gitignored symlink to `docs/src`, so
 *    it finds no history and no page gets a date. Dates come from the
 *    repo-root git-date map instead, which skips mechanical commits and
 *    follows renames.
 * 2. `<meta name="robots" content="noindex">` for archive content and the raw
 *    SUMMARY page, so crawlers skip them. This affects crawlers only; Pagefind
 *    indexing is controlled by `data-pagefind-body` / frontmatter `pagefind`.
 *
 * Ref: https://starlight.astro.build/reference/route-data/
 */
import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import { getGitDates } from './utils/git-dates.mjs';

let gitDates: Map<string, string> | undefined;

export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute;
  if (!route) return;

  gitDates ??= getGitDates();
  const key = 'docs/src/' + route.entry.filePath.replace(/\\/g, '/').replace(/^.*?src\/content\/docs\//, '');
  const iso = gitDates.get(key);
  if (iso) route.lastUpdated = new Date(iso);

  const slug = route.id ?? '';
  if (slug.startsWith('archive/') || slug === 'summary') {
    route.head?.push({
      tag: 'meta',
      attrs: { name: 'robots', content: 'noindex' },
    });
  }
});
