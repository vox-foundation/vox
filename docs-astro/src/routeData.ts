/**
 * Starlight route middleware: suppress Pagefind indexing for archive content.
 *
 * Starlight renders `pagefind: false` frontmatter by adding `<meta name="robots"
 * content="noindex"> to the page <head>.  We replicate this for any route whose
 * slug starts with "archive/" so we never have to touch individual files.
 *
 * Ref: https://starlight.astro.build/reference/route-data/
 */
import { defineRouteMiddleware } from '@astrojs/starlight/route-data';

export const onRequest = defineRouteMiddleware((context) => {
  const slug = context.locals.starlightRoute?.id ?? '';

  // Suppress Pagefind for any archive/ path and the raw SUMMARY page
  if (slug.startsWith('archive/') || slug === 'summary') {
    // Inject a <meta name="robots" content="noindex"> into the page head.
    // Starlight's Pagefind integration respects this to skip the page.
    const { head } = context.locals.starlightRoute;
    if (head) {
      head.push({
        tag: 'meta',
        attrs: { name: 'robots', content: 'noindex' },
      });
    }
  }
});
