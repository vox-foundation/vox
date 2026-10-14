// Links docs/src/ into the Starlight content collection directory as a mirror.
// docsLoader() hardcodes src/content/docs/ as its base; this creates a mirror
// directory with per-entry symlinks to docs/src (excluding archive/, .well-known/, SUMMARY.md).
import {
  copyFileSync,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { join, dirname } from 'node:path';
import { discoverMounts, loadMountContract } from '../src/utils/repo-mounts.mjs';

export const MIRROR_MARKER = '.vox-docs-mirror';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const target = join(repoRoot, 'docs', 'src');
const link = join(repoRoot, 'docs-astro', 'src', 'content', 'docs');
const type = process.platform === 'win32' ? 'junction' : 'dir';

export function prepareMirrorDir(mirrorPath, sourceDir = null, customRepoRoot = null) {
  if (existsSync(mirrorPath)) {
    const stat = lstatSync(mirrorPath);
    if (stat.isSymbolicLink()) {
      unlinkSync(mirrorPath);
    } else if (stat.isDirectory()) {
      if (!existsSync(join(mirrorPath, MIRROR_MARKER))) {
        throw new Error(
          `[setup-content] ${mirrorPath} exists but is not a mirror (missing ${MIRROR_MARKER}). Refusing to remove.`
        );
      }
      rmSync(mirrorPath, { recursive: true, force: true });
    }
  }

  mkdirSync(mirrorPath, { recursive: true });
  writeFileSync(join(mirrorPath, MIRROR_MARKER), '');

  if (sourceDir && existsSync(sourceDir)) {
    if (existsSync(join(sourceDir, 'repo'))) {
      throw new Error(
        `[setup-content] ${sourceDir} contains a top-level 'repo' directory. Refusing to shadow real docs dir.`
      );
    }

    const EXCLUDED = new Set(['archive', '.well-known', 'SUMMARY.md', MIRROR_MARKER]);
    const entries = readdirSync(sourceDir);
    let mirroredCount = 0;
    const excludedList = [];

    for (const entry of entries) {
      if (EXCLUDED.has(entry)) {
        excludedList.push(entry);
        continue;
      }

      const srcEntry = join(sourceDir, entry);
      const destEntry = join(mirrorPath, entry);
      const entryStat = lstatSync(srcEntry);
      const linkType = process.platform === 'win32'
        ? (entryStat.isDirectory() ? 'junction' : 'file')
        : (entryStat.isDirectory() ? 'dir' : 'file');

      try {
        symlinkSync(srcEntry, destEntry, linkType);
        mirroredCount++;
      } catch (err) {
        if (err.code === 'EPERM' || err.code === 'EACCES') {
          console.warn(`[setup-content] symlinkSync failed for ${entry} (${err.code}), falling back to copy`);
          cpSync(srcEntry, destEntry, { recursive: true });
          mirroredCount++;
        } else {
          throw err;
        }
      }
    }

    console.log(
      `[setup-content] Mirrored ${mirroredCount} entries into ${mirrorPath} (excluded: ${excludedList.join(', ')})`
    );

    // Mount out-of-tree repo markdown files under repo/
    const repoMirrorDir = join(mirrorPath, 'repo');
    mkdirSync(repoMirrorDir, { recursive: true });

    const activeRepoRoot = customRepoRoot || repoRoot;
    const contract = loadMountContract();
    const mounts = discoverMounts({ repoRoot: activeRepoRoot, docsSrc: sourceDir, contract });
    let mountedCount = 0;
    let symlinkedMounts = 0;
    let wrappedMounts = 0;

    for (const mount of mounts) {
      const srcFile = join(activeRepoRoot, mount.repoPath);
      const destFile = join(repoMirrorDir, `${mount.route}.md`);

      if (mount.hasTitle) {
        try {
          symlinkSync(srcFile, destFile, 'file');
          symlinkedMounts++;
        } catch (err) {
          if (err.code === 'EPERM' || err.code === 'EACCES') {
            console.warn(`[setup-content] symlinkSync failed for mounted file ${mount.repoPath} (${err.code}), falling back to copy`);
            copyFileSync(srcFile, destFile);
            symlinkedMounts++;
          } else {
            throw err;
          }
        }
      } else {
        const raw = readFileSync(srcFile, 'utf8');
        let title = contract.titles?.[mount.repoPath];
        const lines = raw.split('\n');
        let headingText = null;
        const remainingLines = [];
        let stripped = false;

        for (const line of lines) {
          if (!stripped && /^#\s+(.+)$/.test(line)) {
            const m = line.match(/^#\s+(.+)$/);
            headingText = m[1].trim();
            stripped = true;
            continue;
          }
          remainingLines.push(line);
        }

        if (!title) {
          title = headingText || mount.repoPath;
        }

        const frontmatter = [
          '---',
          `title: ${JSON.stringify(title)}`,
          `mounted_from: ${JSON.stringify(mount.repoPath)}`,
          'status: "current"',
          '---',
          '',
          '',
        ].join('\n');

        const wrapperContent = frontmatter + remainingLines.join('\n');
        writeFileSync(destFile, wrapperContent, 'utf8');
        wrappedMounts++;
      }
      mountedCount++;
    }

    console.log(
      `[setup-content] Mounted ${mountedCount} repo docs in ${repoMirrorDir} (${symlinkedMounts} symlinked, ${wrappedMounts} wrapped)`
    );
  }
}

function runSetup() {
  prepareMirrorDir(link, target, repoRoot);

  // Second link: expose repo-root examples/ so remark-vox-include can resolve
  // {{#include ../../../examples/golden/X.vox}} from any docs section subdirectory.
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

  // Copy static image assets to public/assets so /assets/* links in docs resolve.
  const assetsSrc = join(repoRoot, 'docs', 'src', 'assets');
  const assetsDest = join(repoRoot, 'docs-astro', 'public', 'assets');
  if (existsSync(assetsSrc)) {
    cpSync(assetsSrc, assetsDest, { recursive: true });
    console.log('[setup-content] Copied docs/src/assets → docs-astro/public/assets');
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  runSetup();
}
