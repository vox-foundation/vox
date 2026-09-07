// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLlmsTxt from 'starlight-llms-txt';
import { voxGrammar } from './src/plugins/vox-grammar.mjs';
import { getSidebar } from './src/utils/sidebar.mjs';
import { remarkVoxInclude } from './src/plugins/remark-vox-include.mjs';

export default defineConfig({
  site: 'https://voxlang.org/',
  // Process {{#include path:anchor}} directives in code blocks (mdBook SSOT pattern).
  // Build fails loudly for any unresolved path/anchor — preventing silent blank code blocks.
  markdown: {
    remarkPlugins: [remarkVoxInclude],
  },
  integrations: [
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
        },
      },
      plugins: [
        starlightLlmsTxt({
          projectName: 'Vox',
          description: 'Vox is a pre-1.0 (0.6.0) AI-native full-stack language that compiles a .vox file to a database schema, type-safe server, and browser UI. Build from source; see https://voxlang.org/reference/stability/ for maturity. Current syntax uses bare table / query / mutation / server / tool — not @endpoint.',
          llmsFullTxt: true,
        }),
      ],
      lastUpdated: true,
      pagefind: true,
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
