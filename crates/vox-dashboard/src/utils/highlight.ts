/**
 * Lazy-initialised shiki highlighter singleton for the Vox webview.
 *
 * Shiki v4 ships `createHighlighter` which accepts bundled languages by name
 * and TextMate grammars as inlined JSON objects via the `langs` option. The
 * Vox TextMate grammar is imported statically so esbuild inlines it at bundle
 * time — the webview sandbox has no filesystem access, so a `path:` string
 * would silently fall back to `bash` at runtime.
 *
 * Languages not in the bundled set or the Vox grammar fall back to `bash` so
 * `codeToHtml` never throws on an unknown language id.
 */
import { createHighlighter, type Highlighter, type LanguageRegistration } from 'shiki';
// Static import — esbuild resolves and inlines this JSON at build time so the
// grammar is always available inside the webview sandbox (no filesystem reads).
import voxGrammar from './vox.tmLanguage.json';

let _hl: Highlighter | null = null;
let _initPromise: Promise<Highlighter> | null = null;

const BUNDLED_LANGS = [
    'rust',
    'typescript',
    'javascript',
    'python',
    'css',
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
            // Inline Vox TextMate grammar — imported statically above so esbuild
            // bundles it. The cast is required because the JSON type doesn't
            // carry the full LanguageRegistration discriminant.
            voxGrammar as unknown as LanguageRegistration,
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
        const normalizedLang = lang.toLowerCase();
        const searchLang = normalizedLang === 'voxcode' ? 'vox' : normalizedLang;
        const safeLang = loaded.find(l => l.toLowerCase() === searchLang) || 'bash';
        return hl.codeToHtml(code, {
            lang: safeLang,
            theme: dark ? 'github-dark' : 'github-light',
        });
    } catch {
        // Never propagate — degrade to a plain pre block
        return `<pre>${code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')}</pre>`;
    }
}
