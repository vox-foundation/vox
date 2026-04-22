// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { voxGrammar } from './src/plugins/vox-grammar.mjs';
import { getSidebar } from './src/utils/sidebar.mjs';

export default defineConfig({
  integrations: [
    starlight({
      title: 'Vox: The AI-Native Programming Language',
      description: 'Official documentation for Vox, the AI-native full-stack programming language.',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/vox-foundation/vox' }
      ],
      editLink: {
        baseUrl: 'https://github.com/vox-foundation/vox/edit/main/docs/src/',
      },
      // Sidebar is dynamically generated from SUMMARY.md to maintain SSOT
      sidebar: getSidebar(),
      expressiveCode: {
        shiki: {
          langs: [voxGrammar],
        },
      },
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
