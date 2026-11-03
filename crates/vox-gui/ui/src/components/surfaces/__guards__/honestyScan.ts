export type Violation = {
  file: string;
  line: number;
  kind: 'placeholder' | 'dead-handler' | 'unguarded-mock';
  snippet: string;
};

// Visible "this isn't real yet" prose. The `placeholder` alternative is guarded
// with (?![:=-]) so it does NOT match the Tailwind `placeholder:` pseudo-class or
// an input `placeholder=` attribute — those are legitimate, not honesty violations.
const PLACEHOLDER =
  /\b(not\s+(yet\s+)?(implemented|wired|available|working|hooked|connected)|coming\s+soon|placeholder(?![:=-])|todo:\s*wire)\b/i;
// empty or no-op arrow body: onClick={() => {}} / onClick={() => void 0} / onClick={() => null}
const DEAD_HANDLER =
  /on(Click|Submit|Change|Press)=\{\s*\(\s*[^)]*\)\s*=>\s*(\{\s*\}|void\s+0|null|undefined)\s*\}/;
// Unguarded e2e/test mock imports or direct __TAURI_INTERNALS__ stub assignments in shipped surfaces
const UNGUARDED_MOCK =
  /(?:from\s+['"][^'"]*e2e\/lib\/tauriMock|window\.__TAURI_INTERNALS__\s*=)/;

// A line whose visible content is a comment carries no shipped UI text; "not wired"
// explaining a deliberate decision is honest documentation, not a dishonest element.
function isCommentLine(raw: string): boolean {
  const t = raw.trim();
  return t.startsWith('//') || t.startsWith('*') || t.startsWith('/*') || t.startsWith('{/*');
}

export function scanSource(file: string, text: string): Violation[] {
  const out: Violation[] = [];
  text.split('\n').forEach((raw, i) => {
    const line = i + 1;
    if (isCommentLine(raw)) return;
    if (PLACEHOLDER.test(raw)) out.push({ file, line, kind: 'placeholder', snippet: raw.trim() });
    if (DEAD_HANDLER.test(raw)) out.push({ file, line, kind: 'dead-handler', snippet: raw.trim() });
    if (UNGUARDED_MOCK.test(raw)) out.push({ file, line, kind: 'unguarded-mock', snippet: raw.trim() });
  });
  return out;
}
