// Links docs/src/ into the Starlight content collection directory.
// docsLoader() hardcodes src/content/docs/ as its base; this makes that path
// point at the actual source without moving files.
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readlinkSync,
  rmSync,
  symlinkSync,
} from 'node:fs';
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

// Second link: expose repo-root examples/ so remark-vox-include can resolve
// {{#include ../../../examples/golden/X.vox}} from any docs section subdirectory.
// From docs-astro/src/content/docs/<section>/, going up 3 levels reaches
// docs-astro/src/, so the plugin looks for docs-astro/src/examples/golden/X.vox.
const examplesTarget = join(repoRoot, 'examples');
const examplesLink = join(repoRoot, 'docs-astro', 'src', 'examples');

function createExamplesLink() {
  mkdirSync(join(repoRoot, 'docs-astro', 'src'), { recursive: true });
  symlinkSync(examplesTarget, examplesLink, type);
  console.log(`[setup-content] Created ${type}: docs-astro/src/examples → examples`);
}

if (!existsSync(examplesLink)) {
  createExamplesLink();
} else {
  const stat = lstatSync(examplesLink);
  if (!stat.isSymbolicLink()) {
    throw new Error(
      `[setup-content] docs-astro/src/examples exists but is not a symlink (is a ${stat.isDirectory() ? 'directory' : 'file'}). Remove it manually and re-run.`
    );
  }
  const actual = readlinkSync(examplesLink);
  if (actual !== examplesTarget) {
    console.log(`[setup-content] Stale examples link (${actual} → ${examplesTarget}), recreating...`);
    rmSync(examplesLink, { recursive: true });
    createExamplesLink();
  } else {
    console.log('[setup-content] docs-astro/src/examples already points to examples, skipping.');
  }
}

// Copy agent discovery files so CF Pages serves /.well-known/* (Starlight
// excludes that directory from the content collection).
const wellKnownSrc = join(repoRoot, 'docs', 'src', '.well-known');
const wellKnownDest = join(repoRoot, 'docs-astro', 'public', '.well-known');
if (existsSync(wellKnownSrc)) {
  mkdirSync(wellKnownDest, { recursive: true });
  for (const name of readdirSync(wellKnownSrc)) {
    copyFileSync(join(wellKnownSrc, name), join(wellKnownDest, name));
  }
  console.log('[setup-content] Copied docs/src/.well-known → docs-astro/public/.well-known');
}

// Origin robots.txt (Cloudflare may prepend managed signals). Keep the
// sitemap host on voxlang.org, not the retired vox.foundation domain.
const robotsSrc = join(repoRoot, 'docs', 'src', 'robots.txt');
const robotsDest = join(repoRoot, 'docs-astro', 'public', 'robots.txt');
if (existsSync(robotsSrc)) {
  copyFileSync(robotsSrc, robotsDest);
  console.log('[setup-content] Copied docs/src/robots.txt → docs-astro/public/robots.txt');
}
