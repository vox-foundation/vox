// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  evaluateGuiSurfaceRubric,
  runUniversalCvFlywheelPass,
  type GuiDomAuditMetrics,
} from '../../src/lib/cvFlywheel';
import { richMockInitScript } from '../lib/tauriMockRich';
import { SURFACE_STATES, TERMINAL_CV_STATES } from './states';
import { collectTerminalAuditMetrics } from './terminalAudits';

function cleanBaseDomMetrics(overrides: Partial<GuiDomAuditMetrics> = {}): GuiDomAuditMetrics {
  return {
    surfaceId: 'console',
    outerOverflowPx: 0,
    scrollHostOverflowPx: 0,
    clippedInteractiveCount: 0,
    overlappingControlPairs: 0,
    unstyledNativeSelectCount: 0,
    lightThemeContrastLeakCount: 0,
    iconIssueCount: 0,
    controlHeightSpreadPx: 0,
    rawEmojiCount: 0,
    unlitLandmarkCount: 0,
    missingDockPanelIds: [],
    axeViolations: 0,
    viewportCoverageRatio: 0.82,
    skeletonCount: 0,
    rawBacktickCount: 0,
    digitalColorLeakCount: 0,
    ...overrides,
  };
}

describe('collectTerminalAuditMetrics', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    document.body.innerHTML = '';
    document.documentElement.setAttribute('data-theme', 'arcane');
  });

  it('is 100% self-contained when serialized via Function.prototype.toString()', () => {
    document.body.innerHTML = `
      <div data-terminal-well="true" style="background-color: rgb(21, 17, 13); color: rgb(245, 239, 226); font-family: 'JetBrains Mono', ui-monospace, monospace;">
        <div data-testid="term-block-output" style="font-family: 'JetBrains Mono', ui-monospace, monospace;">
          <span style="color: rgb(110, 231, 183);">PASS vox check</span>
        </div>
      </div>
    `;
    const rehydrated = new Function(`return (${collectTerminalAuditMetrics.toString()});`)() as (
      doc?: Document,
    ) => ReturnType<typeof collectTerminalAuditMetrics>;
    const res = rehydrated(document);
    expect(res.terminalStartupSpamCount).toBe(0);
    expect(res.xtermHelperLeakCount).toBe(0);
    expect(res.terminalFontProportionalLeakCount).toBe(0);
    expect(res.terminalContrastViolationCount).toBe(0);
  });

  it('detects shell startup hook spam and passes clean terminal output', () => {
    document.body.innerHTML = `
      <div data-testid="term-block-output">]133;A __vox_ps PROMPT_COMMAND=precmd_functions</div>
      <div data-testid="chat-term-block-output">export PS1="vox$ "</div>
      <div class="xterm-rows"><div>clean build finished in 0.42s</div></div>
    `;
    const dirty = collectTerminalAuditMetrics(document);
    expect(dirty.terminalStartupSpamCount).toBe(2);
    expect(dirty.samples.some((s) => s.includes('startup-spam'))).toBe(true);

    document.body.innerHTML = `
      <div data-testid="term-block-output">vox check: 0 errors across 54 crates</div>
      <div data-testid="chat-term-block-output">cargo test -p vox-terminal-core: ok</div>
    `;
    expect(collectTerminalAuditMetrics(document).terminalStartupSpamCount).toBe(0);
  });

  it('detects leaky .xterm-helper-textarea boxes and passes zero-box contained textarea', () => {
    document.body.innerHTML = `
      <textarea id="leaky" class="xterm-helper-textarea" style="opacity: 1; background-color: rgb(251, 247, 238); outline: 2px solid rgb(212, 175, 55);"></textarea>
    `;
    const leakyEl = document.getElementById('leaky') as HTMLElement;
    leakyEl.getBoundingClientRect = () =>
      ({ width: 120, height: 24, left: 0, top: 0, right: 120, bottom: 24 }) as DOMRect;
    expect(collectTerminalAuditMetrics(document).xtermHelperLeakCount).toBe(1);

    document.body.innerHTML = `
      <textarea id="contained" class="xterm-helper-textarea" style="opacity: 0; width: 0px; height: 0px; background: transparent; outline: none; box-shadow: none;"></textarea>
    `;
    const containedEl = document.getElementById('contained') as HTMLElement;
    containedEl.getBoundingClientRect = () =>
      ({ width: 0, height: 0, left: 0, top: 0, right: 0, bottom: 0 }) as DOMRect;
    expect(collectTerminalAuditMetrics(document).xtermHelperLeakCount).toBe(0);
  });

  it('detects proportional font stacks and M-vs-i glyph width drift > 0.1px', () => {
    document.body.innerHTML = `
      <div data-testid="term-block-output" style="font-family: 'Palatino Linotype', Georgia, serif;">proportional leak</div>
      <div data-terminal-mono="true" style="font-family: 'Cinzel', serif;">header leak</div>
    `;
    expect(collectTerminalAuditMetrics(document).terminalFontProportionalLeakCount).toBe(2);

    document.body.innerHTML = `
      <div class="xterm" style="font-family: 'JetBrains Mono', ui-monospace, monospace;">fixed width</div>
    `;
    const getCtxSpy = vi
      .spyOn(HTMLCanvasElement.prototype, 'getContext')
      .mockImplementation((() => ({
        font: '',
        measureText: (txt: string) => ({ width: txt.startsWith('M') ? 92.4 : 41.2 }),
      })) as unknown as typeof HTMLCanvasElement.prototype.getContext);
    expect(collectTerminalAuditMetrics(document).terminalFontProportionalLeakCount).toBe(1);

    getCtxSpy.mockImplementation((() => ({
      font: '',
      measureText: () => ({ width: 78.0 }),
    })) as unknown as typeof HTMLCanvasElement.prototype.getContext);
    expect(collectTerminalAuditMetrics(document).terminalFontProportionalLeakCount).toBe(0);
  });

  it('enforces WCAG >= 4.5:1 across standard themes and >= 7.0:1 AAA in high-contrast', () => {
    document.documentElement.setAttribute('data-theme', 'arcane');
    document.body.innerHTML = `
      <div data-terminal-well="true" style="background-color: rgb(21, 17, 13);">
        <span id="bad-ansi0" style="color: rgb(28, 25, 23);">unreadable black</span>
        <span id="good-ansi2" style="color: rgb(110, 231, 183);">calibrated green</span>
      </div>
    `;
    expect(collectTerminalAuditMetrics(document).terminalContrastViolationCount).toBe(1);

    // 5.2:1 passes AA (4.5) in arcane, but fails AAA (7.0) in high-contrast
    document.body.innerHTML = `
      <div data-terminal-well="true" style="background-color: rgb(0, 0, 0);">
        <span style="color: rgb(122, 122, 122);">mid gray 4.9:1</span>
      </div>
    `;
    document.documentElement.setAttribute('data-theme', 'slate');
    expect(collectTerminalAuditMetrics(document).terminalContrastViolationCount).toBe(0);

    document.documentElement.setAttribute('data-theme', 'high-contrast');
    expect(collectTerminalAuditMetrics(document).terminalContrastViolationCount).toBe(1);

    document.body.innerHTML = `
      <div data-terminal-well="true" style="background-color: rgb(0, 0, 0);">
        <span style="color: rgb(255, 255, 255);">crisp AAA white</span>
      </div>
    `;
    expect(collectTerminalAuditMetrics(document).terminalContrastViolationCount).toBe(0);
  });

  it('audits aria-hidden=".xterm-rows" children inside visible .xterm and flags blank .xterm-rows or opacity contrast drops', () => {
    document.documentElement.setAttribute('data-theme', 'arcane');
    document.body.innerHTML = `
      <div class="xterm" style="background-color: rgb(21, 17, 13);">
        <div class="xterm-rows" aria-hidden="true"><div></div></div>
      </div>
    `;
    const blankRes = collectTerminalAuditMetrics(document);
    expect(blankRes.terminalContrastViolationCount).toBe(1);
    expect(blankRes.samples).toContain('xterm-blank-rows');

    document.body.innerHTML = `
      <div class="xterm" style="background-color: rgb(21, 17, 13);">
        <div class="xterm-rows" aria-hidden="true">
          <div><span style="color: rgb(139, 126, 109); opacity: 0.75;">dimmed ANSI0</span></div>
        </div>
      </div>
    `;
    expect(collectTerminalAuditMetrics(document).terminalContrastViolationCount).toBe(1);

    document.body.innerHTML = `
      <div class="xterm" style="background-color: rgb(21, 17, 13);">
        <div class="xterm-rows" aria-hidden="true">
          <div><span style="color: rgb(245, 239, 226);">READY vox-terminal-core</span></div>
        </div>
      </div>
    `;
    expect(collectTerminalAuditMetrics(document).terminalContrastViolationCount).toBe(0);
  });
});

describe('cvFlywheel terminal defect rubric penalties', () => {
  it('deducts >= 0.30 and forces visualAfterScore < 0.92 whenever any terminal invariant fails', () => {
    const crispPixels = { edgeSharpness: 90, luminanceContrast: 68, noiseFloor: 4.1 };
    const cleanRubric = evaluateGuiSurfaceRubric(cleanBaseDomMetrics(), crispPixels);

    for (const defectKey of [
      'terminalStartupSpamCount',
      'xtermHelperLeakCount',
      'terminalFontProportionalLeakCount',
      'terminalContrastViolationCount',
    ] as const) {
      const brokenDom = cleanBaseDomMetrics({ [defectKey]: 1 });
      const brokenRubric = evaluateGuiSurfaceRubric(brokenDom, crispPixels);
      expect(cleanRubric.technicalQuality - brokenRubric.technicalQuality).toBeGreaterThanOrEqual(0.299);
      expect(cleanRubric.promptAdherence - brokenRubric.promptAdherence).toBeGreaterThanOrEqual(0.299);
      expect(cleanRubric.styleConsistency - brokenRubric.styleConsistency).toBeGreaterThanOrEqual(0.299);

      const verdict = runUniversalCvFlywheelPass({
        surfaceId: 'console',
        passNumber: 1,
        rawPrompt: 'Verify Dual-Surface terminal invariants',
        beforeMetrics: { ...crispPixels, rubricScore: 0.62, vlmRubric: brokenRubric },
        afterMetrics: { ...crispPixels, rubricScore: 0.95, vlmRubric: brokenRubric },
        afterDom: brokenDom,
      });
      expect(verdict.visualAfterScore).toBeLessThan(0.92);
      expect(verdict.nextBootstrapDirectives.some((d) => d.toLowerCase().includes('terminal') || d.toLowerCase().includes('xterm') || d.toLowerCase().includes('ansi'))).toBe(true);
    }
  });

  it('serializes richMockInitScript cleanly and registers all 8 TERMINAL_CV_STATES', async () => {
    expect(TERMINAL_CV_STATES.map((s) => s.name)).toEqual([
      'block-stream-ansi',
      'split-panes',
      'history-search-open',
      'json-ledger-table',
      'runbooks-open',
      'approval-card-pending',
      'auto-tui-alt-screen',
      'chat-terminal-dock',
    ]);
    expect(SURFACE_STATES['console']?.map((s) => s.name)).toEqual([
      'block-stream-ansi',
      'split-panes',
      'history-search-open',
      'json-ledger-table',
      'runbooks-open',
      'approval-card-pending',
      'auto-tui-alt-screen',
    ]);
    expect(SURFACE_STATES['chat']?.some((s) => s.name === 'chat-terminal-dock')).toBe(true);

    const w = window as any;
    w.__MOCK_TERM_APPROVAL_ACTIVE__ = true;
    w.__MOCK_CHAT_TERM_DOCK__ = true;
    w.__MOCK_TERM_ALT_SCREEN__ = true;
    new Function(richMockInitScript('console'))();

    const invoke = w.__TAURI_INTERNALS__.invoke as (cmd: string, args?: any) => Promise<any>;
    const list = await invoke('term_session_list');
    expect(list).toHaveLength(2);
    expect(list[0].alt_screen).toBe(true);

    const attached = await invoke('term_session_attach', { sessionId: 's-1' });
    expect(attached.blocks).toHaveLength(5);
    expect(attached.blocks[0].output[0].text).toContain('\x1b[30m');
    expect(attached.blocks[0].output[0].text).toContain('crates/vox-gui/src/main.rs:42:5');

    const history = await invoke('term_history_query', { query: 'audit' });
    expect(history[0].command).toBe('vox audit code');

    const runbooks = await invoke('term_runbooks_list');
    expect(runbooks.map((r: any) => r.id)).toEqual(['rb-verify-terminal', 'rb-check-tokens']);

    const approvals = await invoke('invoke_mcp_tool', { tool: 'vox_pending_approvals' });
    expect(approvals.result.data.approvals[0].approval_id).toBe('apr-term-446');

    const chatMsgs = await invoke('chat_get_messages');
    const lastEvents = chatMsgs[chatMsgs.length - 1].events;
    expect(lastEvents.some((e: any) => e.tool === 'vox_terminal_exec')).toBe(true);
  });
});
