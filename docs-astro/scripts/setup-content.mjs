// Links docs/src/ into the Starlight content collection directory.
// docsLoader() hardcodes src/content/docs/ as its base; this makes that path
// point at the actual source without moving files.
import { existsSync, mkdirSync, symlinkSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const target = join(repoRoot, 'docs', 'src');
const link = join(repoRoot, 'docs-astro', 'src', 'content', 'docs');

if (!existsSync(link)) {
  mkdirSync(join(repoRoot, 'docs-astro', 'src', 'content'), { recursive: true });
  const type = process.platform === 'win32' ? 'junction' : 'dir';
  symlinkSync(target, link, type);
  console.log(`[setup-content] Created ${type}: docs-astro/src/content/docs → docs/src`);
} else {
  console.log('[setup-content] docs-astro/src/content/docs already exists, skipping.');
}
