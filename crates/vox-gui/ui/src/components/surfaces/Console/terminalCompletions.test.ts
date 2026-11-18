import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as transport from '../../../transport';
import {
  PROMPT_TOKEN_CLASSES,
  isIncompleteContinuation,
  tokenizePromptBuffer,
  tokenizePromptLine,
} from './promptHighlighter';
import { classifyPromptIntent, formatExecutionPayload } from './intentRouter';
import {
  applyTerminalSuggestion,
  computeLocalTerminalCompletions,
  resolveTerminalCompletions,
  utf16IndexToUtf8ByteOffset,
  utf8ByteOffsetToUtf16Index,
} from './terminalCompletions';

vi.mock('../../../transport', () => ({
  termComplete: vi.fn(),
  discoverySuggest: vi.fn().mockResolvedValue([]),
}));

describe('Task 1: promptHighlighter, intentRouter, and terminalCompletions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('promptHighlighter', () => {
    it('tokenizes shell, flags, strings, variables, operators, macros, slash, comments, and preserves exact bytes', () => {
      const sample =
        'git commit -m "feat: warp" \'raw\' $USER ${HOME} --amend -v @file:Cargo.toml @term:last | head > out.log && echo ok || true >> err.log # trailing comment\n';
      const tokens = tokenizePromptLine(sample);
      expect(tokens.map((t) => t.text).join('')).toBe(sample);
      expect(tokenizePromptBuffer(sample).map((t) => t.text).join('')).toBe(sample);

      const kinds = new Set(tokens.map((t) => t.kind));
      expect(kinds.has('command')).toBe(true);
      expect(kinds.has('subcommand')).toBe(true);
      expect(kinds.has('flag')).toBe(true);
      expect(kinds.has('string')).toBe(true);
      expect(kinds.has('variable')).toBe(true);
      expect(kinds.has('operator')).toBe(true);
      expect(kinds.has('macro')).toBe(true);
      expect(kinds.has('comment')).toBe(true);
      expect(kinds.has('text')).toBe(true);

      const slashSample = '/clear now\n';
      const slashTokens = tokenizePromptLine(slashSample);
      expect(slashTokens.map((t) => t.text).join('')).toBe(slashSample);
      expect(slashTokens[0]).toEqual({ text: '/clear', kind: 'slash' });

      // Token classes use semantic design tokens with uniform 400 weight and zero hex literals or italic shifts
      for (const cls of Object.values(PROMPT_TOKEN_CLASSES)) {
        expect(cls).not.toMatch(/#[0-9a-fA-F]{3,8}/);
        expect(cls).not.toContain('italic');
        expect(cls).not.toContain('font-semibold');
        expect(cls).not.toContain('text-brass');
        expect(cls).toContain('font-normal');
      }
    });

    it('detects incomplete multi-line continuations accurately', () => {
      expect(isIncompleteContinuation('echo "hello')).toBe(true);
      expect(isIncompleteContinuation("echo 'unclosed")).toBe(true);
      expect(isIncompleteContinuation('cargo build \\')).toBe(true);
      expect(isIncompleteContinuation('ls -la |')).toBe(true);
      expect(isIncompleteContinuation('cargo check &&')).toBe(true);
      expect(isIncompleteContinuation('cargo test ||')).toBe(true);

      expect(isIncompleteContinuation('echo "hello \\"world\\""')).toBe(false);
      expect(isIncompleteContinuation('cargo build \\\n  --release')).toBe(false);
      expect(isIncompleteContinuation('ls -la | head -n 5')).toBe(false);
      expect(isIncompleteContinuation('echo ok # unclosed " in comment')).toBe(false);
    });
  });

  describe('intentRouter', () => {
    it('routes explicit prefixes (!cmd, ?prompt, /ask, /slash, !nu, .vox) and preserves /slash and ?agent in nu mode', () => {
      const askSlash = classifyPromptIntent('/ask how do actors work', 'nu');
      expect(askSlash.kind).toBe('agent');
      expect(askSlash.modeBadge).toBe('agent');
      expect(askSlash.toSubmitPayload('/ask how do actors work')).toBe('/ask how do actors work');

      const qAgent = classifyPromptIntent('?how to parse json', 'nu');
      expect(qAgent.kind).toBe('agent');
      expect(qAgent.modeBadge).toBe('agent');
      expect(qAgent.toSubmitPayload('?how to parse json')).toBe('?how to parse json');

      const slash = classifyPromptIntent('/model flash', 'nu');
      expect(slash.kind).toBe('slash');
      expect(slash.modeBadge).toBe('slash');
      expect(slash.toSubmitPayload('/model flash')).toBe('/model flash');

      const clearSlash = classifyPromptIntent('/clear', 'default');
      expect(clearSlash.modeBadge).toBe('slash');
      expect(formatExecutionPayload('/clear', 'default')).toBe('/clear');

      const bangShell = classifyPromptIntent('!cargo check', 'agent');
      expect(bangShell.kind).toBe('shell');
      expect(bangShell.modeBadge).toBe('shell');
      expect(bangShell.toSubmitPayload('!cargo check')).toBe('cargo check');

      const nuLocked = classifyPromptIntent('ls | where size > 1kb', 'nu');
      expect(nuLocked.kind).toBe('nu');
      expect(nuLocked.modeBadge).toBe('nu');
      expect(nuLocked.toSubmitPayload('ls | where size > 1kb')).toBe('!nu ls | where size > 1kb');

      const nuExplicit = classifyPromptIntent('!nu ps | sort-by cpu', 'agent');
      expect(nuExplicit.kind).toBe('nu');
      expect(nuExplicit.toSubmitPayload('!nu ps | sort-by cpu')).toBe('!nu ps | sort-by cpu');

      const voxCmd = classifyPromptIntent('vox run scripts/setup.vox', 'default');
      expect(voxCmd.kind).toBe('vox');
      expect(voxCmd.modeBadge).toBe('vox');

      const voxScript = classifyPromptIntent('./scripts/build.vox', 'default');
      expect(voxScript.kind).toBe('vox');
      expect(voxScript.modeBadge).toBe('vox');
    });

    it('surfaces non-hijacking AI Assist pill for natural-language questions while keeping Enter on shell', () => {
      const nl1 = classifyPromptIntent('how do i list open ports', 'default');
      expect(nl1.kind).toBe('shell');
      expect(nl1.modeBadge).toBe('shell');
      expect(nl1.showAiAssistPill).toBe(true);
      expect(nl1.toSubmitPayload('how do i list open ports')).toBe('how do i list open ports');

      const nl2 = classifyPromptIntent('why did cargo fail', 'shell');
      expect(nl2.kind).toBe('shell');
      expect(nl2.showAiAssistPill).toBe(true);
      expect(nl2.toSubmitPayload('ls -la')).toBe('!ls -la');

      // Operators or flags suppress AI Assist pill
      const shellWithPipe = classifyPromptIntent('what file | grep foo', 'default');
      expect(shellWithPipe.showAiAssistPill).toBe(false);

      const shellWithFlag = classifyPromptIntent('list all --json items', 'default');
      expect(shellWithFlag.showAiAssistPill).toBe(false);
    });
  });

  describe('terminalCompletions', () => {
    it('maps all 9 IPC badge categories, merges local @macro and /slash items, and computes ghostSuffix', async () => {
      vi.mocked(transport.termComplete).mockResolvedValueOnce([
        { label: 'cargo', insert_text: 'cargo', kind: 'executable', score: 950, replace_start: 0, replace_end: 3 },
        { label: 'crates/', insert_text: 'crates/', kind: 'directory', score: 900, replace_start: 0, replace_end: 3 },
        { label: 'Cargo.toml', insert_text: 'Cargo.toml', kind: 'path', score: 890, replace_start: 0, replace_end: 3 },
        { label: 'main', insert_text: 'main', kind: 'git_ref', score: 880, replace_start: 0, replace_end: 3 },
        { label: '--release', insert_text: '--release', kind: 'flag', score: 870, replace_start: 0, replace_end: 3 },
        { label: 'check', insert_text: 'check', kind: 'subcommand', score: 860, replace_start: 0, replace_end: 3 },
        { label: '@term:last', insert_text: '@term:last', kind: 'vox_macro', score: 850, replace_start: 0, replace_end: 3 },
        { label: '/clear', insert_text: '/clear', kind: 'slash', score: 840, replace_start: 0, replace_end: 3 },
        { label: 'cargo test -p vox-cli', insert_text: 'cargo test -p vox-cli', kind: 'history', score: 830, replace_start: 0, replace_end: 3 },
      ]);

      const res = await resolveTerminalCompletions('car', 3, { cwd: '/repo' });
      const badges = new Set(res.items.map((i) => i.badge));
      expect(badges).toEqual(
        new Set(['BIN', 'DIR', 'FILE', 'GIT', 'FLAG', 'SUB', 'MACRO', 'SLASH', 'HIST']),
      );
      expect(res.ghostSuffix).toBe('go');

      const applied = applyTerminalSuggestion('git checkout ma', {
        ...res.items.find((i) => i.badge === 'GIT')!,
        replaceStart: 13,
        replaceEnd: 15,
        insertText: 'main',
      });
      expect(applied).toEqual({ value: 'git checkout main', cursor: 17 });
    });

    it('degrades cleanly to local computeIntelliSenseItems when transport.termComplete returns null or is undefined', async () => {
      vi.mocked(transport.termComplete).mockResolvedValueOnce(null);

      const atRes = await resolveTerminalCompletions('@', 1);
      expect(atRes.items.some((i) => i.insertText === '@file:' || i.insertText === '@term:last')).toBe(true);
      expect(atRes.items.some((i) => i.insertText === '@term:last')).toBe(true);

      const slashRes = await resolveTerminalCompletions('/cl', 3);
      expect(slashRes.items.some((i) => i.insertText.startsWith('/clear'))).toBe(true);

      const histRes = computeLocalTerminalCompletions('cargo check --w', 15, {
        historyCommands: ['cargo check --workspace --all-targets'],
      });
      expect(histRes.ghostSuffix).toBe('orkspace --all-targets');
      expect(histRes.items[0]?.badge).toBe('HIST');
    });

    it('converts UTF-16 cursor offsets to UTF-8 byte offsets and maps Rust UTF-8 spans back to UTF-16 safely (CR-01)', async () => {
      // '🚀' is 2 UTF-16 code units and 4 UTF-8 bytes; 'é' is 1 UTF-16 code unit and 2 UTF-8 bytes
      const line = "echo '🚀é' && cargo ch";
      const utf16Cursor = line.length; // 22 UTF-16 code units
      const expectedUtf8Cursor = 25; // 25 UTF-8 bytes
      expect(utf16IndexToUtf8ByteOffset(line, utf16Cursor)).toBe(expectedUtf8Cursor);
      expect(utf8ByteOffsetToUtf16Index(line, 23)).toBe(20);

      vi.mocked(transport.termComplete).mockResolvedValueOnce([
        {
          label: 'check',
          insert_text: 'check',
          kind: 'subcommand',
          score: 92, // backend 0..100 scale normalized to 920
          replace_start: 23, // UTF-8 byte start of 'ch'
          replace_end: 25, // UTF-8 byte end of 'ch'
        },
      ]);

      const res = await resolveTerminalCompletions(line, utf16Cursor, { cwd: '/repo' });
      expect(transport.termComplete).toHaveBeenCalledWith(
        expect.objectContaining({ line, cursor: expectedUtf8Cursor }),
      );
      const checkItem = res.items.find((i) => i.label === 'check')!;
      expect(checkItem.score).toBe(920);
      expect(checkItem.replaceStart).toBe(20);
      expect(checkItem.replaceEnd).toBe(22);
      expect(res.ghostSuffix).toBe('eck');

      const applied = applyTerminalSuggestion(line, checkItem);
      expect(applied).toEqual({
        value: "echo '🚀é' && cargo check",
        cursor: "echo '🚀é' && cargo check".length,
      });
    });
  });
});
