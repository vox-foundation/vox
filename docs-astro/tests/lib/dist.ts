import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { join, resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = resolve(__filename, '..');
export const DIST_DIR = resolve(__dirname, '../../dist');

/**
 * Reads `dist/<route>/index.html`.
 * `route` should be provided without leading or trailing slashes.
 */
export function readDist(route: string): string {
  if (!existsSync(DIST_DIR)) {
    throw new Error(`dist directory does not exist at ${DIST_DIR}. Run 'pnpm build' first.`);
  }

  const cleanRoute = route.replace(/^\/+|\/+$/g, '');
  const filePath = cleanRoute ? join(DIST_DIR, cleanRoute, 'index.html') : join(DIST_DIR, 'index.html');

  if (!existsSync(filePath)) {
    throw new Error(`File not found: ${filePath}`);
  }

  return readFileSync(filePath, 'utf8');
}

/**
 * Recursively lists all `.html` files in `dist/`, relative to `dist/`.
 */
export function listDistHtml(): string[] {
  if (!existsSync(DIST_DIR)) {
    throw new Error(`dist directory does not exist at ${DIST_DIR}. Run 'pnpm build' first.`);
  }

  const results: string[] = [];

  function walk(currentDir: string) {
    const entries = readdirSync(currentDir);
    for (const entry of entries) {
      const fullPath = join(currentDir, entry);
      const stat = statSync(fullPath);
      if (stat.isDirectory()) {
        walk(fullPath);
      } else if (entry.endsWith('.html')) {
        results.push(relative(DIST_DIR, fullPath).replace(/\\/g, '/'));
      }
    }
  }

  walk(DIST_DIR);
  return results;
}
