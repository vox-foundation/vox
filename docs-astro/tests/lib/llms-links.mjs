/**
 * Helper to extract and check URLs in llms.txt files.
 */

/**
 * Extracts internal URLs belonging to `origin` from llms text content (Markdown links or bare URLs),
 * dedupes them, and optionally rewrites the origin to `baseUrl`.
 *
 * @param {string} text
 * @param {{ origin?: string, baseUrl?: string }} [options]
 * @returns {string[]}
 */
export function extractLlmsUrls(text, options = {}) {
  const origin = (options.origin ?? 'https://voxlang.org').replace(/\/+$/, '');
  const baseUrl = options.baseUrl ? options.baseUrl.replace(/\/+$/, '') : null;

  if (typeof text !== 'string' || !text.trim()) return [];

  const foundUrls = new Set();

  // 1. Markdown links: [text](url)
  const mdLinkRegex = /\[(?:[^\]]*)\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g;
  let match;
  while ((match = mdLinkRegex.exec(text)) !== null) {
    foundUrls.add(match[1].trim());
  }

  // 2. Bare URLs: https?://...
  const bareUrlRegex = /https?:\/\/[^\s)\]>"']+/g;
  while ((match = bareUrlRegex.exec(text)) !== null) {
    foundUrls.add(match[0].trim());
  }

  const results = [];
  const originPattern = new RegExp(`^${origin.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(\\/.*)?$`);

  for (const rawUrl of foundUrls) {
    const cleanUrl = rawUrl.split('#')[0].split('?')[0];
    if (!cleanUrl) continue;

    if (originPattern.test(cleanUrl)) {
      if (baseUrl) {
        const rewritten = cleanUrl.replace(origin, baseUrl);
        results.push(rewritten);
      } else {
        results.push(cleanUrl);
      }
    }
  }

  return Array.from(new Set(results));
}

/**
 * Checks a list of URLs with a given fetcher function under concurrency limit.
 *
 * @param {string[]} urls
 * @param {(url: string) => Promise<{ status: number }>} fetcher
 * @param {number} [concurrency=8]
 * @returns {Promise<{ ok: boolean, failures: Array<{ url: string, status?: number, error?: string }> }>}
 */
export async function checkUrls(urls, fetcher, concurrency = 8) {
  const failures = [];
  const queue = [...urls];

  async function worker() {
    while (queue.length > 0) {
      const url = queue.shift();
      if (!url) break;
      try {
        const resp = await fetcher(url);
        const status = resp?.status ?? 0;
        if (status < 200 || status >= 300) {
          failures.push({ url, status });
        }
      } catch (err) {
        failures.push({ url, error: err.message || String(err) });
      }
    }
  }

  const workers = Array.from({ length: Math.min(concurrency, Math.max(1, urls.length)) }, () => worker());
  await Promise.all(workers);

  return {
    ok: failures.length === 0,
    failures,
  };
}
