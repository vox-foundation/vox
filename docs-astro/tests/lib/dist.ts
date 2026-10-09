import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Absolute path to the built site, `docs-astro/dist`. */
export const DIST_DIR = fileURLToPath(new URL('../../dist', import.meta.url));

function requireDist(): void {
  if (!existsSync(DIST_DIR)) {
    throw new Error(`${DIST_DIR} does not exist; run pnpm build first`);
  }
}

/** HTML of `dist/<route>/index.html`; `route` has no leading or trailing slash. */
export function readDist(route: string): string {
  requireDist();
  return readFileSync(join(DIST_DIR, route, 'index.html'), 'utf8');
}

/** Every `.html` file under dist, relative to it, with `/` separators. */
export function listDistHtml(): string[] {
  requireDist();
  return (readdirSync(DIST_DIR, { recursive: true }) as string[])
    .filter((file) => file.endsWith('.html'))
    .map((file) => relative(DIST_DIR, join(DIST_DIR, file)).split('\\').join('/'));
}
