import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { getGitDates } from '../../src/utils/git-dates.mjs';
import { discoverMounts } from '../../src/utils/repo-mounts.mjs';
import { DIST_DIR, readDist } from '../lib/dist';

const EDIT_RE = /href="https:\/\/github\.com\/vox-foundation\/vox\/edit\/main\/([^"#?]+)"/;
const TIME_RE = /<time datetime="([^"]+)"/;
const repoRoot = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();

/** Routes (`<route>` in `/repo/<route>/`) of every built mounted repo page. */
function mountedRoutes(): string[] {
  const dir = join(DIST_DIR, 'repo');
  if (!existsSync(dir)) return [];
  return readdirSync(dir).filter((name) => existsSync(join(dir, name, 'index.html')));
}

/** Repo path the page's "Edit page" link points at. */
function editSource(route: string): string | undefined {
  return readDist(route).match(EDIT_RE)?.[1];
}

test('root AGENTS.md renders at /repo/agents-md/', () => {
  expect(existsSync(join(DIST_DIR, 'repo', 'agents-md', 'index.html'))).toBe(true);
  expect(readDist('repo/agents-md')).toContain('Agents Policy');
});

test('the hand-written /agents/ page is still served', () => {
  expect(existsSync(join(DIST_DIR, 'agents', 'index.html'))).toBe(true);
});

test('no superpowers plan is mounted', () => {
  const routes = mountedRoutes();
  expect(routes.length).toBeGreaterThan(3);
  expect(routes.filter((route) => route.startsWith('docs-superpowers'))).toEqual([]);
});

test('/repo/agents-md/ edit link points at the root AGENTS.md', () => {
  expect(editSource('repo/agents-md')).toBe('AGENTS.md');
});

test('edit links of docs pages and every mounted page name an existing repo file', () => {
  const routes = ['tutorials/tut-getting-started', 'reference/cli', 'agents', ...mountedRoutes().map((r) => `repo/${r}`)];
  const missing = routes.filter((route) => {
    const source = editSource(route);
    return !source || !existsSync(join(repoRoot, decodeURI(source)));
  });
  expect(missing).toEqual([]);
  expect(editSource('tutorials/tut-getting-started')).toBe('docs/src/tutorials/tut-getting-started.md');
  expect(editSource('agents')).toBe('docs/src/AGENTS.md');
});

test('/repo/agents-md/ is dated from the root AGENTS.md history', () => {
  const mounted = discoverMounts({ repoRoot }).map((mount) => mount.repoPath);
  const expected = getGitDates(repoRoot, { paths: mounted }).get('AGENTS.md');
  expect(expected, 'no git date for AGENTS.md').toBeTruthy();
  const shown = readDist('repo/agents-md').match(TIME_RE)?.[1];
  expect(shown, '/repo/agents-md/ has no <time datetime>').toBeTruthy();
  expect(new Date(shown!).toISOString()).toBe(new Date(expected!).toISOString());
});

test('a docs link to ../../../AGENTS.md is rewritten to /repo/agents-md/', () => {
  expect(readDist('ci/runner-contract')).toContain('href="/repo/agents-md/"');
});
