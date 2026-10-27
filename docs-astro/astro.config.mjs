// @ts-check
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLlmsTxt from 'starlight-llms-txt';
import sitemap from '@astrojs/sitemap';
import { voxGrammar } from './src/plugins/vox-grammar.mjs';
import { getSidebar } from './src/utils/sidebar.mjs';
import { remarkVoxInclude } from './src/plugins/remark-vox-include.mjs';
import { remarkDocLinks } from './src/plugins/remark-doc-links.mjs';
import { remarkVoxDiagram, assertNoMisses } from './src/plugins/remark-vox-diagram.mjs';
import { listDocPages, noindexRoutes, internalsDocIds } from './src/utils/page-index.mjs';

const docsSrc = new URL('../docs/src', import.meta.url).pathname;
const docPages = listDocPages(docsSrc);
const noindex = noindexRoutes(docPages);
const internalsExcluded = internalsDocIds(docPages);

// VoxDiagram pre-render output (gitignored; `pnpm diagrams` writes it). Its sha256 rides in the
// plugin options so Astro's content data store is invalidated whenever a diagram changes (D-15).
const manifestFile = new URL('./.vox-diagram/manifest.json', import.meta.url);
const manifestDigest = existsSync(manifestFile)
  ? createHash('sha256').update(readFileSync(manifestFile)).digest('hex')
  : 'none';

export default defineConfig({
  site: 'https://voxlang.org/',
  // Process {{#include path:anchor}} directives in code blocks (mdBook SSOT pattern).
  // Build fails loudly for any unresolved path/anchor — preventing silent blank code blocks.
  markdown: {
    remarkPlugins: [
      remarkVoxInclude,
      [remarkDocLinks, { repoUrl: 'https://github.com/vox-foundation/vox' }],
    ],
  },
  integrations: [
    {
      name: 'vox-diagram',
      hooks: {
        'astro:config:setup': ({ command, updateConfig, addWatchFile }) => {
          addWatchFile(manifestFile);
          updateConfig({
            markdown: {
              remarkPlugins: [
                [
                  remarkVoxDiagram,
                  { manifestPath: fileURLToPath(manifestFile), strict: command === 'build', digest: manifestDigest },
                ],
              ],
            },
          });
        },
        // A dev restart (the manifest is a watched file) does not re-sync content, so cached md pages
        // would keep the old figure or placeholder. The digest in the plugin options changes Astro's
        // config digest, and a sync clears the store when that differs (89-RESEARCH Pitfall 3 / A4).
        'astro:server:setup': ({ refreshContent }) => refreshContent?.({}),
        // md pages render during content sync before this hook, and a remark throw does not fail
        // an md build, so this is what fails it; the done hook catches any miss recorded later.
        'astro:build:start': () => assertNoMisses(),
        'astro:build:done': () => assertNoMisses(),
      },
    },
    sitemap({
      filter: (page) => {
        const p = new URL(page).pathname;
        const normalized = p.endsWith('/') ? p : `${p}/`;
        return !noindex.has(p) && !noindex.has(normalized) && p !== '/retired/' && p !== '/retired';
      },
    }),
    starlight({
      title: 'Vox: The AI-Native Programming Language',
      description: 'Pre-1.0 documentation for Vox, an AI-native full-stack programming language that compiles .vox to Rust and TypeScript.',
      routeMiddleware: './src/routeData.ts',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/vox-foundation/vox' }
      ],
      editLink: {
        baseUrl: 'https://github.com/vox-foundation/vox/edit/main/docs/src/',
      },
      // Sidebar is generated from each page's frontmatter (category /
      // sort_order / title) by src/utils/sidebar.mjs; section order comes
      // from contracts/documentation/docs-sidebar-section-order.v1.json.
      // SUMMARY.md is gitignored and only ever EXCLUDED (content.config.ts,
      // routeData.ts) -- it is never read as input.
      sidebar: getSidebar(),
      expressiveCode: {
        shiki: {
          langs: [voxGrammar],
          // Fences are replaced by the remark plugin; the alias keeps stray ones highlight-safe.
          langAlias: { 'vox-diagram': 'text' },
        },
      },
      plugins: [
        starlightLlmsTxt({
          projectName: 'Vox',
          description: 'Vox is a pre-1.0 (0.6.0) AI-native full-stack language that compiles a .vox file to a database schema, type-safe server, and browser UI. Build from source; see https://voxlang.org/reference/stability/ for maturity. Current syntax uses bare table / query / mutation / server / tool — not @endpoint.',
          exclude: internalsExcluded,
          // The SVG's <desc> and labels are noise for LLMs; its fence source stays (details > pre).
          customSelectors: { all: ['svg.vd-diagram'] },
        }),
      ],
      lastUpdated: true,
      pagefind: true,
      customCss: ['./src/styles/reader-layout.css', './src/styles/vox-diagram.css'],
      components: {
        Sidebar: './src/components/Sidebar.astro',
        TableOfContents: './src/components/TableOfContents.astro',
        PageSidebar: './src/components/PageSidebar.astro',
        Banner: './src/components/Banner.astro',
      },
      head: [
        {
          tag: 'script',
          content: `(function(){try{var s=localStorage.getItem('vox-sidebar-state');var t=localStorage.getItem('vox-toc-state');document.documentElement.dataset.sidebarState=(s==='collapsed'?'collapsed':'pinned');document.documentElement.dataset.tocState=(t==='hidden'?'hidden':'visible');}catch(e){}})();`,
        },
      ],
    }),
  ],
  vite: {
    resolve: {
      alias: {
        '@docs-src': new URL('../docs/src', import.meta.url).pathname,
      },
    },
  },
  // Content lives in docs/src/ (symlinked or copied in Phase 3).
  srcDir: './src',
  outDir: './dist',
  publicDir: './public',
});
