/**
 * Pre-render every ```vox-diagram fence under docs/src into
 * docs-astro/.vox-diagram/manifest.json (VoxDiagram Phase 89, D-01/D-20).
 *
 * cargo must run with cwd = repo root: the pipeline reads `docs/src` relative to it, and
 * `pnpm --dir docs-astro ...` starts in docs-astro/. `--dev` keeps `pnpm dev` alive after a
 * failure (the remark plugin shows a placeholder for the missing diagram); build fails hard.
 */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, rmSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const REPO_ROOT = fileURLToPath(new URL('../../', import.meta.url));
export const RENDER_COMMAND = ['cargo', 'run', '-p', 'vox-doc-pipeline', '--', '--render-diagrams'];

const MANIFEST = fileURLToPath(new URL('../.vox-diagram/manifest.json', import.meta.url));
const STORE = fileURLToPath(new URL('../node_modules/.astro/data-store.json', import.meta.url));

const digest = () => (existsSync(MANIFEST) ? createHash('sha256').update(readFileSync(MANIFEST)).digest('hex') : 'none');

/**
 * Astro's content data store caches rendered markdown, and `astro dev` does not re-render a page
 * whose source did not change when the manifest does. Deleting the store after a manifest change
 * forces a re-sync (89-RESEARCH Pitfall 3). Returns whether it deleted anything.
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
  const before = digest();
  const code = renderDiagrams({ dev: process.argv.includes('--dev') });
  if (clearStaleStore({ before, after: digest() })) {
    console.log('render-diagrams: manifest changed, cleared the Astro content store');
  }
  process.exit(code);
}
