import { describe, it, expect, vi, beforeEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { Terminal } from '@xterm/xterm';
import {
  auditSourceTextAgainstThemeContract,
  computeContrastRatio,
} from '../../../lib/themeContract';
import {
  TERMINAL_MONO_FONT_STACK,
  XTERM_FONT_FAMILY,
  resolveAnsi16Palette,
  resolveTerminalTheme,
} from './terminalTheme';
import { WebglTerminalAddon, renderTermBlockForAgent } from './webglAddon';
import {
  SearchTerminalAddon,
  WebLinksTerminalAddon,
  Unicode11TerminalAddon,
  SerializeTerminalAddon,
} from './xtermAddons';

function createMockBufferTerminal(
  lines: Array<{ text: string; isWrapped?: boolean }>,
  extra: Record<string, unknown> = {},
): Terminal {
  return {
    cols: 80,
    rows: 24,
    buffer: {
      active: {
        length: lines.length,
        getLine: (y: number) => {
          const entry = lines[y];
          if (!entry) return undefined;
          return {
            isWrapped: Boolean(entry.isWrapped),
            length: entry.text.length,
            translateToString: (trimRight?: boolean) =>
              trimRight ? entry.text.replace(/\s+$/, '') : entry.text,
          };
        },
      },
    },
    select: vi.fn(),
    clearSelection: vi.fn(),
    scrollToLine: vi.fn(),
    registerLinkProvider: vi.fn((provider) => {
      (extra as { provider?: unknown }).provider = provider;
      return { dispose: vi.fn() };
    }),
    ...extra,
  } as unknown as Terminal;
}

describe('terminalTheme — 4-profile WCAG >= 4.5:1 & ThemeContract compliance', () => {
  it('exports canonical font stacks without banned font names', () => {
    expect(TERMINAL_MONO_FONT_STACK).toBe(
      "var(--font-terminal-mono, 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace)",
    );
    expect(XTERM_FONT_FAMILY).toBe(
      "'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace",
    );
  });

  it('guarantees WCAG >= 4.5:1 across all 16 ANSI foreground slots on all 4 profile backgrounds', () => {
    const profileMatrix: Array<{ pref: string; expectedBg: string; minRatio: number }> = [
      { pref: 'arcane', expectedBg: '#15110d', minRatio: 4.5 },
      { pref: 'travertine', expectedBg: '#171512', minRatio: 4.5 },
      { pref: 'glacier', expectedBg: '#171512', minRatio: 4.5 },
      { pref: 'slate', expectedBg: '#0c0e10', minRatio: 4.5 },
      { pref: 'void', expectedBg: '#0c0e10', minRatio: 4.5 },
      { pref: 'high-contrast', expectedBg: '#000000', minRatio: 7.0 },
    ];

    for (const { pref, expectedBg, minRatio } of profileMatrix) {
      const theme = resolveTerminalTheme(pref);
      expect(theme.background?.toLowerCase()).toBe(expectedBg.toLowerCase());
      const palette = resolveAnsi16Palette(pref);
      for (const [slot, fgHex] of Object.entries(palette)) {
        const ratio = computeContrastRatio(fgHex, expectedBg);
        expect(
          ratio,
          `${pref} ${slot} (${fgHex} on ${expectedBg}) contrast=${ratio}`,
        ).toBeGreaterThanOrEqual(minRatio);
      }
    }
  });

  it('passes auditSourceTextAgainstThemeContract with zero violations', () => {
    const srcPath = join(import.meta.dirname, 'terminalTheme.ts');
    const raw = readFileSync(srcPath, 'utf8');
    const findings = auditSourceTextAgainstThemeContract(
      'src/components/surfaces/Console/terminalTheme.ts',
      raw,
    );
    expect(findings).toEqual([]);
  });
});

describe('WebLinksTerminalAddon — 1-based ILinkProvider & scheme allowlist', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('emits 1-based ranges for http/https, localhost, and file:line:col while blocking unsafe URL schemes', () => {
    const opened: Array<{ kind: string; value: string }> = [];
    const fileEvents: unknown[] = [];
    const onFileEvent = (e: Event) => fileEvents.push((e as CustomEvent).detail);
    window.addEventListener('vox://open-file-location', onFileEvent);

    const holder: {
      provider?: {
        provideLinks: (
          bufferLineNumber: number,
          cb: (links: Array<{ range: unknown; text: string; activate: (ev: MouseEvent, text: string) => void }> | undefined) => void,
        ) => void;
      };
    } = {};

    const term = createMockBufferTerminal(
      [
        { text: 'Docs: https://vox.dev/docs, local localhost:3000/api!' },
        { text: '  --> crates/vox-cli/src/main.rs:42:9 panicked' },
        {
          text: [
            'Ignore javascript:alert(1)',
            'data:text/plain;charset=utf-8,dir/payload.sh:1',
            'file:///etc/ssh/sshd_config.conf:1',
            '-n/foo.rs:1',
            '../secret.rs:1',
          ].join(' '),
        },
      ],
      holder,
    );

    const addon = new WebLinksTerminalAddon({
      onOpenLocator: (loc) => {
        opened.push(loc);
      },
    });
    addon.activate(term);
    expect(holder.provider).toBeDefined();

    // Line 1 (bufferLineNumber = 1 -> 0-based index 0)
    let line1Links: Array<{ range: unknown; text: string; activate: (ev: MouseEvent, text: string) => void }> | undefined;
    holder.provider!.provideLinks(1, (links) => {
      line1Links = links;
    });
    expect(line1Links).toHaveLength(2);
    expect(line1Links![0]).toMatchObject({
      text: 'https://vox.dev/docs',
      range: {
        start: { x: 7, y: 1 },
        end: { x: 26, y: 1 },
      },
    });
    expect(line1Links![1]).toMatchObject({
      text: 'localhost:3000/api',
      range: {
        start: { x: 35, y: 1 },
        end: { x: 52, y: 1 },
      },
    });

    line1Links![0]!.activate(new MouseEvent('click'), line1Links![0]!.text);
    line1Links![1]!.activate(new MouseEvent('click'), line1Links![1]!.text);
    expect(opened).toEqual([
      { kind: 'web', value: 'https://vox.dev/docs' },
      { kind: 'web', value: 'http://localhost:3000/api' },
    ]);

    // Line 2 (bufferLineNumber = 2 -> 0-based index 1)
    let line2Links: Array<{ range: unknown; text: string; activate: (ev: MouseEvent, text: string) => void }> | undefined;
    holder.provider!.provideLinks(2, (links) => {
      line2Links = links;
    });
    expect(line2Links).toHaveLength(1);
    expect(line2Links![0]).toMatchObject({
      text: 'crates/vox-cli/src/main.rs:42:9',
      range: {
        start: { x: 7, y: 2 },
        end: { x: 37, y: 2 },
      },
    });
    line2Links![0]!.activate(new MouseEvent('click'), line2Links![0]!.text);
    expect(opened[2]).toEqual({ kind: 'file', value: 'crates/vox-cli/src/main.rs' });
    expect(fileEvents).toEqual([
      {
        path: 'crates/vox-cli/src/main.rs',
        line: 42,
        col: 9,
        value: 'crates/vox-cli/src/main.rs:42:9',
      },
    ]);

    // Line 3 (unsafe schemes rejected)
    let line3Links: unknown;
    holder.provider!.provideLinks(3, (links) => {
      line3Links = links;
    });
    expect(line3Links ?? []).toEqual([]);

    window.removeEventListener('vox://open-file-location', onFileEvent);
    addon.dispose();
  });

  it('no-ops safely on minimal mock Terminal lacking registerLinkProvider', () => {
    const minimalTerm = { cols: 80, rows: 24 } as unknown as Terminal;
    const addon = new WebLinksTerminalAddon();
    expect(() => addon.activate(minimalTerm)).not.toThrow();
    expect(() => addon.dispose()).not.toThrow();
  });
});

describe('SearchTerminalAddon, Unicode11TerminalAddon, SerializeTerminalAddon & WebglTerminalAddon', () => {
  it('SearchTerminalAddon finds next/previous case-insensitively, selects, scrolls, and clears', () => {
    const term = createMockBufferTerminal([
      { text: 'alpha Error one' },
      { text: 'no match here' },
      { text: 'beta error two and ERROR three' },
    ]);
    const search = new SearchTerminalAddon();
    search.activate(term);

    const first = search.findNext('error');
    expect(first).toEqual({ activeMatchIndex: 1, totalMatches: 3 });
    expect(term.select).toHaveBeenLastCalledWith(6, 0, 5);
    expect(term.scrollToLine).toHaveBeenLastCalledWith(0);

    const second = search.findNext('error');
    expect(second).toEqual({ activeMatchIndex: 2, totalMatches: 3 });
    expect(term.select).toHaveBeenLastCalledWith(5, 2, 5);
    expect(term.scrollToLine).toHaveBeenLastCalledWith(2);

    const prev = search.findPrevious('error');
    expect(prev).toEqual({ activeMatchIndex: 1, totalMatches: 3 });
    expect(term.select).toHaveBeenLastCalledWith(6, 0, 5);

    const wrapPrev = search.findPrevious('error');
    expect(wrapPrev).toEqual({ activeMatchIndex: 3, totalMatches: 3 });
    expect(term.select).toHaveBeenLastCalledWith(19, 2, 5);

    expect(search.findNext('nonexistent')).toEqual({ activeMatchIndex: 0, totalMatches: 0 });

    const cjkTerm = createMockBufferTerminal([{ text: '中文 error here' }]);
    const cjkSearch = new SearchTerminalAddon();
    cjkSearch.activate(cjkTerm);
    expect(cjkSearch.findNext('error')).toEqual({ activeMatchIndex: 1, totalMatches: 1 });
    expect(cjkTerm.select).toHaveBeenLastCalledWith(5, 0, 5);
    cjkSearch.dispose();

    search.clear();
    expect(term.clearSelection).toHaveBeenCalled();
    search.dispose();
  });

  it('Unicode11TerminalAddon registers version 11 wcwidth and charProperties width bits safely', () => {
    let registeredProvider: {
      version: string;
      wcwidth: (cp: number) => 0 | 1 | 2;
      charProperties?: (cp: number, preceding: number) => number;
    } | undefined;
    const unicodeApi = {
      versions: ['6'],
      activeVersion: '6',
      register: vi.fn((p) => {
        registeredProvider = p;
      }),
    };
    const term = createMockBufferTerminal([], { unicode: unicodeApi });
    const u11 = new Unicode11TerminalAddon();
    u11.activate(term);

    expect(unicodeApi.register).toHaveBeenCalledOnce();
    expect(unicodeApi.activeVersion).toBe('11');
    expect(registeredProvider?.wcwidth(0x4e2d)).toBe(2);
    expect(registeredProvider?.wcwidth(0x0301)).toBe(0);
    expect(registeredProvider?.wcwidth(0x0041)).toBe(1);
    expect(((registeredProvider?.charProperties?.(0x0041, 0) ?? 0) >> 1) & 3).toBe(1);
    expect(((registeredProvider?.charProperties?.(0x4e2d, 0) ?? 0) >> 1) & 3).toBe(2);
    expect(((registeredProvider?.charProperties?.(0x0301, 0) ?? 0) >> 1) & 3).toBe(0);

    const throwingTerm = {} as Terminal;
    Object.defineProperty(throwingTerm, 'unicode', {
      get() {
        throw new Error('allowProposedApi must be set');
      },
    });
    expect(() => new Unicode11TerminalAddon().activate(throwingTerm)).not.toThrow();
  });

  it('SerializeTerminalAddon reconstructs wrapped and unwrapped buffer lines', () => {
    const term = createMockBufferTerminal([
      { text: '$ echo long-line-', isWrapped: false },
      { text: 'continued-here   ', isWrapped: true },
      { text: 'second line      ', isWrapped: false },
    ]);
    const serializer = new SerializeTerminalAddon();
    serializer.activate(term);
    expect(serializer.serialize()).toBe('$ echo long-line-continued-here\nsecond line');
    serializer.dispose();
  });

  it('WebglTerminalAddon reports renderer mode fallback and strips ANSI in renderTermBlockForAgent', () => {
    const modes: Array<'webgl2' | 'dom'> = [];
    const addon = new WebglTerminalAddon({
      onRendererModeChange: (m) => modes.push(m),
    });
    let lostCount = 0;
    addon.onContextLoss(() => {
      lostCount += 1;
    });

    const hostEl = document.createElement('div');
    addon.activate({ element: hostEl } as unknown as Terminal);
    expect(addon.isWebglActive()).toBe(false);
    expect(modes).toContain('dom');

    addon.simulateContextLoss();
    expect(lostCount).toBe(1);
    addon.dispose();
    expect(addon.isDisposed()).toBe(true);

    const loseContext = vi.fn();
    const getContextSpy = vi
      .spyOn(HTMLCanvasElement.prototype, 'getContext')
      .mockImplementation(((contextId: string) => {
        if (contextId === 'webgl2') {
          return {
            getExtension: (name: string) => (name === 'WEBGL_lose_context' ? { loseContext } : null),
          } as unknown as WebGL2RenderingContext;
        }
        return null;
      }) as never);

    const probeModes: Array<'webgl2' | 'dom'> = [];
    const probedAddon = new WebglTerminalAddon({
      onRendererModeChange: (m) => probeModes.push(m),
    });
    probedAddon.activate({ element: hostEl } as unknown as Terminal);
    expect(loseContext).toHaveBeenCalledOnce();
    expect(probedAddon.isWebglActive()).toBe(true);
    expect(probeModes).toEqual(['webgl2']);
    probedAddon.simulateContextLoss();
    expect(probeModes).toEqual(['webgl2', 'dom']);
    getContextSpy.mockRestore();

    expect(
      renderTermBlockForAgent(
        {
          id: 1,
          kind: 'Shell',
          status: 'Ok',
          input: 'cargo check',
          output: [{ stream: 'Stdout', text: '\x1b[32mFinished\x1b[0m dev\n', ts_ms: 1 }],
          exit_code: 0,
          started_ms: 1,
          ended_ms: 2,
        },
        'fallback',
      ),
    ).toBe('$ cargo check\nFinished dev\n(exit 0)');
  });
});
