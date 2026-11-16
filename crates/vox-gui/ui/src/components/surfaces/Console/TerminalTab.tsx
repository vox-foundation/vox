import '@xterm/xterm/css/xterm.css';
import React, { useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import {
  termSessionOpen, termSessionAttach, termSessionInput, termSessionSubmit,
  termSessionResize, listenTermOutput, listenTermEvent, type TermBlockDto,
} from '../../../transport';
import {
  createInitialSessionPaneState, hydrateSessionFromAttachment, reduceSessionEvent,
  scanAltScreenTransition, type SessionPaneState,
} from './sessionReducer';
import { sgrToSpans } from './sgrToSpans';
import { TERMINAL_MONO_STYLE, XTERM_FONT_FAMILY, resolveTerminalTheme } from './terminalTheme';
import { WebglTerminalAddon } from './webglAddon';
import {
  SearchTerminalAddon, SerializeTerminalAddon, Unicode11TerminalAddon,
  WebLinksTerminalAddon, type SearchMatchStats,
} from './xtermAddons';

export interface PendingLine { text: string; seq: number; }

export interface ReplaySnapshotPayload {
  text: string;
  epoch: number;
  seq?: number;
  append?: boolean;
}

export interface TerminalTabProps {
  tabId?: string;
  sessionId?: string | null;
  pendingLine?: PendingLine | null;
  onSessionResolved?: (sessionId: string) => void;
  onBlocksChange?: (blocks: TermBlockDto[]) => void;
  onBlock?: (block: TermBlockDto) => void;
  onOutputSeq?: (seq: number) => void;
  onUserInput?: () => void;
  active?: boolean;
  suppressHelperTextarea?: boolean;
  skipInitialAttach?: boolean;
  replaySnapshot?: ReplaySnapshotPayload | null;
  searchQuery?: string;
  searchDirectionStep?: { seq: number; dir: 'next' | 'prev' } | null;
  onSearchStats?: (stats: SearchMatchStats) => void;
  onAltScreenHint?: (active: boolean) => void;
  onWebglFallback?: () => void;
  className?: string;
}

const safeRun = (fn: () => void): void => {
  try { fn(); } catch { /* headless / mock fallback */ }
};

function syncHelperTextarea(host: HTMLElement | null, suppress: boolean | undefined): void {
  if (!host || suppress === undefined) return;
  const textarea = host.querySelector<HTMLTextAreaElement>('.xterm-helper-textarea');
  if (!textarea) return;
  if (suppress) {
    textarea.tabIndex = -1;
    textarea.setAttribute('aria-hidden', 'true');
  } else {
    textarea.tabIndex = 0;
    textarea.removeAttribute('aria-hidden');
  }
}

export function TerminalTab({
  tabId = 'tab-default', sessionId, pendingLine = null,
  onSessionResolved, onBlocksChange, onBlock, onOutputSeq, onUserInput, active, suppressHelperTextarea,
  skipInitialAttach = false, replaySnapshot = null, searchQuery,
  searchDirectionStep = null, onSearchStats, onAltScreenHint, onWebglFallback, className,
}: TerminalTabProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const searchAddonRef = useRef<SearchTerminalAddon | null>(null);
  const activeSessionRef = useRef<string | null>(sessionId ?? null);
  const paneStateRef = useRef<SessionPaneState>(createInitialSessionPaneState({ sessionId: sessionId ?? null }));
  const callbacksRef = useRef({
    onBlock, onBlocksChange, onSessionResolved, onOutputSeq, onUserInput,
    onSearchStats, onAltScreenHint, onWebglFallback, suppressHelperTextarea,
  });
  callbacksRef.current = {
    onBlock, onBlocksChange, onSessionResolved, onOutputSeq, onUserInput,
    onSearchStats, onAltScreenHint, onWebglFallback, suppressHelperTextarea,
  };

  const lastReplayEpochRef = useRef<number | null>(null);
  const lastSearchStepSeqRef = useRef<number | null>(null);
  const [mockReplay, setMockReplay] = useState<string>('');

  const forceRealXterm: boolean =
    typeof window !== 'undefined' &&
    Boolean((window as unknown as { __VOX_E2E_REAL_XTERM__?: boolean }).__VOX_E2E_REAL_XTERM__);
  const isMock: boolean =
    !forceRealXterm &&
    typeof window !== 'undefined' &&
    Boolean((window as unknown as { __TAURI_CALLS__?: unknown }).__TAURI_CALLS__);

  useEffect(() => {
    let term: Terminal | null = null;
    let fit: FitAddon | null = null;
    let webgl: WebglTerminalAddon | null = null;
    let searchAddon: SearchTerminalAddon | null = null;
    let webLinksAddon: WebLinksTerminalAddon | null = null;
    let unicode11Addon: Unicode11TerminalAddon | null = null;
    let serializeAddon: SerializeTerminalAddon | null = null;

    let resizeObs: ResizeObserver | null = null;
    let themeObs: MutationObserver | null = null;
    let fitRafId: number | null = null;

    if (!isMock) {
      try {
        let fallbackNotified = false;
        const notifyFallbackOnce = () => {
          if (fallbackNotified) return;
          fallbackNotified = true;
          callbacksRef.current.onWebglFallback?.();
        };

        term = new Terminal({
          allowProposedApi: true, convertEol: true, cursorBlink: false,
          fontFamily: XTERM_FONT_FAMILY, fontSize: 12, lineHeight: 1.2, letterSpacing: 0,
          theme: resolveTerminalTheme(),
        });
        fit = new FitAddon();
        webgl = new WebglTerminalAddon({
          onRendererModeChange: (mode) => {
            if (mode === 'dom') notifyFallbackOnce();
          },
        });
        searchAddon = new SearchTerminalAddon();
        webLinksAddon = new WebLinksTerminalAddon();
        unicode11Addon = new Unicode11TerminalAddon();
        serializeAddon = new SerializeTerminalAddon();

        const t = term;
        const f = fit;
        safeRun(() => t.loadAddon(f));
        if (hostRef.current) {
          const host = hostRef.current;
          safeRun(() => t.open(host));
          syncHelperTextarea(host, callbacksRef.current.suppressHelperTextarea);
          if (typeof ResizeObserver !== 'undefined') {
            resizeObs = new ResizeObserver(() => {
              if (fitRafId !== null && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(fitRafId);
              const runFit = () => {
                fitRafId = null;
                if (host.clientWidth > 0 && host.clientHeight > 0) safeRun(() => f.fit());
              };
              if (typeof requestAnimationFrame === 'function') fitRafId = requestAnimationFrame(runFit);
              else runFit();
            });
            resizeObs.observe(host);
          }
        }
        if (typeof document !== 'undefined' && document.documentElement && typeof MutationObserver !== 'undefined') {
          themeObs = new MutationObserver(() => {
            safeRun(() => {
              const nextTheme = resolveTerminalTheme();
              if (t.options) t.options.theme = nextTheme;
              else (t as unknown as { options: { theme: unknown } }).options = { theme: nextTheme };
            });
          });
          themeObs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'class'] });
        }
        for (const addon of [webgl, searchAddon, webLinksAddon, unicode11Addon, serializeAddon]) {
          safeRun(() => t.loadAddon(addon));
        }
        const w = webgl;
        webgl.onContextLoss(() => {
          safeRun(() => w.dispose());
          notifyFallbackOnce();
        });
        safeRun(() => f.fit());

        termRef.current = term;
        fitRef.current = fit;
        searchAddonRef.current = searchAddon;

        term.onData?.((d) => {
          callbacksRef.current.onUserInput?.();
          const sid = activeSessionRef.current;
          if (sid) termSessionInput(sid, d).catch(() => {});
        });
        (term as unknown as { onResize?: (cb: (s: { cols: number; rows: number }) => void) => void }).onResize?.(
          ({ cols, rows }) => {
            const sid = activeSessionRef.current;
            if (sid) termSessionResize(sid, cols, rows).catch(() => {});
          },
        );
      } catch {
        term = null;
      }
    }

    const cleanupInstance = () => {
      if (fitRafId !== null && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(fitRafId);
      resizeObs?.disconnect();
      themeObs?.disconnect();
      for (const d of [serializeAddon, unicode11Addon, webLinksAddon, searchAddon, webgl]) {
        safeRun(() => d?.dispose?.());
      }
      safeRun(() => term?.dispose?.());
      termRef.current = null;
      fitRef.current = null;
      searchAddonRef.current = null;
    };

    if (sessionId === null) {
      activeSessionRef.current = null;
      return cleanupInstance;
    }

    let disposed = false;
    let unOut: (() => void) | undefined;
    let unEv: (() => void) | undefined;

    const writeOut = (text: string) => {
      if (term) term.write(text);
      else setMockReplay((prev) => prev + text);
    };

    const attachStreamListeners = async (sid: string) => {
      const u1 = await listenTermOutput((payload) => {
        if (payload.session_id !== sid) return;
        if (typeof payload.seq === 'number') {
          if (payload.seq <= paneStateRef.current.seq) return;
          paneStateRef.current.seq = payload.seq;
          callbacksRef.current.onOutputSeq?.(payload.seq);
        }
        const transition = scanAltScreenTransition(payload.data);
        if (transition !== null) callbacksRef.current.onAltScreenHint?.(transition);
        writeOut(payload.data);
      });
      if (disposed) u1();
      else unOut = u1;
    };

    if (skipInitialAttach) {
      activeSessionRef.current = sessionId ?? null;
      if (sessionId) attachStreamListeners(sessionId).catch(() => {});
      return () => { disposed = true; unOut?.(); cleanupInstance(); };
    }

    (async () => {
      try {
        const sid = sessionId || (await termSessionOpen(term?.cols || 80, term?.rows || 24));
        if (disposed) return;
        activeSessionRef.current = sid;
        callbacksRef.current.onSessionResolved?.(sid);

        const snap = await termSessionAttach(sid);
        if (disposed || !snap) return;

        const hydrated = hydrateSessionFromAttachment(createInitialSessionPaneState({ sessionId: sid }), snap);
        paneStateRef.current = hydrated;
        if (snap.alt_screen === true) callbacksRef.current.onAltScreenHint?.(true);
        if (snap.replay_utf8) {
          if (term) term.write(snap.replay_utf8);
          else setMockReplay(snap.replay_utf8);
        }
        callbacksRef.current.onBlocksChange?.(hydrated.blocks);
        const completed = [...hydrated.blocks].reverse().find((b) => b.status !== 'Running');
        if (completed) callbacksRef.current.onBlock?.(completed);

        await attachStreamListeners(sid);
        if (disposed) return;

        const u2 = await listenTermEvent(async (payload) => {
          if (payload.session_id !== sid) return;
          const next = reduceSessionEvent(paneStateRef.current, payload.event);
          paneStateRef.current = next;
          callbacksRef.current.onBlocksChange?.(next.blocks);

          if (payload.event && typeof payload.event === 'object') {
            if ('AltScreenChanged' in payload.event) callbacksRef.current.onAltScreenHint?.(next.altScreen);
            if ('BlockClosed' in payload.event) {
              const closedId = Number((payload.event as { BlockClosed?: { id?: unknown } }).BlockClosed?.id);
              const closedBlock = next.blocks.find((b) => b.id === closedId);
              if (closedBlock) callbacksRef.current.onBlock?.(closedBlock);
            }
          }

          if (next.resyncRequired) {
            paneStateRef.current = { ...next, resyncRequired: false };
            safeRun(() => term?.reset?.());
            if (isMock) setMockReplay('');
          }
        });
        if (disposed) u2();
        else unEv = u2;
      } catch { /* session attach failed in headless mode */ }
    })();

    return () => { disposed = true; unOut?.(); unEv?.(); cleanupInstance(); };
  }, [tabId, sessionId, skipInitialAttach, isMock]);

  useEffect(() => {
    syncHelperTextarea(hostRef.current, suppressHelperTextarea);
    if (active) {
      safeRun(() => fitRef.current?.fit?.());
      safeRun(() => termRef.current?.focus?.());
    }
  }, [suppressHelperTextarea, active]);

  useEffect(() => {
    if (!replaySnapshot || replaySnapshot.epoch === lastReplayEpochRef.current) return;
    lastReplayEpochRef.current = replaySnapshot.epoch;
    if (typeof replaySnapshot.seq === 'number') {
      paneStateRef.current.seq = Math.max(paneStateRef.current.seq, replaySnapshot.seq);
    }
    const isAppend = replaySnapshot.append === true;
    if (!isAppend) {
      safeRun(() => termRef.current?.reset?.());
      if (isMock) setMockReplay(replaySnapshot.text);
    } else if (isMock && replaySnapshot.text) {
      setMockReplay((prev) => prev + replaySnapshot.text);
    }
    if (termRef.current && replaySnapshot.text) {
      const t = termRef.current;
      t.write(replaySnapshot.text);
      if (typeof requestAnimationFrame === 'function') {
        requestAnimationFrame(() => {
          safeRun(() => t.refresh?.(0, Math.max(0, (t.rows || 24) - 1)));
        });
      }
    }
  }, [replaySnapshot, isMock]);

  useEffect(() => {
    if (searchQuery === undefined || !searchAddonRef.current) return;
    if (!searchQuery) {
      searchAddonRef.current.clear();
      callbacksRef.current.onSearchStats?.({ activeMatchIndex: 0, totalMatches: 0 });
      return;
    }
    callbacksRef.current.onSearchStats?.(searchAddonRef.current.findNext(searchQuery));
  }, [searchQuery]);

  useEffect(() => {
    if (!searchDirectionStep || searchDirectionStep.seq === lastSearchStepSeqRef.current) return;
    lastSearchStepSeqRef.current = searchDirectionStep.seq;
    if (!searchQuery || !searchAddonRef.current) return;
    const stats = searchDirectionStep.dir === 'prev'
      ? searchAddonRef.current.findPrevious(searchQuery)
      : searchAddonRef.current.findNext(searchQuery);
    callbacksRef.current.onSearchStats?.(stats);
  }, [searchDirectionStep, searchQuery]);

  useEffect(() => {
    const sid = activeSessionRef.current;
    if (pendingLine && sid) termSessionSubmit(sid, pendingLine.text).catch(() => {});
  }, [pendingLine]);

  if (isMock) {
    return (
      <div
        role="application" aria-label="terminal" data-testid="terminal-viewport"
        tabIndex={suppressHelperTextarea ? undefined : 0}
        data-terminal-well="true" data-terminal-mono="true"
        className={`box-border h-full w-full ${suppressHelperTextarea ? 'overflow-hidden' : 'overflow-y-auto'} rounded border border-(--terminal-well-border) bg-(--terminal-well-bg) p-3 text-xs leading-[1.45] text-(--terminal-well-fg) ${className ?? ''}`}
        style={TERMINAL_MONO_STYLE}
      >
        <div className="mb-2 flex flex-wrap items-center justify-between gap-2 border-b border-(--terminal-well-border) pb-1 text-[11px] font-semibold text-(--terminal-well-cursor)">
          <span>VOX TERMINAL SESSION ({activeSessionRef.current ?? sessionId ?? 's-1'})</span>
          <span className="rounded border border-(--terminal-well-border) bg-(--terminal-well-selection) px-2 py-1 text-[11px] text-(--terminal-well-fg)">
            WebGL Block Renderer - OSC-633 SessionHost Active
          </span>
        </div>
        {mockReplay ? (
          <pre data-terminal-mono="true" className="m-0 whitespace-pre-wrap" style={TERMINAL_MONO_STYLE}>
            {sgrToSpans(mockReplay)}
          </pre>
        ) : (
          <div className="space-y-1">
            <div className="text-(--terminal-well-fg)">~/workspace/vox $ vox status --json</div>
            <div className="text-(--terminal-well-muted)">
              {'  branch: main - commit: 98ea5d2 - working_tree: clean - clavis: verified'}
            </div>
            <div className="pt-1 text-(--terminal-well-fg)">~/workspace/vox $ vox run</div>
            <div className="font-semibold text-(--terminal-well-cursor)">
              [OK] Ready - SessionHost attached to local orchestrator daemon.
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div
      ref={hostRef} role="application" aria-label="terminal" data-testid="terminal-viewport"
      data-terminal-well="true" data-terminal-mono="true"
      className={`rounded border border-(--terminal-well-border) bg-(--terminal-well-bg) text-(--terminal-well-fg) ${className ?? ''}`.trim()}
      style={{ height: '100%', width: '100%', ...TERMINAL_MONO_STYLE }}
    />
  );
}
