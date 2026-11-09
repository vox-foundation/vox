// @ts-check
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightGitHubAlerts from 'starlight-github-alerts';
import starlightLinksValidator from 'starlight-links-validator';
import starlightLlmsTxt from 'starlight-llms-txt';
import sitemap from '@astrojs/sitemap';
import { unified } from '@astrojs/markdown-remark';
import { voxGrammar } from './src/plugins/vox-grammar.mjs';
import { getSidebar } from './src/utils/sidebar.mjs';
import { remarkVoxInclude } from './src/plugins/remark-vox-include.mjs';
import { remarkDocLinks } from './src/plugins/remark-doc-links.mjs';
import { remarkStripTitleH1 } from './src/plugins/remark-strip-title-h1.mjs';
import { voxDiagram } from './src/plugins/vox-diagram-integration.mjs';
import { linksValidatorExclude } from './src/utils/links-validator-exclude.mjs';
import { listDocPages, noindexRoutes, internalsDocIds } from './src/utils/page-index.mjs';

// Version SSOT is root Cargo.toml [workspace.package] (AGENTS.md §Versioning Policy).
const voxVersion =
  readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf8').match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? 'unknown';
const docsSrc = new URL('../docs/src', import.meta.url).pathname;
const docPages = listDocPages(docsSrc);
const noindex = noindexRoutes(docPages);
const internalsExcluded = internalsDocIds(docPages);

// VoxDiagram pre-render output (gitignored; `pnpm diagrams` writes it).
const manifestFile = new URL('./.vox-diagram/manifest.json', import.meta.url);

export default defineConfig({
  site: 'https://voxlang.org/',
  // Process {{#include path:anchor}} directives in code blocks (mdBook SSOT pattern).
  // Build fails loudly for any unresolved path/anchor — preventing silent blank code blocks.
  markdown: {
    processor: unified({
      remarkPlugins: [
        remarkStripTitleH1,
        remarkVoxInclude,
        [remarkDocLinks, { repoUrl: 'https://github.com/vox-foundation/vox' }],
      ],
    }),
  },
  integrations: [
    voxDiagram({ manifestFile }),
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
        starlightGitHubAlerts(),
        // Defaults on purpose: failOnError stays true so broken internal links fail the build.
        starlightLinksValidator({ exclude: linksValidatorExclude(docPages) }),
        starlightLlmsTxt({
          projectName: 'Vox',
          description: `Vox (vox-foundation/vox, voxlang.org; unrelated to vox-lang.dev or MrSmith33/vox) is a pre-1.0 (${voxVersion}) AI-native full-stack language that compiles a .vox file to a database schema, type-safe server, and browser UI. Build from source; see https://voxlang.org/reference/stability/ for maturity. Current syntax uses bare table / query / mutation / server / tool — not @endpoint.`,
          exclude: internalsExcluded,
          // The SVG's <desc> and labels are noise for LLMs; its fence source stays (details > pre).
          // Backstop only: 89-03 found llms-full.txt unchanged without it; kept so a later change
          // (here or in starlight-llms-txt) cannot leak the svg text into it.
          customSelectors: { all: ['svg.vd-diagram'] },
        }),
      ],
      lastUpdated: true,
      pagefind: true,
      customCss: ['./src/styles/reader-layout.css', './src/styles/vox-diagram.css'],
      components: {
        ThemeProvider: './src/components/ThemeProvider.astro',
        SiteTitle: './src/components/SiteTitle.astro',
        Sidebar: './src/components/Sidebar.astro',
        TableOfContents: './src/components/TableOfContents.astro',
        PageSidebar: './src/components/PageSidebar.astro',
        Banner: './src/components/Banner.astro',
      },
      head: [
        {
          tag: 'script',
          content: `(function(){try{var s=localStorage.getItem('vox-sidebar-state');var t=localStorage.getItem('vox-toc-state');document.documentElement.dataset.sidebarState=(s==='hidden'||s==='collapsed'?'hidden':'pinned');document.documentElement.dataset.tocState=(t==='hidden'?'hidden':'visible');}catch(e){}})();`,
        },
      ],
    }),
  ],
  vite: {
    resolve: {
      alias: [
        {
          find: '@docs-src',
          replacement: new URL('../docs/src', import.meta.url).pathname,
        },
        // docs/src .mdx files live outside docs-astro, so a bare import cannot find docs-astro/node_modules.
        {
          find: /^@astrojs\/starlight\/components$/,
          replacement: fileURLToPath(import.meta.resolve('@astrojs/starlight/components')),
        },
      ],
    },
  },
  // Content lives in docs/src/ (symlinked or copied in Phase 3).
  srcDir: './src',
  outDir: './dist',
  publicDir: './public',
});
