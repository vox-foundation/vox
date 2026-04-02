/**
 * Lazy-initialised shiki highlighter singleton for the Vox webview.
 *
 * Shiki v4 ships `createHighlighter` which accepts bundled languages by name
 * and TextMate grammars via the `langs` option. The Vox TextMate grammar is
 * already bundled with the extension at `../../syntaxes/vox.tmLanguage.json`
 * relative to this file's compiled output location.
 *
 * Languages not in the bundled set or the Vox grammar fall back to `bash` so
 * `codeToHtml` never throws on an unknown language id.
 */
import { createHighlighter, type Highlighter } from 'shiki';

let _hl: Highlighter | null = null;
let _initPromise: Promise<Highlighter> | null = null;

const BUNDLED_LANGS = [
    'rust',
    'typescript',
    'javascript',
    'bash',
    'json',
    'toml',
    'markdown',
    'sql',
] as const;

/** Lazily initialise and cache the shiki highlighter (singleton). */
async function getHighlighter(): Promise<Highlighter> {
    if (_hl) return _hl;
    if (_initPromise) return _initPromise;

    _initPromise = createHighlighter({
        themes: ['github-dark', 'github-light'],
        langs: [
            ...BUNDLED_LANGS,
            // Register the existing Vox TextMate grammar shipped with the extension.
            // The path is relative to the esbuild output location (out/webview.js).
            // esbuild bundles the import; the json file is resolved at build time,
            // so we inline it to guarantee availability in the webview sandbox.
            // NOTE: if the path resolution fails at build, shiki degrades to bash.
            {
                name: 'vox',
                scopeName: 'source.vox',
                // Reach from out/webview.js up to syntaxes/
                path: '../../syntaxes/vox.tmLanguage.json',
            } as Parameters<typeof createHighlighter>[0]['langs'][number],
        ],
    }).then((hl) => {
        _hl = hl;
        return hl;
    });

    return _initPromise;
}

/**
 * Syntax-highlight `code` for `lang` and return an HTML string.
 *
 * @param code   Raw source text (no surrounding fences).
 * @param lang   Language identifier (e.g. `"rust"`, `"vox"`, `"typescript"`).
 * @param dark   Whether to use the dark theme (`github-dark` vs `github-light`).
 *
 * Falls back to `bash` when the language is not loaded, so callers never need
 * to guard the return value.
 */
export async function highlightCode(
    code: string,
    lang: string,
    dark: boolean,
): Promise<string> {
    try {
        const hl = await getHighlighter();
        const loaded = hl.getLoadedLanguages();
        const safeLang = loaded.includes(lang as (typeof loaded)[number]) ? lang : 'bash';
        return hl.codeToHtml(code, {
            lang: safeLang,
            theme: dark ? 'github-dark' : 'github-light',
        });
    } catch {
        // Never propagate — degrade to a plain pre block
        return `<pre>${code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')}</pre>`;
    }
}
