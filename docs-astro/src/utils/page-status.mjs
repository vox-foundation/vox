/**
 * Single source for what a page's frontmatter `status` means on the published
 * site: whether it gets a banner, a robots `noindex`, and a place in the
 * Internals sidebar group (which also keeps it out of the sitemap and
 * llms.txt). Vocabulary: docs/src/contributors/documentation-governance.md.
 *
 * Banner text is a constant keyed by status; the frontmatter value is only a
 * lookup key and is never interpolated into the HTML Starlight renders.
 */

const NONE = Object.freeze({ internals: false, noindex: false, banner: null });

export const STATUS_POLICY = Object.freeze({
  research: Object.freeze({
    internals: true,
    noindex: true,
    banner:
      '<strong>Internals — research note:</strong> investigation or findings, not a description of shipped behaviour.',
  }),
  roadmap: Object.freeze({
    internals: true,
    noindex: true,
    banner:
      '<strong>Internals — roadmap:</strong> describes planned work that may not match the current code.',
  }),
  deprecated: Object.freeze({
    internals: false,
    noindex: true,
    banner: '<strong>Deprecated:</strong> kept for migration notes only; do not build on this.',
  }),
  legacy: Object.freeze({
    internals: false,
    noindex: true,
    banner: '<strong>Legacy:</strong> still present, but not the recommended path.',
  }),
});

/** Policy for a frontmatter `status` value; unknown or missing values get no treatment. */
export function statusPolicy(status) {
  const key = typeof status === 'string' ? status.trim().toLowerCase() : '';
  return Object.hasOwn(STATUS_POLICY, key) ? STATUS_POLICY[key] : NONE;
}

export function isInternalsStatus(status) {
  return statusPolicy(status).internals;
}

export function bannerFor(status) {
  return statusPolicy(status).banner;
}
