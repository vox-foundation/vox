// Links docs/src/ into the Starlight content collection directory.
// docsLoader() hardcodes src/content/docs/ as its base; this makes that path
// point at the actual source without moving files.
import { existsSync, lstatSync, mkdirSync, readlinkSync, rmSync, symlinkSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join, dirname } from 'node:path';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const target = join(repoRoot, 'docs', 'src');
const link = join(repoRoot, 'docs-astro', 'src', 'content', 'docs');
const type = process.platform === 'win32' ? 'junction' : 'dir';

function createLink() {
  mkdirSync(join(repoRoot, 'docs-astro', 'src', 'content'), { recursive: true });
  symlinkSync(target, link, type);
  console.log(`[setup-content] Created ${type}: docs-astro/src/content/docs → docs/src`);
}

if (!existsSync(link)) {
  createLink();
} else {
  // Validate the existing path is a symlink/junction pointing at the correct target.
  const stat = lstatSync(link);
  if (!stat.isSymbolicLink()) {
    throw new Error(
      `[setup-content] docs-astro/src/content/docs exists but is not a symlink (is a ${stat.isDirectory() ? 'directory' : 'file'}). Remove it manually and re-run.`
    );
  }
  const actual = readlinkSync(link);
  if (actual !== target) {
    console.log(`[setup-content] Stale link (${actual} → ${target}), recreating...`);
    rmSync(link);
    createLink();
  } else {
    console.log('[setup-content] docs-astro/src/content/docs already points to docs/src, skipping.');
  }
}
