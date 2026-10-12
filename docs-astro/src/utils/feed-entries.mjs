import { statusPolicy } from './page-status.mjs';

/**
 * The docs the RSS feed lists, newest Git date first, at most `limit`. Pages
 * whose status is noindex (research, roadmap, deprecated, legacy) are left out,
 * the same set the sitemap drops, so the feed never advertises Internals pages
 * as documentation updates. Docs with no Git date are skipped.
 *
 * @template {{ data: { status?: unknown } }} D
 * @param {readonly D[]} docs
 * @param {(doc: D) => string | undefined} dateFor
 * @param {number} [limit]
 * @returns {{ doc: D; date: string }[]}
 */
export function feedEntries(docs, dateFor, limit = 30) {
  return docs
    .filter((doc) => !statusPolicy(doc.data.status).noindex)
    .map((doc) => ({ doc, date: dateFor(doc) }))
    .filter((entry) => Boolean(entry.date))
    .sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())
    .slice(0, limit);
}
