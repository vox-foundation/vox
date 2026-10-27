/**
 * Pre-render every ```vox-diagram fence under docs/src into
 * docs-astro/.vox-diagram/manifest.json (VoxDiagram Phase 89, D-01/D-20).
 *
 * cargo must run with cwd = repo root: the pipeline reads `docs/src` relative to it, and
 * `pnpm --dir docs-astro ...` starts in docs-astro/. `--dev` keeps `pnpm dev` alive after a
 * failure (the remark plugin shows a placeholder for the missing diagram); build fails hard.
 */
import { spawnSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const REPO_ROOT = fileURLToPath(new URL('../../', import.meta.url));
export const RENDER_COMMAND = ['cargo', 'run', '-p', 'vox-doc-pipeline', '--', '--render-diagrams'];

const STORE = fileURLToPath(new URL('../.astro/data-store.json', import.meta.url));

/**
 * Astro's content data store caches rendered markdown, and a page whose source did not change is
 * not re-rendered when the manifest does (89-RESEARCH Pitfall 3). Deleting the store forces a
 * re-sync. `astro.config.mjs` calls this at every (re)start, before content sync: calling it from
 * the CLI instead deletes the store under a running dev server, which then 404s every page.
 * Returns whether it deleted anything.
 */
export function clearStaleStore({ before, after, storePath = STORE }) {
  if (before === after || !existsSync(storePath)) return false;
  rmSync(storePath);
  return true;
}

export function renderDiagrams({ dev = false } = {}) {
  const [cmd, ...cmdArgs] = RENDER_COMMAND;
  const res = spawnSync(cmd, cmdArgs, { cwd: REPO_ROOT, stdio: 'inherit' });
  if (res.error || res.status !== 0) {
    const why = res.error ? `: ${res.error.message}` : '';
    console.error(
      `render-diagrams: pre-render failed${why}. Fix the fences above, then run \`${RENDER_COMMAND.join(' ')}\` from the repo root (${REPO_ROOT}) or \`pnpm diagrams\`.`,
    );
    return dev ? 0 : 1;
  }
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exit(renderDiagrams({ dev: process.argv.includes('--dev') }));
}
