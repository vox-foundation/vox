/**
 * Pre-render every ```vox-diagram fence under docs/src, and in every repo file the site mounts
 * under repo/ (passed as `--mount <path>`), into docs-astro/.vox-diagram/manifest.json
 * (VoxDiagram Phase 89, D-01/D-20).
 *
 * cargo must run with cwd = repo root: the pipeline reads `docs/src` relative to it, and
 * `pnpm --dir docs-astro ...` starts in docs-astro/. `--dev` keeps `pnpm dev` alive after a
 * failure (the remark plugin shows a placeholder for the missing diagram); build fails hard.
 */
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { discoverMounts, loadMountContract } from '../src/utils/repo-mounts.mjs';

export const REPO_ROOT = fileURLToPath(new URL('../../', import.meta.url));
export const RENDER_COMMAND = ['cargo', 'run', '-p', 'vox-doc-pipeline', '--', '--render-diagrams'];

/**
 * `--mount <path>` for every repo file setup-content mounts under repo/; the remark plugin renders
 * those pages too, so their fences need manifest entries. Discovery stays in repo-mounts.mjs.
 */
export function mountArgs(repoRoot = REPO_ROOT) {
  const contract = loadMountContract(join(repoRoot, 'contracts/documentation/site-mounted-repo-docs.v1.json'));
  const mounts = discoverMounts({ repoRoot, docsSrc: join(repoRoot, 'docs', 'src'), contract });
  return mounts.flatMap((m) => ['--mount', m.repoPath]);
}

/** `spawn` is replaceable for tests. */
export function renderDiagrams({ dev = false, spawn = spawnSync } = {}) {
  const [cmd, ...cmdArgs] = RENDER_COMMAND;
  const res = spawn(cmd, [...cmdArgs, ...mountArgs()], { cwd: REPO_ROOT, stdio: 'inherit' });
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
