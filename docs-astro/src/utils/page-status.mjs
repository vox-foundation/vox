/**
 * Single source of truth for documentation page status policy.
 *
 * Status values from frontmatter determine:
 * - banner: whether a status banner is displayed and its HTML text
 * - noindex: whether search engine crawlers are told to ignore the page
 * - internals: whether the page belongs in the collapsed "Internals" sidebar section
 */

export const STATUS_POLICY = {
  research: {
    internals: true,
    noindex: true,
    banner:
      '<strong>Internals — research note:</strong> investigation or findings, not a description of shipped behaviour.',
  },
  roadmap: {
    internals: true,
    noindex: true,
    banner:
      '<strong>Internals — roadmap:</strong> describes planned work that may not match the current code.',
  },
  deprecated: {
    internals: false,
    noindex: true,
    banner: '<strong>Deprecated:</strong> kept for migration notes only; do not build on this.',
  },
  legacy: {
    internals: false,
    noindex: true,
    banner: '<strong>Legacy:</strong> still present, but not the recommended path.',
  },
};

const DEFAULT_POLICY = Object.freeze({
  internals: false,
  noindex: false,
  banner: null,
});

/**
 * Returns the policy object for a given frontmatter status string.
 * Status lookup is case-insensitive and trims surrounding whitespace.
 */
export function statusPolicy(status) {
  if (typeof status !== 'string') {
    return DEFAULT_POLICY;
  }
  const key = status.trim().toLowerCase();
  return STATUS_POLICY[key] || DEFAULT_POLICY;
}

/**
 * Returns true if the status belongs in the Internals section (research, roadmap).
 */
export function isInternalsStatus(status) {
  return Boolean(statusPolicy(status).internals);
}

/**
 * Returns the HTML banner content for a status, or null if no banner applies.
 */
export function bannerFor(status) {
  return statusPolicy(status).banner;
}
