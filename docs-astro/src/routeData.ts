/**
 * Starlight route middleware.
 *
 * 1. "Last updated" dates and edit links. Starlight's own lookups use the
 *    entry's path under `src/content/docs`, a gitignored mirror of links into
 *    `docs/src` (plus `repo/` mounts of repo Markdown), so its git dates are
 *    empty and its edit URLs name a path that does not exist on GitHub. Both
 *    come from the page's true repo file instead: `mounted_from` for a
 *    generated mount page, otherwise the real path behind the mirror link.
 *    Dates use the repo-root git-date map, which skips mechanical commits and
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
import { execFileSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { getGitDates } from './utils/git-dates.mjs';
import { statusPolicy } from './utils/page-status.mjs';
import { discoverMounts } from './utils/repo-mounts.mjs';

const EDIT_BASE = 'https://github.com/vox-foundation/vox/edit/main/';

let gitDates: Map<string, string> | undefined;
let repoRoot: string | null | undefined;

/** Real path of the repo root, from git (this module runs from a bundled chunk). */
function getRepoRoot(): string | null {
  if (repoRoot === undefined) {
    try {
      repoRoot = realpathSync(execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim());
    } catch {
      repoRoot = null;
    }
  }
  return repoRoot;
}

/**
 * Repo-relative path of the file a content entry renders (`docs/src/x.md`,
 * `AGENTS.md`). `filePath` is relative to the Astro root, `docs-astro/`.
 */
function sourcePath(filePath: string, mountedFrom: unknown): string {
  if (typeof mountedFrom === 'string') return mountedFrom;
  const posix = filePath.replace(/\\/g, '/');
  const root = getRepoRoot();
  if (root) {
    try {
      const rel = relative(root, realpathSync(join(root, 'docs-astro', posix)));
      if (!rel.startsWith('..')) return rel.split(sep).join('/');
    } catch {
      // fall back to the mirror layout below
    }
  }
  return 'docs/src/' + posix.replace(/^.*?src\/content\/docs\//, '');
}

/** docs/src dates, then mounted repo files' dates (a separate log, so docs/src dates are unchanged by mounts). */
function loadGitDates(): Map<string, string> {
  const root = getRepoRoot();
  const dates = new Map(getGitDates(root ?? undefined));
  if (root) {
    const mounted = discoverMounts({ repoRoot: root }).map((mount) => mount.repoPath);
    if (mounted.length) for (const [path, iso] of getGitDates(root, { paths: mounted })) if (!dates.has(path)) dates.set(path, iso);
  }
  return dates;
}

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

  gitDates ??= loadGitDates();
  const source = sourcePath(route.entry.filePath, route.entry.data.mounted_from);
  const iso = gitDates.get(source);
  if (iso) route.lastUpdated = new Date(iso);
  if (route.editUrl && typeof route.entry.data.editUrl !== 'string') route.editUrl = new URL(EDIT_BASE + encodeURI(source));

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
