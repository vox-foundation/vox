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
 * 3. Frontmatter `status` (rule in utils/page-status.mjs): a banner, robots
 *    `noindex`, and for Internals pages Pagefind tags that label them in search
 *    results. Pages stay in the Pagefind index.
 *
 * Ref: https://starlight.astro.build/reference/route-data/
 */
import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import { getGitDates } from './utils/git-dates.mjs';
import { statusPolicy } from './utils/page-status.mjs';

let gitDates: Map<string, string> | undefined;

type HeadEntry = { tag: string; attrs?: Record<string, string | boolean | undefined>; content?: string };

function pushMetaOnce(head: HeadEntry[] | undefined, attrs: Record<string, string>) {
  if (!head) return;
  const same = (entry: HeadEntry) =>
    entry.tag === 'meta' &&
    Object.keys(attrs).length === Object.keys(entry.attrs ?? {}).length &&
    Object.entries(attrs).every(([key, value]) => entry.attrs?.[key] === value);
  if (!head.some(same)) head.push({ tag: 'meta', attrs });
}

const NOINDEX = { name: 'robots', content: 'noindex' };

export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute;
  if (!route) return;

  gitDates ??= getGitDates();
  const key = 'docs/src/' + route.entry.filePath.replace(/\\/g, '/').replace(/^.*?src\/content\/docs\//, '');
  const iso = gitDates.get(key);
  if (iso) route.lastUpdated = new Date(iso);

  const slug = route.id ?? '';
  if (slug.startsWith('archive/') || slug === 'summary') {
    pushMetaOnce(route.head, NOINDEX);
  }

  const data = route.entry.data as typeof route.entry.data & { status?: string };
  const policy = statusPolicy(data.status);
  if (policy.banner && !data.banner) data.banner = { content: policy.banner };
  if (policy.noindex) pushMetaOnce(route.head, NOINDEX);
  if (policy.internals) {
    // Astro escapes attribute values, so the frontmatter title is passed as-is.
    pushMetaOnce(route.head, { 'data-pagefind-filter': 'section[content]', content: 'Internals' });
    pushMetaOnce(route.head, { 'data-pagefind-meta': 'title[content]', content: `Internals — ${data.title}` });
  }
});
