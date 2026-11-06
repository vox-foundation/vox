/**
 * Exclude predicate for starlight-links-validator.
 *
 * Two narrow classes of false positives are excluded; every other finding must
 * be fixed at its source (hand-authored page, or the generator of a
 * `*.generated.md`), never by widening this predicate.
 *
 * 1. `.mdx` pages: docs/src `.mdx` pages are symlinked into the content dir and
 *    Vite realpaths them, so the validator cannot derive their routes. Links
 *    FROM an `.mdx` page and links TO an `.mdx` route are therefore unverifiable.
 * 2. `http://127.0.0.1:11434`: the documented VOX_LOCAL_ENDPOINT probe default.
 *
 * @param {{ relPath: string, id: string }[]} docPages result of listDocPages()
 * @returns {({ file, link }: { file: string, link: string }) => boolean}
 */
export function linksValidatorExclude(docPages) {
  const mdxRoutes = new Set(docPages.filter((p) => p.relPath.endsWith('.mdx')).map((p) => p.id));
  return ({ file, link }) => {
    const inMdx = file.endsWith('.mdx');
    // Same-page hash: only unverifiable when the page itself is .mdx.
    if (link.startsWith('#')) return inMdx;
    if (inMdx) return true;
    if (link.startsWith('http://127.0.0.1:11434')) return true;
    const path = link.split(/[?#]/)[0] || '/';
    return mdxRoutes.has(path.replace(/^\/+|\/+$/g, ''));
  };
}
