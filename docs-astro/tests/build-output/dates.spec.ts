import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { getGitDates, readIgnoreRevs } from '../../src/utils/git-dates.mjs';
import { DIST_DIR, listDistHtml, readDist } from '../lib/dist';

const TIME_RE = /<time datetime="([^"]+)"/;
const STRIP_COMMIT = '4e98a0f87ee3cbae99925d6f14843e8fc00af192';

const repoRoot = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();
const git = (...args: string[]) => execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();

function renderedDate(route: string): Date | undefined {
  const match = readDist(route).match(TIME_RE);
  return match ? new Date(match[1]) : undefined;
}

test('at least 90% of docs pages render a Last updated date', () => {
  const pages = listDistHtml().filter(
    (file) =>
      file.endsWith('/index.html') &&
      !['retired/', '_llms-txt/', 'pagefind/'].some((prefix) => file.startsWith(prefix)),
  );
  const undated = pages.filter((file) => !TIME_RE.test(readDist(file.slice(0, -'/index.html'.length))));
  const coverage = (pages.length - undated.length) / pages.length;
  expect(pages.length).toBeGreaterThan(300);
  expect(coverage, `undated pages (first 20): ${undated.slice(0, 20).join(', ')}`).toBeGreaterThanOrEqual(0.9);
});

for (const route of ['tutorials/tut-getting-started', 'reference/installation']) {
  test(`${route} shows the git-date map date`, () => {
    const expected = getGitDates().get(`docs/src/${route}.md`);
    expect(expected, `no git date for docs/src/${route}.md`).toBeTruthy();
    expect(renderedDate(route)?.toISOString()).toBe(new Date(expected!).toISOString());
  });
}

test('a page last touched by an ignored bulk commit shows an earlier date', () => {
  const ignored = readIgnoreRevs(repoRoot);
  const touched = git('show', '--name-only', '--format=', STRIP_COMMIT, '--', 'docs/src').split('\n');

  const candidate = touched.find((file) => {
    if (!/\.mdx?$/.test(file) || git('log', '-1', '--format=%H', '--', file) !== STRIP_COMMIT) return false;
    const route = file.replace(/^docs\/src\//, '').replace(/\.mdx?$/, '').toLowerCase();
    if (!existsSync(join(DIST_DIR, route, 'index.html'))) return false;
    return git('log', '--format=%H', '--', file).split('\n').some((sha) => !ignored.has(sha));
  });
  test.skip(!candidate, `no built page has ${STRIP_COMMIT.slice(0, 9)} as its newest commit`);

  const route = candidate!.replace(/^docs\/src\//, '').replace(/\.mdx?$/, '').toLowerCase();
  const commitDate = new Date(git('log', '-1', '--format=%cI', STRIP_COMMIT));
  const shown = renderedDate(route);
  test.info().annotations.push({ type: 'page', description: `${candidate} -> /${route}/` });
  expect(shown, `${route} has no <time datetime>`).toBeTruthy();
  expect(shown!.getTime()).toBeLessThan(commitDate.getTime());
});
