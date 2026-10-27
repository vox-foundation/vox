/**
 * Astro integration around remark-vox-diagram: registers the plugin with the manifest digest and
 * fails a build that would ship a diagram fence without its pre-rendered SVG (VoxDiagram Phase 89).
 */

import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { remarkVoxDiagram, assertNoMisses, resetMisses } from './remark-vox-diagram.mjs';

/** `manifestFile` is a file URL; it need not exist (every fence is then a miss). */
export function voxDiagram({ manifestFile }) {
  // The digest rides in the plugin options, so Astro's content data store is invalidated whenever a
  // diagram changes (D-15).
  const digest = existsSync(manifestFile)
    ? createHash('sha256').update(readFileSync(manifestFile)).digest('hex')
    : 'none';
  let pending = null;
  return {
    name: 'vox-diagram',
    hooks: {
      'astro:config:setup': ({ command, config, updateConfig, addWatchFile }) => {
        resetMisses();
        addWatchFile(manifestFile);
        if (command === 'build') {
          // An md render that throws is logged and cached in the build's content store with no
          // body, and the glob loader skips a file whose content and config digest did not change,
          // so a rebuild would pass without running this plugin and ship an empty page. A build
          // that did not finish cleanly (a miss, any other error, a kill) leaves this marker; the
          // next build then drops the store so every page renders, and is checked, again.
          pending = new URL('vox-diagram-build-pending', config.cacheDir);
          if (existsSync(pending)) rmSync(new URL('data-store.json', config.cacheDir), { force: true });
          mkdirSync(config.cacheDir, { recursive: true });
          writeFileSync(pending, '');
        }
        updateConfig({
          markdown: {
            remarkPlugins: [
              [remarkVoxDiagram, { manifestPath: fileURLToPath(manifestFile), strict: command === 'build', digest }],
            ],
          },
        });
      },
      // A dev restart (the manifest is a watched file) does not re-sync content, so cached md pages
      // would keep the old figure or placeholder. The digest in the plugin options changes Astro's
      // config digest, and a sync clears the store when that differs (89-RESEARCH Pitfall 3 / A4).
      'astro:server:setup': ({ refreshContent }) => refreshContent?.({}),
      // md pages render during content sync before this hook, and a remark throw does not fail an
      // md build, so this is what fails it; the done hook catches any miss recorded later.
      'astro:build:start': () => assertNoMisses(),
      'astro:build:done': () => {
        assertNoMisses();
        if (pending) rmSync(pending, { force: true });
      },
    },
  };
}
