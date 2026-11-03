import GithubSlugger, { slug as defaultSlug } from 'github-slugger';

/**
 * Derives Starlight/Astro document slug from a repository-relative path under docs/src.
 *
 * Algorithm mirrors Astro's getContentEntryIdAndSlug:
 * 1. Strip file extension (.md or .mdx)
 * 2. Split into path segments
 * 3. Run each segment through github-slugger slug()
 * 4. Rejoin with '/'
 * 5. Drop trailing '/index' (and map bare 'index' to '')
 * 6. Lowercase
 *
 * Examples:
 * - 'index.md' -> ''
 * - 'tutorials/index.md' -> 'tutorials'
 * - 'reference/cli.md' -> 'reference/cli'
 * - 'architecture/qwen-3.7-eval.md' -> 'architecture/qwen-37-eval'
 * - 'AGENTS.md' -> 'agents'
 */
export function docSlug(relPath) {
  if (!relPath) return '';

  // Normalize backslashes
  const normalized = relPath.replace(/\\/g, '/');

  // Strip .md / .mdx
  const withoutExt = normalized.replace(/\.(md|mdx)$/i, '');

  const segments = withoutExt.split('/');
  const slugger = new GithubSlugger();
  const slugSegments = segments.map((seg) => slugger.slug(seg));

  let slug = slugSegments.join('/').toLowerCase();

  if (slug === 'index') {
    return '';
  }

  if (slug.endsWith('/index')) {
    slug = slug.slice(0, -'/index'.length);
  }

  return slug;
}
