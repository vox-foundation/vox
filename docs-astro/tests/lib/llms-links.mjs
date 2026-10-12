// Link extraction and resolution for the post-deploy llms smoke checks.

const MARKDOWN_TARGET = /\]\(\s*<?(https?:\/\/[^\s)>]+)>?(?:\s+"[^"]*")?\s*\)/g;
const BARE_URL = /https?:\/\/[^\s<>"'`()[\]{}]+/g;
const TRAILING_PUNCTUATION = /[.,;:!?*_]+$/;

/**
 * Same-origin URLs listed in an llms file: Markdown link targets and bare URLs,
 * fragment-stripped, deduped and sorted. External URLs are out of scope. With
 * `baseUrl`, `origin` is rewritten so a local preview can be checked.
 *
 * @param {string} text
 * @param {{ origin?: string, baseUrl?: string }} [options]
 * @returns {string[]}
 */
export function extractLlmsUrls(text, { origin = 'https://voxlang.org', baseUrl } = {}) {
  const wanted = new URL(origin).origin;
  const candidates = [
    ...Array.from(text.matchAll(MARKDOWN_TARGET), (match) => match[1]),
    ...Array.from(text.matchAll(BARE_URL), (match) => match[0].replace(TRAILING_PUNCTUATION, '')),
  ];
  const urls = new Set();
  for (const candidate of candidates) {
    let url;
    try {
      url = new URL(candidate);
    } catch {
      continue;
    }
    if (url.origin !== wanted) continue;
    url.hash = '';
    const path = url.pathname + url.search;
    urls.add(baseUrl ? baseUrl.replace(/\/+$/, '') + path : url.origin + path);
  }
  return [...urls].sort();
}

/**
 * Resolve every URL with `fetcher(url) -> { status }` (which must follow
 * redirects), at most 8 at a time. Non-2xx answers and thrown errors are failures.
 *
 * @param {string[]} urls
 * @param {(url: string) => Promise<{ status: number }>} fetcher
 * @param {{ concurrency?: number }} [options]
 * @returns {Promise<{ ok: boolean, failures: { url: string, status?: number, error?: string }[] }>}
 */
export async function checkUrls(urls, fetcher, { concurrency = 8 } = {}) {
  const failures = [];
  let next = 0;
  const worker = async () => {
    while (next < urls.length) {
      const url = urls[next++];
      try {
        const { status } = await fetcher(url);
        if (status < 200 || status > 299) failures.push({ url, status });
      } catch (error) {
        failures.push({ url, error: String(error?.message ?? error) });
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(concurrency, urls.length) }, worker));
  failures.sort((a, b) => urls.indexOf(a.url) - urls.indexOf(b.url));
  return { ok: failures.length === 0, failures };
}
