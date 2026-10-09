import { test, expect } from '@playwright/test';
import { existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { DIST_DIR, readDist } from '../lib/dist';

/** Routes (`<route>` in `/repo/<route>/`) of every built mounted repo page. */
function mountedRoutes(): string[] {
  const dir = join(DIST_DIR, 'repo');
  if (!existsSync(dir)) return [];
  return readdirSync(dir).filter((name) => existsSync(join(dir, name, 'index.html')));
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
