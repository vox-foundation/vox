import { slug } from 'github-slugger';

/**
 * The id Starlight's docs loader assigns to `docs/src/<relPath>`: Astro's glob
 * loader github-slugs each path segment and drops a trailing `/index`
 * (astro/dist/content/utils.js `getContentEntryIdAndSlug`). The home page maps
 * to `""`. Example: `architecture/qwen-3.7-x.md` -> `architecture/qwen-37-x`.
 */
export function docSlug(relPath) {
  const id = relPath
    .replace(/\\/g, '/')
    .replace(/\.mdx?$/, '')
    .split('/')
    .map((segment) => slug(segment))
    .join('/')
    .replace(/\/index$/, '')
    .toLowerCase();
  return id === 'index' ? '' : id;
}
