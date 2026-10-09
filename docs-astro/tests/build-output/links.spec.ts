import { test, expect } from '@playwright/test';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DIST_DIR, listDistHtml, readDist } from '../lib/dist';

const REPO_ROOT = fileURLToPath(new URL('../../..', import.meta.url));
const GITHUB_RE = /^https:\/\/github\.com\/vox-foundation\/vox\/(?:blob|tree)\/main\/([^#?]*)/;

const hrefs = (html: string) => [...html.matchAll(/\bhref="([^"]*)"/g)].map((m) => m[1].replace(/&amp;/g, '&'));
const isRelative = (href: string) => !/^([a-z][a-z0-9+.-]*:|\/|#)/i.test(href);
const isMarkdown = (href: string) => /\.mdx?(#|$)/.test(href);

/** True when a relative `href` on the page at `file` (dist-relative) climbs above the site root. */
function climbsAboveRoot(file: string, href: string): boolean {
  let depth = file.split('/').length - 1;
  for (const segment of href.split(/[?#]/)[0].split('/')) {
    if (segment === '..') depth -= 1;
    else if (segment && segment !== '.') depth += 1;
    if (depth < 0) return true;
  }
  return false;
}

/** Every `href` on every built page, as `{ page, href }`. */
function allLinks(): { page: string; href: string }[] {
  return listDistHtml().flatMap((page) =>
    hrefs(readFileSync(join(DIST_DIR, page), 'utf8')).map((href) => ({ page, href })),
  );
}

const report = (offenders: { page: string; href: string }[]) =>
  offenders.slice(0, 20).map(({ page, href }) => `${page}: ${href}`);

test.describe('repo-relative links', () => {
  test('tutorial links to other docs pages render as site routes', () => {
    const links = hrefs(readDist('tutorials/tut-getting-started'));
    expect(links).toContain('/reference/installation/');
    expect(links.filter((href) => isRelative(href) && isMarkdown(href))).toEqual([]);
  });

  const links = allLinks();

  test('no page has a relative .md href', () => {
    const offenders = links.filter(({ href }) => isRelative(href) && isMarkdown(href));
    expect(report(offenders), `${offenders.length} relative .md hrefs`).toEqual([]);
  });

  test('no relative href climbs above the site root', () => {
    const offenders = links.filter(({ page, href }) => isRelative(href) && climbsAboveRoot(page, href));
    expect(report(offenders), `${offenders.length} hrefs above the root`).toEqual([]);
  });

  test('every GitHub blob/tree link names a path in the repository', () => {
    const github = links.filter(({ href }) => GITHUB_RE.test(href));
    console.log(`links.spec: ${github.length} GitHub blob/tree links checked`);
    expect(github.length).toBeGreaterThan(1000);
    const offenders = github.filter(({ href }) => {
      const path = decodeURI(href.match(GITHUB_RE)![1]);
      return !existsSync(join(REPO_ROOT, path));
    });
    expect(report(offenders), `${offenders.length} GitHub links to missing paths`).toEqual([]);
  });
});
