/**
 * Version-free model family keys: a line-by-line mirror of the Rust
 * `vox_orchestrator::models::family::family_key` (model-routing plan, Task 2).
 * The GUI shows a family instead of a version whenever a model id was not
 * read from the live OpenRouter catalog.
 */

/** Release-stage words that do not start a new family. */
const QUALIFIERS = new Set(['preview', 'beta', 'exp', 'experimental', 'latest']);

/** `8b`, `70b`, `a22b`: a parameter size is a different cost point, so it stays in the key. */
function isSizeToken(tok: string): boolean {
  if (!tok.endsWith('b')) return false;
  const body = tok.slice(0, -1).replace(/^a+/, '');
  return /^[0-9.]+$/.test(body) && /[0-9]/.test(body);
}

/** `org/name-words[:free]`, lowercased, with version numbers, `v4` markers and stage words dropped. */
export function familyKey(slug: string): string {
  const lower = slug.toLowerCase();
  const colon = lower.indexOf(':');
  const base = colon >= 0 ? lower.slice(0, colon) : lower;
  const variant = colon >= 0 ? lower.slice(colon + 1) : '';
  const slash = base.indexOf('/');
  const org = slash >= 0 ? base.slice(0, slash) : '';
  const name = slash >= 0 ? base.slice(slash + 1) : base;
  const words: string[] = [];
  for (const tok of name.split('-')) {
    if (tok === '' || QUALIFIERS.has(tok)) continue;
    if (!/[0-9]/.test(tok) || isSizeToken(tok)) {
      words.push(tok);
      continue;
    }
    if (/^[0-9.]+$/.test(tok) || /^v[0-9.]+$/.test(tok)) continue;
    const letters = tok.replace(/[^a-z]/g, '');
    if (letters) words.push(letters);
  }
  let key = org ? `${org}/${words.join('-')}` : words.join('-');
  if (variant === 'free') key += ':free';
  return key;
}

/**
 * A versioned cloud model id (Claude, GPT, Gemini, o-series, DeepSeek, Llama, Qwen, Kimi, GLM, Grok release
 * numbers). The GUI never hardcodes one; `src/__tests__/noVersionedModelIds.test.ts` enforces it.
 */
export const VERSIONED_CLOUD_MODEL_ID =
  /(opus|sonnet|haiku|fable)-\d|gpt-\d|gemini-\d|\bo\d-mini|deepseek-v\d|llama-?\d|qwen\d|kimi-k\d|glm-\d|grok-\d/i;
