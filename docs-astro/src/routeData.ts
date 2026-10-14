/**
 * Starlight route middleware: suppress crawlers for archive content,
 * provide accurate Git-derived lastUpdated dates, and apply status policy
 * (banner, robots noindex, and Pagefind internals metadata).
 *
 * Ref: https://starlight.astro.build/reference/route-data/
 */
import { existsSync, realpathSync, statSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import { getGitDates } from './utils/git-dates.mjs';
import { statusPolicy, bannerFor } from './utils/page-status.mjs';
import { discoverMounts, loadMountContract, DEFAULT_REPO_ROOT } from './utils/repo-mounts.mjs';

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function resolveContentPath(filePath: string): string {
  if (isAbsolute(filePath)) return filePath;
  if (filePath.startsWith('docs-astro/')) {
    return resolve(DEFAULT_REPO_ROOT, filePath);
  }
  if (filePath.startsWith('src/content/docs/')) {
    return resolve(DEFAULT_REPO_ROOT, 'docs-astro', filePath);
  }
  const direct = resolve(DEFAULT_REPO_ROOT, 'docs-astro', filePath);
  if (existsSync(direct)) return direct;
  return resolve(DEFAULT_REPO_ROOT, 'docs-astro/src/content/docs', filePath);
}

let gitDatesMap: Map<string, string> | null = null;

export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute;
  if (!route) return;

  const slug = route.id ?? '';
  const { head } = route;

  // Suppress search engine crawling for any archive/ path and the raw SUMMARY page
  if (slug.startsWith('archive/') || slug === 'summary') {
    if (head) {
      const hasRobots = head.some(
        (t) => t.tag === 'meta' && t.attrs?.name === 'robots' && t.attrs?.content === 'noindex'
      );
      if (!hasRobots) {
        head.push({
          tag: 'meta',
          attrs: { name: 'robots', content: 'noindex' },
        });
      }
    }
  }

  // Handle status policy: banner, noindex, internals pagefind metadata
  const entryData = route.entry?.data as Record<string, any> | undefined;
  if (entryData) {
    const status = entryData.status;
    const policy = statusPolicy(status);

    const bannerContent = bannerFor(status);
    if (bannerContent && !entryData.banner) {
      entryData.banner = { content: bannerContent };
    }

    if (policy.noindex && head) {
      const hasRobots = head.some(
        (t) => t.tag === 'meta' && t.attrs?.name === 'robots' && t.attrs?.content === 'noindex'
      );
      if (!hasRobots) {
        head.push({
          tag: 'meta',
          attrs: { name: 'robots', content: 'noindex' },
        });
      }
    }

    if (policy.internals && head) {
      head.push({
        tag: 'meta',
        attrs: { 'data-pagefind-filter': 'section[content]', content: 'Internals' },
      });
      const title = entryData.title ? escapeHtml(String(entryData.title)) : '';
      head.push({
        tag: 'meta',
        attrs: { 'data-pagefind-meta': 'title[content]', content: `Internals — ${title}` },
      });
    }
  }

  // Set per-page lastUpdated from Git dates and editUrl for mounted pages
  if (route.entry?.filePath) {
    if (!gitDatesMap) {
      const contract = loadMountContract();
      const mounts = discoverMounts({ repoRoot: DEFAULT_REPO_ROOT, contract });
      const mountPaths = mounts.map((m: any) => m.repoPath);
      gitDatesMap = getGitDates(DEFAULT_REPO_ROOT, { paths: ['docs/src', ...mountPaths] });
    }

    const diskPath = resolveContentPath(route.entry.filePath);
    let sourcePath: string | null = null;
    if (entryData?.mounted_from && typeof entryData.mounted_from === 'string') {
      sourcePath = entryData.mounted_from;
    } else {
      try {
        const real = realpathSync(diskPath);
        const rel = relative(DEFAULT_REPO_ROOT, real).replace(/\\/g, '/');
        if (!rel.startsWith('..') && !isAbsolute(rel)) {
          sourcePath = rel;
        }
      } catch {
        // Fallback
      }
      if (!sourcePath) {
        const cleanPath = route.entry.filePath.replace(/^src\/content\/docs\//, '');
        sourcePath = `docs/src/${cleanPath}`;
      }
    }

    if (sourcePath) {
      if (entryData) {
        entryData._sourcePath = sourcePath;
      }
      let iso = gitDatesMap.get(sourcePath);
      if (!iso) {
        try {
          const statTarget = existsSync(diskPath) ? diskPath : resolve(DEFAULT_REPO_ROOT, sourcePath);
          iso = statSync(statTarget).mtime.toISOString();
        } catch {
          // ignore
        }
      }
      if (iso) {
        route.lastUpdated = new Date(iso);
      }

      const cleanSource = sourcePath.replace(/^\/+/, '');
      route.editUrl = new URL(`https://github.com/vox-foundation/vox/edit/main/${cleanSource}`);
    }
  }
});
